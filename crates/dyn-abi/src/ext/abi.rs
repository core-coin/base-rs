use crate::{DynYlmValue, Error as CrateError, Result, Specifier};
use alloc::vec::Vec;
use base_json_abi::{Constructor, Error, Function, Param};
use base_primitives::Selector;
use base_ylm_types::abi::Decoder;

mod sealed {
    pub trait Sealed {}
    impl Sealed for super::Constructor {}
    impl Sealed for super::Error {}
    impl Sealed for super::Function {}
}
use sealed::Sealed;

/// Provides ABI encoding and decoding functionality.
///
/// This trait is sealed and cannot be implemented for types outside of this
/// crate. It is implemented only for the following types:
/// - [`Constructor`]
/// - [`Error`]
/// - [`Function`]
pub trait JsonAbiExt: Sealed {
    /// ABI-encodes the given values, prefixed by this item's selector, if any.
    ///
    /// The selector is:
    /// - `None` for [`Constructor`],
    /// - `Some(self.selector())` for [`Error`] and [`Function`].
    ///
    /// This behaviour is to ensure consistency with `ethabi`.
    ///
    /// To encode the data without the selector, use
    /// [`abi_encode_input_raw`](JsonAbiExt::abi_encode_input_raw).
    ///
    /// # Errors
    ///
    /// This function will return an error if the given values do not match the
    /// expected input types.
    fn abi_encode_input(&self, values: &[DynYlmValue]) -> Result<Vec<u8>>;

    /// ABI-encodes the given values, without prefixing the data with the item's
    /// selector.
    ///
    /// For [`Constructor`], this is the same as
    /// [`abi_encode_input`](JsonAbiExt::abi_encode_input).
    ///
    /// # Errors
    ///
    /// This function will return an error if the given values do not match the
    /// expected input types.
    fn abi_encode_input_raw(&self, values: &[DynYlmValue]) -> Result<Vec<u8>>;

    /// ABI-decodes the given data according to this item's input types.
    ///
    /// # Errors
    ///
    /// This function will return an error if the decoded data does not match
    /// the expected input types.
    fn abi_decode_input(&self, data: &[u8], validate: bool) -> Result<Vec<DynYlmValue>>;
}

/// Provide ABI encoding and decoding for the [`Function`] type.
///
/// This trait is sealed and cannot be implemented for types outside of this
/// crate. It is implemented only for [`Function`].
pub trait FunctionExt: JsonAbiExt + Sealed {
    /// ABI-encodes the given values.
    ///
    /// Note that, contrary to
    /// [`abi_encode_input`](JsonAbiExt::abi_encode_input), this method does
    /// not prefix the return data with the function selector.
    ///
    /// # Errors
    ///
    /// This function will return an error if the given values do not match the
    /// expected input types.
    fn abi_encode_output(&self, values: &[DynYlmValue]) -> Result<Vec<u8>>;

    /// ABI-decodes the given data according to this functions's output types.
    ///
    /// This method does not check for any prefixes or selectors.
    fn abi_decode_output(&self, data: &[u8], validate: bool) -> Result<Vec<DynYlmValue>>;
}

impl JsonAbiExt for Constructor {
    #[inline]
    fn abi_encode_input(&self, values: &[DynYlmValue]) -> Result<Vec<u8>> {
        encode_typeck(&self.inputs, values)
    }

    #[inline]
    fn abi_encode_input_raw(&self, values: &[DynYlmValue]) -> Result<Vec<u8>> {
        encode_typeck(&self.inputs, values)
    }

    #[inline]
    fn abi_decode_input(&self, data: &[u8], validate: bool) -> Result<Vec<DynYlmValue>> {
        abi_decode(data, &self.inputs, validate)
    }
}

impl JsonAbiExt for Error {
    #[inline]
    fn abi_encode_input(&self, values: &[DynYlmValue]) -> Result<Vec<u8>> {
        encode_typeck(&self.inputs, values).map(prefix_selector(self.selector()))
    }

    #[inline]
    fn abi_encode_input_raw(&self, values: &[DynYlmValue]) -> Result<Vec<u8>> {
        encode_typeck(&self.inputs, values)
    }

    #[inline]
    fn abi_decode_input(&self, data: &[u8], validate: bool) -> Result<Vec<DynYlmValue>> {
        abi_decode(data, &self.inputs, validate)
    }
}

impl JsonAbiExt for Function {
    #[inline]
    fn abi_encode_input(&self, values: &[DynYlmValue]) -> Result<Vec<u8>> {
        encode_typeck(&self.inputs, values).map(prefix_selector(self.selector()))
    }

    #[inline]
    fn abi_encode_input_raw(&self, values: &[DynYlmValue]) -> Result<Vec<u8>> {
        encode_typeck(&self.inputs, values)
    }

    #[inline]
    fn abi_decode_input(&self, data: &[u8], validate: bool) -> Result<Vec<DynYlmValue>> {
        abi_decode(data, &self.inputs, validate)
    }
}

impl FunctionExt for Function {
    #[inline]
    fn abi_encode_output(&self, values: &[DynYlmValue]) -> Result<Vec<u8>> {
        encode_typeck(&self.outputs, values)
    }

