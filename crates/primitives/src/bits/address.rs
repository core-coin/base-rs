use crate::{aliases::U160, utils::keccak256, ChecksumAddress, FixedBytes};
use core::{fmt, str};
use ruint::aliases::U256;

const MAINNET: u64 = 203;
const DEVIN: u64 = 171;
const PRIVATE: u64 = 206;

/// Error type for address checksum validation.
#[derive(Debug, Copy, Clone)]
pub enum AddressError {
    /// Error while decoding hex.
    Hex(hex::FromHexError),

    /// Invalid ERC-55 checksum.
    InvalidChecksum,
}

impl From<hex::FromHexError> for AddressError {
    #[inline]
    fn from(value: hex::FromHexError) -> Self {
        Self::Hex(value)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for AddressError {
    #[inline]
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Hex(err) => Some(err),
            Self::InvalidChecksum => None,
        }
    }
}

impl fmt::Display for AddressError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Hex(err) => err.fmt(f),
            Self::InvalidChecksum => f.write_str("Bad address checksum"),
        }
    }
}

wrap_fixed_bytes!(
    // we implement Display with the checksum, so we don't derive it
    extra_derives: [],
    /// An Ethereum address, 20 bytes in length.
    ///
    /// This type is separate from [`B160`](crate::B160) / [`FixedBytes<20>`]
    /// and is declared with the [`wrap_fixed_bytes!`] macro. This allows us
    /// to implement address-specific functionality.
    ///
    /// The main difference with the generic [`FixedBytes`] implementation is that
    /// [`Display`] formats the address using its [EIP-55] checksum
    /// ([`to_checksum`]).
    /// Use [`Debug`] to display the raw bytes without the checksum.
    ///
    /// [EIP-55]: https://eips.ethereum.org/EIPS/eip-55
    /// [`Debug`]: fmt::Debug
    /// [`Display`]: fmt::Display
    /// [`to_checksum`]: Address::to_checksum
    ///
    /// # Examples
    ///
    /// Parsing and formatting:
    ///
    /// ```
    /// use alloy_primitives::{address, Address};
    ///
    /// let checksummed = "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045";
    /// let expected = address!("d8da6bf26964af9d7eed9e03e53415d37aa96045");
    /// let address = Address::parse_checksummed(checksummed, None).expect("valid checksum");
    /// assert_eq!(address, expected);
    ///
    /// // Format the address with the checksum
    /// assert_eq!(address.to_string(), checksummed);
    /// assert_eq!(address.to_checksum(None), checksummed);
    ///
    /// // Format the compressed checksummed address
    /// assert_eq!(format!("{address:#}"), "0xd8dA…6045");
    ///
    /// // Format the address without the checksum
    /// assert_eq!(format!("{address:?}"), "0xd8da6bf26964af9d7eed9e03e53415d37aa96045");
    /// ```
    pub struct Address<20>;
);

impl From<U160> for Address {
    #[inline]
    fn from(value: U160) -> Self {
        Self(FixedBytes(value.to_be_bytes()))
    }
}

impl From<Address> for U160 {
    #[inline]
    fn from(value: Address) -> Self {
        Self::from_be_bytes(value.0 .0)
    }
}

impl Address {
    /// Creates an Ethereum address from an EVM word's upper 20 bytes
    /// (`word[12..]`).
    ///
    /// # Examples
    ///
    /// ```
    /// # use alloy_primitives::{address, b256, Address};
    /// let word = b256!("000000000000000000000000d8da6bf26964af9d7eed9e03e53415d37aa96045");
    /// assert_eq!(Address::from_word(word), address!("d8da6bf26964af9d7eed9e03e53415d37aa96045"));
    /// ```
    #[inline]
    #[must_use]
    pub fn from_word(word: FixedBytes<32>) -> Self {
        Self(FixedBytes(word[12..].try_into().unwrap()))
    }

    /// Left-pads the address to 32 bytes (EVM word size).
    ///
    /// # Examples
    ///
    /// ```
    /// # use alloy_primitives::{address, b256, Address};
    /// assert_eq!(
    ///     address!("d8da6bf26964af9d7eed9e03e53415d37aa96045").into_word(),
    ///     b256!("000000000000000000000000d8da6bf26964af9d7eed9e03e53415d37aa96045"),
    /// );
    /// ```
    #[inline]
    #[must_use]
    pub fn into_word(&self) -> FixedBytes<32> {
        let mut word = [0; 32];
        word[12..].copy_from_slice(self.as_slice());
        FixedBytes(word)
    }

