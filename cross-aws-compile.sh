#!/bin/bash
export RUSTFLAGS="-C linker=aarch64-linux-gnu-gcc"
cargo build --release --target=aarch64-unknown-linux-musl
scp target/aarch64-unknown-linux-musl/release/sq ec2-user@phext.io:/exo/sq
