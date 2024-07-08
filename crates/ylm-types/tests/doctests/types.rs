use base_primitives::IcanAddress;
use base_ylm_types::{ylm, YlmType};

// Type definition: generates a new struct that implements `YlmType`
ylm! {
    type MyType is uint256;
}

// Type aliases
type B32 = ylm! { bytes32 };
// This is equivalent to the following:
// type B32 = base_ylm_types::ylm_data::Bytes<32>;

type YlmArrayOf<T> = ylm! { T[] };
type YlmTuple = ylm! { tuple(address, bytes, string) };

#[test]
fn types() {
    let _ = <ylm!(bool)>::abi_encode(&true);
    let _ = B32::abi_encode(&[0; 32]);
    let _ = YlmArrayOf::<ylm!(bool)>::abi_encode(&vec![true, false]);
    let _ = YlmTuple::abi_encode(&(IcanAddress::ZERO, vec![0; 32], "hello".to_string()));
}
