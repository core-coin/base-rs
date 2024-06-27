use base_ylm_types::ylm;

ylm! {
    function missingParens;
}

ylm! {
    function missingSemi1()
}

ylm! {
    function missingSemi2() external
}

ylm! {
    function missingSemi3() returns (uint256)
}

// OK
ylm! {
    function semiNotBrace1() {}
}

// OK
ylm! {
    function semiNotBrace2() external {}
}

// OK
ylm! {
    function semiNotBrace3() returns (uint256) {}
}

ylm! {
    function singleComma(,);
}

// OK
ylm! {
    function trailingComma1(bytes,);
    function trailingComma2(bytes a,);
    function trailingComma3(bytes memory a,);
}

ylm! {
    function badReturn1() returns;
}

ylm! {
    function badReturn2() returns();
}

// OK
ylm! {
    function a() private;
    function b() internal;
    function c() public;
    function d() external;

    function e() pure;
    function f() view;
    function g() constant;
    function h() payable;

    function i() virtual;
    function j() immutable;

    function k() override(Interface.k);
    function l() myModifier("a", 0);

    function m() external view returns (uint256);
    function n() public pure returns (uint256,);
}

fn main() {}
