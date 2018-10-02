// TODO: can we only compile this file when we build for wasm?
use cpu::CPUWrapper;
use wasm_bindgen::prelude::*;

pub static mut GBA: CPUWrapper = CPUWrapper::new();

#[wasm_bindgen]
extern {
    #[wasm_bindgen(js_namespace = console)]
    fn log(msg: &str);
}

#[wasm_bindgen]
pub fn upload_rom(data: &[u8]) {
    unsafe {
        GBA.cpu.mem.load_rom(data)
    }
}

#[wasm_bindgen]
pub fn get_registers() -> *const u32 {
    unsafe {
        GBA.cpu.r[0] = 142131;
        GBA.cpu.r[15] = 12345678;
        &GBA.cpu.r as *const u32
    }
}
