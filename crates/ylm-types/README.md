# base-ylm-types

Compile-time representation of Core's type system with ABI and EIP-712
support.

This crate provides a developer-friendly interface to Core's type system,
by representing Ylem types. See [type_system.md](./type_system.md) for a rundown, and the
[crate docs] for more information

[crate docs]: https://docs.rs/base-ylm-types/latest/base_ylm_types/

### Features

- static representation of Ylem types
- ABI encoding and decoding
- EIP-712 encoding and decoding
- EIP-712 Domain object w/ `serde` support

### Usage

See the [crate docs] for more details.

```rust
// Declare a solidity type in standard solidity
ylm! {
    struct Foo {
        bar: u256;
        baz: bool;
    }
}

// A corresponding Rust struct is generated!
let foo = Foo {
    bar: 42.into(),
    baz: true,
};

// Works for UDTs
ylm! { type MyType is uint8; }
let my_type = MyType::from(42u8);

// For errors
ylm! {
    error MyError(
        string message,
    );
}

// And for functions!
ylm! { function myFunc() external returns (uint256); }
```

## Licensing

This crate is an extensive rewrite of the
[ethabi](https://github.com/rust-ethereum/ethabi) crate by the parity team.
That codebase is used under the terms of the **MIT** license. We have preserved
the original license notice in files incorporating `ethabi` code.
