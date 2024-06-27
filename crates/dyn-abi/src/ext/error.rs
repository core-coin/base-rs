use crate::{DynYlmError, Specifier};
use alloc::vec::Vec;
use base_json_abi::Error;
use base_primitives::{sha3, Selector};

mod sealed {
    pub trait Sealed {}
    impl Sealed for base_json_abi::Error {}
}
use sealed::Sealed;

impl Specifier<DynYlmError> for Error {
    fn resolve(&self) -> crate::Result<DynYlmError> {
        let signature = self.signature();
        let selector = Selector::from_slice(&sha3(signature)[0..4]);

        let mut body = Vec::with_capacity(self.inputs.len());
        for param in &self.inputs {
            body.push(param.resolve()?);
        }

        Ok(DynYlmError::new_unchecked(selector, crate::DynYlmType::Tuple(body)))
    }
}

/// Provides error encoding and decoding for the [`Error`] type.
pub trait ErrorExt: Sealed {
    /// Decode the error from the given data.
    fn decode_error(&self, data: &[u8]) -> crate::Result<crate::DecodedError>;
}

impl ErrorExt for base_json_abi::Error {
    fn decode_error(&self, data: &[u8]) -> crate::Result<crate::DecodedError> {
        self.resolve()?.decode_error(data)
    }
}
