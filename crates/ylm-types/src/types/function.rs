use crate::{
    abi::{Token, TokenSeq},
    private::YlmTypeValue,
    Result, Word, YlmType,
};
use alloc::vec::Vec;

/// A Ylem function call.
///
/// # Implementer's Guide
///
/// It should not be necessary to implement this trait manually. Instead, use
/// the [`ylm!`](crate::ylm!) procedural macro to parse Ylem syntax into
/// types that implement this trait.
pub trait YlmCall: Sized {
    /// The underlying tuple type which represents this type's arguments.
    ///
    /// If this type has no arguments, this will be the unit type `()`.
    type Parameters<'a>: YlmType<Token<'a> = Self::Token<'a>>;

    /// The arguments' corresponding [TokenSeq] type.
    type Token<'a>: TokenSeq<'a>;

    /// The function's return struct.
    type Return;

    /// The underlying tuple type which represents this type's return values.
    ///
    /// If this type has no return values, this will be the unit type `()`.
    type ReturnTuple<'a>: YlmType<Token<'a> = Self::ReturnToken<'a>>;

    /// The returns' corresponding [TokenSeq] type.
    type ReturnToken<'a>: TokenSeq<'a>;

    /// The function's ABI signature.
    const SIGNATURE: &'static str;

    /// The function selector: `sha3(SIGNATURE)[0..4]`
    const SELECTOR: [u8; 4];

    /// Convert from the tuple type used for ABI encoding and decoding.
    fn new(tuple: <Self::Parameters<'_> as YlmType>::RustType) -> Self;

    /// Tokenize the call's arguments.
    fn tokenize(&self) -> Self::Token<'_>;

    /// The size of the encoded data in bytes, **without** its selector.
    #[inline]
    fn abi_encoded_size(&self) -> usize {
        if let Some(size) = <Self::Parameters<'_> as YlmType>::ENCODED_SIZE {
            return size;
        }

        // `total_words` includes the first dynamic offset which we ignore.
        let offset = <<Self::Parameters<'_> as YlmType>::Token<'_> as Token>::DYNAMIC as usize * 32;
        (self.tokenize().total_words() * Word::len_bytes()).saturating_sub(offset)
    }

    /// ABI decode this call's arguments from the given slice, **without** its
    /// selector.
    #[inline]
    fn abi_decode_raw(data: &[u8], validate: bool) -> Result<Self> {
        <Self::Parameters<'_> as YlmType>::abi_decode_sequence(data, validate).map(Self::new)
    }

    /// ABI decode this call's arguments from the given slice, **with** the
    /// selector.
    #[inline]
    fn abi_decode(data: &[u8], validate: bool) -> Result<Self> {
        let data = data
            .strip_prefix(&Self::SELECTOR)
            .ok_or_else(|| crate::Error::type_check_fail_sig(data, Self::SIGNATURE))?;
        Self::abi_decode_raw(data, validate)
    }

    /// ABI encode the call to the given buffer **without** its selector.
    #[inline]
    fn abi_encode_raw(&self, out: &mut Vec<u8>) {
        out.reserve(self.abi_encoded_size());
        out.extend(crate::abi::encode_sequence(&self.tokenize()));
    }

    /// ABI encode the call to the given buffer **with** its selector.
    #[inline]
    fn abi_encode(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(4 + self.abi_encoded_size());
        out.extend(&Self::SELECTOR);
        self.abi_encode_raw(&mut out);
        out
    }

    /// ABI decode this call's return values from the given slice.
    fn abi_decode_returns(data: &[u8], validate: bool) -> Result<Self::Return>;

    /// ABI encode the call's return values.
    #[inline]
    fn abi_encode_returns<'a, E>(e: &'a E) -> Vec<u8>
    where
        E: YlmTypeValue<Self::ReturnTuple<'a>>,
    {
        crate::abi::encode_sequence(&e.stv_to_tokens())
    }
}

/// A Ylem constructor.
pub trait YlmConstructor: Sized {
    /// The underlying tuple type which represents this type's arguments.
    ///
    /// If this type has no arguments, this will be the unit type `()`.
    type Parameters<'a>: YlmType<Token<'a> = Self::Token<'a>>;

    /// The arguments' corresponding [TokenSeq] type.
    type Token<'a>: TokenSeq<'a>;

    /// Convert from the tuple type used for ABI encoding and decoding.
    fn new(tuple: <Self::Parameters<'_> as YlmType>::RustType) -> Self;

    /// Tokenize the call's arguments.
    fn tokenize(&self) -> Self::Token<'_>;

    /// The size of the encoded data in bytes.
    #[inline]
    fn abi_encoded_size(&self) -> usize {
        if let Some(size) = <Self::Parameters<'_> as YlmType>::ENCODED_SIZE {
            return size;
        }

        // `total_words` includes the first dynamic offset which we ignore.
        let offset = <<Self::Parameters<'_> as YlmType>::Token<'_> as Token>::DYNAMIC as usize * 32;
        (self.tokenize().total_words() * Word::len_bytes()).saturating_sub(offset)
    }

    /// ABI encode the call to the given buffer.
    #[inline]
    fn abi_encode(&self) -> Vec<u8> {
        crate::abi::encode_sequence(&self.tokenize())
    }
}
