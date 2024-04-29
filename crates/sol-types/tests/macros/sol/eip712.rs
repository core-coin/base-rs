use alloy_primitives::B256;
use alloy_sol_types::{eip712_domain, sol, SolStruct};

#[test]
fn encode_type_nesting() {
    sol! {
        struct A {
            uint256 a;
        }

        struct B {
            bytes32 b;
        }

        struct C {
            A a;
            B b;
        }

        struct D {
            C c;
            A a;
            B b;
        }
    }

    assert_eq!(A::eip712_encode_type(), "A(uint256 a)");
    assert_eq!(B::eip712_encode_type(), "B(bytes32 b)");
    assert_eq!(C::eip712_encode_type(), "C(A a,B b)A(uint256 a)B(bytes32 b)");
    assert_eq!(D::eip712_encode_type(), "D(C c,A a,B b)A(uint256 a)B(bytes32 b)C(A a,B b)");
}

#[test]
fn encode_data_nesting() {
    sol! {
        struct Person {
            string name;
            address wallet;
        }

        struct Mail {
            Person from;
            Person to;
            string contents;
        }
    }
    let domain = eip712_domain! {};

    let mail = Mail {
        from: Person {
            name: "Cow".to_owned(),
            wallet: "0x0000CD2a3d9F938E13CD947Ec05AbC7FE734Df8DD826".parse().unwrap(),
        },
        to: Person {
            name: "Bob".to_owned(),
            wallet: "0x0000bBbBBBBbbBBBbbbBbbBbbbbBBbBbbbbBbBbbBBbB".parse().unwrap(),
        },
        contents: "Hello, Bob!".to_owned(),
    };

    assert_eq!(
        alloy_sol_types::SolStruct::eip712_signing_hash(&mail, &domain),
        "be504c79df6f0a61fbafb0d84827b301d2e888d9e578eea504654f73e33705be".parse::<B256>().unwrap()
    )
}