    /// Encodes an Ethereum address to its [EIP-55] checksum into a heap-allocated string.
    ///
    /// You can optionally specify an [EIP-155 chain ID] to encode the address
    /// using [EIP-1191].
    ///
    /// [EIP-55]: https://eips.ethereum.org/EIPS/eip-55
    /// [EIP-155 chain ID]: https://eips.ethereum.org/EIPS/eip-155
    /// [EIP-1191]: https://eips.ethereum.org/EIPS/eip-1191
    ///
    /// # Examples
    ///
    /// ```
    /// # use alloy_primitives::{address, Address};
    /// let address = address!("d8da6bf26964af9d7eed9e03e53415d37aa96045");
    ///
    /// let checksummed: String = address.to_checksum(None);
    /// assert_eq!(checksummed, "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045");
    ///
    /// let checksummed: String = address.to_checksum(Some(1));
    /// assert_eq!(checksummed, "0xD8Da6bf26964Af9d7EEd9e03e53415d37AA96045");
    /// ```
    #[inline]
    #[must_use]
    pub fn to_ican(&self, network_id: u64) -> ChecksumAddress {
        let network_prefix = match network_id {
            1 => MAINNET,
            3 => DEVIN,
            _ => PRIVATE,
        };
        let network_prefix = U256::from(network_prefix);

        let mut value = [0; 32];
        value[12..].copy_from_slice(&self.0 .0);
        let value_new = U256::from_be_bytes(value);
        let value = (value_new << 16) + (network_prefix << 8);
        let mut v = value;
        let mut s = U256::from(0);
        let mut x = U256::from(1);
        for _ in 0..44 {
            let t = v & U256::from(0x0f);
            s = s + t * x;
            x *= U256::from(10) + U256::from(90) * (t / U256::from(10));
            v >>= 4;
        }
        s = s % U256::from(97);
        s = U256::from(98) - s;
        s = (s % U256::from(10)) + (s / U256::from(10)) * U256::from(16);
        let result: U256 = value_new + (s << 160) + (network_prefix << 168);
        ChecksumAddress::from_word(result.into())
    }

    /// Instantiate by hashing public key bytes.
    ///
    /// # Panics
    ///
    /// If the input is not exactly 64 bytes
    pub fn from_raw_public_key(pubkey: &[u8]) -> Self {
        assert_eq!(pubkey.len(), 114, "raw public key must be 64 bytes");
        let digest = keccak256(pubkey);
        Self::from_slice(&digest[12..])
    }

    // /// Converts an ECDSA verifying key to its corresponding Ethereum address.
    // #[inline]
    // #[cfg(feature = "k256")]
    // #[doc(alias = "from_verifying_key")]
    // pub fn from_public_key(pubkey: &k256::ecdsa::VerifyingKey) -> Self {
    //     use k256::elliptic_curve::sec1::ToEncodedPoint;
    //     let affine: &k256::AffinePoint = pubkey.as_ref();
    //     let encoded = affine.to_encoded_point(false);
    //     Self::from_raw_public_key(&encoded.as_bytes()[1..])
    // }
    //
    // /// Converts an ECDSA signing key to its corresponding Ethereum address.
    // #[inline]
    // #[cfg(feature = "k256")]
    // #[doc(alias = "from_signing_key")]
    // pub fn from_private_key(private_key: &k256::ecdsa::SigningKey) -> Self {
    //     Self::from_public_key(private_key.verifying_key())
    // }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hex;

    #[test]
    fn parse() {
        let expected = hex!("0102030405060708090a0b0c0d0e0f1011121314");
        assert_eq!(
            "0102030405060708090a0b0c0d0e0f1011121314".parse::<Address>().unwrap().into_array(),
            expected
        );
        assert_eq!(
            "0x0102030405060708090a0b0c0d0e0f1011121314".parse::<Address>().unwrap(),
            expected
        );
    }

    #[test]
    fn checksum_network_id() {
        let addresses = [
            (
                "0x632ed69c17d318372233bcbac849317f4de784e2",
                "cb88632ed69c17d318372233bcbac849317f4de784e2",
                1,
            ),
            (
                "0x58b39698a44bdae37f881e68dce073823a48a631",
                "cb1958b39698a44bdae37f881e68dce073823a48a631",
                1,
            ),
            (
                "2215c43fc213c02182c8389f2bc32408e2c50922",
                "ab792215c43fc213c02182c8389f2bc32408e2c50922",
                3,
            ),
            (
                "01216456e807f27206341d9a04177af91a7abbc0",
                "ab6501216456e807f27206341d9a04177af91a7abbc0",
                3,
            ),
        ];
        for (address, expected, network_id) in addresses {
            let parsed: Address = address.parse().unwrap();
            let expected: ChecksumAddress = expected.parse().unwrap();
            let parsed = parsed.to_ican(network_id);
            assert_eq!(parsed, expected);
        }
    }

    // #[test]
    // fn test_raw_public_key_to_address() {
    //     let addr = "0Ac1dF02185025F65202660F8167210A80dD5086".parse::<Address>().unwrap();
    //
    //     let pubkey_bytes = hex::decode("76698beebe8ee5c74d8cc50ab84ac301ee8f10af6f28d0ffd6adf4d6d3b9b762d46ca56d3dad2ce13213a6f42278dabbb53259f2d92681ea6a0b98197a719be3").unwrap();
    //
    //     assert_eq!(Address::from_raw_public_key(&pubkey_bytes), addr);
    // }
}
