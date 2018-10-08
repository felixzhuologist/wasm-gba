// TODO: can we only compile this file when we build for wasm?
use cpu::CPUWrapper;
use wasm_bindgen::prelude::*;
use console_error_panic_hook;
use std::panic;

pub static mut GBA: CPUWrapper = CPUWrapper::new();

#[wasm_bindgen]
extern {
    #[wasm_bindgen(js_namespace = console)]
    fn log(msg: &str);

    #[wasm_bindgen(js_namespace = console)]
    fn error(msg: &str);
}

// A macro to provide `println!(..)`-style syntax for `console.log` logging.
macro_rules! log {
    ($($t:tt)*) => (log(&format!($($t)*)))
}

macro_rules! error {
    ($($t:tt)*) => (error(&format!($($t)*)))
}

/// should be called once to initialize panic hook
#[wasm_bindgen]
pub fn set_panic_hook() {
    panic::set_hook(Box::new(|inf| {
        console_error_panic_hook::hook(inf);
        error!("CPU dump:");
        unsafe {
            error!("Failed instruction: {:#?}", GBA.last_instruction.clone());
            error!("CPSR: {:#?}", GBA.cpu.cpsr);
            error!("User registers: {:#X?}", GBA.cpu.r);
        }
    }));
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
pub fn get_bg_palette() -> *const u8 {
    unsafe { &GBA.cpu.mem.palette.bg as *const u32 as *const u8 }
}

#[wasm_bindgen]
pub fn get_sprite_palette() -> *const u8 {
    unsafe { &GBA.cpu.mem.palette.sprite as *const u32 as *const u8 }
}

#[wasm_bindgen]
pub fn get_vram() -> *const u8 {
    unsafe { &GBA.cpu.mem.raw.vram as *const u8 }
}

#[wasm_bindgen]
pub fn step() -> bool {
    unsafe { GBA.step(); GBA.cpu.should_flush }
}

#[wasm_bindgen]
pub fn frame() {
    unsafe { GBA.frame(); }
}

#[wasm_bindgen]
pub fn get_cpsr() -> u32 {
    unsafe { GBA.cpu.cpsr.to_u32() }
}
