mod addrs;
mod framebuffer;
mod palette;
pub mod io;
pub mod oam;

use std;
use util;
use mem::io::addrs::*;
use mem::io::dma::TimingMode;
use self::addrs::*;

pub struct Memory {
    pub raw: RawMemory,
    // these are parsed versions of raw data stored in memory that must be updated
    // on write so that the values are in sync with the actual raw data
    pub graphics: io::graphics::LCD,
    pub dma: io::dma::DMA,
    pub int: io::interrupt::Interrupt,
    pub sprites: oam::Sprites,
    pub palette: palette::Palette,

    // waitstates for reading from ROM, can be configured by writing to REG_WSCNT
    /// waitstates for a non sequential read from ROM
    rom_n_cycle: u8,
    /// if true, sequential reads from ROM are fast and otherwise they are slow.
    /// fast will always be 1 cycle but the number of cycles for a slow sequential
    /// read depends on which mirror data is being read from
    rom_s_cycle_fast: bool,

    pub framebuffer: framebuffer::FrameBuffer,
}

impl Memory {
    pub const fn new() -> Memory {
        Memory {
            raw: RawMemory::new(),
            graphics: io::graphics::LCD::new(),
            dma: io::dma::DMA::new(),
            int: io::interrupt::Interrupt::new(),
            sprites: oam::Sprites::new(),
            palette: palette::Palette::new(),
            rom_n_cycle: 4,
            rom_s_cycle_fast: false,
            framebuffer: framebuffer::FrameBuffer::new(),
        }
    }

    pub fn get_byte(&self, addr: u32) -> u8 {
        let addr = canonicalize_addr(addr);
        self.raw.get_byte(addr)
    }

    pub fn get_halfword(&self, addr: u32) -> u16 {
        let addr = canonicalize_addr(addr);
        self.raw.get_halfword(addr)
    }

    pub fn get_word(&self, addr: u32) -> u32 {
        let addr = canonicalize_addr(addr);
        self.raw.get_word(addr)
    }

    pub fn set_byte(&mut self, addr: u32, val: u8) {
        let addr = canonicalize_addr(addr);
        self.raw.set_byte(addr, val);

        match addr {
            GRAPHICS_START...GRAPHICS_END =>
                self.update_graphics_byte(addr, val),
            DMA_START...DMA_END =>
                self.update_dma_byte(addr, val),
            INT_START...INT_END =>
                self.update_int_byte(addr, val),
            OAM_START...OAM_END =>
                self.update_oam_byte(addr, val),
            PAL_START...PAL_END =>
                self.update_pal_byte(addr, val),
            _ => ()
        }
    }

    // how should boundaries be handled? e.g. if start of a mapped segment is
    // at addr 100 and we write word to addr 98, should still update that
    // mapped segment?

    pub fn set_halfword(&mut self, addr: u32, val: u32) {
        let addr = canonicalize_addr(addr);
        self.raw.set_halfword(addr, val);

        match addr {
            GRAPHICS_START...GRAPHICS_END =>
                self.update_graphics_hw(addr, val),
            DMA_START...DMA_END =>
                self.update_dma_hw(addr, val),
            INT_START...INT_END =>
                self.update_int_hw(addr, val),
            OAM_START...OAM_END =>
                self.update_oam_hw(addr, val),
            PAL_START...PAL_END =>
                self.update_pal_hw(addr, val),
            _ => ()
        }
    }

    pub fn set_word(&mut self, addr: u32, val: u32) {
        let addr = canonicalize_addr(addr);
        self.raw.set_word(addr, val);

        match addr {
            GRAPHICS_START...GRAPHICS_END =>
                self.update_graphics_word(addr, val),
            DMA_START...DMA_END =>
                self.update_dma_word(addr, val),
            INT_START...INT_END =>
                self.update_int_word(addr, val),
            OAM_START...OAM_END =>
                self.update_oam_word(addr, val),
            PAL_START...PAL_END =>
                self.update_pal_word(addr, val),
            _ => ()
        }
    }

