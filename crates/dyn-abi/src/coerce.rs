use crate::{dynamic::ty::as_tuple, DynYlmType, DynYlmValue, Result};
use alloc::vec::Vec;
use base_primitives::{Function, IcanAddress, Sign, I256, U256};
use base_ylm_types::Word;
use core::fmt;
use hex::FromHexError;
use parser::utils::{array_parser, char_parser, spanned};
use winnow::{
    ascii::{alpha0, alpha1, digit1, hex_digit0, hex_digit1, space0},
    combinator::{cut_err, dispatch, empty, fail, opt, preceded, trace},
    error::{
        AddContext, ContextError, ErrMode, ErrorKind, FromExternalError, StrContext,
        StrContextValue,
    },
    stream::Stream,
    token::take_while,
    PResult, Parser,
};

impl DynYlmType {
    /// Coerces a string into a [`DynYlmValue`] via this type.
    ///
    /// # Syntax
    ///
    /// - [`Bool`](DynYlmType::Bool): `true|false`
    /// - [`Int`](DynYlmType::Int): `[+-]?{Uint}`
    /// - [`Uint`](DynYlmType::Uint): `{literal}(\.[0-9]+)?(\s*{unit})?`
    ///   - literal: base 2, 8, 10, or 16 integer literal. If not in base 10, must be prefixed with
    ///     `0b`, `0o`, or `0x` respectively.
    ///   - unit: same as [Ylem ether units](https://docs.soliditylang.org/en/latest/units-and-global-variables.html#ether-units)
    ///   - decimals with more digits than the unit's exponent value are not allowed
    /// - [`FixedBytes`](DynYlmType::FixedBytes): `(0x)?[0-9A-Fa-f]{$0*2}`
    /// - [`IcanAddress`](DynYlmType::Address): `[0-9A-Fa-f]{44}`
    /// - [`Function`](DynYlmType::Function): `(0x)?[0-9A-Fa-f]{48}`
    /// - [`Bytes`](DynYlmType::Bytes): `(0x)?[0-9A-Fa-f]+`
    /// - [`String`](DynYlmType::String): `.*`
    ///   - can be surrounded by a pair of `"` or `'`
    ///   - trims whitespace if not surrounded
    /// - [`Array`](DynYlmType::Array): any number of the inner type delimited by commas (`,`) and
    ///   surrounded by brackets (`[]`)
    /// - [`FixedArray`](DynYlmType::FixedArray): exactly the given number of the inner type
    ///   delimited by commas (`,`) and surrounded by brackets (`[]`)
    /// - [`Tuple`](DynYlmType::Tuple): the inner types delimited by commas (`,`) and surrounded by
    ///   parentheses (`()`)
    #[cfg_attr(
        feature = "eip712",
        doc = "- [`CustomStruct`](DynYlmType::CustomStruct): the same as `Tuple`"
    )]
    ///
    /// # Examples
    ///
    /// ```
    /// use base_dyn_abi::{DynYlmType, DynYlmValue};
    /// use base_primitives::U256;
    ///
    /// let ty: DynYlmType = "(uint256,string)[]".parse()?;
    /// let value = ty.coerce_str("[(0, \"hello\"), (42, \"world\")]")?;
    /// assert_eq!(
    ///     value,
    ///     DynYlmValue::Array(vec![
    ///         DynYlmValue::Tuple(vec![
    ///             DynYlmValue::Uint(U256::from(0), 256),
    ///             DynYlmValue::String(String::from("hello"))
    ///         ]),
    ///         DynYlmValue::Tuple(vec![
    ///             DynYlmValue::Uint(U256::from(42), 256),
    ///             DynYlmValue::String(String::from("world"))
    ///         ]),
    ///     ])
    /// );
    /// assert!(value.matches(&ty));
    /// assert_eq!(value.as_type().unwrap(), ty);
    /// # Ok::<_, base_dyn_abi::Error>(())
    /// ```
    #[doc(alias = "tokenize")] // from ethabi
    pub fn coerce_str(&self, s: &str) -> Result<DynYlmValue> {
        ValueParser::new(self)
            .parse(s)
            .map_err(|e| crate::Error::TypeParser(parser::Error::parser(e)))
    }
}

struct ValueParser<'a> {
    ty: &'a DynYlmType,
    list_end: Option<char>,
}

impl<'i> Parser<&'i str, DynYlmValue, ContextError> for ValueParser<'_> {
    fn parse_next(&mut self, input: &mut &'i str) -> PResult<DynYlmValue, ContextError> {
        #[cfg(feature = "debug")]
        let name = self.ty.ylm_type_name();
        #[cfg(not(feature = "debug"))]
        let name = "value_parser";
        trace(name, move |input: &mut &str| match self.ty {
            DynYlmType::Bool => bool(input).map(DynYlmValue::Bool),
            &DynYlmType::Int(size) => {
                int(size).parse_next(input).map(|int| DynYlmValue::Int(int, size))
            }
            &DynYlmType::Uint(size) => {
                uint(size).parse_next(input).map(|uint| DynYlmValue::Uint(uint, size))
            }
            &DynYlmType::FixedBytes(size) => {
                fixed_bytes(size).parse_next(input).map(|word| DynYlmValue::FixedBytes(word, size))
            }
            DynYlmType::Address => address(input).map(DynYlmValue::Address),
            DynYlmType::Function => function(input).map(DynYlmValue::Function),
            DynYlmType::Bytes => bytes(input).map(DynYlmValue::Bytes),
            DynYlmType::String => {
                self.string().parse_next(input).map(|s| DynYlmValue::String(s.into()))
            }
            DynYlmType::Array(ty) => self.in_list(']', |this| {
                this.with(ty).array().parse_next(input).map(DynYlmValue::Array)
            }),
            DynYlmType::FixedArray(ty, len) => self.in_list(']', |this| {
                this.with(ty).fixed_array(*len).parse_next(input).map(DynYlmValue::FixedArray)
            }),
            as_tuple!(DynYlmType tys) => {
                self.in_list(')', |this| this.tuple(tys).parse_next(input).map(DynYlmValue::Tuple))
            }
        })
        .parse_next(input)
    }
}

impl<'a> ValueParser<'a> {
    #[inline]
    const fn new(ty: &'a DynYlmType) -> Self {
        Self { list_end: None, ty }
    }

