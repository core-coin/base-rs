use crate::{DynToken, DynYlmValue, Error, Result, Specifier, Word, YlmType};
use alloc::{borrow::Cow, boxed::Box, string::String, vec::Vec};
use base_primitives::{
    try_vec,
    utils::{box_try_new, vec_try_with_capacity},
};
use base_ylm_types::{abi::Decoder, ylm_data};
use core::{fmt, iter::zip, num::NonZeroUsize, str::FromStr};
use parser::TypeSpecifier;

#[cfg(feature = "eip712")]
macro_rules! as_tuple {
    ($ty:ident $t:tt) => {
        $ty::Tuple($t) | $ty::CustomStruct { tuple: $t, .. }
    };
}
#[cfg(not(feature = "eip712"))]
macro_rules! as_tuple {
    ($ty:ident $t:tt) => {
        $ty::Tuple($t)
    };
}
pub(crate) use as_tuple;

/// A dynamic Ylem type.
///
/// Equivalent to an enum wrapper around all implementers of [`YlmType`].
///
/// This is used to represent Ylem types that are not known at compile time.
/// It is used in conjunction with [`DynToken`] and [`DynYlmValue`] to allow for
/// dynamic ABI encoding and decoding.
///
/// # Examples
///
/// Parsing Ylem type strings:
///
/// ```
/// use base_dyn_abi::DynYlmType;
///
/// let type_name = "(bool,address)[]";
/// let ty = DynYlmType::parse(type_name)?;
/// assert_eq!(
///     ty,
///     DynYlmType::Array(Box::new(DynYlmType::Tuple(
///         vec![DynYlmType::Bool, DynYlmType::Address,]
///     )))
/// );
/// assert_eq!(ty.ylm_type_name(), type_name);
///
/// // alternatively, you can use the FromStr impl
/// let ty2 = type_name.parse::<DynYlmType>()?;
/// assert_eq!(ty, ty2);
/// # Ok::<_, base_dyn_abi::Error>(())
/// ```
///
/// Decoding dynamic types:
///
/// ```
/// use base_dyn_abi::{DynYlmType, DynYlmValue};
/// use base_primitives::U256;
///
/// let my_type = DynYlmType::Uint(256);
/// let my_data: DynYlmValue = U256::from(183u64).into();
///
/// let encoded = my_data.abi_encode();
/// let decoded = my_type.abi_decode(&encoded)?;
///
/// assert_eq!(decoded, my_data);
///
/// let my_type = DynYlmType::Array(Box::new(my_type));
/// let my_data = DynYlmValue::Array(vec![my_data.clone()]);
///
/// let encoded = my_data.abi_encode();
/// let decoded = my_type.abi_decode(&encoded)?;
///
/// assert_eq!(decoded, my_data);
/// # Ok::<_, base_dyn_abi::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DynYlmType {
    /// Boolean.
    Bool,
    /// Signed Integer.
    Int(usize),
    /// Unsigned Integer.
    Uint(usize),
    /// Fixed-size bytes, up to 32.
    FixedBytes(usize),
    /// Address.
    Address,
    /// Function.
    Function,

    /// Dynamic bytes.
    Bytes,
    /// String.
    String,

    /// Dynamically sized array.
    Array(Box<DynYlmType>),
    /// Fixed-sized array.
    FixedArray(Box<DynYlmType>, usize),
    /// Tuple.
    Tuple(Vec<DynYlmType>),

    /// User-defined struct.
    #[cfg(feature = "eip712")]
    CustomStruct {
        /// Name of the struct.
        name: String,
        /// Prop names.
        prop_names: Vec<String>,
        /// Inner types.
        tuple: Vec<DynYlmType>,
    },
}

impl fmt::Display for DynYlmType {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.ylm_type_name())
    }
}

impl FromStr for DynYlmType {
    type Err = Error;

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

impl DynYlmType {
    /// Parses a Ylem type name string into a [`DynYlmType`].
    ///
    /// # Examples
    ///
    /// ```
    /// # use base_dyn_abi::DynYlmType;
    /// let type_name = "uint256";
    /// let ty = DynYlmType::parse(type_name)?;
    /// assert_eq!(ty, DynYlmType::Uint(256));
    /// assert_eq!(ty.ylm_type_name(), type_name);
    /// assert_eq!(ty.to_string(), type_name);
    ///
    /// // alternatively, you can use the FromStr impl
    /// let ty2 = type_name.parse::<DynYlmType>()?;
    /// assert_eq!(ty2, ty);
    /// # Ok::<_, base_dyn_abi::Error>(())
    /// ```
    #[inline]
    pub fn parse(s: &str) -> Result<Self> {
        TypeSpecifier::parse(s).map_err(Error::TypeParser).and_then(|t| t.resolve())
    }

    /// Calculate the nesting depth of this type. Simple types have a nesting
    /// depth of 0, while all other types have a nesting depth of at least 1.
    pub fn nesting_depth(&self) -> usize {
        match self {
            DynYlmType::Bool
            | DynYlmType::Int(_)
            | DynYlmType::Uint(_)
            | DynYlmType::FixedBytes(_)
            | DynYlmType::Address
            | DynYlmType::Function
            | DynYlmType::Bytes
            | DynYlmType::String => 0,
            DynYlmType::Array(contents) | DynYlmType::FixedArray(contents, _) => {
                1 + contents.nesting_depth()
            }
            as_tuple!(Self tuple) => 1 + tuple.iter().map(Self::nesting_depth).max().unwrap_or(0),
        }
    }

    /// Fallible cast to the contents of a variant.
    #[inline]
    pub fn as_tuple(&self) -> Option<&[Self]> {
        match self {
            Self::Tuple(t) => Some(t),
            _ => None,
        }
    }

    /// Fallible cast to the contents of a variant.
    #[inline]
    #[allow(clippy::missing_const_for_fn)]
    pub fn as_custom_struct(&self) -> Option<(&str, &[String], &[Self])> {
        match self {
            #[cfg(feature = "eip712")]
            Self::CustomStruct { name, prop_names, tuple } => Some((name, prop_names, tuple)),
            _ => None,
        }
    }

