use super::PublicKey;
use alloc::string::String;
use libgoldilocks::goldilocks::PrivateKey as GoldilocksPrivateKey;

/// Base-rs wrapper for goldilocks ed448 private key.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct PrivateKey {
    inner: GoldilocksPrivateKey,
}

impl PrivateKey {
    /// Create a new random private key.
    pub fn generate() -> Self {
        PrivateKey { inner: libgoldilocks::goldilocks::ed448_generate_key() }
    }

    /// Get the inner private key
    pub const fn key(&self) -> GoldilocksPrivateKey {
        self.inner
    }

    /// Create a new private key from a slice of bytes.
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        bytes.try_into().ok().map(|inner| Self { inner })
    }

    /// Serialize the private key into a byte array.
    pub const fn to_bytes(&self) -> [u8; 57] {
        self.inner
    }

    /// Create a new private key from a hex string.
    pub fn from_hex(hex: &str) -> Option<Self> {
        if hex.len() != 114 || !hex::check_raw(hex) {
            return None;
        }
        Some(PrivateKey { inner: libgoldilocks::goldilocks::hex_to_private_key(hex) })
    }

    /// Return private key as hex string
    pub fn to_hex(&self) -> String {
        hex::encode(self.inner)
    }

    /// Get public key from private key
    pub fn public_key(&self) -> PublicKey {
        PublicKey { inner: libgoldilocks::goldilocks::ed448_derive_public(&self.inner) }
    }

    /// Sign a message with the private key
    pub fn sign(&self, message: &[u8]) -> [u8; 171] {
        let mut full_sig: [u8; 171] = [0; 171];

        let ed448_sig = libgoldilocks::goldilocks::ed448_sign(&self.inner, message);
        full_sig[0..114].copy_from_slice(&ed448_sig);
        full_sig[114..171].copy_from_slice(&self.public_key().to_bytes());

        full_sig
    }
}

#[cfg(test)]
mod tests {
    use super::{PrivateKey, PublicKey};

    #[test]
    fn test_decode() {
        let pk_hex = PrivateKey::from_hex("a8ea212cc24ae0fd029a97b64be540885af0e1b7dc9faf4a591742850c4377f857ae9a8f87df1de98e397a5867dd6f20211ef3f234ae71bc56");
        let pk_bytes = PrivateKey::from_bytes(&[
            168, 234, 33, 44, 194, 74, 224, 253, 2, 154, 151, 182, 75, 229, 64, 136, 90, 240, 225,
            183, 220, 159, 175, 74, 89, 23, 66, 133, 12, 67, 119, 248, 87, 174, 154, 143, 135, 223,
            29, 233, 142, 57, 122, 88, 103, 221, 111, 32, 33, 30, 243, 242, 52, 174, 113, 188, 86,
        ]);
        assert_eq!(pk_hex, pk_bytes);
    }

    #[test]
    fn test_to_bytes() {
        let bytes = &[
            168, 234, 33, 44, 194, 74, 224, 253, 2, 154, 151, 182, 75, 229, 64, 136, 90, 240, 225,
            183, 220, 159, 175, 74, 89, 23, 66, 133, 12, 67, 119, 248, 87, 174, 154, 143, 135, 223,
            29, 233, 142, 57, 122, 88, 103, 221, 111, 32, 33, 30, 243, 242, 52, 174, 113, 188, 86,
        ];
        let pk_bytes = PrivateKey::from_bytes(bytes).unwrap();
        assert_eq!(pk_bytes.to_bytes(), *bytes);
    }

    #[test]
    fn test_to_hex() {
        let hex = "a8ea212cc24ae0fd029a97b64be540885af0e1b7dc9faf4a591742850c4377f857ae9a8f87df1de98e397a5867dd6f20211ef3f234ae71bc56";
        let pk_hex = PrivateKey::from_hex(hex).unwrap();
        assert_eq!(pk_hex.to_hex(), hex);
    }

