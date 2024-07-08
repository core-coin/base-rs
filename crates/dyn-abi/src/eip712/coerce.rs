use crate::{DynYlmType, DynYlmValue, Error, Result, Word};
use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use base_primitives::{Function, IcanAddress, I256, U256};

impl DynYlmType {
    /// Coerce a [`serde_json::Value`] to a [`DynYlmValue`] via this type.
    pub fn coerce_json(&self, value: &serde_json::Value) -> Result<DynYlmValue> {
        let err = || Error::eip712_coerce(self, value);
        match self {
            Self::Bool
            | Self::Int(_)
            | Self::Uint(_)
            | Self::FixedBytes(_)
            | Self::Address
            | Self::Function
            | Self::String
            | Self::Bytes => self.coerce_json_simple(value).ok_or_else(err),

            Self::Array(inner) => array(inner, value)
                .ok_or_else(err)
                .and_then(core::convert::identity)
                .map(DynYlmValue::Array),
            Self::FixedArray(inner, n) => fixed_array(inner, *n, value)
                .ok_or_else(err)
                .and_then(core::convert::identity)
                .map(DynYlmValue::FixedArray),
            Self::Tuple(inner) => tuple(inner, value)
                .ok_or_else(err)
                .and_then(core::convert::identity)
                .map(DynYlmValue::Tuple),
            Self::CustomStruct { name, prop_names, tuple } => {
                custom_struct(name, prop_names, tuple, value)
            }
        }
    }

    fn coerce_json_simple(&self, value: &serde_json::Value) -> Option<DynYlmValue> {
        match self {
            Self::Bool => bool(value).map(DynYlmValue::Bool),
            &Self::Int(n) => int(n, value).map(|x| DynYlmValue::Int(x, n)),
            &Self::Uint(n) => uint(n, value).map(|x| DynYlmValue::Uint(x, n)),
            &Self::FixedBytes(n) => fixed_bytes(n, value).map(|x| DynYlmValue::FixedBytes(x, n)),
            Self::Address => address(value).map(DynYlmValue::Address),
            Self::Function => function(value).map(DynYlmValue::Function),
            Self::String => string(value).map(DynYlmValue::String),
            Self::Bytes => bytes(value).map(DynYlmValue::Bytes),
            _ => unreachable!(),
        }
    }
}

fn bool(value: &serde_json::Value) -> Option<bool> {
    value.as_bool().or_else(|| value.as_str().and_then(|s| s.parse().ok()))
}

fn int(n: usize, value: &serde_json::Value) -> Option<I256> {
    (|| {
        if let Some(num) = value.as_i64() {
            return Some(I256::try_from(num).unwrap());
        }
        value.as_str().and_then(|s| s.parse().ok())
    })()
    .and_then(|x| (x.bits() <= n as u32).then_some(x))
}

fn uint(n: usize, value: &serde_json::Value) -> Option<U256> {
    (|| {
        if let Some(num) = value.as_u64() {
            return Some(U256::from(num));
        }
        value.as_str().and_then(|s| s.parse().ok())
    })()
    .and_then(|x| (x.bit_len() <= n).then_some(x))
}

fn fixed_bytes(n: usize, value: &serde_json::Value) -> Option<Word> {
    if let Some(Ok(buf)) = value.as_str().map(hex::decode) {
        let mut word = Word::ZERO;
        let min = n.min(buf.len());
        if min <= 32 {
            word[..min].copy_from_slice(&buf[..min]);
            return Some(word);
        }
    }
    None
}

fn address(value: &serde_json::Value) -> Option<IcanAddress> {
    value.as_str().and_then(|s| s.parse().ok())
}

fn function(value: &serde_json::Value) -> Option<Function> {
    value.as_str().and_then(|s| s.parse().ok())
}

fn string(value: &serde_json::Value) -> Option<String> {
    value.as_str().map(|s| s.to_string())
}

fn bytes(value: &serde_json::Value) -> Option<Vec<u8>> {
    if let Some(s) = value.as_str() {
        return hex::decode(s).ok();
    }

    let arr = value.as_array()?;
    let mut vec = Vec::with_capacity(arr.len());
    for elem in arr.iter() {
        vec.push(elem.as_u64()?.try_into().ok()?);
    }
    Some(vec)
}

fn tuple(inner: &[DynYlmType], value: &serde_json::Value) -> Option<Result<Vec<DynYlmValue>>> {
    if let Some(arr) = value.as_array() {
        if inner.len() == arr.len() {
            return Some(core::iter::zip(arr, inner).map(|(v, t)| t.coerce_json(v)).collect());
        }
    }
    None
}

fn array(inner: &DynYlmType, value: &serde_json::Value) -> Option<Result<Vec<DynYlmValue>>> {
    if let Some(arr) = value.as_array() {
        return Some(arr.iter().map(|v| inner.coerce_json(v)).collect());
    }
    None
}

fn fixed_array(
    inner: &DynYlmType,
    n: usize,
    value: &serde_json::Value,
) -> Option<Result<Vec<DynYlmValue>>> {
    if let Some(arr) = value.as_array() {
        if arr.len() == n {
            return Some(arr.iter().map(|v| inner.coerce_json(v)).collect());
        }
    }
    None
}

