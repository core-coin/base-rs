//! This crate contains inputs to the `ylm!` macro. It sits in-between
//! the `ylm-macro` and `syn-ylem` crates, and contains an intermediate
//! representation of Ylem items. These items are then expanded into
//! Rust code by the `base-ylm-macro` crate.
//!
//! This crate is not meant to be used directly, but rather is a tool for
//! writing macros that generate Rust code from Ylem code.

#![warn(missing_copy_implementations, missing_debug_implementations, missing_docs, rustdoc::all)]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]
#![deny(unused_must_use, rust_2018_idioms)]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]

extern crate syn_ylem as ast;

/// Tools for working with `#[...]` attributes.
mod attr;
pub use attr::{derives_mapped, docs_str, mk_doc, ContainsYlmAttrs, YlmAttrs};

mod input;
pub use input::{YlmInput, YlmInputKind};

mod expander;
pub use expander::YlmInputExpander;

#[cfg(feature = "json")]
mod json;
