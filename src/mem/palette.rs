use mem::Memory;
use mem::addrs::PAL_START;

// We need to convert to 32 bit RGBA pixel values to be able to use the
// drawImage() API. If we eventually use webGL where we can define a texture
// using 16 bit pixel values directly, this should become a thin wrapper over
// raw pal memory
/// Stores 32 bit RGBA versions of the colors in raw memory.
pub struct Palette {
    pub bg: [u32; 256],
    pub sprite: [u32; 256],
}

impl Palette {
    pub const fn new() -> Palette {
        Palette {
            bg: [0; 256],
            sprite: [0; 256],
        }
    }
}

impl Memory {
    pub fn update_pal_byte(&mut self, addr: u32, _val: u8) {
        let arr = if addr <= 0x50001FF
            { &mut self.palette.bg } else
            { &mut self.palette.sprite };

        let high_color = self.raw.get_halfword(addr & !1);
        let offset = addr - PAL_START;
        let idx = (offset / 2) % 256;
        arr[idx as usize] = high_to_true(high_color);
    }

    pub fn update_pal_hw(&mut self, addr: u32, val: u32) {
        self.update_pal_byte(addr, val as u8);
        self.update_pal_byte(addr + 1, (val >> 8) as u8);
    }

    pub fn update_pal_word(&mut self, addr: u32, val: u32) {
        self.update_pal_hw(addr, val);
        self.update_pal_hw(addr + 2, val >> 16);
    }
}

/// convert 15 bit RGB to 32 bit RGBA
fn high_to_true(color: u16) -> u32 {
    let color = color as u32;
    let red = color & 0x1F;
    let green = (color >> 5) & 0x1F;
    let blue = (color >> 10) & 0x1F;
    // move 5 bits into the higher 5 of the 8 bits for each color, hence an extra
    // left by 3
    0xFF000000 | (red << 19) | (green << 11) | blue << 3
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn write() {
        let mut mem = Memory::new();

        mem.set_halfword(0x5000000, 15);
        assert_eq!(mem.palette.bg[0], high_to_true(15));
        mem.set_halfword(0x500000A, 20);
        assert_eq!(mem.palette.bg[5], high_to_true(20));
        mem.set_halfword(0x50001FE, 1234);
        assert_eq!(mem.palette.bg[255], high_to_true(1234));
        mem.set_halfword(0x5000202, 5432);
        assert_eq!(mem.palette.sprite[1], high_to_true(5432));
        mem.set_halfword(0x50003FE, 21);
        assert_eq!(mem.palette.sprite[255], high_to_true(21));
    }

    #[test]
    fn color_conversion() {
        assert_eq!(
            high_to_true(0b0_10110_01110_10001),
            0b11111111_10001_000_01110_000_10110_000);
    }
}