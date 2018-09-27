use util;

pub struct Memory {
    sysrom: [u8; 0x3FF],
    ewram: [u8; 0x3FF],
    iwram: [u8; 0x7FF],
    io: [u8; 0x3FE],
    pal: [u8; 0x3FF],
    vram: [u8; 0x17FFF],
    oam: [u8; 0x3FF],
    // ROM in the game cartridge appears in this area
    // TODO: allocate on the javascript side?
    // pak: Vec<u8>,
    // either SRAM or flash ROM used for saving game data
    // TODO: allocate on the javascript side?
    // cart: Vec<u8>,
}

impl Memory {
    pub const fn new() -> Memory {
        Memory {
            sysrom: [0; 0x3FF],
            ewram: [0; 0x3FF],
            iwram: [0; 0x7FF],
            io: [0; 0x3FE],
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
            0x00000000...0x00003FFF => (&self.sysrom, addr),
            0x02000000...0x0203FFFF => (&self.ewram, addr - 0x02000000),
            0x03000000...0x03007FFF => (&self.iwram, addr - 0x03000000),
            0x04000000...0x040003FF => (&self.io, addr - 0x04000000),
            0x05000000...0x050003FF => (&self.pal, addr - 0x05000000),
            0x06000000...0x06017FFF => (&self.vram, addr - 0x06000000),
            0x07000000...0x070003FF => (&self.oam, addr - 0x07000000),
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
            0x00000000...0x00003FFF => (&mut self.sysrom, addr),
            0x02000000...0x0203FFFF => (&mut self.ewram, addr - 0x02000000),
            0x03000000...0x03007FFF => (&mut self.iwram, addr - 0x03000000),
            0x04000000...0x040003FF => (&mut self.io, addr - 0x04000000),
            0x05000000...0x050003FF => (&mut self.pal, addr - 0x05000000),
            0x06000000...0x06017FFF => (&mut self.vram, addr - 0x06000000),
            0x07000000...0x070003FF => (&mut self.oam, addr - 0x07000000),
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
        let mut mem = Memory::new();
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
        let mut mem = Memory::new();
        mem.sysrom[0] = 1;
        mem.sysrom[1] = 2;
        mem.sysrom[2] = 3;
        mem.sysrom[3] = 4;
        assert_eq!(mem.get_word(0), 0x04030201);
    }

    #[test]
    fn get_set() {
        let mut mem = Memory::new();
        mem.set_word(0x123, 0xABC001);
        assert_eq!(mem.get_word(0x123), 0xABC001);
    }
}