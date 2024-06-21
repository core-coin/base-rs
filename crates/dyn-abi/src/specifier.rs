//! Contains utilities for parsing Ylem types.
//!
//! This is a simple representation of Ylem type grammar.

use crate::{DynYlmType, Result};
use alloc::vec::Vec;
use base_json_abi::{EventParam, Param};
use parser::{ParameterSpecifier, Parameters, RootType, TupleSpecifier, TypeSpecifier, TypeStem};

#[cfg(feature = "eip712")]
use base_json_abi::InternalType;

/// Trait for items that can be resolved to `DynYlm_____`, i.e. they speicify
/// some Ylem interface item.
///
/// The `Specifier` trait is implemented by types that can be resolved into
/// Ylem interace items, e.g. [`DynYlmType`] or [`DynYlmEvent`](crate::DynYlmEvent).
///
/// ABI and related systems have many different ways of specifying Ylem interfaces.
/// This trait provides a single pattern for resolving those encodings into
/// Ylem interface items.
///
/// `Specifier<DynYlmType>` is implemented for all the [`parser`] types, the
/// [`Param`] and [`EventParam`] structs, and [`str`]. The [`str`]
/// implementation calls [`DynYlmType::parse`].
///
/// # Examples
///
/// ```
/// # use base_dyn_abi::{DynYlmType, Specifier};
/// # use base_ylm_type_parser::{RootType, TypeSpecifier};
/// let my_ty = TypeSpecifier::parse("bool")?.resolve()?;
/// assert_eq!(my_ty, DynYlmType::Bool);
///
/// let my_ty = RootType::parse("uint256")?.resolve()?;
/// assert_eq!(my_ty, DynYlmType::Uint(256));
///
/// assert_eq!("bytes32".resolve()?, DynYlmType::FixedBytes(32));
/// # Ok::<_, base_dyn_abi::Error>(())
/// ```

pub trait Specifier<T> {
    /// Resolve the type into a value.
    fn resolve(&self) -> Result<T>;
}

impl Specifier<DynYlmType> for str {
    #[inline]
    fn resolve(&self) -> Result<DynYlmType> {
        DynYlmType::parse(self)
    }
}

impl Specifier<DynYlmType> for RootType<'_> {
    fn resolve(&self) -> Result<DynYlmType> {
        match self.span() {
            "address" => Ok(DynYlmType::Address),
            "function" => Ok(DynYlmType::Function),
            "bool" => Ok(DynYlmType::Bool),
            "string" => Ok(DynYlmType::String),
            "bytes" => Ok(DynYlmType::Bytes),
            "uint" => Ok(DynYlmType::Uint(256)),
            "int" => Ok(DynYlmType::Int(256)),
            name => {
                if let Some(sz) = name.strip_prefix("bytes") {
                    if let Ok(sz) = sz.parse() {
                        if sz != 0 && sz <= 32 {
                            return Ok(DynYlmType::FixedBytes(sz));
                        }
                    }
                    return Err(parser::Error::invalid_size(name).into());
                }

                // fast path both integer types
                let (s, is_uint) =
                    if let Some(s) = name.strip_prefix('u') { (s, true) } else { (name, false) };

                if let Some(sz) = s.strip_prefix("int") {
                    if let Ok(sz) = sz.parse() {
                        if sz != 0 && sz <= 256 && sz % 8 == 0 {
                            return if is_uint {
                                Ok(DynYlmType::Uint(sz))
                            } else {
                                Ok(DynYlmType::Int(sz))
                            };
                        }
                    }
                    Err(parser::Error::invalid_size(name).into())
                } else {
                    Err(parser::Error::invalid_type_string(name).into())
                }
            }
        }
    }
}

impl Specifier<DynYlmType> for TupleSpecifier<'_> {
    #[inline]
    fn resolve(&self) -> Result<DynYlmType> {
        tuple(&self.types).map(DynYlmType::Tuple)
    }
}

impl Specifier<DynYlmType> for TypeStem<'_> {
    #[inline]
    fn resolve(&self) -> Result<DynYlmType> {
        match self {
            Self::Root(root) => root.resolve(),
            Self::Tuple(tuple) => tuple.resolve(),
        }
    }
}

