use base_ylm_types::ylm;

ylm! {
    error MissingParens1
}

ylm! {
    error MissingParens2;
}

ylm! {
    error MissingSemi()
}

ylm! {
    error TrailingComma(uint256,);
    error Valid(uint256);
}

fn main() {}
