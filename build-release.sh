#!/bin/sh
set -ex

cargo +nightly build --release --target wasm32-unknown-unknown
wasm-bindgen target/wasm32-unknown-unknown/release/gba.wasm --out-dir www

cd www && npm run start