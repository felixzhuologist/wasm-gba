// #![no_std]
#![feature(const_fn)]
#![feature(reverse_bits)]

#[macro_use]
extern crate enum_primitive;
extern crate num;
extern crate wasm_bindgen;
extern crate console_error_panic_hook;

pub use wasm::*;
pub use wasm::GBA;

pub mod cpu;
pub mod mem;
pub mod util;
pub mod wasm;