impl Specifier<DynYlmType> for TypeSpecifier<'_> {
    fn resolve(&self) -> Result<DynYlmType> {
        self.stem.resolve().map(|ty| ty.array_wrap_from_iter(self.sizes.iter().copied()))
    }
}

impl Specifier<DynYlmType> for ParameterSpecifier<'_> {
    #[inline]
    fn resolve(&self) -> Result<DynYlmType> {
        self.ty.resolve()
    }
}

impl Specifier<DynYlmType> for Parameters<'_> {
    #[inline]
    fn resolve(&self) -> Result<DynYlmType> {
        tuple(&self.params).map(DynYlmType::Tuple)
    }
}

impl Specifier<DynYlmType> for Param {
    #[inline]
    fn resolve(&self) -> Result<DynYlmType> {
        resolve_param(
            &self.ty,
            &self.components,
            #[cfg(feature = "eip712")]
            self.internal_type(),
        )
    }
}

impl Specifier<DynYlmType> for EventParam {
    #[inline]
    fn resolve(&self) -> Result<DynYlmType> {
        resolve_param(
            &self.ty,
            &self.components,
            #[cfg(feature = "eip712")]
            self.internal_type(),
        )
    }
}

fn resolve_param(
    ty: &str,
    components: &[Param],
    #[cfg(feature = "eip712")] it: Option<&InternalType>,
) -> Result<DynYlmType> {
    let ty = TypeSpecifier::parse(ty)?;

    // type is simple, and we can resolve it via the specifier
    if components.is_empty() {
        return ty.resolve();
    }

    // type is complex
    let tuple = tuple(components)?;

    #[cfg(feature = "eip712")]
    let resolved = if let Some((_, name)) = it.and_then(|i| i.as_struct()) {
        DynYlmType::CustomStruct {
            // skip array sizes, since we have them already from parsing `ty`
            name: name.split('[').next().unwrap().into(),
            prop_names: components.iter().map(|c| c.name.clone()).collect(),
            tuple,
        }
    } else {
        DynYlmType::Tuple(tuple)
    };

    #[cfg(not(feature = "eip712"))]
    let resolved = DynYlmType::Tuple(tuple);

    Ok(resolved.array_wrap_from_iter(ty.sizes))
}

fn tuple<T: Specifier<DynYlmType>>(slice: &[T]) -> Result<Vec<DynYlmType>> {
    let mut types = Vec::with_capacity(slice.len());
    for ty in slice {
        types.push(ty.resolve()?);
    }
    Ok(types)
}

