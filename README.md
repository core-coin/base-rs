# Base-rs

Base libraries at the root of the Rust Core ecosystem.

Base-rs is a rewrite of [`corebc-rs`] from the ground up, with exciting new
features, high performance, and excellent docs.

[`corebc-rs`] will continue to be maintained until we have achieved
feature-parity in Base-rs. No action is currently needed from devs.

[`corebc-rs`]: https://github.com/core-coin/corebc-rs

## Overview

This repository contains the following crates:

- [`base-core`]: Meta-crate for the entire project
- [`base-primitives`] - Primitive integer and byte types
- [`base-ylm-types`] - Compile-time [ABI] and [EIP-712] implementations
- [`base-ylm-macro`] - The [`ylm!`] procedural macro
- [`base-dyn-abi`] - Run-time [ABI] and [EIP-712] implementations
- [`base-json-abi`] - Full Core [JSON-ABI] implementation
- [`base-ylm-type-parser`] - A simple parser for Ylem type strings
- [`syn-ylem`] - [`syn`]-powered Ylem parser

[`base-core`]: /crates/core
[`base-primitives`]: /crates/primitives
[`base-ylm-types`]: /crates/ylm-types
[`base-ylm-macro`]: /crates/ylm-macro
[`base-dyn-abi`]: /crates/dyn-abi
[`base-json-abi`]: /crates/json-abi
[`base-ylm-type-parser`]: /crates/ylm-type-parser
[`syn-ylem`]: /crates/syn-ylem
[JSON-ABI]: https://docs.soliditylang.org/en/latest/abi-spec.html#json
[ABI]: https://docs.soliditylang.org/en/latest/abi-spec.html
[EIP-712]: https://eips.ethereum.org/EIPS/eip-712
[`ylm!`]: https://docs.rs/alloy-ylm-macro/latest/alloy_ylm_macro/macro.sol.html
[`syn`]: https://github.com/dtolnay/syn

## Supported Rust Versions

<!--
When updating this, also update:
- clippy.toml
- Cargo.toml
- .github/workflows/ci.yml
-->

Base-rs will keep a rolling MSRV (minimum supported rust version) policy of **at
least** 6 months. When increasing the MSRV, the new Rust version must have been
released at least six months ago. The current MSRV is 1.65.0.

Note that the MSRV is not increased automatically, and only as part of a minor
release.

## Contributing

Thanks for your help improving the project! We are so happy to have you! We have
[a contributing guide](./CONTRIBUTING.md) to help you get involved in the
Base-rs project.

Pull requests will not be merged unless CI passes, so please ensure that your
contribution follows the linting rules and passes clippy.

## WASM support

We provide full support for all the `wasm*-*` targets. If a crate does not
build on a WASM target, please [open an issue].

When building for the `wasm32-unknown-unknown` target and the `"getrandom"`
feature is enabled, compilation for the `getrandom` crate will fail. This is
expected: see [their documentation][getrandom] for more details.

To fix this, either disable the `"getrandom"` feature on `base-core` or add
`getrandom` to your dependencies with the `"js"` feature enabled:

```toml
getrandom = { version = "0.2", features = ["js"] }
```

There is currently no plan to provide an official JS/TS-accessible library
interface, as we believe [viem] or [ethers.js] serve that need very well.

[open an issue]: https://github.com/core-coin/base-rs/issues/new/choose
[getrandom]: https://docs.rs/getrandom/#webassembly-support
[viem]: https://viem.sh
[ethers.js]: https://docs.ethers.io/v6/

## Note on `no_std`

All crates in this workspace should support `no_std` environments, with the
`alloc` crate. If you find a crate that does not support `no_std`, please
[open an issue].

[open an issue]: https://github.com/core-coin/base-rs/issues/new/choose

## Credits

None of these crates would have been possible without the great work done in:

- [`ethers.js`](https://github.com/ethers-io/ethers.js/)
- [`rust-web3`](https://github.com/tomusdrw/rust-web3/)
- [`ruint`](https://github.com/recmo/uint)
- [`ethabi`](https://github.com/rust-ethereum/ethabi)
- [`ethcontract-rs`](https://github.com/gnosis/ethcontract-rs/)
- [`guac_rs`](https://github.com/althea-net/guac_rs/)

#### License

<sup>
Licensed under either of <a href="LICENSE-APACHE">Apache License, Version
2.0</a> or <a href="LICENSE-MIT">MIT license</a> at your option.
</sup>

<br>

<sub>
Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in these crates by you, as defined in the Apache-2.0 license,
shall be dual licensed as above, without any additional terms or conditions.
</sub>
