// TODO: can we only compile this file when we build for wasm?
use cpu::CPUWrapper;
use wasm_bindgen::prelude::*;

pub static mut GBA: CPUWrapper = CPUWrapper::new();

#[wasm_bindgen]
extern {
    #[wasm_bindgen(js_namespace = console)]
    fn log(msg: &str);
}

// A macro to provide `println!(..)`-style syntax for `console.log` logging.
macro_rules! log {
    ($($t:tt)*) => (log(&format!($($t)*)))
}

#[wasm_bindgen]
pub fn upload_bios(data: &[u8]) {
    unsafe { GBA.cpu.mem.load_bios(data) }
}

#[wasm_bindgen]
pub fn upload_rom(data: &[u8]) {
    log!("rom size: {:X}", data.len());
    unsafe { GBA.cpu.mem.load_rom(data) }
}

#[wasm_bindgen]
pub fn get_register(i: usize) -> u32 {
    unsafe { GBA.cpu.get_reg(i) }
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
pub fn frame() {
    unsafe { GBA.frame(); }
}

#[wasm_bindgen]
pub fn get_cpsr() -> u32 {
    unsafe { GBA.cpu.cpsr.to_u32() }
}