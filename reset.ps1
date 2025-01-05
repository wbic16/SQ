Remove-Item -Force -Recurse .sq
cargo build
cargo build --release
cargo test
.\target\Debug\sq.exe .\world.phext
