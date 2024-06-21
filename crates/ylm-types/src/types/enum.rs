use crate::{abi::token::WordToken, Result, Word, YlmType};
use alloc::vec::Vec;

/// A Ylem enum. This is always a wrapper around a [`u8`].
///
/// # Implementer's Guide
///
/// It should not be necessary to implement this trait manually. Instead, use
/// the [`ylm!`](crate::ylm!) procedural macro to parse Ylem syntax into
/// types that implement this trait.
pub trait YlmEnum: Sized + Copy + Into<u8> + TryFrom<u8, Error = crate::Error> {
    /// The number of variants in the enum.
    ///
    /// This is generally between 1 and 256 inclusive.
    const COUNT: usize;

    /// Tokenize the enum.
    #[inline]
    fn tokenize(self) -> WordToken {
        WordToken(Word::with_last_byte(self.into()))
    }

    /// ABI decode the enum from the given buffer.
    #[inline]
    fn abi_decode(data: &[u8], validate: bool) -> Result<Self> {
        <crate::ylm_data::Uint<8> as YlmType>::abi_decode(data, validate).and_then(Self::try_from)
    }

    /// ABI encode the enum into the given buffer.
    #[inline]
    fn abi_encode_raw(self, out: &mut Vec<u8>) {
        out.extend(self.tokenize().0);
    }

    /// ABI encode the enum.
    #[inline]
    fn abi_encode(self) -> Vec<u8> {
        self.tokenize().0.to_vec()
    }
}
