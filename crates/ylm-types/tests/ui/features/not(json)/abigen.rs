use base_ylm_types::ylm;

ylm!(EmptyStr, "");

ylm!(PathDoesNotExist, "???");
ylm!("pragma solidity ^0.8.0");
ylm!("pragma solidity ^0.8.0;");

ylm!(NoJsonFeature1, "{}");
ylm!(NoJsonFeature2, "{ \"abi\": [] }");
ylm!(NoJsonFeature3, "[]");

fn main() {}