    /// Returns whether this type is contains a custom struct.
    #[inline]
    #[allow(clippy::missing_const_for_fn)]
    pub fn has_custom_struct(&self) -> bool {
        #[cfg(feature = "eip712")]
        {
            match self {
                Self::CustomStruct { .. } => true,
                Self::Array(t) => t.has_custom_struct(),
                Self::FixedArray(t, _) => t.has_custom_struct(),
                Self::Tuple(t) => t.iter().any(Self::has_custom_struct),
                _ => false,
            }
        }
        #[cfg(not(feature = "eip712"))]
        {
            false
        }
    }

    /// Check that the given [`DynYlmValue`]s match these types.
    ///
    /// See [`matches`](Self::matches) for more information.
    #[inline]
    pub fn matches_many(types: &[Self], values: &[DynYlmValue]) -> bool {
        types.len() == values.len() && zip(types, values).all(|(t, v)| t.matches(v))
    }

    /// Check that the given [`DynYlmValue`] matches this type.
    ///
    /// Note: this will not check any names, but just the types; e.g for
    /// `CustomStruct`, when the "eip712" feature is enabled, this will only
    /// check equality between the lengths and types of the tuple.
    pub fn matches(&self, value: &DynYlmValue) -> bool {
        match self {
            Self::Bool => matches!(value, DynYlmValue::Bool(_)),
            Self::Int(size) => matches!(value, DynYlmValue::Int(_, s) if s == size),
            Self::Uint(size) => matches!(value, DynYlmValue::Uint(_, s) if s == size),
            Self::FixedBytes(size) => matches!(value, DynYlmValue::FixedBytes(_, s) if s == size),
            Self::Address => matches!(value, DynYlmValue::Address(_)),
            Self::Function => matches!(value, DynYlmValue::Function(_)),
            Self::Bytes => matches!(value, DynYlmValue::Bytes(_)),
            Self::String => matches!(value, DynYlmValue::String(_)),
            Self::Array(t) => {
                matches!(value, DynYlmValue::Array(v) if v.iter().all(|v| t.matches(v)))
            }
            Self::FixedArray(t, size) => matches!(
                value,
                DynYlmValue::FixedArray(v) if v.len() == *size && v.iter().all(|v| t.matches(v))
            ),
            Self::Tuple(types) => {
                matches!(value, as_tuple!(DynYlmValue tuple) if zip(types, tuple).all(|(t, v)| t.matches(v)))
            }
            #[cfg(feature = "eip712")]
            Self::CustomStruct { name: _, prop_names, tuple } => {
                if let DynYlmValue::CustomStruct { name: _, prop_names: p, tuple: t } = value {
                    // check just types
                    prop_names.len() == tuple.len()
                        && prop_names.len() == p.len()
                        && tuple.len() == t.len()
                        && zip(tuple, t).all(|(a, b)| a.matches(b))
                } else if let DynYlmValue::Tuple(v) = value {
                    zip(v, tuple).all(|(v, t)| t.matches(v))
                } else {
                    false
                }
            }
        }
    }

    /// Dynamic detokenization.
    // This should not fail when using a token created by `Self::empty_dyn_token`.
    #[allow(clippy::unnecessary_to_owned)] // https://github.com/rust-lang/rust-clippy/issues/8148
    pub fn detokenize(&self, token: DynToken<'_>) -> Result<DynYlmValue> {
        match (self, token) {
            (Self::Bool, DynToken::Word(word)) => {
                Ok(DynYlmValue::Bool(ylm_data::Bool::detokenize(word.into())))
            }

            // cheating here, but it's ok
            (Self::Int(size), DynToken::Word(word)) => {
                Ok(DynYlmValue::Int(ylm_data::Int::<256>::detokenize(word.into()), *size))
            }

            (Self::Uint(size), DynToken::Word(word)) => {
                Ok(DynYlmValue::Uint(ylm_data::Uint::<256>::detokenize(word.into()), *size))
            }

            (Self::FixedBytes(size), DynToken::Word(word)) => Ok(DynYlmValue::FixedBytes(
                ylm_data::FixedBytes::<32>::detokenize(word.into()),
                *size,
            )),

            (Self::Address, DynToken::Word(word)) => {
                Ok(DynYlmValue::Address(ylm_data::Address::detokenize(word.into())))
            }

            (Self::Function, DynToken::Word(word)) => {
                Ok(DynYlmValue::Function(ylm_data::Function::detokenize(word.into())))
            }

            (Self::Bytes, DynToken::PackedSeq(buf)) => Ok(DynYlmValue::Bytes(buf.to_vec())),

            (Self::String, DynToken::PackedSeq(buf)) => {
                Ok(DynYlmValue::String(ylm_data::String::detokenize(buf.into())))
            }

            (Self::Array(t), DynToken::DynSeq { contents, .. }) => {
                t.detokenize_array(contents.into_owned()).map(DynYlmValue::Array)
            }

            (Self::FixedArray(t, size), DynToken::FixedSeq(tokens, _)) => {
                if *size != tokens.len() {
                    return Err(crate::Error::custom(
                        "array length mismatch on dynamic detokenization",
                    ));
                }
                t.detokenize_array(tokens.into_owned()).map(DynYlmValue::FixedArray)
            }

            (Self::Tuple(types), DynToken::FixedSeq(tokens, _)) => {
                if types.len() != tokens.len() {
                    return Err(crate::Error::custom(
                        "tuple length mismatch on dynamic detokenization",
                    ));
                }
                Self::detokenize_many(types, tokens.into_owned()).map(DynYlmValue::Tuple)
            }

            #[cfg(feature = "eip712")]
            (Self::CustomStruct { name, tuple, prop_names }, DynToken::FixedSeq(tokens, len)) => {
                if len != tokens.len() || len != tuple.len() {
                    return Err(crate::Error::custom(
                        "custom length mismatch on dynamic detokenization",
                    ));
                }
                Self::detokenize_many(tuple, tokens.into_owned()).map(|tuple| {
                    DynYlmValue::CustomStruct {
                        name: name.clone(),
                        prop_names: prop_names.clone(),
                        tuple,
                    }
                })
            }

            _ => Err(crate::Error::custom("mismatched types on dynamic detokenization")),
        }
    }

    fn detokenize_array(&self, tokens: Vec<DynToken<'_>>) -> Result<Vec<DynYlmValue>> {
        let mut values = vec_try_with_capacity(tokens.len())?;
        for token in tokens {
            values.push(self.detokenize(token)?);
        }
        Ok(values)
    }

