#!/usr/bin/env bash

set -eo pipefail

export RUSTFLAGS="-Zthreads=1"
export TRYBUILD=overwrite
cargo +nightly test -p base-ylm-types --test compiletest
cargo +nightly test -p base-ylm-types --test compiletest --features json
