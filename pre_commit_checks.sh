!#/bin/bash

cargo check --example triangles --example points --example lines
cargo check --features tokio-threads --example tokio-triangles
cargo clippy --all-features -- -D warnings
cargo fmt --all --check
# LINUX ONLY
if [ $(uname -a | cut -d ' '  -f 1) == "Linux" ]; then
  cargo test --tests --lib
fi
cargo build --lib
cargo build --lib --features tokio-threads
cargo build --example triangles --example lines --example points --example spinning-cube
cargo build --features tokio-threads --example tokio-triangles
cargo build --release --lib
cargo build --release --example triangles --example lines --example points --example spinning-cube
cargo build --release --features tokio-threads --example tokio-triangles
