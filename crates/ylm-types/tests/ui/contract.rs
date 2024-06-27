use base_ylm_types::ylm;

ylm! {
    contract MissingBraces1
}

ylm! {
    contract MissingBraces2 is A
}

ylm! {
    contract MissingInheritance1 is
}

ylm! {
    contract MissingInheritance2 is;
}

ylm! {
    contract C {
        contract Nested {}
    }
}

ylm! {
    interface C {
        library Nested {}
    }
}

ylm! {
    abstract contract C {
        interface Nested {}
    }
}

fn main() {}
