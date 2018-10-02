mod addrs;
pub mod io;

use util;
use mem::io::addrs::*;
use self::addrs::*;

pub struct Memory {
    pub raw: RawMemory,
    // these are parsed versions of raw data stored in memory that must be updated
    // on write so that the values are in sync with the actual raw data
    pub graphics: io::graphics::LCD,
    pub dma: io::dma::DMA,
    pub int: io::interrupt::Interrupt
}

impl Memory {
    pub const fn new() -> Memory {
        Memory {
            raw: RawMemory::new(),
            graphics: io::graphics::LCD::new(),
            dma: io::dma::DMA::new(),
            int: io::interrupt::Interrupt::new(),
        }
    }

    pub fn get_byte(&self, addr: u32) -> u8 {
        self.raw.get_byte(addr)
    }

    pub fn get_halfword(&self, addr: u32) -> u16 {
        self.raw.get_halfword(addr)
    }

    pub fn get_word(&self, addr: u32) -> u32 {
        self.raw.get_word(addr)
    }

    pub fn set_byte(&mut self, addr: u32, val: u8) {
        self.raw.set_byte(addr, val);

        match addr {
            GRAPHICS_START...GRAPHICS_END =>
                self.update_graphics_byte(addr, val),
            DMA_START...DMA_END =>
                self.update_dma_byte(addr, val),
            INT_START...INT_END =>
                self.update_int_byte(addr, val),
            _ => ()
        }
    }

    pub fn set_halfword(&mut self, addr: u32, val: u32) {
        self.raw.set_halfword(addr, val);

        match addr {
            GRAPHICS_START...GRAPHICS_END =>
                self.update_graphics_hw(addr, val),
            DMA_START...DMA_END =>
                self.update_dma_hw(addr, val),
            INT_START...INT_END =>
                self.update_int_hw(addr, val),
            _ => ()
        }
    }

    pub fn set_word(&mut self, addr: u32, val: u32) {
        self.raw.set_word(addr, val);

        match addr {
            GRAPHICS_START...GRAPHICS_END =>
                self.update_graphics_word(addr, val),
            DMA_START...DMA_END =>
                self.update_dma_word(addr, val),
            INT_START...INT_END =>
                self.update_int_word(addr, val),
            _ => ()
        }
    }

    pub fn load_rom(&mut self, data: &[u8]) {
        for i in 0..self.raw.sysrom.len() {
            self.raw.sysrom[i] = data[i];
        }
    }
}

pub struct RawMemory {
    /// contains the BIOS
    pub sysrom: [u8; 0x3FFF],
    /// space for game data/code; largest area of RAM but memory transfers are
    /// 16 bit wide which makes it slower than iwram
    ewram: [u8; 0x3FFFF],
    /// fastest RAM segment which is internally embedded in the CPU chip package
    /// with a 32 bit bus
    iwram: [u8; 0x7FFF],
    /// a mirror of the memory mapped ASIC registers on the GBA used to control
    /// graphics, sound, DMA, timers, etc.
    io: [u8; 0x3FF],
    /// specifies 16 bit color values for the paletted modes
    pal: [u8; 0x3FF],
    /// stores the frame buffer in bitmapped modes or the tile data/tile maps
    /// in text, rotate/scale modes
    vram: [u8; 0x17FFF],
    /// stores the objects/sprites. PAL/VRAM/OAM segments of memory are only
    /// accessible during HBlank/VBlank periods (i.e. when not drawing)
    oam: [u8; 0x3FF],
    // ROM in the game cartridge appears in this area
    // TODO: allocate on the javascript side?
    // pak: Vec<u8>,
    // either SRAM or flash ROM used for saving game data
    // TODO: allocate on the javascript side?
    // cart: Vec<u8>,
}

impl RawMemory {
    pub const fn new() -> RawMemory {
        RawMemory {
            sysrom: [0; 0x3FFF],
            ewram: [0; 0x3FFFF],
            iwram: [0; 0x7FFF],
            io: [0; 0x3FF],
            pal: [0; 0x3FF],
            vram: [0; 0x17FFF],
            oam: [0; 0x3FF],
            // pak: Vec::new(),
            // cart: Vec::new(),
        }
    }

