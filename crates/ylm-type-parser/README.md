# base-ylm-type-parser

Simple and light-weight Ylem type strings parser.

This library is primarily a dependency for the user-facing APIs in
[`base-json-abi`] and [`base-dyn-abi`]. Please see the documentation for
those crates for more information.

This parser generally follows the [Ylem spec], however, it supports only a
subset of possible types, chosen to support ABI coding.

[Ylem spec]: https://docs.soliditylang.org/en/latest/grammar.html#a4.YlemParser.typeName
[`base-json-abi`]: https://docs.rs/base-json-abi/latest/base_json_abi/
[`base-dyn-abi`]: https://docs.rs/base-dyn-abi/latest/base_dyn_abi/

### Usage

The `TypeSpecifier` is the top-level type in this crate. It is a wrapper around
a section of a string (called a `span`). It progressively breaks the strings
down into subspans, and adds metadata about the type. E.g. it tracks the stem
type as well as the sizes of array dimensions. A `TypeSpecifier` is expected to
handle any valid Ylem type string.

```rust
use base_ylm_type_parser::TypeSpecifier;
use core::num::NonZeroUsize;

// Parse a type specifier from a string
let my_type = TypeSpecifier::parse("uint8[2][]").unwrap();

// Read the total span
assert_eq!(
    my_type.span(),
    "uint8[2][]"
);

// A type specifier has a stem type. This is the type string, stripped of its
// array dimensions.
assert_eq!(my_type.stem.span(), "uint8");

// Arrays are represented as a vector of sizes. This allows for deep nesting.
assert_eq!(
    my_type.sizes,
    // `None` is used for dynamic sizes. This is equivalent to `[2][]`
    vec![NonZeroUsize::new(2), None]
);

// Type specifiers also work for complex tuples!
let my_tuple = TypeSpecifier::parse("(uint8,(uint8[],bool))[39]").unwrap();
assert_eq!(
    my_tuple.stem.span(),
    "(uint8,(uint8[],bool))"
);

// Types are NOT resolved, so you can parse custom structs just by name.
let my_struct = TypeSpecifier::parse("MyStruct").unwrap();
```

### Why not support `parse()`?

The `core::str::FromStr` trait is not implemented for `TypeSpecifier` because
of lifetime constraints. Unfortunately, it is impossible to implement this for
a type with a lifetime dependent on the input str. Instead, we recommend using
the `parse` associated functions, or `TryFrom::<&str>::try_from` if a trait is
needed.

### Why not use `syn`?

This is NOT a full syntax library, and is not intended to be used as a
replacement for [`syn-ylem`]. This crate is intended to be used for
parsing type strings present in existing ecosystem tooling, and nothing else.
It is not intended to be used for parsing Ylem source code.

This crate is useful for:

- syntax-checking JSON ABI files
- providing known-good input to [`base-dyn-abi`]
- porting ethers.js code to rust

It is NOT useful for:

- parsing Ylem source code
- generating Rust code from Ylem source code
- generating Ylem source code from rust code

[`syn-ylem`]: https://docs.rs/syn-ylem/latest/syn_ylem/
