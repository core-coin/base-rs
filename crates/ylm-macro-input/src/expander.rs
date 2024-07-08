use crate::YlmInput;
use proc_macro2::TokenStream;

/// Expands a `YlmInput` into a `TokenStream`.
pub trait YlmInputExpander {
    /// Expand a `YlmInput` into a `TokenStream`.
    fn expand(&mut self, input: &YlmInput) -> syn::Result<TokenStream>;
}