pub(crate) fn custom_struct(
    name: &str,
    prop_names: &[String],
    inner: &[DynYlmType],
    value: &serde_json::Value,
) -> Result<DynYlmValue> {
    if let Some(map) = value.as_object() {
        let mut tuple = vec![];
        for (name, ty) in core::iter::zip(prop_names, inner) {
            if let Some(v) = map.get(name) {
                tuple.push(ty.coerce_json(v)?);
            } else {
                return Err(Error::eip712_coerce(
                    &DynYlmType::CustomStruct {
                        name: name.to_string(),
                        prop_names: prop_names.to_vec(),
                        tuple: inner.to_vec(),
                    },
                    value,
                ));
            }
        }
        return Ok(DynYlmValue::CustomStruct {
            name: name.to_string(),
            prop_names: prop_names.to_vec(),
            tuple,
        });
    }

    Err(Error::eip712_coerce(
        &DynYlmType::CustomStruct {
            name: name.to_string(),
            prop_names: prop_names.to_vec(),
            tuple: inner.to_vec(),
        },
        value,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::{borrow::ToOwned, string::ToString};
    use serde_json::json;

    #[test]
    fn test_bytes_num_array() {
        let ty = DynYlmType::Bytes;
        let j = json!([1, 2, 3, 4]);
        assert_eq!(ty.coerce_json(&j), Ok(DynYlmValue::Bytes(vec![1, 2, 3, 4])));
    }

    #[test]
    fn it_coerces() {
        let j = json!({
            "message": {
                "contents": "Hello, Bob!",
                "attachedMoneyInEth": 4.2,
                "from": {
                    "name": "Cow",
                    "wallets": [
                        "0x0000CD2a3d9F938E13CD947Ec05AbC7FE734Df8DD826",
                        "0x0000DeaDbeefdEAdbeefdEadbEEFdeadbeEFdEaDbeeF",
                    ]
                },
                "to": [{
                    "name": "Bob",
                    "wallets": [
                        "0x0000bBbBBBBbbBBBbbbBbbBbbbbBBbBbbbbBbBbbBBbB",
                        "0x0000B0BdaBea57B0BDABeA57b0bdABEA57b0BDabEa57",
                        "0x0000B0B0b0b0b0b0B000000000000000000000000000",
                    ]
                }]
            }
        });

        let ty = DynYlmType::CustomStruct {
            name: "Message".to_owned(),
            prop_names: vec!["contents".to_string(), "from".to_string(), "to".to_string()],
            tuple: vec![
                DynYlmType::String,
                DynYlmType::CustomStruct {
                    name: "Person".to_owned(),
                    prop_names: vec!["name".to_string(), "wallets".to_string()],
                    tuple: vec![
                        DynYlmType::String,
                        DynYlmType::Array(Box::new(DynYlmType::Address)),
                    ],
                },
                DynYlmType::Array(Box::new(DynYlmType::CustomStruct {
                    name: "Person".to_owned(),
                    prop_names: vec!["name".to_string(), "wallets".to_string()],
                    tuple: vec![
                        DynYlmType::String,
                        DynYlmType::Array(Box::new(DynYlmType::Address)),
                    ],
                })),
            ],
        };
        let top = j.as_object().unwrap().get("message").unwrap();

        assert_eq!(
            ty.coerce_json(top),
            Ok(DynYlmValue::CustomStruct {
                name: "Message".to_owned(),
                prop_names: vec!["contents".to_string(), "from".to_string(), "to".to_string()],
                tuple: vec![
                    DynYlmValue::String("Hello, Bob!".to_string()),
                    DynYlmValue::CustomStruct {
                        name: "Person".to_owned(),
                        prop_names: vec!["name".to_string(), "wallets".to_string()],
                        tuple: vec![
                            DynYlmValue::String("Cow".to_string()),
                            vec![
                                DynYlmValue::Address(
                                    "0x0000CD2a3d9F938E13CD947Ec05AbC7FE734Df8DD826"
                                        .parse()
                                        .unwrap()
                                ),
                                DynYlmValue::Address(
                                    "0x0000DeaDbeefdEAdbeefdEadbEEFdeadbeEFdEaDbeeF"
                                        .parse()
                                        .unwrap()
                                ),
                            ]
                            .into()
                        ]
                    },
                    vec![DynYlmValue::CustomStruct {
                        name: "Person".to_owned(),
                        prop_names: vec!["name".to_string(), "wallets".to_string()],
                        tuple: vec![
                            DynYlmValue::String("Bob".to_string()),
                            vec![
                                DynYlmValue::Address(
                                    "0x0000bBbBBBBbbBBBbbbBbbBbbbbBBbBbbbbBbBbbBBbB"
                                        .parse()
                                        .unwrap()
                                ),
                                DynYlmValue::Address(
                                    "0x0000B0BdaBea57B0BDABeA57b0bdABEA57b0BDabEa57"
                                        .parse()
                                        .unwrap()
                                ),
                                DynYlmValue::Address(
                                    "0x0000B0B0b0b0b0b0B000000000000000000000000000"
                                        .parse()
                                        .unwrap()
                                ),
                            ]
                            .into()
                        ]
                    }]
                    .into()
                ]
            })
        );
    }
}
