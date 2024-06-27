//! [`ItemError`] expansion.

use super::{expand_fields, expand_from_into_tuples, expand_tokenize, ExpCtxt};
use ast::ItemError;
use base_ylm_macro_input::{mk_doc, ContainsYlmAttrs};
use proc_macro2::TokenStream;
use quote::quote;
use syn::Result;

/// Expands an [`ItemError`]:
///
/// ```ignore (pseudo-code)
/// pub struct #name {
///     #(pub #parameter_name: #parameter_type,)*
/// }
///
/// impl YlmError for #name {
///     ...
/// }
/// ```
pub(super) fn expand(cx: &ExpCtxt<'_>, error: &ItemError) -> Result<TokenStream> {
    let ItemError { parameters: params, name, .. } = error;
    cx.assert_resolved(params)?;

    let (ylm_attrs, mut attrs) = error.split_attrs()?;
    cx.derives(&mut attrs, params, true);
    let docs = ylm_attrs.docs.or(cx.attrs.docs).unwrap_or(true);
    let abi = ylm_attrs.abi.or(cx.attrs.abi).unwrap_or(false);

    let tokenize_impl = expand_tokenize(params, cx);

    let signature = cx.error_signature(error);
    let selector = crate::utils::selector(&signature);

    let base_ylm_types = &cx.crates.ylm_types;

    let converts = expand_from_into_tuples(&name.0, params, cx);
    let fields = expand_fields(params, cx);
    let doc = docs.then(|| {
        let selector = hex::encode_prefixed(selector.array.as_slice());
        mk_doc(format!(
            "Custom error with signature `{signature}` and selector `{selector}`.\n\
             ```solidity\n{error}\n```"
        ))
    });
    let abi: Option<TokenStream> = abi.then(|| {
        if_json! {
            let error = super::to_abi::generate(error, cx);
            quote! {
                #[automatically_derived]
                impl base_ylm_types::JsonAbiExt for #name {
                    type Abi = base_ylm_types::private::base_json_abi::Error;

                    #[inline]
                    fn abi() -> Self::Abi {
                        #error
                    }
                }
            }
        }
    });
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

            #converts

            #[automatically_derived]
            impl base_ylm_types::YlmError for #name {
                type Parameters<'a> = UnderlyingSolTuple<'a>;
                type Token<'a> = <Self::Parameters<'a> as base_ylm_types::YlmType>::Token<'a>;

                const SIGNATURE: &'static str = #signature;
                const SELECTOR: [u8; 4] = #selector;

                #[inline]
                fn new<'a>(tuple: <Self::Parameters<'a> as base_ylm_types::YlmType>::RustType) -> Self {
                    tuple.into()
                }

                #[inline]
                fn tokenize(&self) -> Self::Token<'_> {
                    #tokenize_impl
                }
            }

            #abi
        };
    };
    Ok(tokens)
}
