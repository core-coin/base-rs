//! [`ItemFunction`] expansion.

use super::{expand_fields, expand_from_into_tuples, expand_tokenize, expand_tuple_types, ExpCtxt};
use ast::{FunctionKind, ItemFunction, Spanned};
use base_ylm_macro_input::{mk_doc, ContainsYlmAttrs};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::Result;

/// Expands an [`ItemFunction`]:
///
/// ```ignore (pseudo-code)
/// pub struct #{name}Call {
///     #(pub #argument_name: #argument_type,)*
/// }
///
/// pub struct #{name}Return {
///     #(pub #return_name: #return_type,)*
/// }
///
/// impl YlmCall for #{name}Call {
///     type Return = #{name}Return;
///     ...
/// }
/// ```
pub(super) fn expand(cx: &ExpCtxt<'_>, function: &ItemFunction) -> Result<TokenStream> {
    let ItemFunction { parameters, returns, name, kind, .. } = function;

    if matches!(kind, FunctionKind::Constructor(_)) {
        return expand_constructor(cx, function);
    }

    if name.is_none() {
        // ignore functions without names (modifiers...)
        return Ok(quote!());
    };

    let returns = returns.as_ref().map(|r| &r.returns).unwrap_or_default();

    cx.assert_resolved(parameters)?;
    if !returns.is_empty() {
        cx.assert_resolved(returns)?;
    }

    let (ylm_attrs, mut call_attrs) = function.split_attrs()?;
    let mut return_attrs = call_attrs.clone();
    cx.derives(&mut call_attrs, parameters, true);
    if !returns.is_empty() {
        cx.derives(&mut return_attrs, returns, true);
    }
    let docs = ylm_attrs.docs.or(cx.attrs.docs).unwrap_or(true);
    let abi = ylm_attrs.abi.or(cx.attrs.abi).unwrap_or(false);

    let call_name = cx.call_name(function);
    let return_name = cx.return_name(function);

    let call_fields = expand_fields(parameters, cx);
    let return_fields = expand_fields(returns, cx);

    let call_tuple = expand_tuple_types(parameters.types(), cx).0;
    let return_tuple = expand_tuple_types(returns.types(), cx).0;

    let converts = expand_from_into_tuples(&call_name, parameters, cx);
    let return_converts = expand_from_into_tuples(&return_name, returns, cx);

    let signature = cx.function_signature(function);
    let selector = crate::utils::selector(&signature);
    let tokenize_impl = expand_tokenize(parameters, cx);

    let call_doc = docs.then(|| {
        let selector = hex::encode_prefixed(selector.array.as_slice());
        mk_doc(format!(
            "Function with signature `{signature}` and selector `{selector}`.\n\
            ```solidity\n{function}\n```"
        ))
    });
    let return_doc = docs.then(|| {
        mk_doc(format!(
            "Container type for the return parameters of the [`{signature}`]({call_name}) function."
        ))
    });

    let abi: Option<TokenStream> = abi.then(|| {
        if_json! {
            let function = super::to_abi::generate(function, cx);
            quote! {
                #[automatically_derived]
                impl base_ylm_types::JsonAbiExt for #call_name {
                    type Abi = base_ylm_types::private::base_json_abi::Function;

                    #[inline]
                    fn abi() -> Self::Abi {
                        #function
                    }
                }
            }
        }
    });

    let base_ylm_types = &cx.crates.ylm_types;

    let tokens = quote! {
        #(#call_attrs)*
        #call_doc
        #[allow(non_camel_case_types, non_snake_case)]
        #[derive(Clone)]
        pub struct #call_name {
            #(#call_fields),*
        }

        #(#return_attrs)*
        #return_doc
        #[allow(non_camel_case_types, non_snake_case)]
        #[derive(Clone)]
        pub struct #return_name {
            #(#return_fields),*
        }

        #[allow(non_camel_case_types, non_snake_case, clippy::style)]
        const _: () = {
            use #base_ylm_types as base_ylm_types;

            { #converts }
            { #return_converts }

            #[automatically_derived]
            impl base_ylm_types::YlmCall for #call_name {
                type Parameters<'a> = #call_tuple;
                type Token<'a> = <Self::Parameters<'a> as base_ylm_types::YlmType>::Token<'a>;

                type Return = #return_name;

                type ReturnTuple<'a> = #return_tuple;
                type ReturnToken<'a> = <Self::ReturnTuple<'a> as base_ylm_types::YlmType>::Token<'a>;

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

                #[inline]
                fn abi_decode_returns(data: &[u8], validate: bool) -> base_ylm_types::Result<Self::Return> {
                    <Self::ReturnTuple<'_> as base_ylm_types::YlmType>::abi_decode_sequence(data, validate).map(Into::into)
                }
            }

            #abi
        };
    };
    Ok(tokens)
}

fn expand_constructor(cx: &ExpCtxt<'_>, constructor: &ItemFunction) -> Result<TokenStream> {
    let ItemFunction { parameters, .. } = constructor;

    let (ylm_attrs, call_attrs) = constructor.split_attrs()?;
    let docs = ylm_attrs.docs.or(cx.attrs.docs).unwrap_or(true);

    let base_ylm_types = &cx.crates.ylm_types;

    let call_name = format_ident!("constructorCall").with_span(constructor.kind.span());
    let call_fields = expand_fields(parameters, cx);
    let call_tuple = expand_tuple_types(parameters.types(), cx).0;
    let converts = expand_from_into_tuples(&call_name, parameters, cx);
    let tokenize_impl = expand_tokenize(parameters, cx);

    let call_doc = docs.then(|| {
        mk_doc(format!(
            "Constructor`.\n\
            ```solidity\n{constructor}\n```"
        ))
    });

    let tokens = quote! {
        #(#call_attrs)*
        #call_doc
        #[allow(non_camel_case_types, non_snake_case)]
        #[derive(Clone)]
        pub struct #call_name {
            #(#call_fields),*
        }

        const _: () = {
            use #base_ylm_types as base_ylm_types;

            { #converts }

            #[automatically_derived]
            impl base_ylm_types::YlmConstructor for #call_name {
                type Parameters<'a> = #call_tuple;
                type Token<'a> = <Self::Parameters<'a> as base_ylm_types::YlmType>::Token<'a>;

                #[inline]
                fn new<'a>(tuple: <Self::Parameters<'a> as base_ylm_types::YlmType>::RustType) -> Self {
                    tuple.into()
                }

                #[inline]
                fn tokenize(&self) -> Self::Token<'_> {
                    #tokenize_impl
                }
            }
        };
    };
    Ok(tokens)
}
