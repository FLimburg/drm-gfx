!#/bin/bash
cargo check --example triangles --example points --example lines
cargo check --features tokio-threads --example tokio-triangles
cargo clippy --all-features -- -D warnings
cargo fmt --all --check
cargo test --tests --lib
