use base_ylm_types::ylm;

ylm! {
    struct A {
        B a;
    }

    struct B {
        A a;
    }
}

ylm! {
    struct A {
        B a;
    }

    struct B {
        C c;
    }

    struct C {
        A a;
    }
}

ylm! {
    struct A {
        B a;
    }

    struct B {
        C c;
    }

    struct C {
        D d;
    }

    struct D {
        A a;
    }
}

fn main() {}
