//! Background modes:
//!     tile modes:
//! 0: 4 tile layers (bg0 - bg3)
//! 1: 2 tile layers + 1 rotates/scaled tile layer (bg0 - bg2)
//! 2: 2 rotated/scaled tile layers (bg2, bg3)
//!     bitmap modes (all use bg 2):
//! 3: 240x160 15 bit bitmap with no page flip
//! 4: 240x160 8 bit bitmap with page flip. the 8 bits here are an index into
//!    the background palette at 0x5000000
//! 5: 160x128 15 bit bitmap with page flip

use super::addrs::*;
use super::super::Memory;
use core::cmp::min;

/// Contains all graphics related information from the LCD display I/O registers.
/// The data in this struct is a mirror of the data from addresses
/// 0x4000000 - 0x4000060, and is updated using set_byte() each time that segment
/// of memory is updated.
pub struct GraphicsIO {
    disp_cnt: DispCnt,
    disp_stat: DispStat,
    /// Stores the current Y location of the current line being drawn
    vcount: u8,
    bg_cnt: [BgCnt; 4],
    bg_offset_x: [u16; 4],
    bg_offset_y: [u16; 4],
    bg_affine: [BgAffineParams; 2],

    window_coords: [WindowCoords; 2],
    // win0 inside, win1 inside, win0 outside, win1 outside
    window_settings: [WindowSettings; 4],

    bg_mos_hsize: u8,
    bg_mos_vsize: u8,
    obj_mos_hsize: u8,
    obj_mos_vsize: u8,
    blend_params: BlendParams,

    alpha_a_coef: f32,
    alpha_b_coef: f32,
    brightness_coef: f32,
}

impl GraphicsIO {
    pub const fn new() -> GraphicsIO {
        GraphicsIO {
            disp_cnt: DispCnt::new(),
            disp_stat: DispStat::new(),
            vcount: 0,
            bg_cnt: [
                BgCnt::new(),
                BgCnt::new(),
                BgCnt::new(),
                BgCnt::new(),
            ],
            bg_offset_x: [0; 4],
            bg_offset_y: [0; 4],
            bg_affine: [
                BgAffineParams::new(),
                BgAffineParams::new(),
            ],
            window_coords: [
                WindowCoords::new(),
                WindowCoords::new(),
            ],
            window_settings: [
                WindowSettings::new(),
                WindowSettings::new(),
                WindowSettings::new(),
                WindowSettings::new(),
            ],
            bg_mos_hsize: 0,
            bg_mos_vsize: 0,
            obj_mos_hsize: 0,
            obj_mos_vsize: 0,
            blend_params: BlendParams::new(),
            alpha_a_coef: 0.0,
            alpha_b_coef: 0.0,
            brightness_coef: 0.0,
        }
    }
}