    #[inline]
    fn in_list<F: FnOnce(&mut Self) -> R, R>(&mut self, list_end: char, f: F) -> R {
        let prev = core::mem::replace(&mut self.list_end, Some(list_end));
        let r = f(self);
        self.list_end = prev;
        r
    }

    #[inline]
    const fn with(&self, ty: &'a DynYlmType) -> Self {
        Self { list_end: self.list_end, ty }
    }

    #[inline]
    fn string<'s, 'i: 's>(&'s self) -> impl Parser<&'i str, &'i str, ContextError> + 's {
        trace("string", |input: &mut &'i str| {
            let Some(delim) = input.chars().next() else {
                return Ok("");
            };
            let has_delim = matches!(delim, '"' | '\'');
            if has_delim {
                *input = &input[1..];
            }

            // TODO: escapes?
            let mut s = if has_delim || self.list_end.is_some() {
                let (chs, l) = if has_delim {
                    ([delim, '\0'], 1)
                } else if let Some(c) = self.list_end {
                    ([',', c], 2)
                } else {
                    unreachable!()
                };
                let min = if has_delim { 0 } else { 1 };
                take_while(min.., move |c: char| !unsafe { chs.get_unchecked(..l) }.contains(&c))
                    .parse_next(input)?
            } else {
                input.next_slice(input.len())
            };

            if has_delim {
                cut_err(char_parser(delim))
                    .context(StrContext::Label("string"))
                    .parse_next(input)?;
            } else {
                s = s.trim_end();
            }

            Ok(s)
        })
    }

    #[inline]
    fn array<'i: 'a>(self) -> impl Parser<&'i str, Vec<DynYlmValue>, ContextError> + 'a {
        #[cfg(feature = "debug")]
        let name = format!("{}[]", self.ty);
        #[cfg(not(feature = "debug"))]
        let name = "array";
        trace(name, array_parser(self))
    }

    #[inline]
    fn fixed_array<'i: 'a>(
        self,
        len: usize,
    ) -> impl Parser<&'i str, Vec<DynYlmValue>, ContextError> + 'a {
        #[cfg(feature = "debug")]
        let name = format!("{}[{len}]", self.ty);
        #[cfg(not(feature = "debug"))]
        let name = "fixed_array";
        trace(
            name,
            array_parser(self).try_map(move |values: Vec<DynYlmValue>| {
                if values.len() == len {
                    Ok(values)
                } else {
                    Err(Error::FixedArrayLengthMismatch(len, values.len()))
                }
            }),
        )
    }

    #[inline]
    #[allow(clippy::ptr_arg)]
    fn tuple<'i: 's, 't: 's, 's>(
        &'s self,
        tuple: &'t Vec<DynYlmType>,
    ) -> impl Parser<&'i str, Vec<DynYlmValue>, ContextError> + 's {
        #[cfg(feature = "debug")]
        let name = DynYlmType::Tuple(tuple.clone()).to_string();
        #[cfg(not(feature = "debug"))]
        let name = "tuple";
        trace(name, move |input: &mut &'i str| {
            space0(input)?;
            char_parser('(').parse_next(input)?;

            let mut values = Vec::with_capacity(tuple.len());
            for (i, ty) in tuple.iter().enumerate() {
                if i > 0 {
                    space0(input)?;
                    char_parser(',').parse_next(input)?;
                }
                space0(input)?;
                values.push(self.with(ty).parse_next(input)?);
            }

            space0(input)?;
            char_parser(')').parse_next(input)?;

            Ok(values)
        })
    }
}

#[derive(Debug)]
enum Error {
    IntOverflow,
    FractionalNotAllowed(U256),
    TooManyDecimals(usize, usize),
    InvalidFixedBytesLength(usize),
    FixedArrayLengthMismatch(usize, usize),
    EmptyHexStringWithoutPrefix,
}

#[cfg(feature = "std")]
impl std::error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IntOverflow => f.write_str("number too large to fit in target type"),
            Self::TooManyDecimals(expected, actual) => {
                write!(f, "expected at most {expected} decimals, got {actual}")
            }
            Self::FractionalNotAllowed(n) => write!(
                f,
                "non-zero fraction 0.{n} not allowed without specifying non-wei units (gwei, ether, etc.)"
            ),
            Self::InvalidFixedBytesLength(len) => {
                write!(f, "fixed bytes length {len} greater than 32")
            }
            Self::FixedArrayLengthMismatch(expected, actual) => write!(
                f,
                "fixed array length mismatch: expected {expected} elements, got {actual}"
            ),
            Self::EmptyHexStringWithoutPrefix => f.write_str("expected hex digits or the `0x` prefix for an empty hex string"),
        }
    }
}

#[inline]
fn bool(input: &mut &str) -> PResult<bool> {
    trace(
        "bool",
        dispatch! {alpha1.context(StrContext::Label("boolean"));
            "true" => empty.value(true),
            "false" => empty.value(false),
            _ => fail
        }
        .context(StrContext::Label("boolean")),
    )
    .parse_next(input)
}

#[inline]
fn int<'i>(size: usize) -> impl Parser<&'i str, I256, ContextError> {
    #[cfg(feature = "debug")]
    let name = format!("int{size}");
    #[cfg(not(feature = "debug"))]
    let name = "int";
    trace(
        name,
        (int_sign, uint(size)).try_map(move |(sign, abs)| {
            if !sign.is_negative() && abs.bit_len() > size - 1 {
                return Err(Error::IntOverflow);
            }
            I256::checked_from_sign_and_abs(sign, abs).ok_or(Error::IntOverflow)
        }),
    )
}

#[inline]
fn int_sign(input: &mut &str) -> PResult<Sign> {
    trace("int_sign", |input: &mut &str| match input.as_bytes().first() {
        Some(b'+') => {
            *input = &input[1..];
            Ok(Sign::Positive)
        }
        Some(b'-') => {
            *input = &input[1..];
            Ok(Sign::Negative)
        }
        Some(_) | None => Ok(Sign::Positive),
    })
    .parse_next(input)
}

