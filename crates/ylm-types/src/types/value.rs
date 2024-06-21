use super::YlmType;
use crate::{
    abi::TokenSeq,
    private::YlmTypeValue,
    ylm_data::{self, ByteCount, SupportedFixedBytes},
    Result, Word,
};
use alloc::{borrow::Cow, string::String, vec::Vec};
use base_primitives::{Bytes, FixedBytes, Function, IcanAddress, I256, U256};

/// A Ylem value.
///
/// This is a convenience trait that re-exports the logic in [`YlmType`] with
/// less generic implementations so that they can be used as methods with `self`
/// receivers.
///
/// See [`YlmType`] for more information.
///
/// # Implementer's Guide
///
/// It should not be necessary to implement this trait manually. Instead, use
/// the [`ylm!`](crate::ylm!) procedural macro to parse Ylem syntax into
/// types that implement this trait.
///
/// # Examples
///
/// ```
/// use base_ylm_types::YlmValue;
///
/// let my_values = ("hello", 0xdeadbeef_u32, true, [0x42_u8; 24]);
/// let _ = my_values.abi_encode();
/// let _ = my_values.abi_encode_packed();
/// assert_eq!(my_values.ylm_type_name(), "(string,uint32,bool,bytes24)");
/// ```
pub trait YlmValue: YlmTypeValue<Self::YlmType> {
    /// The Ylem type that this type corresponds to.
    type YlmType: YlmType;

    /// The name of the associated Ylem type.
    ///
    /// See [`YlmType::YLM_NAME`] for more information.
    #[inline]
    fn ylm_name(&self) -> &'static str {
        Self::YlmType::YLM_NAME
    }

    /// The name of the associated Ylem type.
    ///
    /// See [`YlmType::ylm_type_name`] for more information.
    #[deprecated(since = "0.6.3", note = "use `ylm_name` instead")]
    #[inline]
    fn ylm_type_name(&self) -> Cow<'static, str> {
        self.ylm_name().into()
    }

    /// Tokenizes the given value into this type's token.
    ///
    /// See [`YlmType::tokenize`] for more information.
    #[inline]
    fn tokenize(&self) -> <Self::YlmType as YlmType>::Token<'_> {
        <Self as YlmTypeValue<Self::YlmType>>::stv_to_tokens(self)
    }

    /// Detokenize a value from the given token.
    ///
    /// See [`YlmType::detokenize`] for more information.
    #[inline]
    fn detokenize(token: <Self::YlmType as YlmType>::Token<'_>) -> Self
    where
        Self: From<<Self::YlmType as YlmType>::RustType>,
    {
        Self::from(<Self::YlmType as YlmType>::detokenize(token))
    }

    /// Calculate the ABI-encoded size of the data.
    ///
    /// See [`YlmType::abi_encoded_size`] for more information.
    #[inline]
    fn abi_encoded_size(&self) -> usize {
        <Self as YlmTypeValue<Self::YlmType>>::stv_abi_encoded_size(self)
    }

    /// Encode this data according to EIP-712 `encodeData` rules, and hash it
    /// if necessary.
    ///
    /// See [`YlmType::eip712_data_word`] for more information.
    #[inline]
    fn eip712_data_word(&self) -> Word {
        <Self as YlmTypeValue<Self::YlmType>>::stv_eip712_data_word(self)
    }

    /// Non-standard Packed Mode ABI encoding.
    ///
    /// See [`YlmType::abi_encode_packed_to`] for more information.
    #[inline]
    fn abi_encode_packed_to(&self, out: &mut Vec<u8>) {
        <Self as YlmTypeValue<Self::YlmType>>::stv_abi_encode_packed_to(self, out)
    }

    /// Non-standard Packed Mode ABI encoding.
    ///
    /// See [`YlmType::abi_encode_packed`] for more information.
    #[inline]
    fn abi_encode_packed(&self) -> Vec<u8> {
        let mut out = Vec::new();
        <Self as YlmTypeValue<Self::YlmType>>::stv_abi_encode_packed_to(self, &mut out);
        out
    }

    /// ABI-encodes the value.
    ///
    /// See [`YlmType::abi_encode`] for more information.
    #[inline]
    fn abi_encode(&self) -> Vec<u8> {
        Self::YlmType::abi_encode(self)
    }

    /// Encodes an ABI sequence.
    ///
    /// See [`YlmType::abi_encode_sequence`] for more information.
    #[inline]
    fn abi_encode_sequence(&self) -> Vec<u8>
    where
        for<'a> <Self::YlmType as YlmType>::Token<'a>: TokenSeq<'a>,
    {
        Self::YlmType::abi_encode_sequence(self)
    }

    /// Encodes an ABI sequence suitable for function parameters.
    ///
    /// See [`YlmType::abi_encode_params`] for more information.
    #[inline]
    fn abi_encode_params(&self) -> Vec<u8>
    where
        for<'a> <Self::YlmType as YlmType>::Token<'a>: TokenSeq<'a>,
    {
        Self::YlmType::abi_encode_params(self)
    }

    /// ABI-decode this type from the given data.
    ///
    /// See [`YlmType::abi_decode`] for more information.
    fn abi_decode(data: &[u8], validate: bool) -> Result<Self>
    where
        Self: From<<Self::YlmType as YlmType>::RustType>,
    {
        Self::YlmType::abi_decode(data, validate).map(Self::from)
    }

    /// ABI-decode this type from the given data.
    ///
    /// See [`YlmType::abi_decode_params`] for more information.
    #[inline]
    fn abi_decode_params<'de>(data: &'de [u8], validate: bool) -> Result<Self>
    where
        Self: From<<Self::YlmType as YlmType>::RustType>,
        <Self::YlmType as YlmType>::Token<'de>: TokenSeq<'de>,
    {
        Self::YlmType::abi_decode_params(data, validate).map(Self::from)
    }

    /// ABI-decode this type from the given data.
    ///
    /// See [`YlmType::abi_decode_sequence`] for more information.
    #[inline]
    fn abi_decode_sequence<'de>(data: &'de [u8], validate: bool) -> Result<Self>
    where
        Self: From<<Self::YlmType as YlmType>::RustType>,
        <Self::YlmType as YlmType>::Token<'de>: TokenSeq<'de>,
    {
        Self::YlmType::abi_decode_sequence(data, validate).map(Self::from)
    }
}

