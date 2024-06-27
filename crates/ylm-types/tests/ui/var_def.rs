use base_ylm_types::ylm;

// OK
ylm! {
    struct Simple {
        uint a;
    }

    mapping(int => Simple) public simpleMap;
}

// Not OK
ylm! {
    struct Complex1 {
        uint[] a;
    }

    mapping(int => Complex1) public complexMap;
}

// OK
ylm! {
    struct DoubleComplex {
        Complex2 a;
    }
    struct Complex2 {
        uint[] a;
    }

    mapping(int => DoubleComplex) public complexMap;
}

fn main() {}