// TODO: get rid of update_graphics_byte, since all of these registers are
// 16 bits anyway. If a single byte does get updated, should just call the hw
// update but rounded down to the nearest hw
impl Memory {
    pub fn update_graphics_byte(&mut self, addr: u32, val: u8) {
        let graphics = &mut self.graphics;
        match addr {
            DISPCNT_LO => {
                if (val & 0x7) <= 5 {
                    graphics.disp_cnt.bg_mode = val & 0x7;
                }
                graphics.disp_cnt.frame_base =
                    if (val & 0x10) > 0 { 0x600A000 } else { 0x60000000 };
                graphics.disp_cnt.hblank_interval_free = (val & 0x20) == 0x20;
            },
            DISPCNT_HI => {
                for i in 0..4 {
                    graphics.disp_cnt.bg_enabled[i] = (val & (1 << i)) > 0;
                }
                graphics.disp_cnt.window_enabled[0] = (val & 0x20) == 0x20;
                graphics.disp_cnt.window_enabled[1] = (val & 0x40) == 0x40;
                graphics.disp_cnt.obj_win_enabled = (val & 0x80) == 0x80;
            },
            DISPSTAT_LO => {
                graphics.disp_stat.vblank_irq_enabled = (val & 0x8) == 0x8;
                graphics.disp_stat.hblank_irq_enabled = (val & 0x10) == 0x10;
                graphics.disp_stat.vcount_irq_enabled = (val & 0x20) == 0x20;
            },
            DISPSTAT_HI => {
                graphics.disp_stat.vcount_line_trigger = val
            },
            BGCNT_START...BGCNT_END => {
                let bg = ((addr - BGCNT_START) / 2) as usize;
                if addr % 2 == 1 { // high byte
                    graphics.bg_cnt[bg].map_addr =
                        0x6000000 + (val as u32 & 0x1F)*0x800;
                    graphics.bg_cnt[bg].overflow = (val & 0x20) == 0x20;
                    let (width, height) = match val >> 6 { // upper 2 bits
                        0 => (256, 256),
                        1 => (512, 256),
                        2 => (256, 512),
                        3 => (512, 512),
                        _ => panic!("should not get here")
                    };
                    graphics.bg_cnt[bg].width = width;
                    graphics.bg_cnt[bg].height = height;
                } else { // low byte
                    graphics.bg_cnt[bg].priority = val & 3;
                    graphics.bg_cnt[bg].tile_addr =
                        0x6000000 + ((val >> 2) as u32 & 3)*0x4000;
                    graphics.bg_cnt[bg].mosaic_enabled = (val & 0x40) == 0x40;
                    graphics.bg_cnt[bg].depth = if val >= 8 { 8 } else { 4 };
                }
            },
            BG_OFFSET_START...BG_OFFSET_END => {
                let bg = ((addr - BG_OFFSET_START) / 4) as usize;
                if (addr & 0x20) == 0 { // horizontal coord
                    if (addr % 2) == 0 { // low byte
                        graphics.bg_offset_x[bg] &= 0xFF00;
                        graphics.bg_offset_x[bg] |= val as u16;
                    } else { // high byte
                        graphics.bg_offset_x[bg] &= 0x00FF;
                        graphics.bg_offset_x[bg] |= (val as u16 & 3) << 8;
                    }
                } else { // vertical coord
                    if (addr % 2) == 0 { // low byte
                        graphics.bg_offset_y[bg] &= 0xFF00;
                        graphics.bg_offset_y[bg] |= val as u16;
                    } else { // high byte
                        graphics.bg_offset_y[bg] &= 0x00FF;
                        graphics.bg_offset_y[bg] |= (val as u16 & 3) << 8;
                    }
                }
            },
            BG_AFFINE_START...BG_AFFINE_END => {
                let bg = ((addr - BG_AFFINE_START) / 10) as usize;
                let hw_raw = self.raw.get_halfword(addr & !1);
                let word_raw = self.raw.get_word(addr & !1);
                match addr % 16 {
                    0...1 => graphics.bg_affine[bg].dx = to_float_hw(hw_raw),
                    2...3 => graphics.bg_affine[bg].dmx = to_float_hw(hw_raw),
                    4...5 => graphics.bg_affine[bg].dy = to_float_hw(hw_raw),
                    6...7 => graphics.bg_affine[bg].dmy = to_float_hw(hw_raw),
                    8...12 => graphics.bg_affine[bg].ref_x = to_float_word(word_raw),
                    13...15 => graphics.bg_affine[bg].ref_x = to_float_word(word_raw),
                    _ => panic!("should not get here")
                }
            },
            WIN_COORD_START...WIN_COORD_END => {
                match addr - WIN_COORD_START {
                    0 => graphics.window_coords[0].right = min(val, 240),
                    1 => graphics.window_coords[0].left = val,
                    2 => graphics.window_coords[1].right = min(val, 240),
                    3 => graphics.window_coords[1].left = val,
                    4 => graphics.window_coords[0].bottom = min(val, 160),
                    5 => graphics.window_coords[0].top = val,
                    6 => graphics.window_coords[1].bottom = min(val, 160),
                    7 => graphics.window_coords[1].top = val,
                    _ => panic!("should not get here")
                }

                let bg = ((addr >> 1) & 1) as usize;
                let mut coords = &mut graphics.window_coords[bg];
                // TODO: this is done differently in GBE?
                if coords.left < coords.right {
                    coords.right = 240;
                }
                if coords.bottom < coords.top {
                    coords.bottom = 160;
                }
            },
            WIN_SETTINGS_START...WIN_SETTINGS_END => {
                let mut settings = &mut graphics.window_settings[(addr % 8) as usize];
                settings.bg[0] = (val & 1) == 1;
                settings.bg[1] = (val & 2) == 2;
                settings.bg[2] = (val & 4) == 4;
                settings.bg[3] = (val & 8) == 8;
                settings.sprite = (val & 16) == 16;
                settings.blend =  (val & 32) == 32;
            },
            MOSAIC_LO => {
                graphics.bg_mos_hsize = val & 0xF;
                graphics.bg_mos_vsize = val >> 4;
            },
            MOSAIC_HI => {
                graphics.obj_mos_hsize = val & 0xF;
                graphics.obj_mos_vsize = val >> 4;
            },
            BLDCNT_LO => {
                graphics.blend_params.source[0] = (val & 1) == 1;
                graphics.blend_params.source[1] = (val & 2) == 2;
                graphics.blend_params.source[2] = (val & 4) == 4;
                graphics.blend_params.source[3] = (val & 8) == 8;
                graphics.blend_params.source[4] = (val & 16) == 16;
                graphics.blend_params.source[5] = (val & 32) == 32;
                graphics.blend_params.mode = match val >> 6 {
                    0 => BlendType::Off,
                    1 => BlendType::AlphaBlend,
                    2 => BlendType::Lighten,
                    3 => BlendType::Darken,
                    _ => panic!("should not get here"),
                };
            },
            BLDCNT_HI => {
                graphics.blend_params.target[0] = (val & 1) == 1;
                graphics.blend_params.target[1] = (val & 2) == 2;
                graphics.blend_params.target[2] = (val & 4) == 4;
                graphics.blend_params.target[3] = (val & 8) == 8;
                graphics.blend_params.target[4] = (val & 16) == 16;
                graphics.blend_params.target[5] = (val & 32) == 32;
            },
            BLDALPHA_LO => { graphics.alpha_a_coef = to_coeff(val); },
            BLDALPHA_HI => { graphics.alpha_b_coef = to_coeff(val); },
            BLDY => { graphics.brightness_coef = to_coeff(val); },
            _ => () // unused
        }
    }

