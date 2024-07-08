// Copyright 2015-2020 Parity Technologies
// Copyright 2023-2023 Alloy Contributors

// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Ylem type modeling and [ABI] and [EIP-712] codec implementation.
//!
//! This crate provides tools for expressing Ylem types in Rust, and for
//! encoding these representations into ABI blobs suitable for smart contract
//! processing. In other words, you can represent your smart contract args in
//! native Rust, easily encode them to pass to a smart contract, and easily
//! decode the smart contract return values.
//!
//! We do this by representing Ylem types in rust via the [`YlmType`] trait.
//! This trait maps Ylem types to Rust types via the associated
//! [`YlmType::RustType`].
//!
//! The ABI encoding and decoding is implemented in the [`abi`] module, see [its
//! documentation](abi) to learn how it works.
//!
//! [ABI]: https://docs.soliditylang.org/en/latest/abi-spec.html
//! [EIP-712]: https://eips.ethereum.org/EIPS/eip-712
//!
//! ```
//! use base_ylm_types::{ylm_data::*, YlmType, YlmValue};
//!
//! // Represent a Ylem type in rust
//! type MyYlmType = FixedArray<Bool, 2>;
//!
//! let data = [true, false];
//! let validate = true;
//!
//! // YlmTypes expose their Ylem name :)
//! assert_eq!(&MyYlmType::ylm_type_name(), "bool[2]");
//!
//! // YlmTypes are used to transform Rust into ABI blobs, and back.
//! let encoded: Vec<u8> = MyYlmType::abi_encode(&data);
//! let decoded: [bool; 2] = MyYlmType::abi_decode(&encoded, validate)?;
//! assert_eq!(data, decoded);
//!
//! // This is more easily done with the `YlmValue` trait:
//! let encoded: Vec<u8> = data.abi_encode();
//! let decoded: [bool; 2] = <[bool; 2]>::abi_decode(&encoded, validate)?;
//! assert_eq!(data, decoded);
//! # Ok::<_, base_ylm_types::Error>(())
//! ```
//!
//! ## [`ylm!`]
//!
//! The [`ylm!`] procedural macro provides a convenient way to define
//! custom [`YlmType`]s and reference primitive ones. See
//! [its documentation][ylm!] for details on how to use it.
//!
//! ## [`YlmStruct`]
//!
//! The [`YlmStruct`] trait primarily provides EIP-712 signing support.
//!
//! ```
//! # use base_ylm_types::{ylm, YlmStruct};
//! # use base_primitives::U256;
//! // `ylm!` allows you to define struct types!
//! // You can just paste Ylem into the macro and it should work :)
//! ylm! {
//!     struct MyStruct {
//!         uint256 a;
//!         bytes32 b;
//!         address[] c;
//!     }
//! }
//!
//! ylm! {
//!     struct MyStruct2 {
//!         MyStruct a;
//!         bytes32 b;
//!         address[] c;
//!     }
//! }
//!
//! // All structs generated with `ylm!` implement `crate::YlmType` &
//! // `crate::YlmStruct`. This means you get eip-712 signing for freeeeee
//! let my_struct = MyStruct {
//!     a: U256::from(1),
//!     b: [0; 32].into(),
//!     c: vec![Default::default()],
//! };
//!
//! // The `eip712_domain` macro lets you easily define an EIP-712 domain
//! // object :)
//! let my_domain = base_ylm_types::eip712_domain!(
//!    name: "MyDomain",
//!    version: "1",
//! );
//!
//! // Because all the hard work is done by the `ylm!` macro, EIP-712 is as easy
//! // as calling `eip712_signing_hash` with your domain
//! let signing_hash = my_struct.eip712_signing_hash(&my_domain);
//! ```
//!
//! ### [`ylm!`] User-defined Value Types
//!
//! Support for user-defined value types is new! These are currently
//! implemented as wrapper types. Watch this space for more
//! features!
//!
//! ```
//! # use base_ylm_types::{ylm, ylm_data, YlmType};
//! # use base_primitives::U256;
//! // We also also support Ylem value types
//! ylm! {
//!     type MyValueType is uint256;
//! }
//!
//! // UDTs are encoded as their underlying type
//! let mvt = MyValueType::from(U256::from(1));
//! assert_eq!(mvt.abi_encode(), ylm_data::Uint::<256>::abi_encode(&U256::from(1)));
//! ```
//!
//! ## Tokenization/Detokenization
//!
//! The process of converting from a Rust type to a to an abi token is called
//! "Tokenization". Typical users will not access tokenizaiton directly.
//! Power users should use the [`YlmType::tokenize()`] and
//! [`YlmType::detokenize()`] methods.
//!
//! When implementing your own [`YlmType`], a variety of `From` impls have been
//! provided on the token structs to aid in converting from Rust data to tokens.
//!
//! ## Encoding/Decoding
//!
//! The process of converting from a [`Token`] to a serialized ABI blob is
//! called "Encoding". It is the reciprocal of decoding.
//!
//! ABI encoding and decoding operates on sequences of tokens.
//!
//! The [`YlmType`] encoding and decoding methods operate on Rust types. We
//! recommend users use them wherever possible. We do not recommend that users
//! interact with Tokens, except when implementing their own [`YlmType`].
//!
//! [`Token`]: abi::Token

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
// TODO: remove when https://github.com/proptest-rs/proptest/pull/427 is merged
#![allow(unknown_lints, non_local_definitions)]