macro_rules! deref_impls {
    ($($(#[$attr:meta])* [$($gen:tt)*] $t:ty),+ $(,)?) => {$(
        $(#[$attr])*
        impl<$($gen)*> Specifier<DynYlmType> for $t {
            #[inline]
            fn resolve(&self) -> Result<DynYlmType> {
                (**self).resolve()
            }
        }
    )+};
}

deref_impls! {
    [] alloc::string::String,
    [T: ?Sized + Specifier<DynYlmType>] &T,
    [T: ?Sized + Specifier<DynYlmType>] &mut T,
    [T: ?Sized + Specifier<DynYlmType>] alloc::boxed::Box<T>,
    [T: ?Sized + alloc::borrow::ToOwned + Specifier<DynYlmType>] alloc::borrow::Cow<'_, T>,
    [T: ?Sized + Specifier<DynYlmType>] alloc::rc::Rc<T>,
    [T: ?Sized + Specifier<DynYlmType>] alloc::sync::Arc<T>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::boxed::Box;

    fn parse(s: &str) -> Result<DynYlmType> {
        s.parse()
    }

    #[test]
    fn extra_close_parens() {
        parse("(bool,uint256))").unwrap_err();
        parse("bool,uint256))").unwrap_err();
        parse("bool,uint256)").unwrap_err();
    }

    #[test]
    fn extra_open_parents() {
        parse("((bool,uint256)").unwrap_err();
        parse("((bool,uint256").unwrap_err();
        parse("(bool,uint256").unwrap_err();
    }

    #[test]
    fn it_parses_tuples() {
        assert_eq!(parse("(bool,)"), Ok(DynYlmType::Tuple(vec![DynYlmType::Bool])));
        assert_eq!(
            parse("(uint256,uint256)"),
            Ok(DynYlmType::Tuple(vec![DynYlmType::Uint(256), DynYlmType::Uint(256)]))
        );
        assert_eq!(
            parse("(uint256,uint256)[2]"),
            Ok(DynYlmType::FixedArray(
                Box::new(DynYlmType::Tuple(vec![DynYlmType::Uint(256), DynYlmType::Uint(256)])),
                2
            ))
        );
    }

    #[test]
    fn nested_tuples() {
        assert_eq!(
            parse("(bool,(uint256,uint256))"),
            Ok(DynYlmType::Tuple(vec![
                DynYlmType::Bool,
                DynYlmType::Tuple(vec![DynYlmType::Uint(256), DynYlmType::Uint(256)])
            ]))
        );
        assert_eq!(
            parse("(((bool),),)"),
            Ok(DynYlmType::Tuple(vec![DynYlmType::Tuple(vec![DynYlmType::Tuple(vec![
                DynYlmType::Bool
            ])])]))
        );
    }

    #[test]
    fn empty_tuples() {
        assert_eq!(parse("()"), Ok(DynYlmType::Tuple(vec![])));
        assert_eq!(
            parse("((),())"),
            Ok(DynYlmType::Tuple(vec![DynYlmType::Tuple(vec![]), DynYlmType::Tuple(vec![])]))
        );
        assert_eq!(
            parse("((()))"),
            Ok(DynYlmType::Tuple(vec![DynYlmType::Tuple(vec![DynYlmType::Tuple(vec![])])]))
        );
    }

    #[test]
    fn it_parses_simple_types() {
        assert_eq!(parse("uint256"), Ok(DynYlmType::Uint(256)));
        assert_eq!(parse("uint8"), Ok(DynYlmType::Uint(8)));
        assert_eq!(parse("uint"), Ok(DynYlmType::Uint(256)));
        assert_eq!(parse("address"), Ok(DynYlmType::Address));
        assert_eq!(parse("bool"), Ok(DynYlmType::Bool));
        assert_eq!(parse("string"), Ok(DynYlmType::String));
        assert_eq!(parse("bytes"), Ok(DynYlmType::Bytes));
        assert_eq!(parse("bytes32"), Ok(DynYlmType::FixedBytes(32)));
    }

    #[test]
    fn it_parses_complex_solidity_types() {
        assert_eq!(parse("uint256[]"), Ok(DynYlmType::Array(Box::new(DynYlmType::Uint(256)))));
        assert_eq!(
            parse("uint256[2]"),
            Ok(DynYlmType::FixedArray(Box::new(DynYlmType::Uint(256)), 2))
        );
        assert_eq!(
            parse("uint256[2][3]"),
            Ok(DynYlmType::FixedArray(
                Box::new(DynYlmType::FixedArray(Box::new(DynYlmType::Uint(256)), 2)),
                3
            ))
        );
        assert_eq!(
            parse("uint256[][][]"),
            Ok(DynYlmType::Array(Box::new(DynYlmType::Array(Box::new(DynYlmType::Array(
                Box::new(DynYlmType::Uint(256))
            ))))))
        );

        assert_eq!(
            parse("tuple(address,bytes,(bool,(string,uint256)[][3]))[2]"),
            Ok(DynYlmType::FixedArray(
                Box::new(DynYlmType::Tuple(vec![
                    DynYlmType::Address,
                    DynYlmType::Bytes,
                    DynYlmType::Tuple(vec![
                        DynYlmType::Bool,
                        DynYlmType::FixedArray(
                            Box::new(DynYlmType::Array(Box::new(DynYlmType::Tuple(vec![
                                DynYlmType::String,
                                DynYlmType::Uint(256)
                            ])))),
                            3
                        ),
                    ]),
                ])),
                2
            ))
        );
    }

    #[test]
    fn library_enum_workaround() {
        assert_eq!(parse("MyLibrary.MyEnum"), Ok(DynYlmType::Uint(8)));
        assert_eq!(
            parse("MyLibrary.MyEnum[]"),
            Ok(DynYlmType::Array(Box::new(DynYlmType::Uint(8))))
        );
    }
}
