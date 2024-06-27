use base_ylm_types::ylm;

ylm! {}

ylm! {
    struct EmptyStruct {}
}

ylm! {
    enum EmptyEnum {}
}

// OK
ylm! {
    contract EmptyContract {}
}

ylm! {
    error EmptyError();
}

ylm! {
    event EmptyEvent();
}

ylm! {
    function emptyFunction();
}

fn main() {}
