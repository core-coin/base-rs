use crate::{hex, signature::SignatureError, IcanAddress, B1368};
use alloc::vec::Vec;
use core::{net, str::FromStr};
use libgoldilocks::{
    errors::LibgoldilockErrors,
    goldilocks::{ed448_sign, ed448_verify_with_error},
    PrehashSigner, SigningKey, VerifyingKey,
};

#[cfg(feature = "arbitrary")]
use arbitrary::Arbitrary;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// An Core ECDSA signature.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Signature {
    sig: B1368,
}

impl PrehashSigner<Signature> for SigningKey {
    fn sign_prehash(&self, prehash: &[u8]) -> Result<Signature, LibgoldilockErrors> {
        let sig = ed448_sign(&self.to_bytes(), prehash);
        let mut sig_with_private_key: [u8; 171] = [0; 171];
        sig_with_private_key[0..114].copy_from_slice(&sig);
        sig_with_private_key[114..171].copy_from_slice(&self.verifying_key().as_bytes());

        Ok(Signature { sig: sig_with_private_key.into() })
    }
}

impl<'a> TryFrom<&'a [u8]> for Signature {
    type Error = SignatureError;

    fn try_from(bytes: &'a [u8]) -> Result<Self, Self::Error> {
        if bytes.len() != 171 {
            return Err(SignatureError::FromBytes("expected exactly 171 bytes"));
        }
        Self::from_bytes(bytes)
    }
}

impl FromStr for Signature {
    type Err = SignatureError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bytes = hex::decode(s)?;
        Self::try_from(&bytes[..])
    }
}

impl From<&crate::Signature> for [u8; 171] {
    #[inline]
    fn from(value: &crate::Signature) -> [u8; 171] {
        value.as_bytes()
    }
}

impl From<crate::Signature> for [u8; 171] {
    #[inline]
    fn from(value: crate::Signature) -> [u8; 171] {
        value.as_bytes()
    }
}

impl From<&crate::Signature> for Vec<u8> {
    #[inline]
    fn from(value: &crate::Signature) -> Self {
        value.as_bytes().to_vec()
    }
}

impl From<crate::Signature> for Vec<u8> {
    #[inline]
    fn from(value: crate::Signature) -> Self {
        value.as_bytes().to_vec()
    }
}

#[cfg(feature = "rlp")]
impl crate::Signature {
    pub fn decode_rlp_sig(buf: &mut &[u8]) -> Result<Self, alloy_rlp::Error> {
        use alloy_rlp::Decodable;

        let sig: [u8; 171] = Decodable::decode(buf)?;

        Self::from_bytes(&sig)
            .map_err(|_| alloy_rlp::Error::Custom("attempted to decode invalid field element"))
    }
}

impl Signature {
    /// Returns the `r` component of this signature.
    #[inline]
    pub const fn sig(&self) -> B1368 {
        self.sig
    }

    /// Returns the `s` component of this signature.

    /// Returns the byte-array representation of this signature.
    ///
    /// The first 32 bytes are the `r` value, the second 32 bytes the `s` value
    /// and the final byte is the `v` value in 'Electrum' notation.
    #[inline]
    pub fn as_bytes(&self) -> [u8; 171] {
        let mut sig = [0u8; 171];
        sig.copy_from_slice(self.sig.as_slice());
        sig
    }

    #[inline]
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, SignatureError> {
        let sig = B1368::from_slice(bytes);
        Ok(Self { sig })
    }

    /// Length of RLP RS field encoding
    #[cfg(feature = "rlp")]
    pub fn rlp_len(&self) -> usize {
        alloy_rlp::Encodable::length(&self.sig)
    }

    /// Write R and S to an RLP buffer in progress.
    #[cfg(feature = "rlp")]
    pub fn write_rlp(&self, out: &mut dyn alloy_rlp::BufMut) {
        alloy_rlp::Encodable::encode(&self.sig, out);
    }

