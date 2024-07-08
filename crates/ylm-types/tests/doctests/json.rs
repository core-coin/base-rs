use base_ylm_types::{ylm, YlmCall};

ylm!(
    MyJsonContract1,
    r#"[
        {
            "inputs": [
                { "name": "bar", "type": "uint256" },
                { 
                    "internalType": "struct MyJsonContract.MyStruct",
                    "name": "baz",
                    "type": "tuple",
                    "components": [
                        { "name": "a", "type": "bool[]" },
                        { "name": "b", "type": "bytes18[][]" }
                    ]
                }
            ],
            "outputs": [],
            "stateMutability": "view",
            "name": "foo",
            "type": "function"
        }
    ]"#
);

// This is the same as:
ylm! {
    interface MyJsonContract2 {
        struct MyStruct {
            bool[] a;
            bytes18[][] b;
        }

        function foo(uint256 bar, MyStruct baz) external view;
    }
}

#[test]
fn abigen() {
    assert_eq!(MyJsonContract1::fooCall::SIGNATURE, MyJsonContract2::fooCall::SIGNATURE,);
}
