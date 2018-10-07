//! The logic of reading values from OAM/palette/VRAM etc. and determining
//! what color each pixel on the screen is goes here. Each pixel has 5 bits each
//! for RGB, and 1 pixel for alpha

use mem::Memory;
use mem::oam::Sprite;

pub const WIDTH: usize = 240;
pub const HEIGHT: usize = 160;

pub struct FrameBuffer {
    pixels: [[u32; WIDTH]; HEIGHT]
}

impl FrameBuffer {
    pub const fn new() -> FrameBuffer {
        FrameBuffer {
            pixels: [[0; WIDTH]; HEIGHT],
        }
    }
}

impl Memory {
    /// Update the framebuffer at the given pixel. Will try to render sprites/
    /// backgrounds in order of priority; if there no objects at this pixel then
    /// use the first background palette color as a fallback
    pub fn update_pixel(&mut self, row: u32, col: u32) {
        // self.framebuffer.pixels[row as usize][col as usize] = (0..4)
        //     .filter_map(|i| self.by_priority(i, row, col))
        //     .next()
        //     .unwrap_or(self.palette.bg[0])
    }

    fn by_priority(&self, priority: u8, row: u32, col: u32) -> Option<u32> {
        self.render_sprites(priority, row, col)
            .or_else(|| self.render_bgs(priority, row, col))
    }

    fn render_sprites(&self, priority: u8, row: u32, col: u32) -> Option<u32> {
        self.sprites.sprites.iter()
            .filter(|ref sprite| sprite.priority == priority)
            .filter_map(|ref sprite| self.render_sprite_pixel(sprite, row, col))
            .next()
    }

    fn render_bgs(&self, priority: u8, row: u32, col: u32) -> Option<u32> {
        self.graphics.bg_cnt.iter().enumerate()
            .filter(|(_, bg)| bg.priority == priority)
            .filter_map(|(i, _)| self.render_bg_pixel(i, row, col))
            .next()
    }
 
    // background modes:
    //     tile modes:
    // 0: 4 tile layers (bg0 - bg3)
    // 1: 2 tile layers + 1 affine tile layer (bg0 - bg2)
    // 2: 2 affine tile layers (bg2, bg3)
    //     bitmap modes (all use bg 2):
    // 3: 240x160 15 bit bitmap with no page flip
    // 4: 240x160 8 bit bitmap with page flip. the 8 bits here are an index into
    //    the background palette at 0x5000000
    // 5: 160x128 15 bit bitmap with page flip
    fn render_bg_pixel(&self, bg: usize, row: u32, col: u32) -> Option<u32> {
        match (self.graphics.disp_cnt.bg_mode, bg) {
            (0, _) => self.render_tile_bg(bg, row, col),
            (1, 0) => self.render_tile_bg(bg, row, col),
            (1, 1) => self.render_tile_bg(bg, row, col),
            (1, 2) => self.render_affine_bg(bg, row, col),
            (2, 2) => self.render_affine_bg(bg, row, col),
            (2, 3) => self.render_affine_bg(bg, row, col),
            (3, 2) => self.render_bitmap_bg(bg, row, col),
            (4, 2) => self.render_bitmap_bg(bg, row, col),
            (5, 2) => self.render_bitmap_bg(bg, row, col),
            _ => None
        }
    }

    fn render_sprite_pixel(
        &self,
        _sprite: &Sprite,
        _row: u32,
        _col: u32) -> Option<u32> {
        None
    }

    fn render_tile_bg(&self, _bg: usize, _row: u32, _col: u32) -> Option<u32> {
        None
    }

    fn render_affine_bg(&self, _bg: usize, _row: u32, _col: u32) -> Option<u32> {
        None
    }

    fn render_bitmap_bg(&self, _bg: usize, _row: u32, _col: u32) -> Option<u32> {
        None
    }
}