#[allow(unused_extern_crates)]
extern crate self as base_ylm_types;

#[macro_use]
extern crate alloc;

#[macro_use]
mod macros;

pub mod abi;

mod errors;
pub use errors::{Error, Result};

#[cfg(feature = "json")]
mod ext;
#[cfg(feature = "json")]
pub use ext::JsonAbiExt;

mod impl_core;

mod types;
pub use types::{
    data_type as ylm_data, decode_revert_reason, ContractError, EventTopic, GenericContractError,
    GenericRevertReason, Panic, PanicKind, Revert, Selectors, TopicList, YlmCall, YlmConstructor,
    YlmEnum, YlmError, YlmEvent, YlmEventInterface, YlmInterface, YlmStruct, YlmType, YlmValue,
};

pub mod utils;

mod eip712;
pub use eip712::Eip712Domain;

/// The ABI word type.
pub type Word = base_primitives::B256;

#[doc(no_inline)]
pub use base_ylm_macro::ylm;

// Not public API.
#[doc(hidden)]
#[allow(missing_debug_implementations)]
pub mod private {
    pub use super::{
        abi::RECURSION_LIMIT,
        utils::{just_ok, next_multiple_of_32, words_for, words_for_len},
    };
    pub use alloc::{
        borrow::{Cow, ToOwned},
        boxed::Box,
        collections::BTreeMap,
        string::{String, ToString},
        vec,
        vec::Vec,
    };
    pub use base_primitives::{
        bytes, sha3, Bytes, FixedBytes, Function, IcanAddress as Address, LogData, Signed, Uint,
        B256, I256, U256,
    };
    pub use core::{
        borrow::{Borrow, BorrowMut},
        convert::From,
        default::Default,
        option::Option,
        result::Result,
    };

    pub use Option::{None, Some};
    pub use Result::{Err, Ok};

    #[cfg(feature = "json")]
    pub use base_json_abi;

    /// An ABI-encodable is any type that may be encoded via a given `YlmType`.
    ///
    /// The `YlmType` trait contains encoding logic for a single associated
    /// `RustType`. This trait allows us to plug in encoding logic for other
    /// `RustTypes`.
    ///
    /// **Note:** this trait is an implementation detail. As such, it should not
    /// be implemented directly unless implementing a custom
    /// [`YlmType`](crate::YlmType), which is also discouraged. Consider
    /// using [`YlmValue`](crate::YlmValue) instead.
    pub trait YlmTypeValue<T: super::YlmType> {
        // Note: methods are prefixed with `stv_` to avoid name collisions with
        // the `YlmValue` trait.
        fn stv_to_tokens(&self) -> T::Token<'_>;
        #[inline(always)]
        fn stv_abi_encoded_size(&self) -> usize {
            T::ENCODED_SIZE.unwrap()
        }
        fn stv_abi_encode_packed_to(&self, out: &mut Vec<u8>);
        fn stv_eip712_data_word(&self) -> super::Word;
    }

    #[inline(always)]
    pub const fn u256(n: u64) -> U256 {
        U256::from_limbs([n, 0, 0, 0])
    }

    pub struct AssertTypeEq<T>(pub T);
}
