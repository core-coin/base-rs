use base_ylm_types::ylm;

mod function {
    use super::*;

    ylm! {
        function overloaded();
        function overloaded(uint256);
        function overloaded(uint256,address);
        function overloaded(address);
        function overloaded(address,string);
    }

    ylm! {
        function overloadTaken();
        function overloadTaken(uint256);
        function overloadTaken_0();
        function overloadTaken_1();
        function overloadTaken_2();
    }

    ylm! {
        function sameOverload();
        function sameOverload();
    }

    ylm! {
        function sameTysOverload1(uint256[]memory a);
        function sameTysOverload1(uint256[]storage b);
    }

    ylm! {
        function sameTysOverload2(string memory,string storage);
        function sameTysOverload2(string storage b,string calldata);
    }
}

mod event {
    use super::*;

    ylm! {
        event overloaded();
        event overloaded(uint256);
        event overloaded(uint256,address);
        event overloaded(address);
        event overloaded(address,string);
    }

    ylm! {
        event overloadTaken();
        event overloadTaken(uint256);
        event overloadTaken_0();
        event overloadTaken_1();
        event overloadTaken_2();
    }

    ylm! {
        event sameOverload();
        event sameOverload();
    }

    ylm! {
        event sameTysOverload1(uint256[] a);
        event sameTysOverload1(uint256[] b);
    }

    ylm! {
        event sameTysOverload2(string, string);
        event sameTysOverload2(string, string);
    }
}

/*
mod error {
    use super::*;

    ylm! {
        error overloaded();
        error overloaded(uint256);
        error overloaded(uint256,address);
        error overloaded(address);
        error overloaded(address,string);
    }

    ylm! {
        error overloadTaken();
        error overloadTaken(uint256);
        error overloadTaken_0();
        error overloadTaken_1();
        error overloadTaken_2();
    }

    ylm! {
        error sameOverload();
        error sameOverload();
    }

    ylm! {
        error sameTysOverload1(uint256[] a);
        error sameTysOverload1(uint256[] b);
    }

    ylm! {
        error sameTysOverload2(string, string);
        error sameTysOverload2(string, string);
    }
}
*/

fn main() {}