macro_rules! impl_ylm_value {
    ($($(#[$attr:meta])* [$($gen:tt)*] $rust:ty => $sol:ty [$($where:tt)*];)+) => {$(
        $(#[$attr])*
        impl<$($gen)*> YlmValue for $rust $($where)* {
            type YlmType = $sol;
        }
    )*};
}

impl_ylm_value! {
    // Basic
    [] bool => ylm_data::Bool [];

    [] i8 => ylm_data::Int::<8> [];
    [] i16 => ylm_data::Int::<16> [];
    [] i32 => ylm_data::Int::<32> [];
    [] i64 => ylm_data::Int::<64> [];
    [] i128 => ylm_data::Int::<128> [];
    [] I256 => ylm_data::Int::<256> [];

    // TODO: `u8` is specialized to encode as `bytes` or `bytesN`
    // [] u8 => ylm_data::Uint::<8> [];
    [] u16 => ylm_data::Uint::<16> [];
    [] u32 => ylm_data::Uint::<32> [];
    [] u64 => ylm_data::Uint::<64> [];
    [] u128 => ylm_data::Uint::<128> [];
    [] U256 => ylm_data::Uint::<256> [];

    [] IcanAddress => ylm_data::Address [];
    [] Function => ylm_data::Function [];
    [const N: usize] FixedBytes<N> => ylm_data::FixedBytes<N> [where ByteCount<N>: SupportedFixedBytes];
    [const N: usize] [u8; N] => ylm_data::FixedBytes<N> [where ByteCount<N>: SupportedFixedBytes];

    // `bytes` and `string` are specialized below.

    // Generic
    [T: YlmValue] Vec<T> => ylm_data::Array<T::YlmType> [];
    [T: YlmValue] [T] => ylm_data::Array<T::YlmType> [];
    [T: YlmValue, const N: usize] [T; N] => ylm_data::FixedArray<T::YlmType, N> [];

    ['a, T: ?Sized + YlmValue] &'a T => T::YlmType [where &'a T: YlmTypeValue<T::YlmType>];
    ['a, T: ?Sized + YlmValue] &'a mut T => T::YlmType [where &'a mut T: YlmTypeValue<T::YlmType>];
}

macro_rules! tuple_impls {
    ($count:literal $($ty:ident),+) => {
        impl<$($ty: YlmValue,)+> YlmValue for ($($ty,)+) {
            type YlmType = ($($ty::YlmType,)+);
        }
    };
}

impl YlmValue for () {
    type YlmType = ();
}

all_the_tuples!(tuple_impls);

// Empty `bytes` and `string` specialization
impl YlmValue for str {
    type YlmType = ylm_data::String;

    #[inline]
    fn abi_encode(&self) -> Vec<u8> {
        if self.is_empty() {
            crate::abi::EMPTY_BYTES.to_vec()
        } else {
            <Self::YlmType as YlmType>::abi_encode(self)
        }
    }
}

impl YlmValue for [u8] {
    type YlmType = ylm_data::Bytes;

    #[inline]
    fn abi_encode(&self) -> Vec<u8> {
        if self.is_empty() {
            crate::abi::EMPTY_BYTES.to_vec()
        } else {
            <Self::YlmType as YlmType>::abi_encode(self)
        }
    }
}

impl YlmValue for String {
    type YlmType = ylm_data::String;

    #[inline]
    fn abi_encode(&self) -> Vec<u8> {
        self[..].abi_encode()
    }
}

impl YlmValue for Bytes {
    type YlmType = ylm_data::Bytes;

    #[inline]
    fn abi_encode(&self) -> Vec<u8> {
        self[..].abi_encode()
    }
}

impl YlmValue for Vec<u8> {
    type YlmType = ylm_data::Bytes;

    #[inline]
    fn abi_encode(&self) -> Vec<u8> {
        self[..].abi_encode()
    }
}

#[cfg(test)]
#[allow(clippy::type_complexity)]
mod tests {
    use super::*;

    // Make sure these are in scope
    #[allow(unused_imports)]
    use crate::{private::YlmTypeValue as _, YlmType as _};

    #[test]
    fn inference() {
        false.ylm_name();
        false.abi_encoded_size();
        false.eip712_data_word();
        false.abi_encode_packed_to(&mut vec![]);
        false.abi_encode_packed();
        false.abi_encode();
        (false,).abi_encode_sequence();
        (false,).abi_encode_params();

        "".ylm_name();
        "".abi_encoded_size();
        "".eip712_data_word();
        "".abi_encode_packed_to(&mut vec![]);
        "".abi_encode_packed();
        "".abi_encode();
        ("",).abi_encode_sequence();
        ("",).abi_encode_params();

        let _ = String::abi_decode(b"", false);
        let _ = bool::abi_decode(b"", false);
    }

    #[test]
    fn basic() {
        assert_eq!(false.abi_encode(), Word::ZERO[..]);
        assert_eq!(true.abi_encode(), Word::with_last_byte(1)[..]);

        assert_eq!(0i8.abi_encode(), Word::ZERO[..]);
        assert_eq!(0i16.abi_encode(), Word::ZERO[..]);
        assert_eq!(0i32.abi_encode(), Word::ZERO[..]);
        assert_eq!(0i64.abi_encode(), Word::ZERO[..]);
        assert_eq!(0i128.abi_encode(), Word::ZERO[..]);
        assert_eq!(I256::ZERO.abi_encode(), Word::ZERO[..]);

        assert_eq!(0u16.abi_encode(), Word::ZERO[..]);
        assert_eq!(0u32.abi_encode(), Word::ZERO[..]);
        assert_eq!(0u64.abi_encode(), Word::ZERO[..]);
        assert_eq!(0u128.abi_encode(), Word::ZERO[..]);
        assert_eq!(U256::ZERO.abi_encode(), Word::ZERO[..]);

        assert_eq!(IcanAddress::ZERO.abi_encode(), Word::ZERO[..]);
        assert_eq!(Function::ZERO.abi_encode(), Word::ZERO[..]);

        let encode_bytes = |b: &[u8]| {
            let last = Word::new({
                let mut buf = [0u8; 32];
                buf[..b.len()].copy_from_slice(b);
                buf
            });
            [
                &Word::with_last_byte(0x20)[..],
                &Word::with_last_byte(b.len() as u8)[..],
                if b.is_empty() { b } else { &last[..] },
            ]
            .concat()
        };

        // empty `bytes`
        assert_eq!(b"".abi_encode(), encode_bytes(b""));
        assert_eq!((b"" as &[_]).abi_encode(), encode_bytes(b""));
        // `bytes1`
        assert_eq!(b"a".abi_encode()[0], b'a');
        assert_eq!(b"a".abi_encode()[1..], Word::ZERO[1..]);
        // `bytes`
        assert_eq!((b"a" as &[_]).abi_encode(), encode_bytes(b"a"));

        assert_eq!("".abi_encode(), encode_bytes(b""));
        assert_eq!("a".abi_encode(), encode_bytes(b"a"));
        assert_eq!(String::new().abi_encode(), encode_bytes(b""));
        assert_eq!(String::from("a").abi_encode(), encode_bytes(b"a"));
        assert_eq!(Vec::<u8>::new().abi_encode(), encode_bytes(b""));
        assert_eq!(Vec::<u8>::from(&b"a"[..]).abi_encode(), encode_bytes(b"a"));
    }

    #[test]
    fn big() {
        let tuple = (
            false,
            0i8,
            0i16,
            0i32,
            0i64,
            0i128,
            I256::ZERO,
            // 0u8,
            0u16,
            0u32,
            0u64,
            0u128,
            U256::ZERO,
            IcanAddress::ZERO,
            Function::ZERO,
        );
        let encoded = tuple.abi_encode();
        assert_eq!(encoded.len(), 32 * 14);
        assert!(encoded.iter().all(|&b| b == 0));
    }

    #[test]
    fn complex() {
        let tuple = ((((((false,),),),),),);
        assert_eq!(tuple.abi_encode(), Word::ZERO[..]);
        assert_eq!(tuple.ylm_name(), "((((((bool))))))");

        let tuple = (
            42u64,
            "hello world",
            true,
            (
                String::from("aaaa"),
                IcanAddress::with_last_byte(69),
                b"bbbb".to_vec(),
                b"cccc",
                &b"dddd"[..],
            ),
        );
        assert_eq!(tuple.ylm_name(), "(uint64,string,bool,(string,address,bytes,bytes4,bytes))");
    }

    #[test]
    fn derefs() {
        let x: &[IcanAddress; 0] = &[];
        x.abi_encode();
        assert_eq!(x.ylm_name(), "address[0]");

        let x = &[IcanAddress::ZERO];
        x.abi_encode();
        assert_eq!(x.ylm_name(), "address[1]");

        let x = &[IcanAddress::ZERO, IcanAddress::ZERO];
        x.abi_encode();
        assert_eq!(x.ylm_name(), "address[2]");

        let x = &[IcanAddress::ZERO][..];
        x.abi_encode();
        assert_eq!(x.ylm_name(), "address[]");

        let mut x = *b"0";
        let x = (&mut x, *b"aaaa", b"00");
        x.abi_encode();
        assert_eq!(x.ylm_name(), "(bytes1,bytes4,bytes2)");

        let tuple = &(&0u16, &"", b"0", &mut [IcanAddress::ZERO][..]);
        tuple.abi_encode();
        assert_eq!(tuple.ylm_name(), "(uint16,string,bytes1,address[])");
    }

    #[test]
    fn decode() {
        let _: Result<String> = String::abi_decode(b"", false);

        let _: Result<Vec<String>> = Vec::<String>::abi_decode(b"", false);

        let _: Result<(u64, String, U256)> = <(u64, String, U256)>::abi_decode(b"", false);
        let _: Result<(i64, Vec<(u32, String, Vec<FixedBytes<4>>)>, U256)> =
            <(i64, Vec<(u32, String, Vec<FixedBytes<4>>)>, U256)>::abi_decode(b"", false);
    }

    #[test]
    fn empty_spec() {
        assert_eq!("".abi_encode(), crate::abi::EMPTY_BYTES);
        assert_eq!(b"".abi_encode(), crate::abi::EMPTY_BYTES);
        assert_eq!(
            ("", "a").abi_encode(),
            <(ylm_data::String, ylm_data::String)>::abi_encode(&("", "a"))
        );
        assert_eq!(
            ("a", "").abi_encode(),
            <(ylm_data::String, ylm_data::String)>::abi_encode(&("a", ""))
        );
        assert_eq!(
            (&b""[..], &b"a"[..]).abi_encode(),
            <(ylm_data::Bytes, ylm_data::Bytes)>::abi_encode(&(b"", b"a"))
        );
        assert_eq!(
            (&b"a"[..], &b""[..]).abi_encode(),
            <(ylm_data::Bytes, ylm_data::Bytes)>::abi_encode(&(b"a", b""))
        );
    }
}