    #[inline]
    fn abi_decode_output(&self, data: &[u8], validate: bool) -> Result<Vec<DynYlmValue>> {
        abi_decode(data, &self.outputs, validate)
    }
}

#[inline]
fn prefix_selector(selector: Selector) -> impl FnOnce(Vec<u8>) -> Vec<u8> {
    move |data| {
        let mut new = Vec::with_capacity(data.len() + 4);
        new.extend_from_slice(&selector[..]);
        new.extend_from_slice(&data[..]);
        new
    }
}

fn encode_typeck(params: &[Param], values: &[DynYlmValue]) -> Result<Vec<u8>> {
    if values.len() != params.len() {
        return Err(CrateError::EncodeLengthMismatch {
            expected: params.len(),
            actual: values.len(),
        });
    }
    for (value, param) in core::iter::zip(values, params) {
        let ty = param.resolve()?;
        if !ty.matches(value) {
            return Err(CrateError::TypeMismatch {
                expected: ty.ylm_type_name().into_owned(),
                actual: value.ylm_type_name().unwrap_or_else(|| "<none>".into()).into_owned(),
            });
        }
    }

    Ok(abi_encode(values))
}

#[inline]
fn abi_encode(values: &[DynYlmValue]) -> Vec<u8> {
    DynYlmValue::encode_seq(values)
}

fn abi_decode(data: &[u8], params: &[Param], validate: bool) -> Result<Vec<DynYlmValue>> {
    let mut values = Vec::with_capacity(params.len());
    let mut decoder = Decoder::new(data, validate);
    for param in params {
        let ty = param.resolve()?;
        let value = ty.abi_decode_inner(&mut decoder, crate::DynToken::decode_single_populate)?;
        values.push(value);
    }
    Ok(values)
}

#[cfg(test)]
mod tests {
    use super::*;
    use base_primitives::{bytes, cAddress, hex, IcanAddress, U256};

    #[test]
    fn can_encode_decode_functions() {
        let json = r#"{
            "inputs": [
                {
                    "internalType": "address",
                    "name": "",
                    "type": "address"
                },
                {
                    "internalType": "address",
                    "name": "",
                    "type": "address"
                }
            ],
            "name": "allowance",
            "outputs": [
                {
                    "internalType": "uint256",
                    "name": "",
                    "type": "uint256"
                }
            ],
            "stateMutability": "view",
            "type": "function"
        }"#;

        let func: Function = serde_json::from_str(json).unwrap();
        assert_eq!(2, func.inputs.len());
        assert_eq!(1, func.outputs.len());
        assert_eq!(func.signature(), "allowance(address,address)");

        // encode
        let expected = base_primitives::hex!(
            "0bf3a456"
            "0000000000000000000011111111111111111111111111111111111111111111"
            "0000000000000000000022222222222222222222222222222222222222222222"
        );
        let input = [
            DynYlmValue::Address(IcanAddress::repeat_byte(0x11)),
            DynYlmValue::Address(IcanAddress::repeat_byte(0x22)),
        ];
        let result = func.abi_encode_input(&input).unwrap();
        assert_eq!(expected[..], result);

        // Fail on unexpected input
        let wrong_input = [
            DynYlmValue::Uint(U256::from(10u8), 256),
            DynYlmValue::Address(IcanAddress::repeat_byte(2u8)),
        ];
        assert!(func.abi_encode_input(&wrong_input).is_err());

        // decode
        let response = U256::from(1u8).to_be_bytes_vec();
        let decoded = func.abi_decode_output(&response, true).unwrap();
        assert_eq!(decoded, [DynYlmValue::Uint(U256::from(1u8), 256)]);

        // Fail on wrong response type
        let bad_response = IcanAddress::repeat_byte(3u8).to_vec();
        assert!(func.abi_decode_output(&bad_response, true).is_err());
        assert!(func.abi_decode_output(&bad_response, false).is_err());
    }

    // https://github.com/foundry-rs/foundry/issues/7280
    // Same as `encode_empty_bytes_array_in_tuple` in ylm-types.
    #[test]
    fn empty_bytes_array() {
        let func = Function::parse("register(bytes,address,bytes[])").unwrap();
        let input = [
            DynYlmValue::Bytes(bytes!("09736b79736b79736b79026f7300").into()),
            DynYlmValue::Address(cAddress!("0000B7b54cd129e6D8B24e6AE652a473449B273eE3E4")),
            DynYlmValue::Array(vec![]),
        ];
        let result = func.abi_encode_input(&input).unwrap();

        let expected = hex!(
            "
            99c0f2b3
            0000000000000000000000000000000000000000000000000000000000000060
            000000000000000000000000B7b54cd129e6D8B24e6AE652a473449B273eE3E4
            00000000000000000000000000000000000000000000000000000000000000a0
            000000000000000000000000000000000000000000000000000000000000000e
            09736b79736b79736b79026f7300000000000000000000000000000000000000
            0000000000000000000000000000000000000000000000000000000000000000
    	"
        );
        assert_eq!(hex::encode(expected), hex::encode(result));
    }
}
