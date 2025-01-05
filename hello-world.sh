#!/bin/sh
echo "Hello World" >hi.txt
cargo run example.phext &
cargo run push 1.1.1/1.1.1/1.1.1 hi.txt
cargo run pull 1.1.1/1.1.1/1.1.1 out.txt
