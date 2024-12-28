cargo build --release
Remove-Item -Recurse -Force .sq
$server = cargo run init &
cargo run insert 1.1.1/1.1.1/1.1.1 "123456789"
cargo run insert 1.1.1/1.1.1/1.1.2 "234567891"
cargo run insert 1.1.1/1.1.1/1.1.3 "345678912"
cargo run insert 1.1.1/1.1.1/1.4.1 "456789123"
cargo run insert 1.1.1/1.1.1/5.3.2 "567891234"
cargo run insert 1.1.1/1.1.6/3.3.3 "678912345"
cargo run insert 1.1.1/1.7.1/5.3.2 "789123456"
cargo run insert 1.1.1/8.1.4/7.3.2 "891234567"
cargo run insert 1.1.9/1.5.1/5.5.2 "912345678"
cargo run insert 1.10.1/6.1.1/9.8.4 "123456789"
cargo run insert 11.4.1/1.1.1/5.5.5 "123456789"
dir world.phext
cargo run select 1.10.1/6.1.1/9.8.4
cargo run shutdown now
$server