    /// Recovers an [`Address`] from this signature and the given message by first prefixing and
    /// hashing the message according to [EIP-191](crate::eip191_hash_message).
    ///
    /// [`IcanAddress`]: IcanAddress
    #[inline]
    pub fn recover_address_from_msg<T: AsRef<[u8]>>(
        &self,
        msg: T,
        network_id: u64,
    ) -> Result<IcanAddress, SignatureError> {
        self.recover_from_msg(msg).map(|vk| IcanAddress::from_public_key(&vk, network_id))
    }

    /// Recovers an [`Address`] from this signature and the given prehashed message.
    ///
    /// [`IcanAddress`]: IcanAddress
    #[inline]
    pub fn recover_address_from_prehash(
        &self,
        prehash: &crate::B256,
        network_id: u64,
    ) -> Result<IcanAddress, SignatureError> {
        self.recover_from_prehash(prehash).map(|vk| IcanAddress::from_public_key(&vk, network_id))
    }

    /// Recovers a [`VerifyingKey`] from this signature and the given message by first prefixing and
    /// hashing the message according to [EIP-191](crate::eip191_hash_message).
    ///
    /// [`VerifyingKey`]: VerifyingKey
    #[inline]
    pub fn recover_from_msg<T: AsRef<[u8]>>(&self, msg: T) -> Result<VerifyingKey, SignatureError> {
        self.recover_from_prehash(&crate::eip191_hash_message(msg))
    }

    /// Recovers a [`VerifyingKey`] from this signature and the given prehashed message.
    ///
    /// [`VerifyingKey`]: VerifyingKey
    #[inline]
    pub fn recover_from_prehash(
        &self,
        prehash: &crate::B256,
    ) -> Result<VerifyingKey, SignatureError> {
        ed448_verify_with_error(
            &self.as_bytes()[114..],
            &self.as_bytes()[..114],
            prehash.as_slice(),
        )
        .map_err(|err| SignatureError::Libgoldilocks(err))?;

        Ok(VerifyingKey::from_bytes(&self.as_bytes()[114..]))
    }
}

#[cfg(feature = "rlp")]
impl alloy_rlp::Encodable for crate::Signature {
    fn encode(&self, out: &mut dyn alloy_rlp::BufMut) {
        alloy_rlp::Header { list: true, payload_length: self.rlp_len() }.encode(out);
        self.write_rlp(out);
    }

    fn length(&self) -> usize {
        let payload_length = self.rlp_len();
        payload_length + alloy_rlp::length_of_length(payload_length)
    }
}

#[cfg(feature = "rlp")]
impl alloy_rlp::Decodable for crate::Signature {
    fn decode(buf: &mut &[u8]) -> Result<Self, alloy_rlp::Error> {
        let header = alloy_rlp::Header::decode(buf)?;
        let pre_len = buf.len();
        let decoded = Self::decode_rlp_sig(buf)?;
        let consumed = pre_len - buf.len();
        if consumed != header.payload_length {
            return Err(alloy_rlp::Error::Custom("consumed incorrect number of bytes"));
        }

        Ok(decoded)
    }
}

#[cfg(test)]
#[allow(unused_imports)]
mod tests {
    use ruint::aliases::U176;

    use super::*;
    use crate::B1368;
    use core::{fmt::Debug, str::FromStr};

    #[test]
    fn test_from_str() {
        let sig = crate::Signature::from_str(
            "ea535a535ff0dbfda0b2c1394bad87311789c1c6eafe6eef48fd509c2e7ba0e67c4774fab8c45abf1c7e22532bb816115bf1da8438fdb81e00e13ca01494adc201c9c35bc32cdd7c1922a0b1121f1d8ed72b37786dfd6e5583b06ad172bdb4f1d2afd41b4444abd2b5901c851fcb3d641200fadc64a37e95ad1bcbaf19625bf95826e6a8cbab42b57fc91b72da98d26bae8bda2d1fc52c508a03724aded17b8cef8253f2116307bbbf7580",
        );
        assert!(sig.is_ok());
        assert_eq!(sig.unwrap().sig().len(), 171);
    }

