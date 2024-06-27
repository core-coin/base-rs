//! [`ItemStruct`] expansion.

use super::{expand_fields, expand_from_into_tuples, expand_tokenize, expand_type, ExpCtxt};
use ast::{Item, ItemStruct, Spanned, Type};
use base_ylm_macro_input::{mk_doc, ContainsYlmAttrs};
use proc_macro2::TokenStream;
use quote::quote;
use std::num::NonZeroU16;
use syn::Result;

/// Expands an [`ItemStruct`]:
///
/// ```ignore (pseudo-code)
/// pub struct #name {
///     #(pub #field_name: #field_type,)*
/// }
///
/// impl YlmStruct for #name {
///     ...
/// }
///
/// // Needed to use in event parameters
/// impl EventTopic for #name {
///     ...
/// }
/// ```
pub(super) fn expand(cx: &ExpCtxt<'_>, s: &ItemStruct) -> Result<TokenStream> {
    let ItemStruct { name, fields, .. } = s;

    let (ylm_attrs, mut attrs) = s.split_attrs()?;

    cx.derives(&mut attrs, fields, true);
    let docs = ylm_attrs.docs.or(cx.attrs.docs).unwrap_or(true);

    let (field_types, field_names): (Vec<_>, Vec<_>) =
        fields.iter().map(|f| (expand_type(&f.ty, &cx.crates), f.name.as_ref().unwrap())).unzip();

    let eip712_encode_type_fns = expand_encode_type_fns(cx, fields, name);

    let tokenize_impl = expand_tokenize(fields, cx);

    let encode_data_impl = match fields.len() {
        0 => unreachable!("struct with zero fields"),
        1 => {
            let name = *field_names.first().unwrap();
            let ty = field_types.first().unwrap();
            quote!(<#ty as base_ylm_types::YlmType>::eip712_data_word(&self.#name).0.to_vec())
        }
        _ => quote! {
            [#(
                <#field_types as base_ylm_types::YlmType>::eip712_data_word(&self.#field_names).0,
            )*].concat()
        },
    };

    let base_ylm_types = &cx.crates.ylm_types;

    let attrs = attrs.iter();
    let convert = expand_from_into_tuples(&name.0, fields, cx);
    let name_s = name.as_string();
    let fields = expand_fields(fields, cx);

    let doc = docs.then(|| mk_doc(format!("```solidity\n{s}\n```")));
    let tokens = quote! {
        #(#attrs)*
        #doc
        #[allow(non_camel_case_types, non_snake_case)]
        #[derive(Clone)]
        pub struct #name {
            #(#fields),*
        }

        #[allow(non_camel_case_types, non_snake_case, clippy::style)]
        const _: () = {
            use #base_ylm_types as base_ylm_types;

            #convert

            #[automatically_derived]
            impl base_ylm_types::YlmValue for #name {
                type YlmType = Self;
            }

            #[automatically_derived]
            impl base_ylm_types::private::YlmTypeValue<Self> for #name {
                #[inline]
                fn stv_to_tokens(&self) -> <Self as base_ylm_types::YlmType>::Token<'_> {
                    #tokenize_impl
                }

                #[inline]
                fn stv_abi_encoded_size(&self) -> usize {
                    // TODO: Avoid cloning
                    let tuple = <UnderlyingRustTuple<'_> as ::core::convert::From<Self>>::from(self.clone());
                    <UnderlyingSolTuple<'_> as base_ylm_types::YlmType>::abi_encoded_size(&tuple)
                }

                #[inline]
                fn stv_eip712_data_word(&self) -> base_ylm_types::Word {
                    <Self as base_ylm_types::YlmStruct>::eip712_hash_struct(self)
                }

                #[inline]
                fn stv_abi_encode_packed_to(&self, out: &mut base_ylm_types::private::Vec<u8>) {
                    // TODO: Avoid cloning
                    let tuple = <UnderlyingRustTuple<'_> as ::core::convert::From<Self>>::from(self.clone());
                    <UnderlyingSolTuple<'_> as base_ylm_types::YlmType>::abi_encode_packed_to(&tuple, out)
                }
            }

            #[automatically_derived]
            impl base_ylm_types::YlmType for #name {
                type RustType = Self;
                type Token<'a> = <UnderlyingSolTuple<'a> as base_ylm_types::YlmType>::Token<'a>;

                const YLM_NAME: &'static str = <Self as base_ylm_types::YlmStruct>::NAME;
                const ENCODED_SIZE: Option<usize> =
                    <UnderlyingSolTuple<'_> as base_ylm_types::YlmType>::ENCODED_SIZE;

                #[inline]
                fn valid_token(token: &Self::Token<'_>) -> bool {
                    <UnderlyingSolTuple<'_> as base_ylm_types::YlmType>::valid_token(token)
                }

                #[inline]
                fn detokenize(token: Self::Token<'_>) -> Self::RustType {
                    let tuple = <UnderlyingSolTuple<'_> as base_ylm_types::YlmType>::detokenize(token);
                    <Self as ::core::convert::From<UnderlyingRustTuple<'_>>>::from(tuple)
                }
            }

            #[automatically_derived]
            impl base_ylm_types::YlmStruct for #name {
                const NAME: &'static str = #name_s;

                #eip712_encode_type_fns

                #[inline]
                fn eip712_encode_data(&self) -> base_ylm_types::private::Vec<u8> {
                    #encode_data_impl
                }
            }

            #[automatically_derived]
            impl base_ylm_types::EventTopic for #name {
                #[inline]
                fn topic_preimage_length(rust: &Self::RustType) -> usize {
                    0usize
                    #(
                        + <#field_types as base_ylm_types::EventTopic>::topic_preimage_length(&rust.#field_names)
                    )*
                }

                #[inline]
                fn encode_topic_preimage(rust: &Self::RustType, out: &mut base_ylm_types::private::Vec<u8>) {
                    out.reserve(<Self as base_ylm_types::EventTopic>::topic_preimage_length(rust));
                    #(
                        <#field_types as base_ylm_types::EventTopic>::encode_topic_preimage(&rust.#field_names, out);
                    )*
                }

                #[inline]
                fn encode_topic(rust: &Self::RustType) -> base_ylm_types::abi::token::WordToken {
                    let mut out = base_ylm_types::private::Vec::new();
                    <Self as base_ylm_types::EventTopic>::encode_topic_preimage(rust, &mut out);
                    base_ylm_types::abi::token::WordToken(
                        base_ylm_types::private::sha3(out)
                    )
                }
            }
        };
    };
    Ok(tokens)
}

