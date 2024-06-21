#![allow(unknown_lints, clippy::incompatible_msrv)]

use base_primitives::{sha3, Address, B256};
use criterion::{criterion_group, criterion_main, Criterion};
use std::hint::black_box;

fn primitives(c: &mut Criterion) {
    let mut g = c.benchmark_group("primitives");
    g.bench_function("address/checksum", |b| {
        let address = Address::random();
        let out = &mut [0u8; 42];
        b.iter(|| {
            let x = address.to_checksum_raw(black_box(out), None);
            black_box(x);
        })
    });
    g.bench_function("sha3/32", |b| {
        let mut out = B256::random();
        b.iter(|| {
            out = sha3(out.as_slice());
            black_box(&out);
        });
    });
    g.finish();
}

criterion_group!(benches, primitives);
criterion_main!(benches);
