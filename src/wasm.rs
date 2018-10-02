// TODO: can we only compile this file when we build for wasm?
use cpu::CPUWrapper;
use wasm_bindgen::prelude::*;

pub static mut GBA: CPUWrapper = CPUWrapper::new();

#[wasm_bindgen]
pub fn upload_rom(data: &[u8]) {
    unsafe {
        GBA.cpu.mem.load_rom(data)
    }
}

#[wasm_bindgen]
pub fn get_registers() -> *const u8 {
    unsafe { &GBA.cpu.r as *const u32 as *const u8 }
}

#[wasm_bindgen]
pub fn get_bios() -> *const u8 {
    unsafe { &GBA.cpu.mem.raw.sysrom as *const u8 }
}

#[wasm_bindgen]
pub fn step() {
    unsafe { GBA.step(); }
}

#[wasm_bindgen]
pub fn get_cpsr() -> u32 {
    unsafe { GBA.cpu.cpsr.to_u32() }
}