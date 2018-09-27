use cpu::CPUWrapper;
use wasm_bindgen::prelude::*;

// static mut GBA: CPUWrapper = CPUWrapper::new();

#[wasm_bindgen]
pub struct GBA {
    cpu: CPUWrapper
}

#[wasm_bindgen]
impl GBA {
    pub fn new() -> GBA {
        GBA { cpu: CPUWrapper::new() }
    }
}
