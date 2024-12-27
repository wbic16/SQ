Remove-Item -Force phext_link
Remote-Item -Force phext_work
cargo build
.\target\Debug\sq.exe .\world.phext