    #[test]
    fn signature_inner() {
        let sig: Result<crate::signature::Signature, crate::SignatureError> = crate::Signature::from_str(
            "ea535a535ff0dbfda0b2c1394bad87311789c1c6eafe6eef48fd509c2e7ba0e67c4774fab8c45abf1c7e22532bb816115bf1da8438fdb81e00e13ca01494adc201c9c35bc32cdd7c1922a0b1121f1d8ed72b37786dfd6e5583b06ad172bdb4f1d2afd41b4444abd2b5901c851fcb3d641200fadc64a37e95ad1bcbaf19625bf95826e6a8cbab42b57fc91b72da98d26bae8bda2d1fc52c508a03724aded17b8cef8253f2116307bbbf7580",
        );
        let inner: B1368 = B1368::from_str("ea535a535ff0dbfda0b2c1394bad87311789c1c6eafe6eef48fd509c2e7ba0e67c4774fab8c45abf1c7e22532bb816115bf1da8438fdb81e00e13ca01494adc201c9c35bc32cdd7c1922a0b1121f1d8ed72b37786dfd6e5583b06ad172bdb4f1d2afd41b4444abd2b5901c851fcb3d641200fadc64a37e95ad1bcbaf19625bf95826e6a8cbab42b57fc91b72da98d26bae8bda2d1fc52c508a03724aded17b8cef8253f2116307bbbf7580").unwrap();
        assert_eq!(sig.unwrap().sig().0, inner.0);
    }

    #[test]
    fn recover_address_from_prehash() {
        let sig = crate::Signature::from_str(
            "ea535a535ff0dbfda0b2c1394bad87311789c1c6eafe6eef48fd509c2e7ba0e67c4774fab8c45abf1c7e22532bb816115bf1da8438fdb81e00e13ca01494adc201c9c35bc32cdd7c1922a0b1121f1d8ed72b37786dfd6e5583b06ad172bdb4f1d2afd41b4444abd2b5901c851fcb3d641200fadc64a37e95ad1bcbaf19625bf95826e6a8cbab42b57fc91b72da98d26bae8bda2d1fc52c508a03724aded17b8cef8253f2116307bbbf7580",
        ).unwrap();
        let addr = sig
            .recover_address_from_prehash(
                &b256!("ce0677bb30baa8cf067c88db9811f4333d131bf8bcf12fe7065d211dce971008"),
                1,
            )
            .unwrap();
        assert_eq!(
            addr,
            IcanAddress::from_str("cb72355e4fdb2edb55c4a747c899505d393aa6628590").unwrap()
        );
    }

    #[test]
    fn recover_address_from_msg() {
        let sig = crate::Signature::from_str(
            "0x1e9e2b20b92cc21257764ffccc5e0ad7f9a350d4e6ece497f5856abb1fb244eaf527035814e28ac4d1eb905fd7ee3bc5b8aab5a79a8243f6804ef8b60e89c248473fde7150d43eb03b27623f354cc8965b8cdfe5029ea8a033d3143fe69a1d86c331b41588c336a050e5e6395508ec7e22004c4a20a489260a4f5829c04101e75ac20947d60eb01fbd29a96d48c02639384d2806c4263340153194e7a3638ec2cca39938c1b74be200f080",
        ).unwrap();
        let addr = sig.recover_address_from_msg("Hello, world!", 1).unwrap();
        assert_eq!(
            addr,
            IcanAddress::from_str("cb43bfd3937bfb2cd1b2b36253b43f60a1487ea4af3c").unwrap()
        );
    }

