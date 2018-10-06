//! The logic of reading values from OAM/palette/VRAM etc. and determining
//! what color each pixel on the screen is goes here. Each pixel has 5 bits each
//! for RGB, and 1 pixel for alpha

use mem::Memory;

pub const WIDTH: usize = 240;
pub const HEIGHT: usize = 160;

pub struct FrameBuffer {
    pixels: [[u16; WIDTH]; HEIGHT]
}

impl FrameBuffer {
    pub const fn new() -> FrameBuffer {
        FrameBuffer {
            pixels: [[0; WIDTH]; HEIGHT],
        }
    }
}

impl Memory {
    pub fn update_pixel(&mut self, _row: u32, _col: u32) {
        unimplemented!()
    }
}
