use core::convert::Infallible;

/// Errors in signature parsing or verification.
#[derive(Debug)]
pub enum SignatureError {
    /// Error converting from bytes.
    FromBytes(&'static str),

    /// Error converting hex to bytes.
    FromHex(hex::FromHexError),

    /// Invalid parity.
    InvalidParity(u64),

    /// Libgoldilocks error
    Libgoldilocks(libgoldilocks::errors::LibgoldilockErrors),
}

impl From<hex::FromHexError> for SignatureError {
    fn from(err: hex::FromHexError) -> Self {
        Self::FromHex(err)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for SignatureError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::FromHex(e) => Some(e),
            _ => None,
        }
    }
}

impl core::fmt::Display for SignatureError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::FromBytes(e) => f.write_str(e),
            Self::FromHex(e) => e.fmt(f),
            Self::InvalidParity(v) => write!(f, "invalid parity: {v}"),
            Self::Libgoldilocks(e) => e.fmt(f),
        }
    }
}

impl From<Infallible> for SignatureError {
    fn from(_: Infallible) -> Self {
        unreachable!()
    }
}