    pub fn update_graphics_hw(&mut self, addr: u32, val: u32) {
        self.update_graphics_byte(addr, val as u8);
        self.update_graphics_byte(addr + 1, (val >> 8) as u8);
    }

    pub fn update_graphics_word(&mut self, addr: u32, val: u32) {
        self.update_graphics_hw(addr, val);
        self.update_graphics_hw(addr + 1, val >> 16);
    }
}

/// Address: 0x4000000 - REG_DISPCNT (The display control register)
///                            R
/// F E D C  B A 9 8  7 6 5 4  3 2 1 0 
/// W U U S  L L L L  F D B A  C M M M 
///
/// 3   (C) = Game Boy Color mode. Read only - should stay at 0. 
/// D   (U) = Enable Window 0
/// E   (V) = Enable Window 1 
/// F   (W) = Enable Sprite Windows
struct DispCnt {
    /// 0-2 (M) = The video mode
    bg_mode: u8,
    /// 4   (A) = This bit controls the starting address of the bitmap in bitmapped modes
    ///           (mode 4 and 5) and is used for page flipping (the user can update
    ///            one of the frames while display the other, then switch)
    frame_base: u32,
    /// 5   (B) = if set, allow access to access VRAM/OAM/PAL sections of memory
    ///           during HBlank
    hblank_interval_free: bool,
    /// 6   (D) = Sets whether sprites stored in VRAM use 1 dimension or 2.
    ///           1 - 1d: tiles are are stored sequentially 
    ///           0 - 2d: each row of tiles is stored 32 x 64 bytes in from the start of the
    ///           previous row.
    // sprite_2d: bool,
    /// 7   (F) = Force the display to go blank when set. This can be used to save power 
    ///           when the display isn't needed, or to blank the screen when it is being
    ///           built up
    // force_blank: bool,
    /// 8-B (L) = enable the display of BGi
    bg_enabled: [bool; 4],
    /// C   (S) = If set, enable display of OAM (sprites). 
    // oam_enabled: bool,
    /// D-E (U) = enable the display of window i
    window_enabled: [bool; 2],
    obj_win_enabled: bool,
}

