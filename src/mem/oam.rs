use num::FromPrimitive;
use mem::Memory;
use mem::addrs::OAM_START;
use util;

pub const BYTES_PER_OAM_ENTRY: u32 = 8;
pub const BYTES_PER_AFFINE_GROUP: u32 = 32;
pub const NUM_SPRITES: usize = 128;
pub const NUM_AFFFINE_SPRITES: usize = 32;

/// Stores all sprite data (a parsed version of OAM). OAM consists of 128 entries
/// of 8 bytes, hence 128 max possible sprites. Each 8 byte entry is divided into
/// 4 16 bit attributes, numbered 0 - 3. The attribute 3 of 4 consecutive
/// sprite entries (0-3, 4-7, etc.) correspond to affine parameters A, B, C, D
/// (dx, dmx, dy, dmy) for a single affine sprite, hence there can be at most
/// 32 affine sprites. Which affine sprite does a group of affine parameters
/// belong to? That's indicated by the affine_group field on a Sprite, which
/// is an index into affine_params
pub struct Sprites {
    sprites: [Sprite; NUM_SPRITES],
    affine_params: [SpriteAffineParams; NUM_AFFFINE_SPRITES],
}

impl Memory {
    pub fn update_oam_byte(&mut self, addr: u32, val: u8) {
        let sprite_num = (addr - OAM_START) / BYTES_PER_OAM_ENTRY;
        let sprite = &mut self.sprites.sprites[sprite_num as usize];
        match addr % BYTES_PER_OAM_ENTRY {
            // attribute 0 (lo)
            0 => { sprite.y = val; },
            // attribute 0 (hi)
            // F E D C  B A 9 8
            // S S A M  T T D D
            // 8-9 (D) = sprite type/mode
            // A-B (T) = gfx mode
            // C   (M) = enables mosaic for this sprite.
            // D   (A) = 256 color if on, 16 color if off
            // E-F (S) = shape
            1 => {
                sprite.mode = SpriteType::from_u8(val & 0b11).unwrap();
                sprite.bit_depth = if (val & 0x20) == 0x20 { 8 } else { 4 };
                sprite.shape = (val >> 6) & 0b11;
            },
            // attribute 1:
            // F E D C  B A 9 8  7 6 5 4  3 2 1 0
            // S S V H  X X X I  I I I I  I I I I  (standard sprites)
            // S S F F  F F F I  I I I I  I I I I  (rotation/scaling on)
            // 0-8 (I) = X coordinate of the sprite (pixels)
            // C   (H) = flip horizontal
            // D   (V) = flip vertical
            // 9-D (F) = affine param index
            // E-F (S) = sprite size
            // TODO: bytes 2 and 3 share attributes so we need to update them
            // together... this means this can get run twice with the same values
            2...3 => {
                let attr1 = self.raw.get_halfword(addr & !1);
                sprite.x = attr1 & 0x1FF;
                sprite.hflip = util::get_bit_hw(attr1, 12);
                sprite.vflip = util::get_bit_hw(attr1, 13);
                sprite.size = (attr1 >> 14) as u8;
                sprite.affine_group = ((attr1 >> 9) & 0b11111) as u8;
            },
            // attribute 2:
            // F E D C  B A 9 8  7 6 5 4  3 2 1 0
            // L L L L  P P T T  T T T T  T T T T
            // 0-9 (T) = tile address is 0x6010000 + T*32.
            // A-B (P) = priority
            // C-F (L) = palette number
            4...5 => {
                let attr2 = self.raw.get_halfword(addr & !1);
                sprite.tile_number = attr2 & 0x3FF;
                sprite.priority = ((attr2 >> 10) & 0b11) as u8;
                sprite.palette_number = ((attr2 >> 12) & 0xF) as u8;
            },
            6...7 => {
                let attr3 = self.raw.get_halfword(addr & !1);
                let affine_group = (addr - OAM_START) / BYTES_PER_AFFINE_GROUP;
                let params = &mut self.sprites.affine_params[affine_group as usize];
                match addr % BYTES_PER_AFFINE_GROUP {
                    0...7 => params.dx = util::to_float_hw(attr3),
                    8...15 => params.dmx = util::to_float_hw(attr3),
                    16...23 => params.dy = util::to_float_hw(attr3),
                    24...31 => params.dmy = util::to_float_hw(attr3),
                    _ => panic!("should not get here"),
                }
            },
            _ => panic!("should not get here"),
        }
    }

