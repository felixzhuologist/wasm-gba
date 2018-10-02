#!/bin/sh
set -ex

cargo +nightly build --target wasm32-unknown-unknown
wasm-bindgen target/wasm32-unknown-unknown/debug/gba.wasm --out-dir www

cd www && npm run start