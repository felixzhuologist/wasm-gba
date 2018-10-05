/// Stores all sprite data (a parsed version of OAM). OAM consists of 128 entries
/// of 8 bytes, hence 128 max possible sprites. Each 8 byte entry is divided into
/// 4 16 bit attributes, numbered 0 - 3. The attribute 3 of 4 consecutive
/// sprite entries (0-3, 4-7, etc.) correspond to affine parameters A, B, C, D
/// (dx, dmx, dy, dmy) for a single affine sprite, hence there can be at most
/// 32 affine sprites. Which affine sprite does a group of affine parameters
/// belong to? That's indicated by the affine_group field on a Sprite, which
/// is an index into affine_params
pub struct Sprites {
    sprites: [Sprite; 128],
    affine_params: [SpriteAffineParams; 32],
}

impl Sprites {
    pub const fn new() -> Sprites {
        Sprites {
            sprites: [Sprite::new(); 128],
            affine_params: [SpriteAffineParams::new(); 32],
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Sprite {
    /// the x coordinate: for regular sprites, this is the upper left corner
    /// and for affine sprites this is the cneter
    x: u16,
    /// the y coordinate: for regular sprites, this is the upper left corner
    /// and for affine sprites this is the cneter
    y: u8,
    width: u8,
    height: u8,

    /// Indicates whether we are using a full 256 color palette, or 16 subpalettes
    bit_depth: u8,
    /// only valid when bit depth is 4: index of the sub palette
    palette_number: u8,

    /// specifies the OAM_AFF_ENTY this sprite uses. only valid for affine sprites
    affine_group: u8,
    /// for affine sprites, this being set means that this sprite uses double
    /// the rendering area. for normal sprites, this hides the sprite
    /// defines what kind of sprite this is
    mode: SpriteType,

    /// flip the entire sprite vertically. only valid for regular sprites
    vflip: bool,
    /// flip the entire sprite horizontally. only valid for regular sprites
    hflip: bool,

    /// higher priorities get drawn first; sprites cover backgrounds of the same
    /// priority and when sprites have the same priority, higher in the OAM gets
    /// drawn first
    priority: u8,
    /// base tile index of the sprite
    tile_number: u16,

    // TODO: implement effects
    // gfx_mode: GfxMode,
    // mosaic_enabled: bool,
}

impl Sprite {
    pub const fn new() -> Sprite {
        Sprite {
            x: 0,
            y: 0,
            width: 0,
            height: 0,
            bit_depth: 0,
            palette_number: 0,
            mode: SpriteType::Normal,
            affine_group: 0,
            vflip: false,
            hflip: false,
            priority: 0,
            tile_number: 0,
        }
    }
}

#[derive(Copy, Clone, Debug)]
struct SpriteAffineParams {
    dx: f32,
    dmx: f32,
    dy: f32,
    dmy: f32,
}

impl SpriteAffineParams {
    pub const fn new() -> SpriteAffineParams {
        SpriteAffineParams {
            dx: 0.0,
            dmx: 0.0,
            dy: 0.0,
            dmy: 0.0
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpriteType {
    Normal = 0,
    Affine,
    Disabled,
    DoubleAffine
}

impl SpriteType {
    pub fn is_affine(&self) -> bool {
        match *self {
            SpriteType::Affine |
            SpriteType::DoubleAffine => false,
            _ => true
        }
    }
}
