//! [`ItemEvent`] expansion.

use super::{anon_name, expand_event_tokenize, expand_tuple_types, expand_type, ty, ExpCtxt};
use ast::{EventParameter, ItemEvent, Spanned, YlmIdent};
use base_ylm_macro_input::{mk_doc, ContainsYlmAttrs};
use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::Result;

/// Expands an [`ItemEvent`]:
///
/// ```ignore (pseudo-code)
/// pub struct #name {
///     #(pub #parameter_name: #parameter_type,)*
/// }
///
/// impl YlmEvent for #name {
///     ...
/// }
/// ```
pub(super) fn expand(cx: &ExpCtxt<'_>, event: &ItemEvent) -> Result<TokenStream> {
    let params = event.params();

    let (ylm_attrs, mut attrs) = event.split_attrs()?;
    cx.derives(&mut attrs, &params, true);
    let docs = ylm_attrs.docs.or(cx.attrs.docs).unwrap_or(true);
    let abi = ylm_attrs.abi.or(cx.attrs.abi).unwrap_or(false);

    cx.assert_resolved(&params)?;
    event.assert_valid()?;

    let name = cx.overloaded_name(event.into());
    let signature = cx.signature(name.as_string(), &params);
    let selector = crate::utils::event_selector(&signature);
    let anonymous = event.is_anonymous();

    // prepend the first topic if not anonymous
    let first_topic = (!anonymous).then(|| quote!(base_ylm_types::ylm_data::FixedBytes<32>));
    let topic_list = event.indexed_params().map(|p| expand_event_topic_type(p, cx));
    let topic_list = first_topic.into_iter().chain(topic_list);

    let (data_tuple, _) = expand_tuple_types(event.non_indexed_params().map(|p| &p.ty), cx);

    // skip first topic if not anonymous, which is the hash of the signature
    let mut topic_i = !anonymous as usize;
    let mut data_i = 0usize;
    let new_impl = event.parameters.iter().enumerate().map(|(i, p)| {
        let name = anon_name((i, p.name.as_ref()));
        let param;
        if p.is_indexed() {
            let i = syn::Index::from(topic_i);
            param = quote!(topics.#i);
            topic_i += 1;
        } else {
            let i = syn::Index::from(data_i);
            param = quote!(data.#i);
            data_i += 1;
        }
        quote!(#name: #param)
    });

    // NOTE: We need to enumerate before filtering.
    let topic_tuple_names = event
        .parameters
        .iter()
        .enumerate()
        .filter(|(_, p)| p.is_indexed())
        .map(|(i, p)| (i, p.name.as_ref()))
        .map(anon_name);

    let topics_impl = if anonymous {
        quote! {(#(self.#topic_tuple_names.clone(),)*)}
    } else {
        quote! {(Self::SIGNATURE_HASH.into(), #(self.#topic_tuple_names.clone(),)*)}
    };

    let encode_first_topic =
        (!anonymous).then(|| quote!(base_ylm_types::abi::token::WordToken(Self::SIGNATURE_HASH)));

    let encode_topics_impl = event.parameters.iter().enumerate().filter(|(_, p)| p.is_indexed()).map(|(i, p)| {
        let name = anon_name((i, p.name.as_ref()));
        let ty = expand_type(&p.ty, &cx.crates);

        if p.indexed_as_hash() {
            quote! {
                <base_ylm_types::ylm_data::FixedBytes<32> as base_ylm_types::EventTopic>::encode_topic(&self.#name)
            }
        } else {
            quote! {
                <#ty as base_ylm_types::EventTopic>::encode_topic(&self.#name)
            }
        }
    });

    let fields = event
        .parameters
        .iter()
        .enumerate()
        .map(|(i, p)| expand_event_topic_field(i, p, p.name.as_ref(), cx));

    let tokenize_body_impl = expand_event_tokenize(&event.parameters, cx);

    let encode_topics_impl = encode_first_topic
        .into_iter()
        .chain(encode_topics_impl)
        .enumerate()
        .map(|(i, assign)| quote!(out[#i] = #assign;));

    let doc = docs.then(|| {
        let selector = hex::encode_prefixed(selector.array.as_slice());
        mk_doc(format!(
            "Event with signature `{signature}` and selector `{selector}`.\n\
            ```solidity\n{event}\n```"
        ))
    });

    let abi: Option<TokenStream> = abi.then(|| {
        if_json! {
            let event = super::to_abi::generate(event, cx);
            quote! {
                #[automatically_derived]
                impl base_ylm_types::JsonAbiExt for #name {
                    type Abi = base_ylm_types::private::base_json_abi::Event;

                    #[inline]
                    fn abi() -> Self::Abi {
                        #event
                    }
                }
            }
        }
    });

    let base_ylm_types = &cx.crates.ylm_types;

    let tokens = quote! {
        #(#attrs)*
        #doc
        #[allow(non_camel_case_types, non_snake_case, clippy::style)]
        pub struct #name {
            #(pub #fields,)*
        }

        #[allow(non_camel_case_types, non_snake_case, clippy::style)]
        const _: () = {
            use #base_ylm_types as base_ylm_types;

            #[automatically_derived]
            impl base_ylm_types::YlmEvent for #name {
                type DataTuple<'a> = #data_tuple;
                type DataToken<'a> = <Self::DataTuple<'a> as base_ylm_types::YlmType>::Token<'a>;

                type TopicList = (#(#topic_list,)*);

                const SIGNATURE: &'static str = #signature;
                const SIGNATURE_HASH: base_ylm_types::private::B256 =
                    base_ylm_types::private::B256::new(#selector);

                const ANONYMOUS: bool = #anonymous;

                #[allow(unused_variables)]
                #[inline]
                fn new(
                    topics: <Self::TopicList as base_ylm_types::YlmType>::RustType,
                    data: <Self::DataTuple<'_> as base_ylm_types::YlmType>::RustType,
                ) -> Self {
                    Self {
                        #(#new_impl,)*
                    }
                }

                #[inline]
                fn tokenize_body(&self) -> Self::DataToken<'_> {
                    #tokenize_body_impl
                }

                #[inline]
                fn topics(&self) -> <Self::TopicList as base_ylm_types::YlmType>::RustType {
                    #topics_impl
                }

                #[inline]
                fn encode_topics_raw(
                    &self,
                    out: &mut [base_ylm_types::abi::token::WordToken],
                ) -> base_ylm_types::Result<()> {
                    if out.len() < <Self::TopicList as base_ylm_types::TopicList>::COUNT {
                        return Err(base_ylm_types::Error::Overrun);
                    }
                    #(#encode_topics_impl)*
                    Ok(())
                }
            }

            impl From<&#name> for base_ylm_types::private::LogData {
                #[inline]
                fn from(this: &#name) -> base_ylm_types::private::LogData {
                    let topics = base_ylm_types::YlmEvent::encode_topics(this).into_iter().map(|t| t.into()).collect();
                    let data = base_ylm_types::YlmEvent::encode_data(this).into();
                    base_ylm_types::private::LogData::new_unchecked(topics, data)
                }
            }

            #abi
        };
    };
    Ok(tokens)
}

fn expand_event_topic_type(param: &EventParameter, cx: &ExpCtxt<'_>) -> TokenStream {
    let base_ylm_types = &cx.crates.ylm_types;
    assert!(param.is_indexed());
    if param.is_abi_dynamic() {
        quote_spanned! {param.ty.span()=> #base_ylm_types::ylm_data::FixedBytes<32> }
    } else {
        expand_type(&param.ty, &cx.crates)
    }
}

fn expand_event_topic_field(
    i: usize,
    param: &EventParameter,
    name: Option<&YlmIdent>,
    cx: &ExpCtxt<'_>,
) -> TokenStream {
    let name = anon_name((i, name));
    let ty = if param.indexed_as_hash() {
        let bytes32 = ast::Type::FixedBytes(name.span(), core::num::NonZeroU16::new(32).unwrap());
        ty::expand_rust_type(&bytes32, &cx.crates)
    } else {
        ty::expand_rust_type(&param.ty, &cx.crates)
    };
    quote!(#name: #ty)
}