fn expand_encode_type_fns(
    cx: &ExpCtxt<'_>,
    fields: &ast::Parameters<syn::token::Semi>,
    name: &ast::YlmIdent,
) -> TokenStream {
    // account for UDVTs and enums which do not implement YlmStruct
    let mut fields = fields.clone();
    fields.visit_types_mut(|ty| {
        let Type::Custom(name) = ty else { return };
        match cx.try_item(name) {
            // keep as custom
            Some(Item::Struct(_)) | None => {}
            // convert to underlying
            Some(Item::Contract(_)) => *ty = Type::Address(ty.span(), None),
            Some(Item::Enum(_)) => *ty = Type::Uint(ty.span(), NonZeroU16::new(8)),
            Some(Item::Udt(udt)) => *ty = udt.ty.clone(),
            Some(item) => abort!(item.span(), "Invalid type in struct field: {:?}", item),
        }
    });

    let root = fields.eip712_signature(name.as_string());

    let custom = fields.iter().filter(|f| f.ty.has_custom());
    let n_custom = custom.clone().count();

    let components_impl = if n_custom > 0 {
        let bits = custom.map(|field| {
            // need to recurse to find the inner custom type
            let mut ty = None;
            field.ty.visit(|field_ty| {
                if ty.is_none() && field_ty.is_custom() {
                    ty = Some(field_ty.clone());
                }
            });
            // cannot panic as this field is guaranteed to contain a custom type
            let ty = expand_type(&ty.unwrap(), &cx.crates);

            quote! {
                components.push(<#ty as base_ylm_types::YlmStruct>::eip712_root_type());
                components.extend(<#ty as base_ylm_types::YlmStruct>::eip712_components());
            }
        });
        let capacity = proc_macro2::Literal::usize_unsuffixed(n_custom);
        quote! {
            let mut components = base_ylm_types::private::Vec::with_capacity(#capacity);
            #(#bits)*
            components
        }
    } else {
        quote! { base_ylm_types::private::Vec::new() }
    };

    let encode_type_impl_opt = (n_custom == 0).then(|| {
        quote! {
            #[inline]
            fn eip712_encode_type() -> base_ylm_types::private::Cow<'static, str> {
                <Self as base_ylm_types::YlmStruct>::eip712_root_type()
            }
        }
    });

    quote! {
        #[inline]
        fn eip712_root_type() -> base_ylm_types::private::Cow<'static, str> {
            base_ylm_types::private::Cow::Borrowed(#root)
        }

        #[inline]
        fn eip712_components() -> base_ylm_types::private::Vec<base_ylm_types::private::Cow<'static, str>> {
            #components_impl
        }

        #encode_type_impl_opt
    }
}