impl DispCnt {
    pub const fn new() -> DispCnt {
        DispCnt {
            bg_mode: 0,
            frame_base: 0,
            hblank_interval_free: false,
            bg_enabled: [false; 4],
            window_enabled: [false; 2],
            obj_win_enabled: false,
        }
    }
}

/// Address: 0x4000004 - REG_DISPSTAT
///                              R R R
/// F E D C  B A 9 8  7 6 5 4  3 2 1 0 
/// T T T T  T T T T  X X Y H  V Z G W 
struct DispStat {
    /// 0   (W) = V Refresh status. This will be 0 during VDraw, and 1 during VBlank. 
    ///           VDraw lasts for 160 scanlines; VBlank follows after that and lasts 68
    ///           scanlines. Checking this is one alternative to checking REG_VCOUNT. 
    vrefresh_status: bool,
    /// 1   (G) = H Refresh status. This will be 0 during HDraw, and 1 during HBlank HDraw
    ///           lasts for approximately 1004 cycles; HBlank follows, and lasts
    ///           approximately 228 cycles, though the time and length of HBlank may in
    ///           fact vary based on the number of sprites and on rotation/scaling/blending
    ///           effects being performed on the current line. 
    hrefresh_status: bool,
    /// 2   (Z) = VCount Triggered Status. Gets set to 1 when a Y trigger interrupt occurs. 
    vcount_triggered: bool,
    /// 3   (V) = Enables LCD's VBlank IRQ. This interrupt goes off at the start of VBlank. 
    vblank_irq_enabled: bool,
    /// 4   (H) = Enables LCD's HBlank IRQ. This interrupt goes off at the start of HBlank.
    hblank_irq_enabled: bool,
    /// 5   (Y) = Enable VCount trigger IRQ. Goes off when VCount line trigger is reached.
    vcount_irq_enabled: bool,
    /// 8-F (T) = Vcount line trigger. Set this to the VCount value you wish to trigger an
    ///           interrupt. 
    vcount_line_trigger: u8
}

impl DispStat {
    pub const fn new() -> DispStat {
        DispStat {
            vrefresh_status: false,
            hrefresh_status: false,
            vcount_triggered: false,
            vblank_irq_enabled: false,
            hblank_irq_enabled: false,
            vcount_irq_enabled: false,
            vcount_line_trigger: 0
        }
    }
}

/// Address: 0x400008 - 0x40001E: Background Registers
/// F E D C  B A 9 8  7 6 5 4  3 2 1 0 
/// Z Z V M  M M M M  A C X X  S S P P 
struct BgCnt {
    /// 0-1 (P) = Priority - 0 highest, 3 is the lowest
    ///           When multiple backgrounds have the same priority, the order
    ///           from front to back is:  BG0, BG1, BG2, BG3.  Sprites of the same
    ///           priority are ordered similarly, with the first sprite in OAM
    ///           appearing in front.
    priority: u8,
    /// 2-3 (S) = Starting address of character tile data
    tile_addr: u32,
    /// 6   (C) = Mosiac effect - 1 on, 0 off
    mosaic_enabled: bool,
    /// 7   (A) = Color palette type -
    ///           1 - standard 256 color pallete
    ///           0 - each tile uses one of 16 different 16 color palettes (no effect on
    ///               rotates/scale backgrounds, which are always 256 color)
    depth: u8,
    /// 8-C (M) = Starting address of character tile map
    ///           Address = 0x6000000 + M * 0x800
    map_addr: u32,
    /// D   (V) = Screen Over. Used to determine whether rotational backgrounds get tiled
    ///           repeatedly at the edges or are displayed as a single "tile" with the area
    ///           outside transparent. This is forced to 0 (read only) for backgrounds 
    ///           0 and 1 (only).
    overflow: bool,
    /// E-F (Z) = Size of tile map
    ///           For "text" backgrounds: 
    ///           00 : 256x256 (32x32 tiles) 
    ///           01 : 512x256 (64x32 tiles) 
    ///           10 : 256x512 (32x64 tiles) 
    ///           11 : 512x512 (64x64 tiles) 