#[inline]
fn uint<'i>(len: usize) -> impl Parser<&'i str, U256, ContextError> {
    #[cfg(feature = "debug")]
    let name = format!("uint{len}");
    #[cfg(not(feature = "debug"))]
    let name = "uint";
    trace(name, move |input: &mut &str| {
        let (s, (intpart, fract)) = spanned((
            prefixed_int,
            opt(preceded(
                '.',
                cut_err(digit1.context(StrContext::Expected(StrContextValue::Description(
                    "at least one digit",
                )))),
            )),
        ))
        .parse_next(input)?;

        let _ = space0(input)?;
        let units = int_units(input)?;

        let uint = if let Some(fract) = fract {
            let fract_uint = U256::from_str_radix(fract, 10)
                .map_err(|e| ErrMode::from_external_error(input, ErrorKind::Verify, e))?;

            if units == 0 && !fract_uint.is_zero() {
                return Err(ErrMode::from_external_error(
                    input,
                    ErrorKind::Verify,
                    Error::FractionalNotAllowed(fract_uint),
                ));
            }

            if fract.len() > units {
                return Err(ErrMode::from_external_error(
                    input,
                    ErrorKind::Verify,
                    Error::TooManyDecimals(units, fract.len()),
                ));
            }

            // (intpart * 10^fract.len() + fract) * 10^(units-fract.len())
            U256::from_str_radix(intpart, 10)
                .map_err(|e| ErrMode::from_external_error(input, ErrorKind::Verify, e))?
                .checked_mul(U256::from(10usize.pow(fract.len() as u32)))
                .and_then(|u| u.checked_add(fract_uint))
                .and_then(|u| u.checked_mul(U256::from(10usize.pow((units - fract.len()) as u32))))
                .ok_or_else(|| {
                    ErrMode::from_external_error(input, ErrorKind::Verify, Error::IntOverflow)
                })
        } else {
            s.parse::<U256>()
                .map_err(|e| ErrMode::from_external_error(input, ErrorKind::Verify, e))?
                .checked_mul(U256::from(10usize.pow(units as u32)))
                .ok_or_else(|| {
                    ErrMode::from_external_error(input, ErrorKind::Verify, Error::IntOverflow)
                })
        }?;

        if uint.bit_len() > len {
            return Err(ErrMode::from_external_error(input, ErrorKind::Verify, Error::IntOverflow));
        }

        Ok(uint)
    })
}

#[inline]
fn prefixed_int<'i>(input: &mut &'i str) -> PResult<&'i str> {
    trace("prefixed_int", |input: &mut &'i str| {
        let has_prefix = matches!(input.get(..2), Some("0b" | "0B" | "0o" | "0O" | "0x" | "0X"));
        let checkpoint = input.checkpoint();
        if has_prefix {
            *input = &input[2..];
            // parse hex since it's the most general
            hex_digit1(input)
        } else {
            digit1(input)
        }
        .map_err(|e| {
            e.add_context(
                input,
                &checkpoint,
                StrContext::Expected(StrContextValue::Description("at least one digit")),
            )
        })
    })
    .parse_next(input)
}

#[inline]
fn int_units(input: &mut &str) -> PResult<usize> {
    trace(
        "int_units",
        dispatch! {alpha0;
            "ether" => empty.value(18),
            "gwei" | "nano" | "nanoether" => empty.value(9),
            "" | "wei" => empty.value(0),
            _ => fail,
        },
    )
    .parse_next(input)
}

#[inline]
fn fixed_bytes<'i>(len: usize) -> impl Parser<&'i str, Word, ContextError> {
    #[cfg(feature = "debug")]
    let name = format!("bytes{len}");
    #[cfg(not(feature = "debug"))]
    let name = "bytesN";
    trace(name, move |input: &mut &str| {
        if len > Word::len_bytes() {
            return Err(ErrMode::from_external_error(
                input,
                ErrorKind::Fail,
                Error::InvalidFixedBytesLength(len),
            )
            .cut());
        }

        let hex = hex_str(input)?;
        let mut out = Word::ZERO;
        match hex::decode_to_slice(hex, &mut out[..len]) {
            Ok(()) => Ok(out),
            Err(e) => Err(hex_error(input, e).cut()),
        }
    })
}

#[inline]
fn address(input: &mut &str) -> PResult<IcanAddress> {
    trace("address", hex_str.try_map(hex::FromHex::from_hex)).parse_next(input)
}

#[inline]
fn function(input: &mut &str) -> PResult<Function> {
    trace("function", hex_str.try_map(hex::FromHex::from_hex)).parse_next(input)
}

#[inline]
fn bytes(input: &mut &str) -> PResult<Vec<u8>> {
    trace("bytes", hex_str.try_map(hex::decode)).parse_next(input)
}

#[inline]
fn hex_str<'i>(input: &mut &'i str) -> PResult<&'i str> {
    trace("hex_str", |input: &mut &'i str| {
        // Allow empty `bytes` only with a prefix.
        let has_prefix = opt("0x").parse_next(input)?.is_some();
        let s = hex_digit0(input)?;
        if !has_prefix && s.is_empty() {
            return Err(ErrMode::from_external_error(
                input,
                ErrorKind::Verify,
                Error::EmptyHexStringWithoutPrefix,
            ));
        }
        Ok(s)
    })
    .parse_next(input)
}