    #[test]
    fn sign_prehash() {
        let prehash = crate::eip191_hash_message("Hello, world!");

        let key = SigningKey::from_str("ce0677bb30baa8cf067c88db9811f4333d131bf8bcf12fe7065d211dce971008ce0677bb30baa8cf067c88db9811f4333d131bf8bcf12fe706");
        let sig: Signature = key.sign_prehash(&prehash.0).unwrap();

        assert_eq!(sig.sig.to_string(), "0x1e9e2b20b92cc21257764ffccc5e0ad7f9a350d4e6ece497f5856abb1fb244eaf527035814e28ac4d1eb905fd7ee3bc5b8aab5a79a8243f6804ef8b60e89c248473fde7150d43eb03b27623f354cc8965b8cdfe5029ea8a033d3143fe69a1d86c331b41588c336a050e5e6395508ec7e22004c4a20a489260a4f5829c04101e75ac20947d60eb01fbd29a96d48c02639384d2806c4263340153194e7a3638ec2cca39938c1b74be200f080");
        assert_eq!(
            prehash.to_string(),
            "0x5a715dc3d0332f9d07824171d604d0cec9475f4299605e8c588d071a0c6c15cc"
        );
    }

    #[test]
    fn recover_from_prehash() {
        let sig = crate::Signature::from_str(
            "1e9e2b20b92cc21257764ffccc5e0ad7f9a350d4e6ece497f5856abb1fb244eaf527035814e28ac4d1eb905fd7ee3bc5b8aab5a79a8243f6804ef8b60e89c248473fde7150d43eb03b27623f354cc8965b8cdfe5029ea8a033d3143fe69a1d86c331b41588c336a050e5e6395508ec7e22004c4a20a489260a4f5829c04101e75ac20947d60eb01fbd29a96d48c02639384d2806c4263340153194e7a3638ec2cca39938c1b74be200f080",
        ).unwrap();
        let key = sig
            .recover_from_prehash(&b256!(
                "5a715dc3d0332f9d07824171d604d0cec9475f4299605e8c588d071a0c6c15cc"
            ))
            .unwrap();
        assert_eq!(key, VerifyingKey::from_str("4c4a20a489260a4f5829c04101e75ac20947d60eb01fbd29a96d48c02639384d2806c4263340153194e7a3638ec2cca39938c1b74be200f080"));
    }

    #[test]
    fn recover_from_msg() {
        let sig = crate::Signature::from_str(
            "1e9e2b20b92cc21257764ffccc5e0ad7f9a350d4e6ece497f5856abb1fb244eaf527035814e28ac4d1eb905fd7ee3bc5b8aab5a79a8243f6804ef8b60e89c248473fde7150d43eb03b27623f354cc8965b8cdfe5029ea8a033d3143fe69a1d86c331b41588c336a050e5e6395508ec7e22004c4a20a489260a4f5829c04101e75ac20947d60eb01fbd29a96d48c02639384d2806c4263340153194e7a3638ec2cca39938c1b74be200f080",
        ).unwrap();
        let addr = sig.recover_from_msg("Hello, world!").unwrap();
        assert_eq!(addr, VerifyingKey::from_str("4c4a20a489260a4f5829c04101e75ac20947d60eb01fbd29a96d48c02639384d2806c4263340153194e7a3638ec2cca39938c1b74be200f080"));
    }