    #[test]
    fn test_sign() {
        let pk = PrivateKey::from_hex("a8ea212cc24ae0fd029a97b64be540885af0e1b7dc9faf4a591742850c4377f857ae9a8f87df1de98e397a5867dd6f20211ef3f234ae71bc56").unwrap();
        let message = b"hello world";
        let signature = pk.sign(message);
        assert_eq!(
            signature,
            [
                // Signature
                0x11, 0x12, 0x0b, 0x07, 0x9b, 0xad, 0xd1, 0xcd, 0x24, 0xd6, 0x3d, 0x1a, 0xe6, 0xbe,
                0x36, 0x94, 0xbd, 0x9b, 0xe6, 0x25, 0xfc, 0xa8, 0x11, 0x3a, 0x81, 0xab, 0x98, 0x8c,
                0xa2, 0x32, 0xe0, 0xfd, 0xc6, 0xe1, 0xdb, 0xa8, 0xa9, 0x69, 0x55, 0x21, 0x71, 0x92,
                0x5c, 0xe7, 0x96, 0x63, 0xe2, 0xe3, 0xfb, 0x76, 0x02, 0x80, 0x2b, 0x11, 0xf2, 0xad,
                0x80, 0x14, 0xbf, 0x4e, 0x67, 0xf2, 0xea, 0x4d, 0xdd, 0x97, 0x03, 0xf2, 0x5b, 0x9d,
                0x34, 0x8a, 0x8d, 0x5b, 0xf8, 0xc7, 0x16, 0x62, 0x38, 0x6c, 0xfe, 0xb9, 0x04, 0xda,
                0x60, 0x56, 0xa3, 0xcf, 0x96, 0xc6, 0xa6, 0xab, 0x9b, 0xd6, 0x48, 0x76, 0x68, 0x12,
                0x91, 0x91, 0x0d, 0xd8, 0xa2, 0xf6, 0x6f, 0x76, 0x62, 0xe5, 0xd8, 0x39, 0xe3, 0x08,
                0x22, 0x00, // Public key
                0xb6, 0x15, 0xe5, 0x7d, 0xd4, 0xd1, 0x5c, 0x3e, 0xd1, 0x32, 0x37, 0x25, 0xc0, 0xba,
                0x8b, 0x1d, 0x7f, 0x6e, 0x74, 0x0d, 0x08, 0xe0, 0xe2, 0x9c, 0x6d, 0x3f, 0xf5, 0x64,
                0xc8, 0x96, 0xc0, 0xc3, 0xdd, 0x28, 0xa9, 0xbb, 0x50, 0x65, 0xe0, 0x67, 0x25, 0xc8,
                0xf9, 0xe3, 0xf7, 0xc2, 0xc6, 0xbb, 0xad, 0x49, 0x00, 0xb7, 0x44, 0x7e, 0xcf, 0x98,
                0x80
            ]
        );
    }

    #[test]
    fn test_public_key() {
        let private_key = PrivateKey::from_hex("a8ea212cc24ae0fd029a97b64be540885af0e1b7dc9faf4a591742850c4377f857ae9a8f87df1de98e397a5867dd6f20211ef3f234ae71bc56").unwrap();
        let public_key = PublicKey::from_hex("b615e57dd4d15c3ed1323725c0ba8b1d7f6e740d08e0e29c6d3ff564c896c0c3dd28a9bb5065e06725c8f9e3f7c2c6bbad4900b7447ecf9880").unwrap();
        assert_eq!(private_key.public_key(), public_key);
    }

    #[test]
    fn test_bad_hex() {
        let private_key = PrivateKey::from_hex("a8ea212cc24ae0fd029a97b64be540885af0e1b7dc9faf4a591742850c4377f857ae9a8f87df1de98e397a5867dd6f20211ef3f234ae71bc5");
        assert_eq!(private_key, None);

        let private_key = PrivateKey::from_hex("a8ea212cc24ae0fd029a97b64be540885af0e1b7dc9faf4a591742850c4377f857ae9a8f87df1de98e397a5867dd6f20211ef3f234ae71bc5w");
        assert_eq!(private_key, None);
    }
}