    pub fn on_vdraw_hook(&mut self) {
        self.graphics.disp_stat.is_vblank = false;
        self.raw.io[(DISPSTAT_LO - IO_START) as usize] &= !1;
    }

    pub fn on_vblank_hook(&mut self) {
        self.graphics.disp_stat.is_vblank = true;
        self.graphics.disp_stat.is_hblank = false;
        self.raw.io[(DISPSTAT_LO - IO_START) as usize] &= !3;
        self.raw.io[(DISPSTAT_LO - IO_START) as usize] |= 1;
        if self.graphics.disp_stat.vblank_irq_enabled {
            self.int.triggered.vblank = true;
            self.raw.io[(IF_LO - IO_START) as usize] |= 1;
        }
        self.check_dma(TimingMode::VBlank);
    }

    pub fn on_hdraw_hook(&mut self) {
        self.graphics.disp_stat.is_hblank = false;
        self.raw.io[(DISPSTAT_LO - IO_START) as usize] &= !2;
    }

    pub fn on_hblank_hook(&mut self) {
        self.graphics.disp_stat.is_hblank = true;
        self.raw.io[(DISPSTAT_LO - IO_START) as usize] |= 2;
        if self.graphics.disp_stat.hblank_irq_enabled {
            self.int.triggered.hblank = true;
            self.raw.io[(IF_LO  - IO_START) as usize] |= 0b10;
        }
        self.check_dma(TimingMode::HBlank);
    }

    pub fn on_vcount_hook(&mut self, vcount: u8) {
        self.graphics.update_vcount(vcount);
        self.raw.io[(VCOUNT_LO - IO_START) as usize] = vcount;
        if self.graphics.disp_stat.vcount_triggered &&
            self.graphics.disp_stat.vcount_irq_enabled {
            self.int.triggered.vcount = true;
            self.raw.io[(IF_LO  - IO_START) as usize] |= 0b100;
        }
    }

    pub fn on_dma_finish_hook(&mut self, channel: usize) {
        if self.dma.channels[channel].irq {
            self.int.triggered.dma[channel] = true;
            self.raw.io[(IF_HI - IO_START) as usize] |= 1 << channel;
        }
    }

    /// Return the number of cycles required to perform a memory access to given
    /// addr. If first access is true, assumes a non sequential access (N cycle),
    /// otherwise assumes a sequential access (S cycle).
    pub fn access_time(&self, addr: u32, first_access: bool) -> u32 {
        let waitstates = match addr {
            EWRAM_START...EWRAM_END => 2,
            VRAM_START...VRAM_END |
            OAM_START...OAM_END => {
                let drawing = !self.graphics.disp_stat.is_hblank &&
                              !self.graphics.disp_stat.is_vblank;
                if drawing { 1 } else { 0 }
            }
            ROM_START...ROM_END =>
                if first_access {
                    self.rom_n_cycle
                } else {
                    if self.rom_s_cycle_fast { 1 } else { 2 }
                },
            ROM_MIRROR1_START...ROM_MIRROR1_END =>
                if first_access {
                    self.rom_n_cycle
                } else {
                    if self.rom_s_cycle_fast { 1 } else { 4 }
                },
            ROM_MIRROR2_START...ROM_MIRROR2_END =>
                if first_access {
                    self.rom_n_cycle
                } else {
                    if self.rom_s_cycle_fast { 1 } else { 8 }
                },
            _ => 0,
        };
        (1 + waitstates).into()
    }

    pub fn load_bios(&mut self, data: &[u8]) {
        for i in 0..self.raw.sysrom.len() {
            self.raw.sysrom[i] = data[i];
        }
    }

    pub fn load_rom(&mut self, data: &[u8]) {
        unsafe {
            self.raw.rom = Some(std::slice::from_raw_parts(
                data as *const [u8] as *const u8,
                data.len()));
        }
    }
}

