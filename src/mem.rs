pub enum MemorySegment {
    //      Internal:
    /// contains the BIOS; on real hardware is executable but not readable/writable
    SysRom,
    /// external work RAM. the largest area of RAM available, but transfers
    /// to/from ewram are 16 bits wide so should be used for THUMB instructions
    EwRam,
    /// internal work RAM located on the CPU with a 32bit bus
    IwRam,
    /// memory mapped IO registers
    IoRam,

    //      Internal (Display):
    /// contains two palettes with 256 entries of 15 bit colors each
    PalRam,
    /// contains data used for backgrounds and sprites; format depends on the
    /// current video mode
    VidRam,
    /// used to control sprites
    OAM,

    //      External:
    /// rom from the game cartridge. transfers are 16 bits wide
    PakRom,
    /// mirrors of the above pak rom to allow for multiple speeds. the first
    /// mirror has a waitstate of 1 and the second has a waitstate of 2
    PakRom1,
    PakRom2,
    /// persistent storage on the pak (e.g. for game saves)
    CartRom,

    Unused
}

fn get_segment(address: u32) -> MemorySegment {
    match address {
        0x00000000...0x00003FFF => MemorySegment::SysRom,
        0x02000000...0x0203FFFF => MemorySegment::EwRam,
        0x03000000...0x03007FFF => MemorySegment::IwRam,
        0x04000000...0x040003FF => MemorySegment::IoRam,
        0x05000000...0x050003FF => MemorySegment::PalRam,
        0x06000000...0x06017FFF => MemorySegment::VidRam,
        0x07000000...0x070003FF => MemorySegment::OAM,
        0x08000000...0x09FFFFFF => MemorySegment::PakRom,
        0x0A000000...0x0BFFFFFF => MemorySegment::PakRom1,
        0x0C000000...0x0DFFFFFF => MemorySegment::PakRom2,
        0x0E000000...0x0E00FFFF => MemorySegment::CartRom,
        _ => MemorySegment::Unused
    }
}

pub struct Memory {
    sysrom: [u8; 0x3FF],
    ewram: [u8; 0x3FF],
    iwram: [u8; 0x7FF],
    io: [u8; 0x3FE],
    pal: [u8; 0x3FF],
    vram: [u8; 0x17FFF],
    oam: [u8; 0x3FF],
    // store cartridge data on the heap so that we can allocate only what is
    // necessary
    pak: Vec<u8>,
    cart: Vec<u8>,
}

impl Memory {
    pub fn new() -> Memory {
        Memory {
            sysrom: [0; 0x3FF],
            ewram: [0; 0x3FF],
            iwram: [0; 0x7FF],
            io: [0; 0x3FE],
            pal: [0; 0x3FF],
            vram: [0; 0x17FFF],
            oam: [0; 0x3FF],
            pak: Vec::new(),
            cart: Vec::new(),
        }
    }

    pub fn get_byte(&self, addr: u32) -> u8 {
        unimplemented!()
    }

    pub fn get_word(&self, addr: u32) -> u32 {
        unimplemented!()
    }

    pub fn set_byte(&self, addr: u32, val: u8) {
        unimplemented!()
    }

    pub fn set_word(&self, addr: u32, val: u32) {
        unimplemented!()
    }
}