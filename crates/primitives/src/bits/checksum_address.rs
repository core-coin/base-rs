use crate::{sha3, Address, FixedBytes};
use core::{borrow::Borrow, fmt, panic, str};
use ruint::aliases::U176;

wrap_fixed_bytes!(
    extra_derives: [],
    pub struct ChecksumAddress<22>;
);

impl From<U176> for ChecksumAddress {
    #[inline]
    fn from(value: U176) -> Self {
        Self(FixedBytes(value.to_be_bytes()))
    }
}

impl From<ChecksumAddress> for U176 {
    #[inline]
    fn from(value: ChecksumAddress) -> Self {
        Self::from_be_bytes(value.0 .0)
    }
}
impl fmt::Display for ChecksumAddress {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
        // let checksum = checksum.as_str();
        // if f.alternate() {
        //     // If the alternate flag is set, use middle-out compression
        //     // "0x" + first 4 bytes + "…" + last 4 bytes
        //     f.write_str(&checksum[..6])?;
        //     f.write_str("…")?;
        //     f.write_str(&checksum[38..])
        // } else {
        //     f.write_str(checksum)
        // }
    }
}

impl ChecksumAddress {
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
        Self(FixedBytes(word[10..].try_into().unwrap()))
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
        word[10..].copy_from_slice(self.as_slice());
        FixedBytes(word)
    }

    pub fn into_address(&self) -> Address {
        let mut address = [0; 20];
        address.copy_from_slice(&self.0 .0[2..]);
        address.into()
    }

    /// Returns the checksum of a formatted address.
    #[inline]
    pub const fn as_str(&self) -> &str {
        unsafe { str::from_utf8_unchecked(self.0.as_slice()) }
    }

    /// Returns the checksum of a formatted address.
    #[inline]
    pub fn as_mut_str(&mut self) -> &mut str {
        unsafe { str::from_utf8_unchecked_mut(self.0.as_mut_slice()) }
    }

    /// Computes the `create` address for this address and nonce:
    ///
    /// `sha3(rlp([sender, nonce]))[12:]`
    ///
    /// # Examples
    ///
    /// ```
    /// # use alloy_primitives::{address, Address};
    /// let sender = address!("b20a608c624Ca5003905aA834De7156C68b2E1d0");
    ///
    /// let expected = address!("00000000219ab540356cBB839Cbe05303d7705Fa");
    /// assert_eq!(sender.create(0), expected);
    ///
    /// let expected = address!("e33c6e89e69d085897f98e92b06ebd541d1daa99");
    /// assert_eq!(sender.create(1), expected);
    /// ```
    #[cfg(feature = "rlp")]
    #[inline]
    #[must_use]
    pub fn create(&self, nonce: u64) -> Self {
        use alloy_rlp::{Encodable, EMPTY_LIST_CODE, EMPTY_STRING_CODE};

        use crate::sha3;

        // max u64 encoded length is `1 + u64::BYTES`
        const MAX_LEN: usize = 1 + (1 + 22) + 9;

        let len = 24 + nonce.length();
        debug_assert!(len <= MAX_LEN);

        let mut out = [0u8; MAX_LEN];

        // list header
        // minus 1 to account for the list header itself
        out[0] = EMPTY_LIST_CODE + len as u8 - 1;

        // address header + address
        out[1] = EMPTY_STRING_CODE + 22;
        out[2..24].copy_from_slice(self.as_slice());

        // nonce
        nonce.encode(&mut &mut out[24..]);

        let hash = sha3(&out[..len]);
        Address::from_word(hash).to_ican(self.network_id())
    }

    /// Computes the `CREATE2` address of a smart contract as specified in
    /// [EIP-1014]:
    ///
    /// `sha3(0xff ++ address ++ salt ++ sha3(init_code))[12:]`
    ///
    /// The `init_code` is the code that, when executed, produces the runtime
    /// bytecode that will be placed into the state, and which typically is used
    /// by high level languages to implement a ‘constructor’.
    ///
    /// [EIP-1014]: https://eips.ethereum.org/EIPS/eip-1014
    ///
    /// # Examples
    ///
    /// ```
    /// # use alloy_primitives::{address, b256, bytes, Address};
    /// let address = address!("8ba1f109551bD432803012645Ac136ddd64DBA72");
    /// let salt = b256!("7c5ea36004851c764c44143b1dcb59679b11c9a68e5f41497f6cf3d480715331");
    /// let init_code = bytes!("6394198df16000526103ff60206004601c335afa6040516060f3");
    /// let expected = address!("533ae9d683B10C02EbDb05471642F85230071FC3");
    /// assert_eq!(address.create2_from_code(salt, init_code), expected);
    /// ```
    #[must_use]
    pub fn create2_from_code<S, C>(&self, salt: S, init_code: C) -> Self
    where
        // not `AsRef` because `[u8; N]` does not implement `AsRef<[u8; N]>`
        S: Borrow<[u8; 32]>,
        C: AsRef<[u8]>,
    {
        self._create2(salt.borrow(), &sha3(init_code.as_ref()).0)
    }

    /// Computes the `CREATE2` address of a smart contract as specified in
    /// [EIP-1014], taking the pre-computed hash of the init code as input:
    ///
    /// `sha3(0xff ++ address ++ salt ++ init_code_hash)[12:]`
    ///
    /// The `init_code` is the code that, when executed, produces the runtime
    /// bytecode that will be placed into the state, and which typically is used
    /// by high level languages to implement a ‘constructor’.
    ///
    /// [EIP-1014]: https://eips.ethereum.org/EIPS/eip-1014
    ///
    /// # Examples
    ///
    /// ```
    /// # use alloy_primitives::{address, b256, Address};
    /// let address = address!("5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f");
    /// let salt = b256!("2b2f5776e38002e0c013d0d89828fdb06fee595ea2d5ed4b194e3883e823e350");
    /// let init_code_hash = b256!("96e8ac4277198ff8b6f785478aa9a39f403cb768dd02cbee326c3e7da348845f");
    /// let expected = address!("0d4a11d5EEaaC28EC3F61d100daF4d40471f1852");
    /// assert_eq!(address.create2(salt, init_code_hash), expected);
    /// ```
    #[must_use]
    pub fn create2<S, H>(&self, salt: S, init_code_hash: H) -> Self
    where
        // not `AsRef` because `[u8; N]` does not implement `AsRef<[u8; N]>`
        S: Borrow<[u8; 32]>,
        H: Borrow<[u8; 32]>,
    {
        self._create2(salt.borrow(), init_code_hash.borrow())
    }

    // non-generic inner function
    fn _create2(&self, salt: &[u8; 32], init_code_hash: &[u8; 32]) -> Self {
        // note: creating a temporary buffer and copying everything over performs
        // much better than calling `Keccak::update` multiple times
        let mut bytes = [0; 87];
        bytes[0] = 0xff;
        bytes[1..23].copy_from_slice(self.as_slice());
        bytes[23..55].copy_from_slice(salt);
        bytes[55..87].copy_from_slice(init_code_hash);
        let hash = sha3(bytes);
        Address::from_word(hash).to_ican(self.network_id())
    }

    /// Gets the network_id from the address
    pub fn network_id(&self) -> u64 {
        match self.0 .0[0] {
            203 => 1,
            171 => 3,
            206 => 1337,
            _ => panic!("Invalid Checksum Address"),
        }
    }

    /// Instantiate by hashing public key bytes.
    ///
    /// # Panics
    ///
    /// If the input is not exactly 64 bytes
    pub fn from_raw_public_key(pubkey: &[u8], network_id: u64) -> Self {
        assert_eq!(pubkey.len(), 64, "raw public key must be 64 bytes");
        let digest = sha3(pubkey);
        Address::from_slice(&digest[12..]).to_ican(network_id)
    }

    /// Converts an ECDSA verifying key to its corresponding Ethereum address.
    #[inline]
    #[cfg(feature = "k256")]
    #[doc(alias = "from_verifying_key")]
    pub fn from_public_key(pubkey: &k256::ecdsa::VerifyingKey) -> Self {
        use k256::elliptic_curve::sec1::ToEncodedPoint;
        let affine: &k256::AffinePoint = pubkey.as_ref();
        let encoded = affine.to_encoded_point(false);
        Self::from_raw_public_key(&encoded.as_bytes()[1..])
    }

    /// Converts an ECDSA signing key to its corresponding Ethereum address.
    #[inline]
    #[cfg(feature = "k256")]
    #[doc(alias = "from_signing_key")]
    pub fn from_private_key(private_key: &k256::ecdsa::SigningKey) -> Self {
        Self::from_public_key(private_key.verifying_key())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::Address;

    // https://ethereum.stackexchange.com/questions/760/how-is-the-address-of-an-ethereum-contract-computed
    #[test]
    #[cfg(feature = "rlp")]
    fn create() {
        let from = "cb82a5fd22b9bee8b8ab877c86e0a2c21765e1d5bfc5".parse::<ChecksumAddress>().unwrap();
        for (nonce, expected) in [
            "cb57718e2b338b99d2587a6dd6c01fc2b97a4296449f",
            "cb812bae2e00797890802e8aa6c162aac5cac4d8990c",
        ]
        .into_iter()
        .enumerate()
        {
            let address = from.create(nonce as u64);
            assert_eq!(address, expected.parse::<ChecksumAddress>().unwrap());
        }
    }

    #[test]
    fn create2_address() {
        let tests = [
            (
                "cb45de3a1cc0c70e5e26ca00c0936ef3873c15ba94bb",
                "0x0000000000000000000000000000000000000000000000000000000000000000",
                "0x00",
                "cb115daba0ebaef63278430b47561d6c85c08543862e",
            ),
            (
                "cb6187e81b02756711a90ed2b9c295fdc5c6776faf4d",
                "0x0000000000000000000000000000000000000000000000000000000000000000",
                "0x00",
                "cb30f34397c21261cf7b807aa2437e7aa6df0106ec8b",
            ),
            (
                "cb950dae4e5fdff3e3ded206a331db829624d1f0a8e0",
                "0x000000000000000000000000feed000000000000000000000000000000000000",
                "0x00",
                "cb752b9a56b5a380b87154ca1e5037c084e9744845bb",
            ),
            (
                "cb08f86a1e1715653df219804cd57b16686ec95fc61a",
                "0x0000000000000000000000000000000000000000000000000000000000000000",
                "0xdeadbeef",
                "cb89bfcdfe71e021a5fdfd29e7f5810c8352d48ea67f",
            ),
            (
                "cb6234873c1b4dd5c683a10cee6419d70fbde8552772",
                "0x00000000000000000000000000000000000000000000000000000000cafebabe",
                "0xdeadbeef",
                "cb7954b09a663ca9bbcdbc84e4224f45c9d265b865b3",
            ),
            (
                "cb6234873c1b4dd5c683a10cee6419d70fbde8552772",
                "0x0000000000000000000000000000000000000000000000000000000000000000",
                "0x",
                "cb88d6fe8ba337218b9b49f4f622dd71e321f445d2aa",
            ),
        ];
        for (from, salt, init_code, expected) in tests {
            let from = from.parse::<ChecksumAddress>().unwrap();

            let salt = hex::decode(salt).unwrap();
            let salt: [u8; 32] = salt.try_into().unwrap();

            let init_code = hex::decode(init_code).unwrap();
            let init_code_hash = sha3(&init_code);

            let expected = expected.parse::<ChecksumAddress>().unwrap();

            assert_eq!(expected, from.create2(salt, init_code_hash));
            assert_eq!(expected, from.create2_from_code(salt, init_code));
        }
    }

    //
    // #[test]
    // #[cfg(all(feature = "rlp", feature = "arbitrary"))]
    // #[cfg_attr(miri, ignore = "doesn't run in isolation and would take too long")]
    // fn create_correctness() {
    //     fn create_slow(address: &Address, nonce: u64) -> Address {
    //         use alloy_rlp::Encodable;
    //
    //         let mut out = vec![];
    //
    //         alloy_rlp::Header { list: true, payload_length: address.length() + nonce.length() }
    //             .encode(&mut out);
    //         address.encode(&mut out);
    //         nonce.encode(&mut out);
    //
    //         Address::from_word(sha3(out))
    //     }
    //
    //     proptest::proptest!(|(address: Address, nonce: u64)| {
    //         proptest::prop_assert_eq!(address.create(nonce), create_slow(&address, nonce));
    //     });
    // }
    //
    // // https://eips.ethereum.org/EIPS/eip-1014
    // #[test]
    // fn create2() {
    //     let tests = [
    //         (
    //             "0000000000000000000000000000000000000000",
    //             "0000000000000000000000000000000000000000000000000000000000000000",
    //             "00",
    //             "4D1A2e2bB4F88F0250f26Ffff098B0b30B26BF38",
    //         ),
    //         (
    //             "deadbeef00000000000000000000000000000000",
    //             "0000000000000000000000000000000000000000000000000000000000000000",
    //             "00",
    //             "B928f69Bb1D91Cd65274e3c79d8986362984fDA3",
    //         ),
    //         (
    //             "deadbeef00000000000000000000000000000000",
    //             "000000000000000000000000feed000000000000000000000000000000000000",
    //             "00",
    //             "D04116cDd17beBE565EB2422F2497E06cC1C9833",
    //         ),
    //         (
    //             "0000000000000000000000000000000000000000",
    //             "0000000000000000000000000000000000000000000000000000000000000000",
    //             "deadbeef",
    //             "70f2b2914A2a4b783FaEFb75f459A580616Fcb5e",
    //         ),
    //         (
    //             "00000000000000000000000000000000deadbeef",
    //             "00000000000000000000000000000000000000000000000000000000cafebabe",
    //             "deadbeef",
    //             "60f3f640a8508fC6a86d45DF051962668E1e8AC7",
    //         ),
    //         (
    //             "00000000000000000000000000000000deadbeef",
    //             "00000000000000000000000000000000000000000000000000000000cafebabe",
    //
    // "deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef",
    //             "1d8bfDC5D46DC4f61D6b6115972536eBE6A8854C",
    //         ),
    //         (
    //             "0000000000000000000000000000000000000000",
    //             "0000000000000000000000000000000000000000000000000000000000000000",
    //             "",
    //             "E33C0C7F7df4809055C3ebA6c09CFe4BaF1BD9e0",
    //         ),
    //     ];
    //     for (from, salt, init_code, expected) in tests {
    //         let from = from.parse::<Address>().unwrap();
    //
    //         let salt = hex::decode(salt).unwrap();
    //         let salt: [u8; 32] = salt.try_into().unwrap();
    //
    //         let init_code = hex::decode(init_code).unwrap();
    //         let init_code_hash = sha3(&init_code);
    //
    //         let expected = expected.parse::<Address>().unwrap();
    //
    //         assert_eq!(expected, from.create2(salt, init_code_hash));
    //         assert_eq!(expected, from.create2_from_code(salt, init_code));
    //     }
    // }
}