pub struct RawMemory {
    /// contains the BIOS
    pub sysrom: [u8; 0x4000],
    /// space for game data/code; largest area of RAM but memory transfers are
    /// 16 bit wide which makes it slower than iwram
    pub ewram: [u8; 0x40000],
    /// fastest RAM segment which is internally embedded in the CPU chip package
    /// with a 32 bit bus
    pub iwram: [u8; 0x8000],
    /// a mirror of the memory mapped ASIC registers on the GBA used to control
    /// graphics, sound, DMA, timers, etc.
    pub io: [u8; 0x400],
    /// specifies 16 bit color values for the paletted modes. There are two
    /// palettes of 256 colors - one for backgrounds and one for sprites.
    pub pal: [u8; 0x400],
    /// stores the frame buffer in bitmapped modes or the tile data/tile maps
    /// in text, rotate/scale modes
    pub vram: [u8; 0x18000],
    /// stores 128 entries of 8 bytes, containing information for each sprite
    pub oam: [u8; 0x400],
    // ROM in the game cartridge appears in this area. This ROM gets uploaded
    // on the javascript side and then a reference to it is set here
    pub rom: Option<&'static [u8]>,
    // either SRAM or flash ROM used for saving game data
    // TODO: allocate on the javascript side?
    // cart: Vec<u8>,
}

impl RawMemory {
    pub const fn new() -> RawMemory {
        RawMemory {
            sysrom: [0; 0x4000],
            ewram: [0; 0x40000],
            iwram: [0; 0x8000],
            io: [0; 0x400],
            pal: [0; 0x400],
            vram: [0; 0x18000],
            oam: [0; 0x400],
            rom: None,
            // pak: Vec::new(),
            // cart: Vec::new(),
        }
    }

    /// given an absolute address into memory, convert it to a reference to
    /// one of the memory segments and an index into that segment
    pub fn get_loc(&self, addr: u32) -> Option<(&[u8], usize)> {
        // TODO: use addr / 0x01000000 instead of a match statement?
        let result: (&[u8], u32) = match addr {
            SYSROM_START...SYSROM_END => (&self.sysrom, addr),
            EWRAM_START...EWRAM_END => (&self.ewram, addr - EWRAM_START),
            IWRAM_START...IWRAM_END => (&self.iwram, addr - IWRAM_START),
            IO_START...IO_END => (&self.io, addr - IO_START),
            PAL_START...PAL_END => (&self.pal, addr - PAL_START),
            VRAM_START...VRAM_END => (&self.vram, addr - VRAM_START),
            OAM_START...OAM_END => (&self.oam, addr - OAM_START),
            ROM_START...ROM_END => (self.rom.unwrap(), addr - ROM_START),
            ROM_MIRROR1_START...ROM_MIRROR1_END =>
                (self.rom.unwrap(), addr - ROM_MIRROR1_START),
            ROM_MIRROR2_START...ROM_MIRROR2_END =>
                (self.rom.unwrap(), addr - ROM_MIRROR2_START),
            0x0E000000...0x0E00FFFF => unimplemented!(),
            _ => { return None; }
        };
        Some((result.0, result.1 as usize))
    }

    pub fn get_loc_mut(&mut self, addr: u32) -> Option<(&mut [u8], usize)> {
        // TODO: use addr / 0x01000000 instead of a match statement?
        let result: (&mut [u8], u32) = match addr {
            SYSROM_START...SYSROM_END => (&mut self.sysrom, addr),
            EWRAM_START...EWRAM_END => (&mut self.ewram, addr - EWRAM_START),
            IWRAM_START...IWRAM_END => (&mut self.iwram, addr - IWRAM_START),
            IO_START...IO_END => (&mut self.io, addr - IO_START),
            PAL_START...PAL_END => (&mut self.pal, addr - PAL_START),
            VRAM_START...VRAM_END => (&mut self.vram, addr - VRAM_START),
            OAM_START...OAM_END => (&mut self.oam, addr - OAM_START),
            ROM_START...ROM_MIRROR2_END => panic!("trying to write to ROM"),
            0x0E000000...0x0E00FFFF => unimplemented!(),
            _ => { return None; }
        };
        Some((result.0, result.1 as usize))
    }

    pub fn get_byte(&self, addr: u32) -> u8 {
        let (segment, idx) = self.get_loc(addr).unwrap_or((&[], 1));
        if idx >= segment.len() { 0 } else { segment[idx] }
    }

