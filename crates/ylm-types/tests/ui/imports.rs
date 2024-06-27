use base_ylm_types::ylm;

ylm! {
    import *;
}

ylm! {
    import * as foo;
}

ylm! {
    import * as foo from;
}

// OK
ylm! {
    import "path";
    import "path" as foo;

    import {} from "path";
    import { a, b as c, d } from "path";

    import * from "path";
    import * as foo from "path";
}

fn main() {}