    fn detokenize_many(types: &[Self], tokens: Vec<DynToken<'_>>) -> Result<Vec<DynYlmValue>> {
        assert_eq!(types.len(), tokens.len());
        let mut values = vec_try_with_capacity(tokens.len())?;
        for (ty, token) in zip(types, tokens) {
            values.push(ty.detokenize(token)?);
        }
        Ok(values)
    }

    #[inline]
    #[allow(clippy::missing_const_for_fn)]
    fn ylm_type_name_simple(&self) -> Option<&'static str> {
        match self {
            Self::Address => Some("address"),
            Self::Function => Some("function"),
            Self::Bool => Some("bool"),
            Self::Bytes => Some("bytes"),
            Self::String => Some("string"),
            _ => None,
        }
    }

    #[inline]
    fn ylm_type_name_raw(&self, out: &mut String) {
        match self {
            Self::Address | Self::Function | Self::Bool | Self::Bytes | Self::String => {
                out.push_str(unsafe { self.ylm_type_name_simple().unwrap_unchecked() });
            }

            Self::FixedBytes(size) | Self::Int(size) | Self::Uint(size) => {
                let prefix = match self {
                    Self::FixedBytes(..) => "bytes",
                    Self::Int(..) => "int",
                    Self::Uint(..) => "uint",
                    _ => unreachable!(),
                };
                out.push_str(prefix);
                out.push_str(itoa::Buffer::new().format(*size));
            }

            as_tuple!(Self tuple) => {
                out.push('(');
                for (i, val) in tuple.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    val.ylm_type_name_raw(out);
                }
                if tuple.len() == 1 {
                    out.push(',');
                }
                out.push(')');
            }
            Self::Array(t) => {
                t.ylm_type_name_raw(out);
                out.push_str("[]");
            }
            Self::FixedArray(t, len) => {
                t.ylm_type_name_raw(out);
                out.push('[');
                out.push_str(itoa::Buffer::new().format(*len));
                out.push(']');
            }
        }
    }

    /// Returns an estimate of the number of bytes needed to format this type.
    ///
    /// This calculation is meant to be an upper bound for valid types to avoid
    /// a second allocation in `ylm_type_name_raw` and thus is almost never
    /// going to be exact.
    fn ylm_type_name_capacity(&self) -> usize {
        match self {
            | Self::Address // 7
            | Self::Function // 8
            | Self::Bool // 4
            | Self::Bytes // 5
            | Self::String // 6
            | Self::FixedBytes(_) // 5 + 2
            | Self::Int(_) // 3 + 3
            | Self::Uint(_) // 4 + 3
            => 8,

            | Self::Array(t) // t + 2
            | Self::FixedArray(t, _) // t + 2 + log10(len)
            => t.ylm_type_name_capacity() + 8,

            as_tuple!(Self tuple) // sum(tuple) + len(tuple) + 2
            => tuple.iter().map(Self::ylm_type_name_capacity).sum::<usize>() + 8,
        }
    }

    /// The Ylem type name. This returns the Ylem type corresponding to
    /// this value, if it is known. A type will not be known if the value
    /// contains an empty sequence, e.g. `T[0]`.
    pub fn ylm_type_name(&self) -> Cow<'static, str> {
        if let Some(s) = self.ylm_type_name_simple() {
            Cow::Borrowed(s)
        } else {
            let mut s = String::with_capacity(self.ylm_type_name_capacity());
            self.ylm_type_name_raw(&mut s);
            Cow::Owned(s)
        }
    }

    /// The Ylem type name, as a `String`.
    ///
    /// Note: this shadows the inherent [`ToString`] implementation, derived
    /// from [`fmt::Display`], for performance reasons.
    #[inline]
    #[allow(clippy::inherent_to_string_shadow_display)]
    pub fn to_string(&self) -> String {
        self.ylm_type_name().into_owned()
    }

    /// Instantiate an empty dyn token, to be decoded into.
    ///
    /// ## Warning
    ///
    /// This function may allocate an unbounded amount of memory based on user
    /// input types. It must be used with care to avoid DOS issues.
    fn empty_dyn_token<'a>(&self) -> Result<DynToken<'a>> {
        Ok(match self {
            Self::Address
            | Self::Function
            | Self::Bool
            | Self::FixedBytes(_)
            | Self::Int(_)
            | Self::Uint(_) => DynToken::Word(Word::ZERO),

            Self::Bytes | Self::String => DynToken::PackedSeq(&[]),

            Self::Array(t) => DynToken::DynSeq {
                contents: Default::default(),
                template: Some(box_try_new(t.empty_dyn_token()?)?),
            },
            &Self::FixedArray(ref t, size) => {
                DynToken::FixedSeq(try_vec![t.empty_dyn_token()?; size]?.into(), size)
            }
            as_tuple!(Self tuple) => {
                let mut tokens = vec_try_with_capacity(tuple.len())?;
                for ty in tuple {
                    tokens.push(ty.empty_dyn_token()?);
                }
                DynToken::FixedSeq(tokens.into(), tuple.len())
            }
        })
    }

    /// Decode an event topic into a [`DynYlmValue`].
    pub(crate) fn decode_event_topic(&self, topic: Word) -> DynYlmValue {
        match self {
            Self::Address
            | Self::Function
            | Self::Bool
            | Self::FixedBytes(_)
            | Self::Int(_)
            | Self::Uint(_) => self.detokenize(DynToken::Word(topic)).unwrap(),
            _ => DynYlmValue::FixedBytes(topic, 32),
        }
    }

    /// Decode a [`DynYlmValue`] from a byte slice. Fails if the value does not
    /// match this type.
    ///
    /// This method is used for decoding single values. It assumes the `data`
    /// argument is an encoded single-element sequence wrapping the `self` type.
    #[inline]
    #[cfg_attr(debug_assertions, track_caller)]
    pub fn abi_decode(&self, data: &[u8]) -> Result<DynYlmValue> {
        self.abi_decode_inner(&mut Decoder::new(data, false), DynToken::decode_single_populate)
    }

    /// Decode a [`DynYlmValue`] from a byte slice. Fails if the value does not
    /// match this type.
    ///
    /// This method is used for decoding function arguments. It tries to
    /// determine whether the user intended to decode a sequence or an
    /// individual value. If the `self` type is a tuple, the `data` will be
    /// decoded as a sequence, otherwise it will be decoded as a single value.
    ///
    /// # Examples
    ///
    /// ```solidity
    /// // This function takes a single simple param:
    /// // DynYlmType::Uint(256).decode_params(data)
    /// function myFunc(uint256 a) public;
    ///
    /// // This function takes 2 params:
    /// // DynYlmType::Tuple(vec![DynYlmType::Uint(256), DynYlmType::Bool])
    /// //     .decode_params(data)
    /// function myFunc(uint256 b, bool c) public;
    /// ```
    #[inline]
    #[cfg_attr(debug_assertions, track_caller)]
    pub fn abi_decode_params(&self, data: &[u8]) -> Result<DynYlmValue> {
        match self {
            Self::Tuple(_) => self.abi_decode_sequence(data),
            _ => self.abi_decode(data),
        }
    }

    /// Decode a [`DynYlmValue`] from a byte slice. Fails if the value does not
    /// match this type.
    #[inline]
    #[cfg_attr(debug_assertions, track_caller)]
    pub fn abi_decode_sequence(&self, data: &[u8]) -> Result<DynYlmValue> {
        self.abi_decode_inner(&mut Decoder::new(data, false), DynToken::decode_sequence_populate)
    }

    /// Calculate the minimum number of ABI words necessary to encode this
    /// type.
    pub fn minimum_words(&self) -> usize {
        match self {
            // word types are always 1
            DynYlmType::Bool |
            DynYlmType::Int(_) |
            DynYlmType::Uint(_) |
            DynYlmType::FixedBytes(_) |
            DynYlmType::Address |
            DynYlmType::Function |
            // packed/dynamic seq types may be empty
            DynYlmType::Bytes |
            DynYlmType::String |
            DynYlmType::Array(_) => 1,
            // fixed-seq types are the sum of their components
            DynYlmType::FixedArray(v, size) => size * v.minimum_words(),
            DynYlmType::Tuple(tuple) => tuple.iter().map(|ty| ty.minimum_words()).sum(),
            #[cfg(feature = "eip712")]
            DynYlmType::CustomStruct { tuple, ..} => tuple.iter().map(|ty| ty.minimum_words()).sum(),
        }
    }

    #[inline]
    #[cfg_attr(debug_assertions, track_caller)]
    pub(crate) fn abi_decode_inner<'d, F>(
        &self,
        decoder: &mut Decoder<'d>,
        f: F,
    ) -> Result<DynYlmValue>
    where
        F: FnOnce(&mut DynToken<'d>, &mut Decoder<'d>) -> Result<()>,
    {
        if self.is_zst() {
            return Ok(self.zero_sized_value().expect("checked"));
        }

        if decoder.remaining_words() < self.minimum_words() {
            return Err(Error::YlmTypes(base_ylm_types::Error::Overrun));
        }

        let mut token = self.empty_dyn_token()?;
        f(&mut token, decoder)?;
        let value = self.detokenize(token).expect("invalid empty_dyn_token");
        debug_assert!(
            self.matches(&value),
            "decoded value does not match type:\n  type: {self:?}\n value: {value:?}"
        );
        Ok(value)
    }

    /// Wrap in an array of the specified size
    #[inline]
    pub(crate) fn array_wrap(self, size: Option<NonZeroUsize>) -> Self {
        match size {
            Some(size) => Self::FixedArray(Box::new(self), size.get()),
            None => Self::Array(Box::new(self)),
        }
    }

    /// Iteratively wrap in arrays.
    #[inline]
    pub(crate) fn array_wrap_from_iter(
        self,
        iter: impl IntoIterator<Item = Option<NonZeroUsize>>,
    ) -> Self {
        iter.into_iter().fold(self, Self::array_wrap)
    }

    /// Return true if the type is zero-sized, e.g. `()` or `T[0]`
    #[inline]
    pub fn is_zst(&self) -> bool {
        match self {
            Self::Array(inner) => inner.is_zst(),
            Self::FixedArray(inner, size) => *size == 0 || inner.is_zst(),
            Self::Tuple(inner) => inner.is_empty() || inner.iter().all(|t| t.is_zst()),
            _ => false,
        }
    }

    #[inline]
    const fn zero_sized_value(&self) -> Option<DynYlmValue> {
        match self {
            Self::Array(_) => Some(DynYlmValue::Array(vec![])),
            Self::FixedArray(_, _) => Some(DynYlmValue::FixedArray(vec![])),
            Self::Tuple(_) => Some(DynYlmValue::Tuple(vec![])),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use base_primitives::{hex, IcanAddress};

    #[test]
    fn dynamically_encodes() {
        let word1 =
            "0000000000000000000001010101010101010101010101010101010101010101".parse().unwrap();
        let word2 =
            "0000000000000000000002020202020202020202020202020202020202020202".parse().unwrap();

        let val = DynYlmValue::Address(IcanAddress::repeat_byte(0x01));
        let token = val.tokenize();
        assert_eq!(token, DynToken::from(word1));

        let val = DynYlmValue::FixedArray(vec![
            IcanAddress::repeat_byte(0x01).into(),
            IcanAddress::repeat_byte(0x02).into(),
        ]);

        let token = val.tokenize();
        assert_eq!(
            token,
            DynToken::FixedSeq(vec![DynToken::Word(word1), DynToken::Word(word2)].into(), 2)
        );
        let mut enc = crate::Encoder::default();
        DynYlmValue::encode_seq_to(val.as_fixed_seq().unwrap(), &mut enc);
        assert_eq!(enc.finish(), vec![word1, word2]);
    }

    // also tests the type name parser
    macro_rules! encoder_tests {
        ($($name:ident($ty:literal, $encoded:literal)),* $(,)?) => {$(
            #[test]
            fn $name() {
                encoder_test($ty, &hex!($encoded));
            }
        )*};
    }

    fn encoder_test(s: &str, encoded: &[u8]) {
        let ty: DynYlmType = s.parse().expect("parsing failed");
        assert_eq!(ty.ylm_type_name(), s, "type names are not the same");

        let value = ty.abi_decode_params(encoded).expect("decoding failed");
        if let Some(value_name) = value.ylm_type_name() {
            assert_eq!(value_name, s, "value names are not the same");
        }

        // Tuples are treated as top-level lists. So if we encounter a
        // dynamic tuple, the total length of the encoded data will include
        // the offset, but the encoding/decoding process will not. To
        // account for this, we add 32 bytes to the expected length when
        // the type is a dynamic tuple.
        let mut len = encoded.len();
        if value.as_tuple().is_some() && value.is_dynamic() {
            len += 32;
        }
        assert_eq!(value.total_words() * 32, len, "dyn_tuple={}", len != encoded.len());

        let re_encoded = value.abi_encode_params();
        assert!(
            re_encoded == encoded,
            "
  type: {ty}
 value: {value:?}
re-enc: {re_enc}
   enc: {encoded}",
            re_enc = hex::encode(re_encoded),
            encoded = hex::encode(encoded),
        );
    }

    encoder_tests! {
        address("address", "0000000000000000000000001111111111111111111111111111111111111111"),

        dynamic_array_of_addresses("address[]", "
            0000000000000000000000000000000000000000000000000000000000000020
            0000000000000000000000000000000000000000000000000000000000000002
            0000000000000000000000001111111111111111111111111111111111111111
            0000000000000000000000002222222222222222222222222222222222222222
        "),

        fixed_array_of_addresses("address[2]", "
            0000000000000000000000001111111111111111111111111111111111111111
            0000000000000000000000002222222222222222222222222222222222222222
        "),

        two_addresses("(address,address)", "
            0000000000000000000011111111111111111111111111111111111111111111
            0000000000000000000022222222222222222222222222222222222222222222
        "),

        fixed_array_of_dynamic_arrays_of_addresses("address[][2]", "
            0000000000000000000000000000000000000000000000000000000000000020
            0000000000000000000000000000000000000000000000000000000000000040
            00000000000000000000000000000000000000000000000000000000000000a0
            0000000000000000000000000000000000000000000000000000000000000002
            0000000000000000000011111111111111111111111111111111111111111111
            0000000000000000000000002222222222222222222222222222222222222222
            0000000000000000000000000000000000000000000000000000000000000002
            0000000000000000000033333333333333333333333333333333333333333333
            0000000000000000000044444444444444444444444444444444444444444444
        "),

        dynamic_array_of_fixed_arrays_of_addresses("address[2][]", "
            0000000000000000000000000000000000000000000000000000000000000020
            0000000000000000000000000000000000000000000000000000000000000002
            0000000000000000000011111111111111111111111111111111111111111111
            0000000000000000000022222222222222222222222222222222222222222222
            0000000000000000000033333333333333333333333333333333333333333333
            0000000000000000000044444444444444444444444444444444444444444444
        "),

        dynamic_array_of_dynamic_arrays("address[][]", "
            0000000000000000000000000000000000000000000000000000000000000020
            0000000000000000000000000000000000000000000000000000000000000002
            0000000000000000000000000000000000000000000000000000000000000040
            0000000000000000000000000000000000000000000000000000000000000080
            0000000000000000000000000000000000000000000000000000000000000001
            0000000000000000000011111111111111111111111111111111111111111111
            0000000000000000000000000000000000000000000000000000000000000001
            0000000000000000000022222222222222222222222222222222222222222222
        "),

        dynamic_array_of_dynamic_arrays2("address[][]", "
            0000000000000000000000000000000000000000000000000000000000000020
            0000000000000000000000000000000000000000000000000000000000000002
            0000000000000000000000000000000000000000000000000000000000000040
            00000000000000000000000000000000000000000000000000000000000000a0
            0000000000000000000000000000000000000000000000000000000000000002
            0000000000000000000011111111111111111111111111111111111111111111
            0000000000000000000000002222222222222222222222222222222222222222
            0000000000000000000000000000000000000000000000000000000000000002
            0000000000000000000033333333333333333333333333333333333333333333
            0000000000000000000044444444444444444444444444444444444444444444
        "),

        fixed_array_of_fixed_arrays("address[2][2]", "
            0000000000000000000011111111111111111111111111111111111111111111
            0000000000000000000022222222222222222222222222222222222222222222
            0000000000000000000033333333333333333333333333333333333333333333
            0000000000000000000044444444444444444444444444444444444444444444
        "),

        fixed_array_of_static_tuples_followed_by_dynamic_type("((uint256,uint256,address)[2],string)", "
            0000000000000000000000000000000000000000000000000000000005930cc5
            0000000000000000000000000000000000000000000000000000000015002967
            0000000000000000000044444444444444444444444444444444444444444444
            000000000000000000000000000000000000000000000000000000000000307b
            00000000000000000000000000000000000000000000000000000000000001c3
            0000000000000000000022222222222222222222222222222222222222222222
            00000000000000000000000000000000000000000000000000000000000000e0
            0000000000000000000000000000000000000000000000000000000000000009
            6761766f66796f726b0000000000000000000000000000000000000000000000
        "),

        empty_array("address[]", "
            0000000000000000000000000000000000000000000000000000000000000020
            0000000000000000000000000000000000000000000000000000000000000000
        "),

        empty_array_2("(address[],address[])", "
            0000000000000000000000000000000000000000000000000000000000000040
            0000000000000000000000000000000000000000000000000000000000000060
            0000000000000000000000000000000000000000000000000000000000000000
            0000000000000000000000000000000000000000000000000000000000000000
        "),

        // Nested empty arrays
        empty_array_3("(address[][],address[][])", "
            0000000000000000000000000000000000000000000000000000000000000040
            00000000000000000000000000000000000000000000000000000000000000a0
            0000000000000000000000000000000000000000000000000000000000000001
            0000000000000000000000000000000000000000000000000000000000000020
            0000000000000000000000000000000000000000000000000000000000000000
            0000000000000000000000000000000000000000000000000000000000000001
            0000000000000000000000000000000000000000000000000000000000000020
            0000000000000000000000000000000000000000000000000000000000000000
        "),

        fixed_bytes("bytes2", "1234000000000000000000000000000000000000000000000000000000000000"),

        string("string", "
            0000000000000000000000000000000000000000000000000000000000000020
            0000000000000000000000000000000000000000000000000000000000000009
            6761766f66796f726b0000000000000000000000000000000000000000000000
        "),

        bytes("bytes", "
            0000000000000000000000000000000000000000000000000000000000000020
            0000000000000000000000000000000000000000000000000000000000000002
            1234000000000000000000000000000000000000000000000000000000000000
        "),

        bytes_2("bytes", "
            0000000000000000000000000000000000000000000000000000000000000020
            000000000000000000000000000000000000000000000000000000000000001f
            1000000000000000000000000000000000000000000000000000000000000200
        "),

        bytes_3("bytes", "
            0000000000000000000000000000000000000000000000000000000000000020
            0000000000000000000000000000000000000000000000000000000000000040
            1000000000000000000000000000000000000000000000000000000000000000
            1000000000000000000000000000000000000000000000000000000000000000
        "),

        two_bytes("(bytes,bytes)", "
            0000000000000000000000000000000000000000000000000000000000000040
            0000000000000000000000000000000000000000000000000000000000000080
            000000000000000000000000000000000000000000000000000000000000001f
            1000000000000000000000000000000000000000000000000000000000000200
            0000000000000000000000000000000000000000000000000000000000000020
            0010000000000000000000000000000000000000000000000000000000000002
        "),

        uint("uint256", "0000000000000000000000000000000000000000000000000000000000000004"),

        int("int256", "0000000000000000000000000000000000000000000000000000000000000004"),

        bool("bool", "0000000000000000000000000000000000000000000000000000000000000001"),

        bool2("bool", "0000000000000000000000000000000000000000000000000000000000000000"),

        comprehensive_test("(uint8,bytes,uint8,bytes)", "
            0000000000000000000000000000000000000000000000000000000000000005
            0000000000000000000000000000000000000000000000000000000000000080
            0000000000000000000000000000000000000000000000000000000000000003
            00000000000000000000000000000000000000000000000000000000000000e0
            0000000000000000000000000000000000000000000000000000000000000040
            131a3afc00d1b1e3461b955e53fc866dcf303b3eb9f4c16f89e388930f48134b
            131a3afc00d1b1e3461b955e53fc866dcf303b3eb9f4c16f89e388930f48134b
            0000000000000000000000000000000000000000000000000000000000000040
            131a3afc00d1b1e3461b955e53fc866dcf303b3eb9f4c16f89e388930f48134b
            131a3afc00d1b1e3461b955e53fc866dcf303b3eb9f4c16f89e388930f48134b
        "),

        comprehensive_test2("(bool,string,uint8,uint8,uint8,uint8[])", "
            0000000000000000000000000000000000000000000000000000000000000001
            00000000000000000000000000000000000000000000000000000000000000c0
            0000000000000000000000000000000000000000000000000000000000000002
            0000000000000000000000000000000000000000000000000000000000000003
            0000000000000000000000000000000000000000000000000000000000000004
            0000000000000000000000000000000000000000000000000000000000000100
            0000000000000000000000000000000000000000000000000000000000000009
            6761766f66796f726b0000000000000000000000000000000000000000000000
            0000000000000000000000000000000000000000000000000000000000000003
            0000000000000000000000000000000000000000000000000000000000000005
            0000000000000000000000000000000000000000000000000000000000000006
            0000000000000000000000000000000000000000000000000000000000000007
        "),

        dynamic_array_of_bytes("bytes[]", "
            0000000000000000000000000000000000000000000000000000000000000020
            0000000000000000000000000000000000000000000000000000000000000001
            0000000000000000000000000000000000000000000000000000000000000020
            0000000000000000000000000000000000000000000000000000000000000026
            019c80031b20d5e69c8093a571162299032018d913930d93ab320ae5ea44a421
            8a274f00d6070000000000000000000000000000000000000000000000000000
        "),

        dynamic_array_of_bytes2("bytes[]", "
            0000000000000000000000000000000000000000000000000000000000000020
            0000000000000000000000000000000000000000000000000000000000000002
            0000000000000000000000000000000000000000000000000000000000000040
            00000000000000000000000000000000000000000000000000000000000000a0
            0000000000000000000000000000000000000000000000000000000000000026
            4444444444444444444444444444444444444444444444444444444444444444
            4444444444440000000000000000000000000000000000000000000000000000
            0000000000000000000000000000000000000000000000000000000000000026
            6666666666666666666666666666666666666666666666666666666666666666
            6666666666660000000000000000000000000000000000000000000000000000
        "),

        static_tuple_of_addresses("(address,address)", "
            0000000000000000000000001111111111111111111111111111111111111111
            0000000000000000000000002222222222222222222222222222222222222222
        "),

        dynamic_tuple("((string,string),)", "
            0000000000000000000000000000000000000000000000000000000000000020
            0000000000000000000000000000000000000000000000000000000000000040
            0000000000000000000000000000000000000000000000000000000000000080
            0000000000000000000000000000000000000000000000000000000000000009
            6761766f66796f726b0000000000000000000000000000000000000000000000
            0000000000000000000000000000000000000000000000000000000000000009
            6761766f66796f726b0000000000000000000000000000000000000000000000
        "),

        dynamic_tuple_of_bytes("((bytes,bytes),)", "
            0000000000000000000000000000000000000000000000000000000000000020
            0000000000000000000000000000000000000000000000000000000000000040
            00000000000000000000000000000000000000000000000000000000000000a0
            0000000000000000000000000000000000000000000000000000000000000026
            4444444444444444444444444444444444444444444444444444444444444444
            4444444444440000000000000000000000000000000000000000000000000000
            0000000000000000000000000000000000000000000000000000000000000026
            6666666666666666666666666666666666666666666666666666666666666666
            6666666666660000000000000000000000000000000000000000000000000000
        "),

        complex_tuple("((uint256,string,address,address),)", "
            0000000000000000000000000000000000000000000000000000000000000020
            1111111111111111111111111111111111111111111111111111111111111111
            0000000000000000000000000000000000000000000000000000000000000080
            0000000000000000000000001111111111111111111111111111111111111111
            0000000000000000000000002222222222222222222222222222222222222222
            0000000000000000000000000000000000000000000000000000000000000009
            6761766f66796f726b0000000000000000000000000000000000000000000000
        "),

        nested_tuple("((string,bool,string,(string,string,(string,string))),)", "
            0000000000000000000000000000000000000000000000000000000000000020
            0000000000000000000000000000000000000000000000000000000000000080
            0000000000000000000000000000000000000000000000000000000000000001
            00000000000000000000000000000000000000000000000000000000000000c0
            0000000000000000000000000000000000000000000000000000000000000100
            0000000000000000000000000000000000000000000000000000000000000004
            7465737400000000000000000000000000000000000000000000000000000000
            0000000000000000000000000000000000000000000000000000000000000006
            6379626f72670000000000000000000000000000000000000000000000000000
            0000000000000000000000000000000000000000000000000000000000000060
            00000000000000000000000000000000000000000000000000000000000000a0
            00000000000000000000000000000000000000000000000000000000000000e0
            0000000000000000000000000000000000000000000000000000000000000005
            6e69676874000000000000000000000000000000000000000000000000000000
            0000000000000000000000000000000000000000000000000000000000000003
            6461790000000000000000000000000000000000000000000000000000000000
            0000000000000000000000000000000000000000000000000000000000000040
            0000000000000000000000000000000000000000000000000000000000000080
            0000000000000000000000000000000000000000000000000000000000000004
            7765656500000000000000000000000000000000000000000000000000000000
            0000000000000000000000000000000000000000000000000000000000000008
            66756e7465737473000000000000000000000000000000000000000000000000
        "),

        params_containing_dynamic_tuple("(address,(bool,string,string),address,address,bool)", "
            0000000000000000000000002222222222222222222222222222222222222222
            00000000000000000000000000000000000000000000000000000000000000a0
            0000000000000000000000003333333333333333333333333333333333333333
            0000000000000000000000004444444444444444444444444444444444444444
            0000000000000000000000000000000000000000000000000000000000000000
            0000000000000000000000000000000000000000000000000000000000000001
            0000000000000000000000000000000000000000000000000000000000000060
            00000000000000000000000000000000000000000000000000000000000000a0
            0000000000000000000000000000000000000000000000000000000000000009
            7370616365736869700000000000000000000000000000000000000000000000
            0000000000000000000000000000000000000000000000000000000000000006
            6379626f72670000000000000000000000000000000000000000000000000000
        "),

        params_containing_static_tuple("(address,(address,bool,bool),address,address)", "
            0000000000000000000000001111111111111111111111111111111111111111
            0000000000000000000000002222222222222222222222222222222222222222
            0000000000000000000000000000000000000000000000000000000000000001
            0000000000000000000000000000000000000000000000000000000000000000
            0000000000000000000000003333333333333333333333333333333333333333
            0000000000000000000000004444444444444444444444444444444444444444
        "),

        dynamic_tuple_with_nested_static_tuples("((((bool,uint16),),uint16[]),)", "
            0000000000000000000000000000000000000000000000000000000000000020
            0000000000000000000000000000000000000000000000000000000000000000
            0000000000000000000000000000000000000000000000000000000000000777
            0000000000000000000000000000000000000000000000000000000000000060
            0000000000000000000000000000000000000000000000000000000000000002
            0000000000000000000000000000000000000000000000000000000000000042
            0000000000000000000000000000000000000000000000000000000000001337
        "),
    }

    // https://github.com/core-coin/base-rs/issues/392
    #[test]
    fn zst_dos() {
        let my_type: DynYlmType = "()[]".parse().unwrap();
        let value = my_type.abi_decode(&hex!("000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000FFFFFFFF"));
        assert_eq!(value, Ok(DynYlmValue::Array(vec![])));
    }

    #[test]
    #[cfg_attr(miri, ignore = "takes too long")]
    fn recursive_dos() {
        // https://github.com/core-coin/base-rs/issues/490
        let payload = "0000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000000a0000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000020";

        // Used to eat 60 gb of memory and then crash.
        let my_type: DynYlmType = "uint256[][][][][][][][][][]".parse().unwrap();
        let decoded = my_type.abi_decode(&hex::decode(payload).unwrap());
        assert_eq!(decoded, Err(base_ylm_types::Error::RecursionLimitExceeded(16).into()));

        // https://github.com/paulmillr/micro-eth-signer/discussions/20
        let payload = &"0000000000000000000000000000000000000000000000000000000000000020\
             000000000000000000000000000000000000000000000000000000000000000a\
             0000000000000000000000000000000000000000000000000000000000000020"
            .repeat(64);
        let my_type: DynYlmType = "uint256[][][][][][][][][][]".parse().unwrap();
        let decoded = my_type.abi_decode(&hex::decode(payload).unwrap());
        assert_eq!(decoded, Err(base_ylm_types::Error::RecursionLimitExceeded(16).into()));

        let my_type: DynYlmType = "bytes[][][][][][][][][][]".parse().unwrap();
        let decoded = my_type.abi_decode(&hex::decode(payload).unwrap());
        assert_eq!(decoded, Err(base_ylm_types::Error::RecursionLimitExceeded(16).into()));
    }

    // https://github.com/core-coin/base-rs/issues/490
    #[test]
    fn large_dyn_array_dos() {
        let payload = "000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000FFFFFFFF";

        // Used to eat 60 gb of memory.
        let my_type: DynYlmType = "uint32[1][]".parse().unwrap();
        let decoded = my_type.abi_decode(&hex::decode(payload).unwrap());
        assert_eq!(decoded, Err(base_ylm_types::Error::Overrun.into()))
    }

    #[test]
    fn fixed_array_dos() {
        let t = "uint32[9999999999]".parse::<DynYlmType>().unwrap();
        let decoded = t.abi_decode(&[]);
        assert_eq!(decoded, Err(base_ylm_types::Error::Overrun.into()))
    }

    macro_rules! packed_tests {
        ($($name:ident($ty:literal, $v:literal, $encoded:literal)),* $(,)?) => {
            mod packed {
                use super::*;

                $(
                    #[test]
                    fn $name() {
                        packed_test($ty, $v, &hex!($encoded));
                    }
                )*
            }
        };
    }

    fn packed_test(t_s: &str, v_s: &str, expected: &[u8]) {
        let ty: DynYlmType = t_s.parse().expect("parsing failed");
        assert_eq!(ty.ylm_type_name(), t_s, "type names are not the same");

        let value = match ty.coerce_str(v_s) {
            Ok(v) => v,
            Err(e) => {
                panic!("failed to coerce to a value: {e}");
            }
        };
        if let Some(value_name) = value.ylm_type_name() {
            assert_eq!(value_name, t_s, "value names are not the same");
        }

        let packed = value.abi_encode_packed();
        assert!(
            packed == expected,
            "
  type: {ty}
 value: {value:?}
packed: {packed}
expect: {expected}",
            packed = hex::encode(packed),
            expected = hex::encode(expected),
        );
    }

    packed_tests! {
        address("address", "11111111111111111111111111111111111111111111", "11111111111111111111111111111111111111111111"),

        bool_false("bool", "false", "00"),
        bool_true("bool", "true", "01"),

        int8_1("int8", "0", "00"),
        int8_2("int8", "1", "01"),
        int8_3("int8", "16", "10"),
        int8_4("int8", "127", "7f"),
        neg_int8_1("int8", "-1", "ff"),
        neg_int8_2("int8", "-16", "f0"),
        neg_int8_3("int8", "-127", "81"),
        neg_int8_4("int8", "-128", "80"),

        int16_1("int16", "0", "0000"),
        int16_2("int16", "1", "0001"),
        int16_3("int16", "16", "0010"),
        int16_4("int16", "127", "007f"),
        int16_5("int16", "128", "0080"),
        int16_6("int16", "8192", "2000"),
        int16_7("int16", "32767", "7fff"),
        neg_int16_1("int16", "-1", "ffff"),
        neg_int16_2("int16", "-16", "fff0"),
        neg_int16_3("int16", "-127", "ff81"),
        neg_int16_4("int16", "-128", "ff80"),
        neg_int16_5("int16", "-129", "ff7f"),
        neg_int16_6("int16", "-32767", "8001"),
        neg_int16_7("int16", "-32768", "8000"),

        int32_1("int32", "0", "00000000"),
        int32_2("int32", "-1", "ffffffff"),
        int64_1("int64", "0", "0000000000000000"),
        int64_2("int64", "-1", "ffffffffffffffff"),
        int128_1("int128", "0", "00000000000000000000000000000000"),
        int128_2("int128", "-1", "ffffffffffffffffffffffffffffffff"),
        int256_1("int256", "0", "0000000000000000000000000000000000000000000000000000000000000000"),
        int256_2("int256", "-1", "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"),

        uint8_1("uint8", "0", "00"),
        uint8_2("uint8", "1", "01"),
        uint8_3("uint8", "16", "10"),
        uint16("uint16", "0", "0000"),
        uint32("uint32", "0", "00000000"),
        uint64("uint64", "0", "0000000000000000"),
        uint128("uint128", "0", "00000000000000000000000000000000"),
        uint256_1("uint256", "0", "0000000000000000000000000000000000000000000000000000000000000000"),
        uint256_2("uint256", "42", "000000000000000000000000000000000000000000000000000000000000002a"),
        uint256_3("uint256", "115792089237316195423570985008687907853269984665640564039457584007913129639935", "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"),

        string_1("string", "a", "61"),
        string_2("string", "ab", "6162"),
        string_3("string", "abc", "616263"),

        bytes_1("bytes", "00", "00"),
        bytes_2("bytes", "0001", "0001"),
        bytes_3("bytes", "000102", "000102"),

        dynamic_array_of_addresses("address[]", "[\
            11111111111111111111111111111111111111111111,\
            22222222222222222222222222222222222222222222\
        ]", "
            0000000000000000000011111111111111111111111111111111111111111111
            0000000000000000000022222222222222222222222222222222222222222222
        "),

        fixed_array_of_addresses("address[2]", "[\
            11111111111111111111111111111111111111111111,\
            22222222222222222222222222222222222222222222\
        ]", "
            0000000000000000000011111111111111111111111111111111111111111111
            0000000000000000000022222222222222222222222222222222222222222222
        "),

        two_addresses("(address,address)", "(\
            11111111111111111111111111111111111111111111,\
            22222222222222222222222222222222222222222222\
        )", "
            11111111111111111111111111111111111111111111
            22222222222222222222222222222222222222222222
        "),

        fixed_array_of_dynamic_arrays_of_addresses("address[][2]", "[\
            [11111111111111111111111111111111111111111111, 22222222222222222222222222222222222222222222],\
            [33333333333333333333333333333333333333333333, 44444444444444444444444444444444444444444444]\
        ]", "
            0000000000000000000011111111111111111111111111111111111111111111
            0000000000000000000022222222222222222222222222222222222222222222
            0000000000000000000033333333333333333333333333333333333333333333
            0000000000000000000044444444444444444444444444444444444444444444
        "),

        dynamic_array_of_fixed_arrays_of_addresses("address[2][]", "[\
            [11111111111111111111111111111111111111111111, 22222222222222222222222222222222222222222222],\
            [33333333333333333333333333333333333333333333, 44444444444444444444444444444444444444444444]\
        ]", "
            0000000000000000000011111111111111111111111111111111111111111111
            0000000000000000000022222222222222222222222222222222222222222222
            0000000000000000000033333333333333333333333333333333333333333333
            0000000000000000000044444444444444444444444444444444444444444444
        "),

        dynamic_array_of_dynamic_arrays("address[][]", "[\
            [11111111111111111111111111111111111111111111],\
            [22222222222222222222222222222222222222222222]\
        ]", "
            0000000000000000000011111111111111111111111111111111111111111111
            0000000000000000000022222222222222222222222222222222222222222222
        "),

        dynamic_array_of_dynamic_arrays2("address[][]", "[\
            [11111111111111111111111111111111111111111111, 22222222222222222222222222222222222222222222],\
            [33333333333333333333333333333333333333333333, 44444444444444444444444444444444444444444444]\
        ]", "
            0000000000000000000011111111111111111111111111111111111111111111
            0000000000000000000022222222222222222222222222222222222222222222
            0000000000000000000033333333333333333333333333333333333333333333
            0000000000000000000044444444444444444444444444444444444444444444
        "),

        dynamic_array_of_dynamic_arrays3("uint32[][]", "[\
            [1, 2],\
            [3, 4]\
        ]", "
            0000000000000000000000000000000000000000000000000000000000000001
            0000000000000000000000000000000000000000000000000000000000000002
            0000000000000000000000000000000000000000000000000000000000000003
            0000000000000000000000000000000000000000000000000000000000000004
        "),
    }
}
