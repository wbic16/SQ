#!/bin/sh
rm -rf .sq
cargo build && cargo build --release && cargo test
.\target\Debug\sq .\world.phext
