use crate::IcanAddress;

#[cfg(feature = "rlp")]
use alloy_rlp::{Buf, BufMut, Decodable, Encodable, EMPTY_STRING_CODE};

/// The `to` field of a transaction. Either a target address, or empty for a
/// contract creation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum IcanTxKind {
    /// A transaction that creates a contract.
    #[default]
    Create,
    /// A transaction that calls a contract or transfer.
    Call(IcanAddress),
}

impl From<Option<IcanAddress>> for IcanTxKind {
    /// Creates a `IcanTxKind::Call` with the `Some` address, `None` otherwise.
    #[inline]
    fn from(value: Option<IcanAddress>) -> Self {
        match value {
            None => IcanTxKind::Create,
            Some(addr) => IcanTxKind::Call(addr),
        }
    }
}

impl From<IcanAddress> for IcanTxKind {
    /// Creates a `IcanTxKind::Call` with the given address.
    #[inline]
    fn from(value: IcanAddress) -> Self {
        IcanTxKind::Call(value)
    }
}

impl IcanTxKind {
    /// Returns the address of the contract that will be called or will receive the transfer.
    pub const fn to(&self) -> Option<&IcanAddress> {
        match self {
            IcanTxKind::Create => None,
            IcanTxKind::Call(to) => Some(to),
        }
    }

    /// Returns true if the transaction is a contract creation.
    #[inline]
    pub const fn is_create(&self) -> bool {
        matches!(self, IcanTxKind::Create)
    }

    /// Returns true if the transaction is a contract call.
    #[inline]
    pub const fn is_call(&self) -> bool {
        matches!(self, IcanTxKind::Call(_))
    }

    /// Calculates a heuristic for the in-memory size of this object.
    #[inline]
    pub const fn size(&self) -> usize {
        core::mem::size_of::<Self>()
    }
}

#[cfg(feature = "rlp")]
impl Encodable for IcanTxKind {
    fn encode(&self, out: &mut dyn BufMut) {
        match self {
            IcanTxKind::Call(to) => to.encode(out),
            IcanTxKind::Create => out.put_u8(EMPTY_STRING_CODE),
        }
    }

    fn length(&self) -> usize {
        match self {
            IcanTxKind::Call(to) => to.length(),
            IcanTxKind::Create => 1, // EMPTY_STRING_CODE is a single byte
        }
    }
}

#[cfg(feature = "rlp")]
impl Decodable for IcanTxKind {
    fn decode(buf: &mut &[u8]) -> alloy_rlp::Result<Self> {
        if let Some(&first) = buf.first() {
            if first == EMPTY_STRING_CODE {
                buf.advance(1);
                Ok(IcanTxKind::Create)
            } else {
                let addr = <IcanAddress as Decodable>::decode(buf)?;
                Ok(IcanTxKind::Call(addr))
            }
        } else {
            Err(alloy_rlp::Error::InputTooShort)
        }
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for IcanTxKind {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.to().serialize(serializer)
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for IcanTxKind {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(Option::<IcanAddress>::deserialize(deserializer)?.into())
    }
}
