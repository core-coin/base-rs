# alloy-primitives

Primitive types shared by [alloy], [foundry], [revm], and [reth].

[alloy]: https://github.com/alloy-rs
[foundry]: https://github.com/foundry-rs/foundry
[revm]: https://github.com/bluealloy/revm
[reth]: https://github.com/paradigmxyz/reth

## Types

- Unsigned integers re-exported from [ruint](https://github.com/recmo/uint)
- Signed integers, as a wrapper around `ruint` integers
- Fixed-size byte arrays via [`FixedBytes`]
  - [`wrap_fixed_bytes!`]: macro for constructing named fixed bytes types
  - [`Address`], which is a fixed-size byte array of 20 bytes, with EIP-55 and
    EIP-1191 checksum support
  - [`fixed_bytes!`], [`address!`] and other macros to construct the types at
    compile time

## Examples

This library has straightforward, basic, types. Usage is correspondingly simple.
Please consult [the documentation][docs] for more information.

[docs]: https://docs.rs/alloy-primitives/latest/alloy_primitives/

```rust
use alloy_primitives::{address, fixed_bytes, Address, FixedBytes, I256, U256};

// FixedBytes
let n: FixedBytes<6> = fixed_bytes!("1234567890ab");
assert_eq!(n, "0x1234567890ab".parse::<FixedBytes<6>>().unwrap());
assert_eq!(n.to_string(), "0x1234567890ab");

// Uint
let mut n: U256 = "42".parse().unwrap();
n += U256::from(10);
assert_eq!(n.to_string(), "52");

// Signed
let mut n: I256 = "-42".parse().unwrap();
n = -n;
assert_eq!(n.to_string(), "42");
```
