//! Common Core utilities.

use crate::B256;
use alloc::{boxed::Box, collections::TryReserveError, vec::Vec};
use core::{fmt, mem::MaybeUninit};

mod units;
use tiny_keccak::Hasher as _;
pub use units::{
    format_ether, format_units, parse_ether, parse_units, ParseUnits, Unit, UnitsError,
};

#[doc(hidden)]
#[deprecated(since = "0.5.0", note = "use `Unit::ETHER.wei()` instead")]
pub const WEI_IN_ETHER: crate::U256 = Unit::ETHER.wei_const();

#[doc(hidden)]
#[deprecated(since = "0.5.0", note = "use `Unit` instead")]
pub type Units = Unit;

/// The prefix used for hashing messages according to EIP-191.
pub const EIP191_PREFIX: &str = "\x19Core Signed Message:\n";

/// Tries to create a `Vec` of `n` elements, each initialized to `elem`.
#[macro_export]
macro_rules! try_vec {
    () => {
        $crate::private::Vec::new()
    };
    ($elem:expr; $n:expr) => {
        $crate::utils::vec_try_from_elem($elem, $n)
    };
    ($($x:expr),+ $(,)?) => {
        match $crate::utils::box_try_new([$($x),+]) {
            ::core::result::Result::Ok(x) => ::core::result::Result::Ok(<[_]>::into_vec(x)),
            ::core::result::Result::Err(e) => ::core::result::Result::Err(e),
        }
    };
}

/// Allocates memory on the heap then places `x` into it, returning an error if the allocation
/// fails.
///
/// Stable version of `Box::try_new`.
#[inline]
pub fn box_try_new<T>(value: T) -> Result<Box<T>, TryReserveError> {
    let mut boxed = box_try_new_uninit::<T>()?;
    unsafe {
        boxed.as_mut_ptr().write(value);
        let ptr = Box::into_raw(boxed);
        Ok(Box::from_raw(ptr.cast()))
    }
}

/// Constructs a new box with uninitialized contents on the heap, returning an error if the
/// allocation fails.
///
/// Stable version of `Box::try_new_uninit`.
#[inline]
pub fn box_try_new_uninit<T>() -> Result<Box<MaybeUninit<T>>, TryReserveError> {
    let mut vec = Vec::<MaybeUninit<T>>::new();

    // Reserve enough space for one `MaybeUninit<T>`.
    vec.try_reserve_exact(1)?;

    // `try_reserve_exact`'s docs note that the allocator might allocate more than requested anyway.
    // Make sure we got exactly 1 element.
    vec.shrink_to(1);

    let ptr = vec.as_mut_ptr();
    core::mem::forget(vec);
    // SAFETY: `vec` is exactly one element long and has not been deallocated.
    Ok(unsafe { Box::from_raw(ptr) })
}

/// Tries to collect the elements of an iterator into a `Vec`.
pub fn try_collect_vec<I: Iterator<Item = T>, T>(iter: I) -> Result<Vec<T>, TryReserveError> {
    let mut vec = Vec::new();
    if let Some(size_hint) = iter.size_hint().1 {
        vec.try_reserve(size_hint.max(4))?;
    }
    vec.extend(iter);
    Ok(vec)
}

/// Tries to create a `Vec` with the given capacity.
#[inline]
pub fn vec_try_with_capacity<T>(capacity: usize) -> Result<Vec<T>, TryReserveError> {
    let mut vec = Vec::new();
    vec.try_reserve(capacity).map(|()| vec)
}

/// Tries to create a `Vec` of `n` elements, each initialized to `elem`.
// Not public API. Use `try_vec!` instead.
#[doc(hidden)]
pub fn vec_try_from_elem<T: Clone>(elem: T, n: usize) -> Result<Vec<T>, TryReserveError> {
    let mut vec = Vec::new();
    vec.try_reserve(n)?;
    vec.resize(n, elem);
    Ok(vec)
}

/// Hash a message according to [EIP-191] (version `0x01`).
///
/// The final message is a UTF-8 string, encoded as follows:
/// `"\x19Core Signed Message:\n" + message.length + message`
///
/// This message is then hashed using [Keccak-256](sha3).
///
/// [EIP-191]: https://eips.ethereum.org/EIPS/eip-191
pub fn eip191_hash_message<T: AsRef<[u8]>>(message: T) -> B256 {
    sha3(eip191_message(message))
}

/// Constructs a message according to [EIP-191] (version `0x01`).
///
/// The final message is a UTF-8 string, encoded as follows:
/// `"\x19Core Signed Message:\n" + message.length + message`
///
/// [EIP-191]: https://eips.ethereum.org/EIPS/eip-191
pub fn eip191_message<T: AsRef<[u8]>>(message: T) -> Vec<u8> {
    fn eip191_message(message: &[u8]) -> Vec<u8> {
        let len = message.len();
        let mut len_string_buffer = itoa::Buffer::new();
        let len_string = len_string_buffer.format(len);

        let mut eth_message = Vec::with_capacity(EIP191_PREFIX.len() + len_string.len() + len);
        eth_message.extend_from_slice(EIP191_PREFIX.as_bytes());
        eth_message.extend_from_slice(len_string.as_bytes());
        eth_message.extend_from_slice(message);
        eth_message
    }

    eip191_message(message.as_ref())
}

