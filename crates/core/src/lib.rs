#![doc = include_str!("../README.md")]
#![warn(
    missing_copy_implementations,
    missing_debug_implementations,
    missing_docs,
    unreachable_pub,
    clippy::missing_const_for_fn,
    rustdoc::all
)]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]
#![deny(unused_must_use, rust_2018_idioms)]
#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(feature = "std", allow(unused_imports))]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]

#[doc(inline)]
pub use base_primitives as primitives;
#[doc(no_inline)]
pub use primitives::{hex, uint};

#[cfg(feature = "dyn-abi")]
#[doc(inline)]
pub use base_dyn_abi as dyn_abi;

#[cfg(feature = "json-abi")]
#[doc(inline)]
pub use base_json_abi as json_abi;

#[cfg(feature = "ylm-types")]
#[doc(inline)]
pub use base_ylm_types as ylm_types;
#[cfg(all(doc, feature = "ylm-types"))] // Show this re-export in docs instead of the wrapper below.
#[doc(no_inline)]
pub use ylm_types::ylm;

#[cfg(feature = "rlp")]
#[doc(inline)]
pub use alloy_rlp as rlp;

/// [`ylm!`](ylm_types::ylm!) macro wrapper to route imports to the correct crate.
///
/// See [`ylm!`](ylm_types::ylm!) for the actual macro documentation.
#[cfg(all(not(doc), feature = "ylm-types"))] // Show the actual macro in docs.
#[doc(hidden)]
#[macro_export]
macro_rules! ylm {
    ($($t:tt)*) => {
        $crate::ylm_types::ylm! {
            #![ylm(base_ylm_types = $crate::ylm_types)]
            $($t)*
        }
    };
}