    //
    // #[cfg(feature = "rlp")]
    // use alloy_rlp::{Decodable, Encodable};
    //
    // #[test]
    // fn signature_from_str() {
    //     let s1 = crate::Signature::from_str(
    //         "0xaa231fbe0ed2b5418e6ba7c19bee2522852955ec50996c02a2fe3e71d30ddaf1645baf4823fea7cb4fcc7150842493847cfb6a6d63ab93e8ee928ee3f61f503500"
    //     ).expect("could not parse 0x-prefixed signature");
    //
    //     let s2 = crate::Signature::from_str(
    //         "aa231fbe0ed2b5418e6ba7c19bee2522852955ec50996c02a2fe3e71d30ddaf1645baf4823fea7cb4fcc7150842493847cfb6a6d63ab93e8ee928ee3f61f503500"
    //     ).expect("could not parse non-prefixed signature");
    //
    //     assert_eq!(s1, s2);
    // }
    //
    // #[cfg(feature = "serde")]
    // #[test]
    // fn deserialize_without_parity() {
    //     let raw_signature_without_y_parity = r#"{
    //         "r":"0xc569c92f176a3be1a6352dd5005bfc751dcb32f57623dd2a23693e64bf4447b0",
    //         "s":"0x1a891b566d369e79b7a66eecab1e008831e22daa15f91a0a0cf4f9f28f47ee05",
    //         "v":"0x1"
    //     }"#;
    //
    //     let signature: crate::Signature =
    //         serde_json::from_str(raw_signature_without_y_parity).unwrap();
    //
    //     let expected = crate::Signature::from_rs_and_parity(
    //         U256::from_str("0xc569c92f176a3be1a6352dd5005bfc751dcb32f57623dd2a23693e64bf4447b0")
    //             .unwrap(),
    //         U256::from_str("0x1a891b566d369e79b7a66eecab1e008831e22daa15f91a0a0cf4f9f28f47ee05")
    //             .unwrap(),
    //         1,
    //     )
    //     .unwrap();
    //
    //     assert_eq!(signature, expected);
    // }
    //
    // #[cfg(feature = "serde")]
    // #[test]
    // fn deserialize_with_parity() {
    //     let raw_signature_with_y_parity = serde_json::json!(
    //         {
    //         "r":"0xc569c92f176a3be1a6352dd5005bfc751dcb32f57623dd2a23693e64bf4447b0",
    //         "s":"0x1a891b566d369e79b7a66eecab1e008831e22daa15f91a0a0cf4f9f28f47ee05",
    //         "v":"0x1",
    //         "yParity": "0x1"
    //     }
    //     );
    //
    //     println!("{raw_signature_with_y_parity}");
    //     let signature: crate::Signature =
    //         serde_json::from_value(raw_signature_with_y_parity).unwrap();
    //
    //     let expected = crate::Signature::from_rs_and_parity(
    //         U256::from_str("0xc569c92f176a3be1a6352dd5005bfc751dcb32f57623dd2a23693e64bf4447b0")
    //             .unwrap(),
    //         U256::from_str("0x1a891b566d369e79b7a66eecab1e008831e22daa15f91a0a0cf4f9f28f47ee05")
    //             .unwrap(),
    //         1,
    //     )
    //     .unwrap();
    //
    //     assert_eq!(signature, expected);
    // }
    //
    // #[cfg(feature = "serde")]
    // #[test]
    // fn serialize_both_parity() {
    //     // this test should be removed if the struct moves to an enum based on tx type
    //     let signature = crate::Signature::from_rs_and_parity(
    //         U256::from_str("0xc569c92f176a3be1a6352dd5005bfc751dcb32f57623dd2a23693e64bf4447b0")
    //             .unwrap(),
    //         U256::from_str("0x1a891b566d369e79b7a66eecab1e008831e22daa15f91a0a0cf4f9f28f47ee05")
    //             .unwrap(),
    //         1,
    //     )
    //     .unwrap();
    //
    //     let serialized = serde_json::to_string(&signature).unwrap();
    //     assert_eq!(
    //         serialized,
    //         r#"{"r":"0xc569c92f176a3be1a6352dd5005bfc751dcb32f57623dd2a23693e64bf4447b0","s":"
    // 0x1a891b566d369e79b7a66eecab1e008831e22daa15f91a0a0cf4f9f28f47ee05","yParity":"0x1"}"#
    //     );
    // }
    //
    // #[cfg(feature = "serde")]
    // #[test]
    // fn serialize_v_only() {
    //     // this test should be removed if the struct moves to an enum based on tx type
    //     let signature = crate::Signature::from_rs_and_parity(
    //         U256::from_str("0xc569c92f176a3be1a6352dd5005bfc751dcb32f57623dd2a23693e64bf4447b0")
    //             .unwrap(),
    //         U256::from_str("0x1a891b566d369e79b7a66eecab1e008831e22daa15f91a0a0cf4f9f28f47ee05")
    //             .unwrap(),
    //         1,
    //     )
    //     .unwrap();
    //
    //     let expected =
    // r#"{"r":"0xc569c92f176a3be1a6352dd5005bfc751dcb32f57623dd2a23693e64bf4447b0","s":"
    // 0x1a891b566d369e79b7a66eecab1e008831e22daa15f91a0a0cf4f9f28f47ee05","yParity":"0x1"}"#;
    //
    //     let serialized = serde_json::to_string(&signature).unwrap();
    //     assert_eq!(serialized, expected);
    // }
    //
    // #[cfg(feature = "serde")]
    // #[test]
    // fn test_bincode_roundtrip() {
    //     let signature = crate::Signature::from_rs_and_parity(
    //         U256::from_str("0xc569c92f176a3be1a6352dd5005bfc751dcb32f57623dd2a23693e64bf4447b0")
    //             .unwrap(),
    //         U256::from_str("0x1a891b566d369e79b7a66eecab1e008831e22daa15f91a0a0cf4f9f28f47ee05")
    //             .unwrap(),
    //         1,
    //     )
    //     .unwrap();
    //
    //     let bin = bincode::serialize(&signature).unwrap();
    //     assert_eq!(bincode::deserialize::<crate::Signature>(&bin).unwrap(), signature);
    // }
    //
    // #[cfg(feature = "rlp")]
    // #[test]
    // fn signature_rlp_decode() {
    //     // Given a hex-encoded byte sequence
    //     let bytes =
    // crate::hex!("
    // f84301a048b55bfa915ac795c431978d8a6a992b628d557da5ff759b307d495a36649353a010002cef538bc0c8e21c46080634a93e082408b0ad93f4a7207e63ec5463793d"
    // );
    //
    //     // Decode the byte sequence into a Signature instance
    //     let result = Signature::decode(&mut &bytes[..]).unwrap();
    //
    //     // Assert that the decoded Signature matches the expected Signature
    //     assert_eq!(
    //         result,
    //         Signature::from_str("48b55bfa915ac795c431978d8a6a992b628d557da5ff759b307d495a3664935310002cef538bc0c8e21c46080634a93e082408b0ad93f4a7207e63ec5463793d01").unwrap()
    //     );
    // }
    //
    // #[cfg(feature = "rlp")]
    // #[test]
    // fn signature_rlp_encode() {
    //     // Given a Signature instance
    //     let sig =
    // Signature::from_str("
    // 48b55bfa915ac795c431978d8a6a992b628d557da5ff759b307d495a36649353efffd310ac743f371de3b9f7f9cb56c0b28ad43601b4ab949f53faa07bd2c8041b"
    // ).unwrap();
    //
    //     // Initialize an empty buffer
    //     let mut buf = vec![];
    //
    //     // Encode the Signature into the buffer
    //     sig.encode(&mut buf);
    //
    //     // Define the expected hex-encoded string
    //     let expected =
    // "f8431ba048b55bfa915ac795c431978d8a6a992b628d557da5ff759b307d495a36649353a0efffd310ac743f371de3b9f7f9cb56c0b28ad43601b4ab949f53faa07bd2c804"
    // ;
    //
    //     // Assert that the encoded buffer matches the expected hex-encoded string
    //     assert_eq!(hex::encode(&buf), expected);
    // }
    //
    // #[cfg(feature = "rlp")]
    // #[test]
    // fn signature_rlp_length() {
    //     // Given a Signature instance
    //     let sig =
    // Signature::from_str("
    // 48b55bfa915ac795c431978d8a6a992b628d557da5ff759b307d495a36649353efffd310ac743f371de3b9f7f9cb56c0b28ad43601b4ab949f53faa07bd2c8041b"
    // ).unwrap();
    //
    //     // Assert that the length of the Signature matches the expected length
    //     assert_eq!(sig.length(), 69);
    // }
}