/// Simple interface to the [`Sha3-256`] hash function.
///
/// [`Sha3`]: https://en.wikipedia.org/wiki/SHA-3
pub fn sha3<T: AsRef<[u8]>>(bytes: T) -> B256 {
    fn sha3(bytes: &[u8]) -> B256 {
        let mut output = MaybeUninit::<B256>::uninit();
        let mut hasher = Sha3::new();
        hasher.update(bytes);
        // SAFETY: Never reads from `output`.
        unsafe { hasher.finalize_into_raw(output.as_mut_ptr().cast()) };

        // SAFETY: Initialized above.
        unsafe { output.assume_init() }
    }

    sha3(bytes.as_ref())
}

/// Simple [`Keccak-256`] hasher.
///
/// Note that the "native-keccak" feature is not supported for this struct, and will default to the
/// [`tiny_keccak`] implementation.
///
/// [`Keccak-256`]: https://en.wikipedia.org/wiki/SHA-3
#[derive(Clone)]
pub struct Sha3 {
    hasher: tiny_keccak::Sha3,
}

impl Default for Sha3 {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for Sha3 {
    #[inline]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Sha3").finish_non_exhaustive()
    }
}

impl Sha3 {
    /// Creates a new [`Sha3`] hasher.
    #[inline]
    pub fn new() -> Self {
        Self { hasher: tiny_keccak::Sha3::v256() }
    }

    /// Absorbs additional input. Can be called multiple times.
    #[inline]
    pub fn update(&mut self, bytes: impl AsRef<[u8]>) {
        self.hasher.update(bytes.as_ref());
    }

    /// Pad and squeeze the state.
    #[inline]
    pub fn finalize(self) -> B256 {
        let mut output = MaybeUninit::<B256>::uninit();
        // SAFETY: The output is 32-bytes.
        unsafe { self.finalize_into_raw(output.as_mut_ptr().cast()) };
        // SAFETY: Initialized above.
        unsafe { output.assume_init() }
    }

    /// Pad and squeeze the state into `output`.
    ///
    /// # Panics
    ///
    /// Panics if `output` is not 32 bytes long.
    #[inline]
    #[track_caller]
    pub fn finalize_into(self, output: &mut [u8]) {
        self.finalize_into_array(output.try_into().unwrap())
    }

    /// Pad and squeeze the state into `output`.
    #[inline]
    pub fn finalize_into_array(self, output: &mut [u8; 32]) {
        self.hasher.finalize(output);
    }

    /// Pad and squeeze the state into `output`.
    ///
    /// # Safety
    ///
    /// `output` must point to a buffer that is at least 32-bytes long.
    #[inline]
    pub unsafe fn finalize_into_raw(self, output: *mut u8) {
        self.finalize_into_array(&mut *output.cast::<[u8; 32]>())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // test vector taken from:
    // https://web3js.readthedocs.io/en/v1.10.0/web3-eth-accounts.html#hashmessage
    #[test]
    fn test_hash_message() {
        let msg = "Hello World";
        let eip191_msg = eip191_message(msg);
        let hash = sha3(&eip191_msg);
        assert_eq!(
            eip191_msg,
            [EIP191_PREFIX.as_bytes(), msg.len().to_string().as_bytes(), msg.as_bytes()].concat()
        );
        assert_eq!(hash, b256!("6e1062427a5c78e549a48e47260552bf9e35d44f747c640bad47bd48e8709f0f"));
        assert_eq!(eip191_hash_message(msg), hash);
    }

    #[test]
    fn sha3_hasher() {
        let expected = b256!("644bcc7e564373040999aac89e7622f3ca71fba1d972fd94a31c3bfbf24e3938");
        assert_eq!(sha3("hello world"), expected);

        let mut hasher = Sha3::new();
        hasher.update(b"hello");
        hasher.update(b" world");

        assert_eq!(hasher.clone().finalize(), expected);

        let mut hash = [0u8; 32];
        hasher.clone().finalize_into(&mut hash);
        assert_eq!(hash, expected);

        let mut hash = [0u8; 32];
        hasher.clone().finalize_into_array(&mut hash);
        assert_eq!(hash, expected);

        let mut hash = [0u8; 32];
        unsafe { hasher.finalize_into_raw(hash.as_mut_ptr()) };
        assert_eq!(hash, expected);
    }

    #[test]
    fn test_try_boxing() {
        let x = Box::new(42);
        let y = box_try_new(42).unwrap();
        assert_eq!(x, y);

        let x = vec![1; 3];
        let y = try_vec![1; 3].unwrap();
        assert_eq!(x, y);

        let x = vec![1, 2, 3];
        let y = try_vec![1, 2, 3].unwrap();
        assert_eq!(x, y);
    }
}
