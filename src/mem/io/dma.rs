//! DMA, which stands for direct memory access, is a feature that the GBA
//! provides to allow for fast data copying. There are 4 DMA channels, with 0
//! having the highest priority:
//!   - 0 is used for time critical operations to internal RAM)
//!   - 1 and 2 are used to transfer sound data
//!   - 3 is for general purpose copies like loading bitmap/tile data
//! DMA is accessed through the IO portion of memory: the user/game writes
//! bits to certain locations which sets the parameters of DMA. When a transfer
//! is requested, the DMA controller takes over the hardware and the CPU is halted
//! until the transfer is complete.

pub struct DMA {
    channels: [DMAChannel; 4],
}

impl DMA {
    pub const fn new() -> DMA {
        DMA {
            channels: [
                DMAChannel::new(),
                DMAChannel::new(),
                DMAChannel::new(),
                DMAChannel::new(),
            ]
        }
    }

    pub fn set_byte(&mut self, addr: u32, val: u8, raw: &[u8]) {
        unimplemented!()
    }

    pub fn set_halfword(&mut self, addr: u32, val: u32, raw: &[u8]) {
        self.set_byte(addr, val as u8, raw);
        self.set_byte(addr + 1, (val >> 8) as u8, raw);
    }

    pub fn set_word(&mut self, addr: u32, val: u32, raw: &[u8]) {
        self.set_halfword(addr, val, raw);
        self.set_halfword(addr + 1, val >> 16, raw);
    }
}

pub struct DMAChannel {
    /// 27 bit for channel 0, 28 bit for 1 - 3
    src: u32,
    // 27 bit for channels 0 - 2, 28 bit for 3
    dest: u32,
    /// 14 bits, number of words/halfwords to copy
    count: u16,
    src_incr: IncrType,
    dest_incr: IncrType,
    /// if timing is VBlank or HBlank, repeat the copy each time
    repeat: bool,
    /// if true copy words, otherwise copy halfwords
    word: bool,
    timing: TimingMode,
    /// if true, raise an interrupt when finished
    irq: bool,
    enabled: bool,
}

impl DMAChannel {
    pub const fn new() -> DMAChannel {
        DMAChannel {
            src: 0,
            dest: 0,
            count: 0,
            src_incr: IncrType::Inc,
            dest_incr: IncrType::Inc,
            repeat: false,
            word: true,
            timing: TimingMode::Now,
            irq: false,
            enabled: false
        }
    }
}
/// Specifies how to modify the src/dest of the channel
pub enum IncrType {
    /// increment after each transfer
    Inc,
    /// decrement after each transfer
    Dec,
    /// address is fixed
    Fixed,
    /// increment during the transfer but then reset so that repeat DMA will
    /// always start at the same address. this is only valid for the dest addr
    Reload,
}

/// Enum specifying when the DMA transfer should start
pub enum TimingMode {
    /// start immediately
    Now,
    /// start at the next VBlank
    VBlank,
    /// start at the next HBlank
    HBlank,
    /// depends on the channel, but currently unimplemented
    Refresh,
}
