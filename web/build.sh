#!/bin/sh
set -e
cd "$(dirname "$0")/.."
VERSION=$(grep -A1 '^name = "wasm-bindgen"$' Cargo.lock | sed -n 's/^version = "\(.*\)"/\1/p')
cargo install wasm-bindgen-cli --version "$VERSION" --locked
cargo build --lib --release --target wasm32-unknown-unknown
wasm-bindgen --target web --no-typescript --out-dir web/pkg \
    target/wasm32-unknown-unknown/release/bsky_gif_unfucker.wasm
echo "Built web/pkg"
