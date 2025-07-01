#!/bin/bash
export RUSTFLAGS="-C linker=aarch64-linux-gnu-gcc"
cargo build --release --target=aarch64-unknown-linux-gnu
ls -l target/aarch64-unknown-linux-gnu/release/