    ///           For rotational backgrounds: 
    ///           00 : 128x128 (16x16 tiles) 
    ///           01 : 256x256 (32x32 tiles) 
    ///           10 : 512x512 (64x64 tiles)
    ///           11 : 1024x1024 (128x128 tiles)
    width: u16,
    height: u16,
}

impl BgCnt {
    pub const fn new() -> BgCnt {
        BgCnt {
            priority: 0,
            tile_addr: 0,
            mosaic_enabled: false,
            depth: 8,
            map_addr: 0,
            overflow: false,
            width: 0,
            height: 0,
        }
    }
}

struct BgAffineParams {
    dx: f32,
    dmx: f32,
    dy: f32,
    dmy: f32,
    ref_x: f32,
    ref_y: f32,
}

impl BgAffineParams {
    pub const fn new() -> BgAffineParams {
        BgAffineParams {
            dx: 0.0,
            dmx: 0.0,
            dy: 0.0,
            dmy: 0.0,
            ref_x: 0.0,
            ref_y: 0.0,      
        }
    }
}

/// Specifies the corners of a window. Note that the upper number is exclusive
/// and the lower number is inclusive (i.e. x in [left, right))
struct WindowCoords {
    top: u8,
    bottom: u8,
    left: u8,
    right: u8,
}

impl WindowCoords {
    pub const fn new() -> WindowCoords {
        WindowCoords {
            top: 0,
            bottom: 0,
            left: 0,
            right: 0,
        }
    }
}

struct WindowSettings {
    pub bg: [bool; 4],
    pub sprite: bool,
    pub blend: bool,
}

impl WindowSettings {
    pub const fn new() -> WindowSettings {
        WindowSettings {
            bg: [false; 4],
            sprite: false,
            blend: false
        }
    }
}

struct BlendParams {
    // bg0-bg3, sprite, backdrop
    pub source: [bool; 6],
    pub mode: BlendType,
    // bg0-bg3, sprite, backdrop
    pub target: [bool; 6]
}

impl BlendParams {
    pub const fn new() -> BlendParams {
        BlendParams {
            source: [false; 6],
            mode: BlendType::Off,
            target: [false; 6]
        }
    }
}

enum BlendType {
    Off,
    AlphaBlend,
    Lighten,
    Darken,
}

/// parse the following format into a float:
/// F E D C  B A 9 8  7 6 5 4  3 2 1 0 
/// S I I I  I I I I  F F F F  F F F F 
/// 0-7 (F) = Fraction 
/// 8-E (I) = Integer 
/// F   (S) = Sign bit 
fn to_float_hw(raw: u16) -> f32 {
    let int = (raw >> 8) as i8 as f32;
    let frac = ((raw & 0xFF) as f32) / 256.0;
    int + frac
}

/// parse the following format into a float:
/// 27 26 25 24  23 22 21 20  19 18 17 16  15 14 13 12  11 10 9 8  7 6 5 4  3 2 1 0
/// S  I  I  I   I  I  I  I   I  I  I  I   I  I  I  I   I  I  I I  F F F F  F F F F 
/// 0-7  (F) - Fraction 
/// 8-26 (I) - Integer 
/// 27   (S) - Sign bit 
fn to_float_word(raw: u32) -> f32 {
    let mut int = (raw >> 8) & 0x7FFFF; // set I bits
    int |= (raw << 4) & 0x80000000; // set sign bit
    let frac = ((raw & 0xFF) as f32) / 256.0;
    (int as i32 as f32) + frac
}

/// takes a 5 bit value and parses it as an effect coefficent
fn to_coeff(raw: u8) -> f32 {
    (min(raw, 16) as f32) / 16.0
}
