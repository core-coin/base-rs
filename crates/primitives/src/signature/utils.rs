use crate::ChainId;

/// Applies [EIP-155](https://eips.ethereum.org/EIPS/eip-155).
#[inline]
pub const fn to_eip155_v(v: u8, chain_id: ChainId) -> ChainId {
    (v as u64) + 35 + chain_id * 2
}

/// Normalize the v value to a single byte.
pub(crate) const fn normalize_v_to_byte(v: u64) -> u8 {
    match v {
        // Case 1: raw/bare
        0..=26 => (v % 4) as u8,
        // Case 2: non-EIP-155 v value
        27..=34 => ((v - 27) % 4) as u8,
        // Case 3: EIP-155 V value
        35.. => ((v - 1) % 2) as u8,
    }
}
