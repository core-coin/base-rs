use alloy_primitives::{hex, IcanAddress, U256};
use alloy_sol_types::{sol, SolEnum, SolType};

sol! {
    struct Foo {
        uint256 bar;
        address[] baz;
    }

    /// Nested struct.
    struct Nested {
        Foo[2] a;
        address b;
    }

    enum Enum {
        A,
        B,
        C,
    }
}

#[test]
fn structs() {
    let my_foo = Foo {
        bar: U256::from(42),
        baz: vec![IcanAddress::repeat_byte(0x11), IcanAddress::repeat_byte(0x22)],
    };

    let _nested = Nested { a: [my_foo.clone(), my_foo.clone()], b: IcanAddress::ZERO };

    let abi_encoded = Foo::abi_encode_sequence(&my_foo);
    assert_eq!(
        abi_encoded,
        hex! {
            "000000000000000000000000000000000000000000000000000000000000002a" // bar
            "0000000000000000000000000000000000000000000000000000000000000040" // baz offset
            "0000000000000000000000000000000000000000000000000000000000000002" // baz length
            "0000000000000000000011111111111111111111111111111111111111111111" // baz[0]
            "0000000000000000000022222222222222222222222222222222222222222222" // baz[1]
        }
    );

    let abi_encoded_enum = Enum::B.abi_encode();
    assert_eq!(
        abi_encoded_enum,
        hex! {
            "0000000000000000000000000000000000000000000000000000000000000001"
        }
    );
}
