#!/bin/sh
rm -f phext_link
rm -f phext_work
cargo build
.\target\Debug\sq .\world.phext