    /// given an absolute address into memory, convert it to a reference to
    /// one of the memory segments and an index into that segment
    pub fn get_loc(&self, addr: u32) -> (&[u8], usize) {
        // TODO: use addr / 0x01000000 instead of a match statement?
        let result: (&[u8], u32) = match addr {
            SYSROM_START...SYSROM_END => (&self.sysrom, addr),
            EWRAM_START...EWRAM_END => (&self.ewram, addr - EWRAM_START),
            IWRAM_START...IWRAM_END => (&self.iwram, addr - IWRAM_START),
            IO_START...IO_END => (&self.io, addr - IO_START),
            PAL_START...PAL_END => (&self.pal, addr - PAL_START),
            VRAM_START...VRAM_END => (&self.vram, addr - VRAM_START),
            OAM_START...OAM_END => (&self.oam, addr - OAM_START),
            // TODO: ROM data
            0x08000000...0x09FFFFFF => unimplemented!(),
            0x0A000000...0x0BFFFFFF => unimplemented!(),
            0x0C000000...0x0DFFFFFF => unimplemented!(),
            0x0E000000...0x0E00FFFF => unimplemented!(),
            _ => panic!("accessing unused memory")
        };
        (result.0, result.1 as usize)
    }

    pub fn get_loc_mut(&mut self, addr: u32) -> (&mut [u8], usize) {
        // TODO: use addr / 0x01000000 instead of a match statement?
        let result: (&mut [u8], u32) = match addr {
            SYSROM_START...SYSROM_END => (&mut self.sysrom, addr),
            EWRAM_START...EWRAM_END => (&mut self.ewram, addr - EWRAM_START),
            IWRAM_START...IWRAM_END => (&mut self.iwram, addr - IWRAM_START),
            IO_START...IO_END => (&mut self.io, addr - IO_START),
            PAL_START...PAL_END => (&mut self.pal, addr - PAL_START),
            VRAM_START...VRAM_END => (&mut self.vram, addr - VRAM_START),
            OAM_START...OAM_END => (&mut self.oam, addr - OAM_START),
            // TODO: ROM data
            0x08000000...0x09FFFFFF => unimplemented!(),
            0x0A000000...0x0BFFFFFF => unimplemented!(),
            0x0C000000...0x0DFFFFFF => unimplemented!(),
            0x0E000000...0x0E00FFFF => unimplemented!(),
            _ => panic!("accessing unused memory")
        };
        (result.0, result.1 as usize)
    }

    pub fn get_byte(&self, addr: u32) -> u8 {
        let (segment, idx) = self.get_loc(addr);
        segment[idx]
    }

    pub fn get_halfword(&self, addr: u32) -> u16 {
        let (segment, idx) = self.get_loc(addr);
        segment[idx] as u16 | (segment[idx + 1] as u16) << 8
    }

    pub fn get_word(&self, addr: u32) -> u32 {
        let (segment, idx) = self.get_loc(addr);
        segment[idx] as u32 |
            (segment[idx + 1] as u32) << 8 |
            (segment[idx + 2] as u32) << 16 |
            (segment[idx + 3] as u32) << 24
    }

    pub fn set_byte(&mut self, addr: u32, val: u8) {
        let (segment, idx) = self.get_loc_mut(addr);
        segment[idx] = val;
    }

    pub fn set_halfword(&mut self, addr: u32, val: u32) {
        let (segment, idx) = self.get_loc_mut(addr);
        segment[idx] = util::get_byte(val, 0) as u8;
        segment[idx + 1] = util::get_byte(val, 8) as u8;
    }

    pub fn set_word(&mut self, addr: u32, val: u32) {
        let (segment, idx) = self.get_loc_mut(addr);
        segment[idx] = util::get_byte(val, 0) as u8;
        segment[idx + 1] = util::get_byte(val, 8) as u8;
        segment[idx + 2] = util::get_byte(val, 16) as u8;
        segment[idx + 3] = util::get_byte(val, 24) as u8;
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn get_byte() {
        let mut mem = RawMemory::new();
        mem.sysrom[0x2FF] = 10;
        assert_eq!(mem.get_byte(0x2FF), 10);
        mem.ewram[2] = 22;
        assert_eq!(mem.get_byte(0x02000002), 22);
        mem.iwram[0x700] = 19;
        assert_eq!(mem.get_byte(0x03000700), 19);
        mem.io[0] = 17;
        assert_eq!(mem.get_byte(0x04000000), 17);
        mem.pal[17] = 1;
        assert_eq!(mem.get_byte(0x05000011), 1);
        mem.vram[0] = 2;
        assert_eq!(mem.get_byte(0x06000000), 2);
        mem.oam[0x3FE] = 30;
        assert_eq!(mem.get_byte(0x070003FE), 30);
    }

    #[test]
    fn endianness() {
        let mut mem = RawMemory::new();
        mem.sysrom[0] = 1;
        mem.sysrom[1] = 2;
        mem.sysrom[2] = 3;
        mem.sysrom[3] = 4;
        assert_eq!(mem.get_word(0), 0x04030201);
    }

    #[test]
    fn get_set() {
        let mut mem = RawMemory::new();
        mem.set_word(0x123, 0xABC001);
        assert_eq!(mem.get_word(0x123), 0xABC001);
    }
}