fn hex_error(input: &&str, e: FromHexError) -> ErrMode<ContextError> {
    let kind = match e {
        FromHexError::InvalidHexCharacter { .. } => unreachable!("{e:?}"),
        FromHexError::InvalidStringLength | FromHexError::OddLength => ErrorKind::Eof,
    };
    ErrMode::from_external_error(input, kind, e)
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::{
        boxed::Box,
        string::{String, ToString},
    };
    use base_primitives::{address, cAddress};
    use core::str::FromStr;

    #[track_caller]
    fn assert_error_contains(e: &impl core::fmt::Display, s: &str) {
        if cfg!(feature = "std") {
            let es = e.to_string();
            assert!(es.contains(s), "{s:?} not in {es:?}");
        }
    }

    #[test]
    fn coerce_bool() {
        assert_eq!(DynYlmType::Bool.coerce_str("true").unwrap(), DynYlmValue::Bool(true));
        assert_eq!(DynYlmType::Bool.coerce_str("false").unwrap(), DynYlmValue::Bool(false));

        assert!(DynYlmType::Bool.coerce_str("").is_err());
        assert!(DynYlmType::Bool.coerce_str("0").is_err());
        assert!(DynYlmType::Bool.coerce_str("1").is_err());
        assert!(DynYlmType::Bool.coerce_str("tru").is_err());
    }

    #[test]
    fn coerce_int() {
        assert_eq!(
            DynYlmType::Int(256)
                .coerce_str("0x1111111111111111111111111111111111111111111111111111111111111111")
                .unwrap(),
            DynYlmValue::Int(I256::from_be_bytes([0x11; 32]), 256)
        );

        assert_eq!(
            DynYlmType::Int(256)
                .coerce_str("0x2222222222222222222222222222222222222222222222222222222222222222")
                .unwrap(),
            DynYlmValue::Int(I256::from_be_bytes([0x22; 32]), 256)
        );

        assert_eq!(
            DynYlmType::Int(256)
                .coerce_str("0x7fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff")
                .unwrap(),
            DynYlmValue::Int(I256::MAX, 256)
        );
        assert!(DynYlmType::Int(256)
            .coerce_str("0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff")
            .is_err());

        assert_eq!(
            DynYlmType::Int(256).coerce_str("0").unwrap(),
            DynYlmValue::Int(I256::ZERO, 256)
        );

        assert_eq!(
            DynYlmType::Int(256).coerce_str("-0").unwrap(),
            DynYlmValue::Int(I256::ZERO, 256)
        );

        assert_eq!(
            DynYlmType::Int(256).coerce_str("+0").unwrap(),
            DynYlmValue::Int(I256::ZERO, 256)
        );

        assert_eq!(
            DynYlmType::Int(256).coerce_str("-1").unwrap(),
            DynYlmValue::Int(I256::MINUS_ONE, 256)
        );

        assert_eq!(
            DynYlmType::Int(256)
                .coerce_str(
                    "57896044618658097711785492504343953926634992332820282019728792003956564819967"
                )
                .unwrap(),
            DynYlmValue::Int(I256::MAX, 256)
        );
        assert_eq!(
            DynYlmType::Int(256).coerce_str("-57896044618658097711785492504343953926634992332820282019728792003956564819968").unwrap(),
            DynYlmValue::Int(I256::MIN, 256)
        );
    }

    #[test]
    fn coerce_int_overflow() {
        assert_eq!(
            DynYlmType::Int(8).coerce_str("126").unwrap(),
            DynYlmValue::Int(I256::try_from(126).unwrap(), 8),
        );
        assert_eq!(
            DynYlmType::Int(8).coerce_str("127").unwrap(),
            DynYlmValue::Int(I256::try_from(127).unwrap(), 8),
        );
        assert!(DynYlmType::Int(8).coerce_str("128").is_err());
        assert!(DynYlmType::Int(8).coerce_str("129").is_err());
        assert_eq!(
            DynYlmType::Int(16).coerce_str("128").unwrap(),
            DynYlmValue::Int(I256::try_from(128).unwrap(), 16),
        );
        assert_eq!(
            DynYlmType::Int(16).coerce_str("129").unwrap(),
            DynYlmValue::Int(I256::try_from(129).unwrap(), 16),
        );

        assert_eq!(
            DynYlmType::Int(8).coerce_str("-1").unwrap(),
            DynYlmValue::Int(I256::MINUS_ONE, 8),
        );
        assert_eq!(
            DynYlmType::Int(16).coerce_str("-1").unwrap(),
            DynYlmValue::Int(I256::MINUS_ONE, 16),
        );
    }

    #[test]
    fn coerce_uint() {
        assert_eq!(
            DynYlmType::Uint(256)
                .coerce_str("0x1111111111111111111111111111111111111111111111111111111111111111")
                .unwrap(),
            DynYlmValue::Uint(U256::from_be_bytes([0x11; 32]), 256)
        );

        assert_eq!(
            DynYlmType::Uint(256)
                .coerce_str("0x2222222222222222222222222222222222222222222222222222222222222222")
                .unwrap(),
            DynYlmValue::Uint(U256::from_be_bytes([0x22; 32]), 256)
        );

        assert_eq!(
            DynYlmType::Uint(256)
                .coerce_str("0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff")
                .unwrap(),
            DynYlmValue::Uint(U256::from_be_bytes([0xff; 32]), 256)
        );

        // 255 bits fails
        assert!(DynYlmType::Uint(255)
            .coerce_str("0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff")
            .is_err());

        assert_eq!(
            DynYlmType::Uint(256)
                .coerce_str("115792089237316195423570985008687907853269984665640564039457584007913129639935")
                .unwrap(),
            DynYlmValue::Uint(U256::MAX, 256)
        );

        assert_eq!(
            DynYlmType::Uint(256).coerce_str("0").unwrap(),
            DynYlmValue::Uint(U256::ZERO, 256)
        );

        assert_eq!(
            DynYlmType::Uint(256).coerce_str("1").unwrap(),
            DynYlmValue::Uint(U256::from(1), 256)
        );
    }

    #[test]
    fn coerce_uint_overflow() {
        assert_eq!(
            DynYlmType::Uint(8).coerce_str("254").unwrap(),
            DynYlmValue::Uint(U256::from(254), 8),
        );
        assert_eq!(
            DynYlmType::Uint(8).coerce_str("255").unwrap(),
            DynYlmValue::Uint(U256::from(255), 8),
        );
        assert!(DynYlmType::Uint(8).coerce_str("256").is_err());
        assert!(DynYlmType::Uint(8).coerce_str("257").is_err());
        assert_eq!(
            DynYlmType::Uint(16).coerce_str("256").unwrap(),
            DynYlmValue::Uint(U256::from(256), 16),
        );
        assert_eq!(
            DynYlmType::Uint(16).coerce_str("257").unwrap(),
            DynYlmValue::Uint(U256::from(257), 16),
        );
    }

    #[test]
    fn coerce_uint_wei() {
        assert_eq!(
            DynYlmType::Uint(256).coerce_str("1wei").unwrap(),
            DynYlmValue::Uint(U256::from(1), 256)
        );
        assert_eq!(
            DynYlmType::Uint(256).coerce_str("1 wei").unwrap(),
            DynYlmValue::Uint(U256::from(1), 256)
        );

        assert!(DynYlmType::Uint(256).coerce_str("1").is_ok());
        assert!(DynYlmType::Uint(256).coerce_str("1.").is_err());
        assert!(DynYlmType::Uint(256).coerce_str("1 .").is_err());
        assert!(DynYlmType::Uint(256).coerce_str("1 .0").is_err());
        assert!(DynYlmType::Uint(256).coerce_str("1.wei").is_err());
        assert!(DynYlmType::Uint(256).coerce_str("1. wei").is_err());
        assert!(DynYlmType::Uint(256).coerce_str("1.0wei").is_err());
        assert!(DynYlmType::Uint(256).coerce_str("1.0 wei").is_err());
        assert!(DynYlmType::Uint(256).coerce_str("1.00wei").is_err());
        assert!(DynYlmType::Uint(256).coerce_str("1.00 wei").is_err());
    }

    #[test]
    fn coerce_uint_gwei() {
        assert_eq!(
            DynYlmType::Uint(256).coerce_str("1nano").unwrap(),
            DynYlmValue::Uint(U256::from_str("1000000000").unwrap(), 256)
        );

        assert_eq!(
            DynYlmType::Uint(256).coerce_str("1nanoether").unwrap(),
            DynYlmValue::Uint(U256::from_str("1000000000").unwrap(), 256)
        );

        assert_eq!(
            DynYlmType::Uint(256).coerce_str("1gwei").unwrap(),
            DynYlmValue::Uint(U256::from_str("1000000000").unwrap(), 256)
        );

        assert_eq!(
            DynYlmType::Uint(256).coerce_str("0.1 gwei").unwrap(),
            DynYlmValue::Uint(U256::from_str("100000000").unwrap(), 256)
        );

        assert_eq!(
            DynYlmType::Uint(256).coerce_str("0.000000001gwei").unwrap(),
            DynYlmValue::Uint(U256::from(1), 256)
        );

        assert_eq!(
            DynYlmType::Uint(256).coerce_str("0.123456789gwei").unwrap(),
            DynYlmValue::Uint(U256::from_str("123456789").unwrap(), 256)
        );

        assert_eq!(
            DynYlmType::Uint(256).coerce_str("123456789123.123456789gwei").unwrap(),
            DynYlmValue::Uint(U256::from_str("123456789123123456789").unwrap(), 256)
        );
    }

    #[test]
    fn coerce_uint_ether() {
        assert_eq!(
            DynYlmType::Uint(256).coerce_str("10000000000ether").unwrap(),
            DynYlmValue::Uint(U256::from_str("10000000000000000000000000000").unwrap(), 256)
        );

        assert_eq!(
            DynYlmType::Uint(256).coerce_str("1ether").unwrap(),
            DynYlmValue::Uint(U256::from_str("1000000000000000000").unwrap(), 256)
        );

        assert_eq!(
            DynYlmType::Uint(256).coerce_str("0.01 ether").unwrap(),
            DynYlmValue::Uint(U256::from_str("10000000000000000").unwrap(), 256)
        );

        assert_eq!(
            DynYlmType::Uint(256).coerce_str("0.000000000000000001ether").unwrap(),
            DynYlmValue::Uint(U256::from(1), 256)
        );

        assert_eq!(
            DynYlmType::Uint(256).coerce_str("0.000000000000000001ether"),
            DynYlmType::Uint(256).coerce_str("1wei"),
        );

        assert_eq!(
            DynYlmType::Uint(256).coerce_str("0.123456789123456789ether").unwrap(),
            DynYlmValue::Uint(U256::from_str("123456789123456789").unwrap(), 256)
        );

        assert_eq!(
            DynYlmType::Uint(256).coerce_str("0.123456789123456000ether").unwrap(),
            DynYlmValue::Uint(U256::from_str("123456789123456000").unwrap(), 256)
        );

        assert_eq!(
            DynYlmType::Uint(256).coerce_str("0.1234567891234560ether").unwrap(),
            DynYlmValue::Uint(U256::from_str("123456789123456000").unwrap(), 256)
        );

        assert_eq!(
            DynYlmType::Uint(256).coerce_str("123456.123456789123456789ether").unwrap(),
            DynYlmValue::Uint(U256::from_str("123456123456789123456789").unwrap(), 256)
        );

        assert_eq!(
            DynYlmType::Uint(256).coerce_str("123456.123456789123456000ether").unwrap(),
            DynYlmValue::Uint(U256::from_str("123456123456789123456000").unwrap(), 256)
        );

        assert_eq!(
            DynYlmType::Uint(256).coerce_str("123456.1234567891234560ether").unwrap(),
            DynYlmValue::Uint(U256::from_str("123456123456789123456000").unwrap(), 256)
        );
    }

    #[test]
    fn coerce_uint_array_ether() {
        assert_eq!(
            DynYlmType::Array(Box::new(DynYlmType::Uint(256)))
                .coerce_str("[ 1   ether,  10 ether ]")
                .unwrap(),
            DynYlmValue::Array(vec![
                DynYlmValue::Uint(U256::from_str("1000000000000000000").unwrap(), 256),
                DynYlmValue::Uint(U256::from_str("10000000000000000000").unwrap(), 256),
            ])
        );
    }

    #[test]
    fn coerce_uint_invalid_units() {
        // 0.1 wei
        assert!(DynYlmType::Uint(256).coerce_str("0.1 wei").is_err());
        assert!(DynYlmType::Uint(256).coerce_str("0.0000000000000000001ether").is_err());

        // 1 ether + 0.1 wei
        assert!(DynYlmType::Uint(256).coerce_str("1.0000000000000000001ether").is_err());

        // 1_000_000_000 ether + 0.1 wei
        assert!(DynYlmType::Uint(256).coerce_str("1000000000.0000000000000000001ether").is_err());

        assert!(DynYlmType::Uint(256).coerce_str("0..1 gwei").is_err());

        assert!(DynYlmType::Uint(256).coerce_str("..1 gwei").is_err());

        assert!(DynYlmType::Uint(256).coerce_str("1. gwei").is_err());

        assert!(DynYlmType::Uint(256).coerce_str(".1 gwei").is_err());

        assert!(DynYlmType::Uint(256).coerce_str("2.1.1 gwei").is_err());

        assert!(DynYlmType::Uint(256).coerce_str(".1.1 gwei").is_err());

        assert!(DynYlmType::Uint(256).coerce_str("1abc").is_err());

        assert!(DynYlmType::Uint(256).coerce_str("1 gwei ").is_err());

        assert!(DynYlmType::Uint(256).coerce_str("g 1 gwei").is_err());

        assert!(DynYlmType::Uint(256).coerce_str("1gwei 1 gwei").is_err());
    }

    #[test]
    fn coerce_fixed_bytes() {
        let mk_word = |sl: &[u8]| {
            let mut out = Word::ZERO;
            out[..sl.len()].copy_from_slice(sl);
            out
        };

        // not actually valid, but we don't care here
        assert_eq!(
            DynYlmType::FixedBytes(0).coerce_str("0x").unwrap(),
            DynYlmValue::FixedBytes(mk_word(&[]), 0)
        );

        assert_eq!(
            DynYlmType::FixedBytes(1).coerce_str("0x00").unwrap(),
            DynYlmValue::FixedBytes(mk_word(&[0x00]), 1)
        );
        assert_eq!(
            DynYlmType::FixedBytes(1).coerce_str("0x00").unwrap(),
            DynYlmValue::FixedBytes(mk_word(&[0x00]), 1)
        );
        assert_eq!(
            DynYlmType::FixedBytes(2).coerce_str("0017").unwrap(),
            DynYlmValue::FixedBytes(mk_word(&[0x00, 0x17]), 2)
        );
        assert_eq!(
            DynYlmType::FixedBytes(3).coerce_str("123456").unwrap(),
            DynYlmValue::FixedBytes(mk_word(&[0x12, 0x34, 0x56]), 3)
        );

        let e = DynYlmType::FixedBytes(1).coerce_str("").unwrap_err();
        assert_error_contains(&e, &Error::EmptyHexStringWithoutPrefix.to_string());
        let e = DynYlmType::FixedBytes(1).coerce_str("0").unwrap_err();
        assert_error_contains(&e, &hex::FromHexError::OddLength.to_string());
        let e = DynYlmType::FixedBytes(1).coerce_str("0x").unwrap_err();
        assert_error_contains(&e, &hex::FromHexError::InvalidStringLength.to_string());
        let e = DynYlmType::FixedBytes(1).coerce_str("0x0").unwrap_err();
        assert_error_contains(&e, &hex::FromHexError::OddLength.to_string());

        let t = DynYlmType::Array(Box::new(DynYlmType::FixedBytes(1)));
        let e = t.coerce_str("[0]").unwrap_err();
        assert_error_contains(&e, &hex::FromHexError::OddLength.to_string());
        let e = t.coerce_str("[0x]").unwrap_err();
        assert_error_contains(&e, &hex::FromHexError::InvalidStringLength.to_string());
        let e = t.coerce_str("[0x0]").unwrap_err();
        assert_error_contains(&e, &hex::FromHexError::OddLength.to_string());

        let t = DynYlmType::Array(Box::new(DynYlmType::Tuple(vec![DynYlmType::FixedBytes(1)])));
        let e = t.coerce_str("[(0)]").unwrap_err();
        assert_error_contains(&e, &hex::FromHexError::OddLength.to_string());
        let e = t.coerce_str("[(0x)]").unwrap_err();
        assert_error_contains(&e, &hex::FromHexError::InvalidStringLength.to_string());
        let e = t.coerce_str("[(0x0)]").unwrap_err();
        assert_error_contains(&e, &hex::FromHexError::OddLength.to_string());
    }

    #[test]
    fn coerce_address() {
        // 38
        assert!(DynYlmType::Address.coerce_str("00000000000000000000000000000000000000").is_err());
        // 39
        assert!(DynYlmType::Address.coerce_str("000000000000000000000000000000000000000").is_err());
        // 40
        assert_eq!(
            DynYlmType::Address.coerce_str("00000000000000000000000000000000000000000000").unwrap(),
            DynYlmValue::Address(IcanAddress::ZERO)
        );
        assert_eq!(
            DynYlmType::Address
                .coerce_str("0x11111111111111111111111111111111111111111111")
                .unwrap(),
            DynYlmValue::Address(IcanAddress::new([0x11; 22]))
        );
        assert_eq!(
            DynYlmType::Address.coerce_str("22222222222222222222222222222222222222222222").unwrap(),
            DynYlmValue::Address(IcanAddress::new([0x22; 22]))
        );
    }

    #[test]
    fn coerce_function() {
        assert_eq!(
            DynYlmType::Function
                .coerce_str("000000000000000000000000000000000000000000000000")
                .unwrap(),
            DynYlmValue::Function(Function::ZERO)
        );
        assert_eq!(
            DynYlmType::Function
                .coerce_str("0x111111111111111111111111111111111111111111111111")
                .unwrap(),
            DynYlmValue::Function(Function::new([0x11; 24]))
        );
        assert_eq!(
            DynYlmType::Function
                .coerce_str("222222222222222222222222222222222222222222222222")
                .unwrap(),
            DynYlmValue::Function(Function::new([0x22; 24]))
        );
    }

    #[test]
    fn coerce_bytes() {
        let e = DynYlmType::Bytes.coerce_str("").unwrap_err();
        assert_error_contains(&e, &Error::EmptyHexStringWithoutPrefix.to_string());

        assert_eq!(DynYlmType::Bytes.coerce_str("0x").unwrap(), DynYlmValue::Bytes(vec![]));
        assert!(DynYlmType::Bytes.coerce_str("0x0").is_err());
        assert!(DynYlmType::Bytes.coerce_str("0").is_err());
        assert_eq!(DynYlmType::Bytes.coerce_str("00").unwrap(), DynYlmValue::Bytes(vec![0]));
        assert_eq!(DynYlmType::Bytes.coerce_str("0x00").unwrap(), DynYlmValue::Bytes(vec![0]));

        assert_eq!(
            DynYlmType::Bytes.coerce_str("123456").unwrap(),
            DynYlmValue::Bytes(vec![0x12, 0x34, 0x56])
        );
        assert_eq!(
            DynYlmType::Bytes.coerce_str("0x0017").unwrap(),
            DynYlmValue::Bytes(vec![0x00, 0x17])
        );

        let t = DynYlmType::Tuple(vec![DynYlmType::Bytes, DynYlmType::Bytes]);
        let e = t.coerce_str("(0, 0x0)").unwrap_err();
        assert_error_contains(&e, &hex::FromHexError::OddLength.to_string());

        // TODO: cut_err in `array_parser` somehow
        /*
        let t = DynYlmType::Array(Box::new(DynYlmType::Tuple(vec![
            DynYlmType::Bytes,
            DynYlmType::Bytes,
        ])));
        let e = t.coerce_str("[(0, 0x0)]").unwrap_err();
        assert_error_contains(&e, &hex::FromHexError::OddLength.to_string());

        let t = DynYlmType::Array(Box::new(DynYlmType::Tuple(vec![
            DynYlmType::Bytes,
            DynYlmType::Bytes,
        ])));
        let e = t.coerce_str("[(0x00, 0x0)]").unwrap_err();
        assert_error_contains(&e, &hex::FromHexError::OddLength.to_string());
        */
    }

    #[test]
    fn coerce_string() {
        assert_eq!(
            DynYlmType::String.coerce_str("gavofyork").unwrap(),
            DynYlmValue::String("gavofyork".into())
        );
        assert_eq!(
            DynYlmType::String.coerce_str("gav of york").unwrap(),
            DynYlmValue::String("gav of york".into())
        );
        assert_eq!(
            DynYlmType::String.coerce_str("\"hello world\"").unwrap(),
            DynYlmValue::String("hello world".into())
        );
        assert_eq!(
            DynYlmType::String.coerce_str("'hello world'").unwrap(),
            DynYlmValue::String("hello world".into())
        );
        assert_eq!(
            DynYlmType::String.coerce_str("'\"hello world\"'").unwrap(),
            DynYlmValue::String("\"hello world\"".into())
        );
        assert_eq!(
            DynYlmType::String.coerce_str("'   hello world '").unwrap(),
            DynYlmValue::String("   hello world ".into())
        );
        assert_eq!(
            DynYlmType::String.coerce_str("'\"hello world'").unwrap(),
            DynYlmValue::String("\"hello world".into())
        );
        assert_eq!(
            DynYlmType::String.coerce_str("a, b").unwrap(),
            DynYlmValue::String("a, b".into())
        );
        assert_eq!(
            DynYlmType::String.coerce_str("hello (world)").unwrap(),
            DynYlmValue::String("hello (world)".into())
        );

        assert!(DynYlmType::String.coerce_str("\"hello world").is_err());
        assert!(DynYlmType::String.coerce_str("\"hello world'").is_err());
        assert!(DynYlmType::String.coerce_str("'hello world").is_err());
        assert!(DynYlmType::String.coerce_str("'hello world\"").is_err());

        assert_eq!(
            DynYlmType::String.coerce_str("Hello, world!").unwrap(),
            DynYlmValue::String("Hello, world!".into())
        );
        let s = "$$g]a\"v/of;[()];2,yo\r)k_";
        assert_eq!(DynYlmType::String.coerce_str(s).unwrap(), DynYlmValue::String(s.into()));
    }

    #[test]
    fn coerce_strings() {
        let arr = DynYlmType::Array(Box::new(DynYlmType::String));
        let mk_arr = |s: &[&str]| {
            DynYlmValue::Array(s.iter().map(|s| DynYlmValue::String(s.to_string())).collect())
        };

        assert_eq!(arr.coerce_str("[]").unwrap(), mk_arr(&[]));
        assert_eq!(arr.coerce_str("[    ]").unwrap(), mk_arr(&[]));

        // TODO: should this be an error?
        // assert!(arr.coerce_str("[,]").is_err());
        // assert!(arr.coerce_str("[ , ]").is_err());

        assert_eq!(arr.coerce_str("[ foo bar ]").unwrap(), mk_arr(&["foo bar"]));
        assert_eq!(arr.coerce_str("[foo bar,]").unwrap(), mk_arr(&["foo bar"]));
        assert_eq!(arr.coerce_str("[  foo bar,  ]").unwrap(), mk_arr(&["foo bar"]));
        assert_eq!(arr.coerce_str("[ foo , bar ]").unwrap(), mk_arr(&["foo", "bar"]));

        assert_eq!(arr.coerce_str("[\"foo\",\"bar\"]").unwrap(), mk_arr(&["foo", "bar"]));

        assert_eq!(arr.coerce_str("['']").unwrap(), mk_arr(&[""]));
        assert_eq!(arr.coerce_str("[\"\"]").unwrap(), mk_arr(&[""]));
        assert_eq!(arr.coerce_str("['', '']").unwrap(), mk_arr(&["", ""]));
        assert_eq!(arr.coerce_str("['', \"\"]").unwrap(), mk_arr(&["", ""]));
        assert_eq!(arr.coerce_str("[\"\", '']").unwrap(), mk_arr(&["", ""]));
        assert_eq!(arr.coerce_str("[\"\", \"\"]").unwrap(), mk_arr(&["", ""]));
    }

    #[test]
    fn coerce_array_of_bytes_and_strings() {
        let ty = DynYlmType::Array(Box::new(DynYlmType::Bytes));
        assert_eq!(ty.coerce_str("[]"), Ok(DynYlmValue::Array(vec![])));
        assert_eq!(ty.coerce_str("[0x]"), Ok(DynYlmValue::Array(vec![DynYlmValue::Bytes(vec![])])));

        let ty = DynYlmType::Array(Box::new(DynYlmType::String));
        assert_eq!(ty.coerce_str("[]"), Ok(DynYlmValue::Array(vec![])));
        assert_eq!(
            ty.coerce_str("[\"\"]"),
            Ok(DynYlmValue::Array(vec![DynYlmValue::String(String::new())]))
        );
        assert_eq!(
            ty.coerce_str("[0x]"),
            Ok(DynYlmValue::Array(vec![DynYlmValue::String("0x".into())]))
        );
    }

    #[test]
    fn coerce_empty_array() {
        assert_eq!(
            DynYlmType::Array(Box::new(DynYlmType::Bool)).coerce_str("[]").unwrap(),
            DynYlmValue::Array(vec![])
        );
        assert_eq!(
            DynYlmType::FixedArray(Box::new(DynYlmType::Bool), 0).coerce_str("[]").unwrap(),
            DynYlmValue::FixedArray(vec![]),
        );
        assert!(DynYlmType::FixedArray(Box::new(DynYlmType::Bool), 1).coerce_str("[]").is_err());
    }

    #[test]
    fn coerce_bool_array() {
        assert_eq!(
            DynYlmType::coerce_str(&DynYlmType::Array(Box::new(DynYlmType::Bool)), "[true, false]")
                .unwrap(),
            DynYlmValue::Array(vec![DynYlmValue::Bool(true), DynYlmValue::Bool(false)])
        );
    }

    #[test]
    fn coerce_bool_array_of_arrays() {
        assert_eq!(
            DynYlmType::coerce_str(
                &DynYlmType::Array(Box::new(DynYlmType::Array(Box::new(DynYlmType::Bool)))),
                "[ [ true, true, false ], [ false]]"
            )
            .unwrap(),
            DynYlmValue::Array(vec![
                DynYlmValue::Array(vec![
                    DynYlmValue::Bool(true),
                    DynYlmValue::Bool(true),
                    DynYlmValue::Bool(false)
                ]),
                DynYlmValue::Array(vec![DynYlmValue::Bool(false)])
            ])
        );
    }

    #[test]
    fn coerce_bool_fixed_array() {
        let ty = DynYlmType::FixedArray(Box::new(DynYlmType::Bool), 3);
        assert!(ty.coerce_str("[]").is_err());
        assert!(ty.coerce_str("[true]").is_err());
        assert!(ty.coerce_str("[true, false]").is_err());
        assert_eq!(
            ty.coerce_str("[true, false, true]").unwrap(),
            DynYlmValue::FixedArray(vec![
                DynYlmValue::Bool(true),
                DynYlmValue::Bool(false),
                DynYlmValue::Bool(true),
            ])
        );
        assert!(ty.coerce_str("[true, false, false, true]").is_err());
    }

    #[test]
    fn single_quoted_in_array_must_error() {
        assert!(DynYlmType::Array(Box::new(DynYlmType::Bool))
            .coerce_str("[true,\"false,false]")
            .is_err());
        assert!(DynYlmType::Array(Box::new(DynYlmType::Bool)).coerce_str("[false\"]").is_err());
        assert!(DynYlmType::Array(Box::new(DynYlmType::Bool))
            .coerce_str("[true,false\"]")
            .is_err());
        assert!(DynYlmType::Array(Box::new(DynYlmType::Bool))
            .coerce_str("[true,\"false\",false]")
            .is_err());
        assert!(DynYlmType::Array(Box::new(DynYlmType::Bool)).coerce_str("[true,false]").is_ok());
    }

    #[test]
    fn tuples() {
        let ty = DynYlmType::Tuple(vec![DynYlmType::String, DynYlmType::Bool, DynYlmType::String]);
        assert_eq!(
            ty.coerce_str("(\"a,]) b\", true, true? ]and] false!)").unwrap(),
            DynYlmValue::Tuple(vec![
                DynYlmValue::String("a,]) b".into()),
                DynYlmValue::Bool(true),
                DynYlmValue::String("true? ]and] false!".into()),
            ])
        );
        assert!(ty.coerce_str("(\"\", true, a, b)").is_err());
        assert!(ty.coerce_str("(a, b, true, a)").is_err());
    }

    #[test]
    fn tuples_arrays_mixed() {
        assert_eq!(
            DynYlmType::Array(Box::new(DynYlmType::Tuple(vec![
                DynYlmType::Array(Box::new(DynYlmType::Tuple(vec![DynYlmType::Bool]))),
                DynYlmType::Array(Box::new(DynYlmType::Tuple(vec![
                    DynYlmType::Bool,
                    DynYlmType::Bool
                ]))),
            ])))
            .coerce_str("[([(true)],[(false,true)])]")
            .unwrap(),
            DynYlmValue::Array(vec![DynYlmValue::Tuple(vec![
                DynYlmValue::Array(vec![DynYlmValue::Tuple(vec![DynYlmValue::Bool(true)])]),
                DynYlmValue::Array(vec![DynYlmValue::Tuple(vec![
                    DynYlmValue::Bool(false),
                    DynYlmValue::Bool(true)
                ])]),
            ])])
        );

        assert_eq!(
            DynYlmType::Tuple(vec![
                DynYlmType::Array(Box::new(DynYlmType::Tuple(vec![DynYlmType::Bool]))),
                DynYlmType::Array(Box::new(DynYlmType::Tuple(vec![
                    DynYlmType::Bool,
                    DynYlmType::Bool
                ]))),
            ])
            .coerce_str("([(true)],[(false,true)])")
            .unwrap(),
            DynYlmValue::Tuple(vec![
                DynYlmValue::Array(vec![DynYlmValue::Tuple(vec![DynYlmValue::Bool(true)])]),
                DynYlmValue::Array(vec![DynYlmValue::Tuple(vec![
                    DynYlmValue::Bool(false),
                    DynYlmValue::Bool(true)
                ])]),
            ])
        );
    }

    #[test]
    fn tuple_array_nested() {
        assert_eq!(
            DynYlmType::Tuple(vec![
                DynYlmType::Array(Box::new(DynYlmType::Tuple(vec![DynYlmType::Address]))),
                DynYlmType::Uint(256),
            ])
            .coerce_str("([(00005c9d55b78febcc2061715ba4f57ecf8ea2711f2c)],2)")
            .unwrap(),
            DynYlmValue::Tuple(vec![
                DynYlmValue::Array(vec![DynYlmValue::Tuple(vec![DynYlmValue::Address(
                    cAddress!("00005c9d55b78febcc2061715ba4f57ecf8ea2711f2c")
                )])]),
                DynYlmValue::Uint(U256::from(2), 256),
            ])
        );
    }

    // keep `n` low to avoid stack overflows (debug mode)
    #[test]
    fn lotsa_array_nesting() {
        let n = 10;

        let mut ty = DynYlmType::Bool;
        for _ in 0..n {
            ty = DynYlmType::Array(Box::new(ty));
        }
        let mut value_str = String::new();
        value_str.push_str(&"[".repeat(n));
        value_str.push_str("true");
        value_str.push_str(&"]".repeat(n));

        let mut value = ty.coerce_str(&value_str).unwrap();
        for _ in 0..n {
            let DynYlmValue::Array(arr) = value else { panic!("{value:?}") };
            assert_eq!(arr.len(), 1);
            value = arr.into_iter().next().unwrap();
        }
        assert_eq!(value, DynYlmValue::Bool(true));
    }

    #[test]
    fn lotsa_tuple_nesting() {
        let n = 10;

        let mut ty = DynYlmType::Bool;
        for _ in 0..n {
            ty = DynYlmType::Tuple(vec![ty]);
        }
        let mut value_str = String::new();
        value_str.push_str(&"(".repeat(n));
        value_str.push_str("true");
        value_str.push_str(&")".repeat(n));

        let mut value = ty.coerce_str(&value_str).unwrap();
        for _ in 0..n {
            let DynYlmValue::Tuple(tuple) = value else { panic!("{value:?}") };
            assert_eq!(tuple.len(), 1);
            value = tuple.into_iter().next().unwrap();
        }
        assert_eq!(value, DynYlmValue::Bool(true));
    }
}
