use crate::{aliases::U160, utils::sha3, FixedBytes, IcanAddress};
use core::{
    borrow::Borrow,
    fmt::{self, Display},
    str,
};
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
    /// An Core address, 20 bytes in length.
    ///
    /// This type is separate from [`B160`](crate::B160) / [`FixedBytes<20>`]
    /// and is declared with the [`wrap_fixed_bytes!`] macro. This allows us
    /// to implement address-specific functionality.
    ///
    /// The main difference with the generic [`FixedBytes`] implementation is that
    /// [`Display`] formats the address using its [EIP-55] checksum
    /// ([`to_checksum`]).
    /// Use [`Debug`] to display the raw bytes without the checksum.
    pub struct Address<20>;
);

impl Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&format!("{:?}", self))
    }
}

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
    /// Creates an Core address from an EVM word's upper 20 bytes
    /// (`word[12..]`).
    ///
    /// # Examples
    ///
    /// ```
    /// # use base_primitives::{address, b256, Address};
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
    /// # use base_primitives::{address, b256, Address};
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

    /// Encodes an Core address to Ican Address
    ///
    /// # Examples
    /// ```
    /// # use base_primitives::{address, Address};
    /// let address = address!("d8da6bf26964af9d7eed9e03e53415d37aa96045");
    ///
    /// let checksummed = address.to_ican(1);
    /// assert_eq!(checksummed, address!("d8dA6BF26964aF9D7eEd9e03E53415D37aA96045").to_ican(1));
    ///
    /// let checksummed = address.to_ican(3);
    /// assert_eq!(checksummed, address!("D8Da6bf26964Af9d7EEd9e03e53415d37AA96045").to_ican(3));
    /// ```
    #[inline]
    #[must_use]
    pub fn to_ican(&self, network_id: u64) -> IcanAddress {
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
            s += t * x;
            x *= U256::from(10) + U256::from(90) * (t / U256::from(10));
            v >>= 4;
        }
        s %= U256::from(97);
        s = U256::from(98) - s;
        s = (s % U256::from(10)) + (s / U256::from(10)) * U256::from(16);
        let result: U256 = value_new + (s << 160) + (network_prefix << 168);
        IcanAddress::from_word(result.into())
    }

    /// Computes the `CREATE2` address
    #[must_use]
    pub fn create2<S, H>(&self, salt: S, init_code_hash: H, network_id: u64) -> IcanAddress
    where
        // not `AsRef` because `[u8; N]` does not implement `AsRef<[u8; N]>`
        S: Borrow<[u8; 32]>,
        H: Borrow<[u8; 32]>,
    {
        self.to_ican(network_id).create2(salt, init_code_hash)
    }

    /// Computes the `CREATE` address
    #[cfg(feature = "rlp")]
    #[inline]
    #[must_use]
    pub fn create(&self, nonce: u64, network_id: u64) -> IcanAddress {
        self.to_ican(network_id).create(nonce)
    }

    /// Instantiate by hashing public key bytes.
    ///
    /// # Panics
    ///
    /// If the input is not exactly 57 bytes
    pub fn from_raw_public_key(pubkey: &[u8]) -> Self {
        assert_eq!(pubkey.len(), 57, "raw public key must be 57 bytes");
        let digest = sha3(pubkey);
        Self::from_slice(&digest[12..])
    }

    // /// Converts an ECDSA verifying key to its corresponding Core address.
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
    // /// Converts an ECDSA signing key to its corresponding Core address.
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
            let expected: IcanAddress = expected.parse().unwrap();
            let parsed = parsed.to_ican(network_id);
            assert_eq!(parsed, expected);
        }
    }

    // #[test]
    // fn test_raw_public_key_to_address() {
    //     let addr = "0Ac1dF02185025F65202660F8167210A80dD5086".parse::<Address>().unwrap();
    //
    //     let pubkey_bytes =
    // hex::decode("
    // 76698beebe8ee5c74d8cc50ab84ac301ee8f10af6f28d0ffd6adf4d6d3b9b762d46ca56d3dad2ce13213a6f42278dabbb53259f2d92681ea6a0b98197a719be3"
    // ).unwrap();
    //
    //     assert_eq!(Address::from_raw_public_key(&pubkey_bytes), addr);
    // }
}
