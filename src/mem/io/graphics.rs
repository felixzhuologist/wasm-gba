use util;

pub struct GraphicsIO {
    disp_cnt: DispCnt,
    disp_stat: DispStat,
    /// Stores the current Y location of the current line being drawn
    vcount: u8,
    bg_cnt: [BgCnt; 4],
    bg_offset_x: [u16; 4],
    bg_offset_y: [u16; 4],
    bg_affine: [BgAffineParams; 2],
    window_x: [u16; 2],
    window_y: [u16; 2],
    /// inside of window 0 and 1
    win_in: [WindowSettings; 2],
    /// outside window and sprite window
    win_out: [WindowSettings; 2],

    bg_mos_hsize: u8,
    bg_mos_vsize: u8,
    obj_mos_hsize: u8,
    obj_mos_vsize: u8,
    blend_params: BlendParams,

    alpha_a_coef: i16,
    alpha_b_coef: i16,
    brightness_coef: i16,
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
            window_x: [0; 2],
            window_y: [0; 2],
            win_in: [
                WindowSettings::new(),
                WindowSettings::new(),
            ],
            win_out: [
                WindowSettings::new(),
                WindowSettings::new(),
            ],
            bg_mos_hsize: 0,
            bg_mos_vsize: 0,
            obj_mos_hsize: 0,
            obj_mos_vsize: 0,
            blend_params: BlendParams::new(),
            alpha_a_coef: 0,
            alpha_b_coef: 0,
            brightness_coef: 0,
        }
    }

    pub fn set_byte(&mut self, addr: u32, val: u8) {
        unimplemented!()
    }

    pub fn set_halfword(&mut self, addr: u32, val: u32) {
        self.set_byte(addr, val as u8);
        self.set_byte(addr + 1, (val >> 8) as u8);
    }

    pub fn set_word(&mut self, addr: u32, val: u32) {
        self.set_halfword(addr, val);
        self.set_halfword(addr + 1, (val >> 16));
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
    ///           and is used for page flipping. See the description of the specific
    ///           video mode for details.
    frame_base: u32,
    /// 5   (B) = Force processing during hblank. Setting this causes the display
    ///           controller to process data earlier and longer, beginning from the end of
    ///           the previous scanline up to the end of the current one. This added
    ///           processing time can help prevent flickering when there are too many
    ///           sprites on a scanline.
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
    /// 0-1 (P) = Priority - 00 highest, 11 lowest
    ///           Priorities are ordered as follows:

    ///           "Front"
    ///           1. Sprite with priority 0
    ///           2. BG with     priority 0

    ///           3. Sprite with priority 1
    ///           4. BG with     priority 1

    ///           5. Sprite with priority 2
    ///           6. BG with     priority 2

    ///           7. Sprite with priority 3
    ///           8. BG with     priority 3

    ///           9. Backdrop
    ///           "Back"

    ///           When multiple backgrounds have the same priority, the order
    ///           from front to back is:  BG0, BG1, BG2, BG3.  Sprites of the same
    ///           priority are ordered similarly, with the first sprite in OAM
    ///           appearing in front.
    priority: u8,
    /// 2-3 (S) = Starting address of character tile data
    ///           Address = 0x6000000 + S * 0x4000
    tile_addr: u32,
    /// 6   (C) = Mosiac effect - 1 on, 0 off
    mosaic_enabled: bool,
    /// 7   (A) = Color palette type -
    ///           1 - standard 256 color pallete
    ///           0 - each tile uses one of 16 different 16 color palettes (no effect on
    ///               rotates/scale backgrounds, which are always 256 color)
    full_depth: bool,
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
            full_depth: false,
            map_addr: 0,
            overflow: false,
            width: 0,
            height: 0,
        }
    }
}

struct BgAffineParams {
    pub dx: i16,
    pub dmx: i16,
    pub dy: i16,
    pub dmy: i16,
    pub x_ref: i16,
    pub y_ref: i16,
}

impl BgAffineParams {
    pub const fn new() -> BgAffineParams {
        BgAffineParams {
            dx: 0,
            dmx: 0,
            dy: 0,
            dmy: 0,
            x_ref: 0,
            y_ref: 0,      
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
            mode: BlendType::Normal,
            target: [false; 6]
        }
    }
}

enum BlendType {
    Normal,
    AlphaBlend,
    BrightnessUp,
    BrightnessDown,
}
