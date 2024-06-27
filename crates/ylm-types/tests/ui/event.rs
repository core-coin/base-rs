use base_ylm_types::ylm;

ylm! {
    event MissingParens1
}

ylm! {
    event MissingParens2 anonymous;
}

ylm! {
    event MissingParens3;
}

ylm! {
    event MissingSemi1()
}

ylm! {
    event MissingSemi2() anonymous
}

ylm! {
    event FourIndexedParameters(bool indexed, bool indexed, bool indexed, bool indexed);
}

ylm! {
    event FiveIndexedParameters(bool indexed, bool indexed, bool indexed, bool indexed, bool indexed);
}

ylm! {
    event FourIndexedParametersAnonymous(bool indexed, bool indexed, bool indexed, bool indexed) anonymous;
}

ylm! {
    event FiveIndexedParametersAnonymous(bool indexed, bool indexed, bool indexed, bool indexed, bool indexed) anonymous;
}

ylm! {
    event ALotOfParameters(bool, bool, bool, bool, bool, bool, bool, bool, bool, bool);
    event ALotOfParametersAnonymous(bool, bool, bool, bool, bool, bool, bool, bool, bool, bool) anonymous;
}

ylm! {
    event TrailingComma(uint256,);
    event Valid(uint256);
}

fn main() {}

struct A {}