    pub fn update_oam_hw(&mut self, addr: u32, val: u32) {
        self.update_oam_byte(addr, val as u8);
        self.update_oam_byte(addr + 1, (val >> 8) as u8);
    }

    pub fn update_oam_word(&mut self, addr: u32, val: u32) {
        self.update_oam_hw(addr, val);
        self.update_oam_hw(addr + 2, val >> 16);
    }
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
    /// the shape and size together determine the dimensions of the sprite
    /// they are both 2 bit values
    shape: u8,
    size: u8,

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
            shape: 0,
            size: 0,
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

    // TODO: store width/height as attributes?
    /// return width and height of this sprite
    pub fn dimensions(&self) -> (u8, u8) {
        match (self.shape, self.size) {
            (0, 0) => (8, 8),
            (0, 1) => (16, 16),
            (0, 2) => (32, 32),
            (0, 3) => (64, 64),
            (1, 0) => (16, 8),
            (1, 1) => (32, 8),
            (1, 2) => (32, 16),
            (1, 3) => (64, 32),
            (2, 0) => (8, 16),
            (2, 1) => (8, 32),
            (2, 2) => (16, 32),
            (2, 3) => (32, 64),
            _ => panic!("invalid shape/size combo")
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

enum_from_primitive! {
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SpriteType {
    Normal = 0,
    Affine,
    Disabled,
    DoubleAffine
}
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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn write() {
        let mut mem = Memory::new();

        mem.set_halfword(0x7000000, 0b1001_0010_0000_1000);
        mem.set_halfword(0x7000002, 0b1111_1110_1100_1010);
        mem.set_halfword(0x7000004, 0b0101_0110_0010_1111);
        {
            let sprite = &mem.sprites.sprites[0];
            assert_eq!(sprite.y, 0x08);
            assert_eq!(sprite.x, 0b0_1100_1010);
            assert_eq!(sprite.mode, SpriteType::Disabled);
            assert_eq!(sprite.shape, 2);
            assert_eq!(sprite.hflip, true);
            assert_eq!(sprite.vflip, true);
            assert_eq!(sprite.size, 3);
            assert_eq!(sprite.affine_group, 31);
            assert_eq!(sprite.tile_number, 0b10_0010_1111);
            assert_eq!(sprite.priority, 1);
            assert_eq!(sprite.palette_number, 0b0101);
            assert_eq!(sprite.dimensions(), (32, 64));
        }

        mem.set_halfword(0x70003F8, 0b0001_0001_1000_1001);
        mem.set_halfword(0x70003FA, 0b0100_1101_1101_1000);
        mem.set_halfword(0x70003FC, 0b1100_0011_0001_0001);
        {
            let sprite = &mem.sprites.sprites[127];
            assert_eq!(sprite.y, 0b1000_1001);
            assert_eq!(sprite.x, 0b1_1101_1000);
            assert_eq!(sprite.mode, SpriteType::Affine);
            assert_eq!(sprite.shape, 0);
            assert_eq!(sprite.hflip, false);
            assert_eq!(sprite.vflip, false);
            assert_eq!(sprite.size, 1);
            assert_eq!(sprite.affine_group, 0b110);
            assert_eq!(sprite.tile_number, 0b11_0001_0001);
            assert_eq!(sprite.priority, 0);
            assert_eq!(sprite.palette_number, 0b1100);
            assert_eq!(sprite.dimensions(), (16, 16));
        }

        mem.set_halfword(0x70003E6, 0x0A00);
        mem.set_halfword(0x70003EE, 0xFF00);
        mem.set_halfword(0x70003F6, 0x0180);
        mem.set_halfword(0x70003FE, 0x0100);
        {
            let params = &mem.sprites.affine_params[31];
            assert_eq!(params.dx, 10.0);
            assert_eq!(params.dmx, -1.0);
            assert_eq!(params.dy, 1.5);
            assert_eq!(params.dmy, 1.0);
        }
    }
}