    pub fn get_halfword(&self, addr: u32) -> u16 {
        self.get_byte(addr) as u16 | (self.get_byte(addr + 1) as u16) << 8
    }

    pub fn get_word(&self, addr: u32) -> u32 {
        self.get_byte(addr) as u32 |
            (self.get_byte(addr + 1) as u32) << 8 |
            (self.get_byte(addr + 2) as u32) << 16 |
            (self.get_byte(addr + 3) as u32) << 24
    }

    pub fn set_byte(&mut self, addr: u32, val: u8) {
        self.get_loc_mut(addr).map(|(segment, idx)| {
            segment[idx] = val;
        });
    }

    pub fn set_halfword(&mut self, addr: u32, val: u32) {
        self.set_byte(addr, util::get_byte(val, 0) as u8);
        self.set_byte(addr + 1, util::get_byte(val, 8) as u8);
    }

    pub fn set_word(&mut self, addr: u32, val: u32) {
        self.set_byte(addr, util::get_byte(val, 0) as u8);
        self.set_byte(addr + 1, util::get_byte(val, 8) as u8);
        self.set_byte(addr + 2, util::get_byte(val, 16) as u8);
        self.set_byte(addr + 3, util::get_byte(val, 24) as u8);
    }
}

/// map any addresses of mirrored segments of memory to the actual segment
fn canonicalize_addr(addr: u32) -> u32 {
    match addr {
        0x0000000...0x0FFFFFF => addr,
        0x2000000...0x2FFFFFF => EWRAM_START + (addr % 0x40000),
        0x3000000...0x3FFFFFF => IWRAM_START + (addr % 0x8000),
        0x4000000...0x40003FF => addr,
        0x4000400...0x4FFFFFF => {
            // the word at 0x4000800 is mirrored every 0x10000 bytes
            let offset = addr % 0x10000;
            if offset < 4 { 0x4000800 + offset } else { addr }
        },
        0x5000000...0x5FFFFFF => PAL_START + (addr % 0x400),
        0x6000000...0x6017FFF => addr,
        // 0x06010000 - 0x06017FFF <=> 0x06018000 - 0x0601FFFF
        0x6018000...0x601FFFF => 0x6010000 + addr - 0x6018000,
        // 0x06000000 - 0x06020000 <=> 0x06000000 - 0x06FFFFFF (every 0x20000 bytes)
        0x6020000...0x6FFFFFF => canonicalize_addr(VRAM_START + addr % 0x20000),
        0x7000000...0x7FFFFFF => OAM_START + (addr % 0x400),
        _ => addr,
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
        mem.set_word(0x3007FFC, 0x300);
        assert_eq!(mem.get_word(0x3007FFC), 0x300);
    }

    #[test]
    fn canonicalize() {
        assert_eq!(canonicalize_addr(0x0123456), 0x0123456);

        assert_eq!(canonicalize_addr(0x2040000), 0x2000000);
        assert_eq!(canonicalize_addr(0x2080020), 0x2000020);

        assert_eq!(canonicalize_addr(0x3000011), 0x3000011);
        assert_eq!(canonicalize_addr(0x3038002), 0x3000002);

        assert_eq!(canonicalize_addr(0x4000123), 0x4000123);
        assert_eq!(canonicalize_addr(0x4111111), 0x4111111);
        assert_eq!(canonicalize_addr(0x4010000), 0x4000800);
        assert_eq!(canonicalize_addr(0x4020003), 0x4000803);

        assert_eq!(canonicalize_addr(0x5006C03), 0x5000003);

        assert_eq!(canonicalize_addr(0x6000ABC), 0x6000ABC);
        assert_eq!(canonicalize_addr(0x6018001), 0x6010001);
        assert_eq!(canonicalize_addr(0x6020001), 0x6000001);
        assert_eq!(canonicalize_addr(0x6038001), 0x6010001);

        assert_eq!(canonicalize_addr(0x70034AA), 0x70000AA);
    }
}
