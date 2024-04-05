use ast::Spanned;
use proc_macro2::{Span, TokenStream};
use quote::ToTokens;
use tiny_keccak::{Hasher, Sha3};

/// Simple interface to the [`sha3`] hash function.
///
/// [`sha3`]: https://en.wikipedia.org/wiki/SHA-3
pub fn sha3<T: AsRef<[u8]>>(bytes: T) -> [u8; 32] {
    let mut output = [0u8; 32];
    let mut hasher = Sha3::v256();
    hasher.update(bytes.as_ref());
    hasher.finalize(&mut output);
    output
}

pub fn selector<T: AsRef<[u8]>>(bytes: T) -> ExprArray<u8> {
    ExprArray::new(sha3(bytes)[..4].to_vec())
}

pub fn event_selector<T: AsRef<[u8]>>(bytes: T) -> ExprArray<u8> {
    ExprArray::new(sha3(bytes).to_vec())
}

pub fn combine_errors(v: impl IntoIterator<Item = syn::Error>) -> syn::Result<()> {
    match v.into_iter().reduce(|mut a, b| {
        a.combine(b);
        a
    }) {
        Some(e) => Err(e),
        None => Ok(()),
    }
}

#[derive(Clone, Debug)]
pub struct ExprArray<T> {
    pub array: Vec<T>,
    pub span: Span,
}

impl<T: PartialOrd> PartialOrd for ExprArray<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.array.partial_cmp(&other.array)
    }
}

impl<T: Ord> Ord for ExprArray<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.array.cmp(&other.array)
    }
}

impl<T: PartialEq> PartialEq for ExprArray<T> {
    fn eq(&self, other: &Self) -> bool {
        self.array == other.array
    }
}

impl<T: Eq> Eq for ExprArray<T> {}

impl<T> Spanned for ExprArray<T> {
    fn span(&self) -> Span {
        self.span
    }

    fn set_span(&mut self, span: Span) {
        self.span = span;
    }
}

impl<T> ExprArray<T> {
    fn new(array: Vec<T>) -> Self {
        Self { array, span: Span::call_site() }
    }
}

impl<T: ToTokens> ToTokens for ExprArray<T> {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        syn::token::Bracket(self.span).surround(tokens, |tokens| {
            for t in &self.array {
                t.to_tokens(tokens);
                syn::token::Comma(self.span).to_tokens(tokens);
            }
        });
    }
}
