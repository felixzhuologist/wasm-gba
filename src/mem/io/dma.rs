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

use num::FromPrimitive;
use super::addrs::*;
use mem::Memory;
use util;

pub struct DMA {
    pub channels: [DMAChannel; 4],
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
}


impl Memory {
    // TODO: should this take val? does it make most sense to implement in
    // terms of byte?
    pub fn update_dma_byte(&mut self, addr: u32, _val: u8) {
        let offset = addr - DMA_START;
        // each channel is 12 bytes: 4 src, 4 dest, 2 count, 2 cnt
        let channel_num = offset as usize / 12;
        match offset % 12 {
            0...3 => { // src
                let src = self.raw.get_word(addr & !3);
                let mut channel = &mut self.dma.channels[channel_num];
                let mask = if channel_num == 0 { 0x7FFFFFF } else { 0xFFFFFFF };
                channel.src = src & mask;
            },
            4...7 => { // dest
                let dest = self.raw.get_word(addr & !3);
                let mut channel = &mut self.dma.channels[channel_num];
                let mask = if channel_num == 3 { 0xFFFFFFF } else { 0x7FFFFFF };
                channel.dest = dest & mask;
            },
            8...9 => { // chunk count
                let count = self.raw.get_halfword(addr & !1);
                let mut channel = &mut self.dma.channels[channel_num];
                channel.count = count & 0x3FFF;
            },
            // F E D C  B A 9 8  7 6 5 4  3 2 1 0 
            // N I M M  X S R A  A B B X  X X X X
            // 5-6 (B) = dest incr type
            // 7-8 (A) = src incr type
            // 9   (R) = repeat 
            // A   (S) = size (word if 1)
            // C-D (M) = timing mode
            // E   (I) = irq
            // F   (N) = enabled
            10...11 => { // cnt register
                let reg = self.raw.get_halfword(addr & !1);
                let mut channel = &mut self.dma.channels[channel_num];
                channel.dest_incr = IncrType::from_u16((reg >> 5) & 0b11).unwrap();
                channel.src_incr = IncrType::from_u16((reg >> 7) & 0b11).unwrap();
                channel.repeat = util::get_bit_hw(reg, 9);
                channel.word = util::get_bit_hw(reg, 10);
                channel.timing = TimingMode::from_u16((reg >> 12) & 0b11).unwrap();
                channel.irq = util::get_bit_hw(reg, 14);
                channel.enabled = util::get_bit_hw(reg, 15);

                if channel.count == 0 {
                    channel.count = 0x4000;
                }
            },
            _ => panic!("should not get here")
        }
    }

    pub fn update_dma_hw(&mut self, addr: u32, val: u32) {
        self.update_dma_byte(addr, val as u8);
        self.update_dma_byte(addr + 1, (val >> 8) as u8);
    }

    pub fn update_dma_word(&mut self, addr: u32, val: u32) {
        self.update_dma_hw(addr, val);
        self.update_dma_hw(addr + 2, val >> 16);
    }

    pub fn check_dma(&mut self, timing: TimingMode) {
        for i in 0..self.dma.channels.len() {
            if self.dma.channels[i].enabled  && self.dma.channels[i].timing == timing {
                self.run_dma(i)
            }
        }
    }

    fn run_dma(&mut self, channel_num: usize) {
        { // scope with mutable borrow on self.dma.channels
            let channel = &mut self.dma.channels[channel_num];

            // word or halfword align the src/dest addrs depending on chunk size
            let mask = if channel.word { !3 } else { !1 };
            let src = channel.src & mask;
            let dest = channel.dest & mask;

            let chunk_size = if channel.word { 4 } else { 2 };
            // TODO: is using copy_from_slice() faster?
            // TODO: can avoid this loop if the dest is fixed
            for _ in 0..(channel.count * chunk_size) {
                // TODO: if update_x_hw or update_x_word get implemented separately
                // from the byte version, should call that here instead
                let val = self.raw.get_byte(src);
                self.raw.set_byte(dest, val);

                channel.src_incr.update_addr(src);
                channel.dest_incr.update_addr(dest);
            }

            // update mapped/raw addrs
            channel.src = src;
            match channel.dest_incr {
                IncrType::Reload => (),
                _ => channel.dest = dest
            }
            self.raw.set_word(DMA_SAD[channel_num], channel.src);
            self.raw.set_word(DMA_DAD[channel_num], channel.dest);

            // update mapped/raw cnt register
            if !channel.repeat {
                channel.enabled = false;
                let old_reg = self.raw.get_word(DMA_CNT[channel_num]);
                self.raw.set_word(DMA_CNT[channel_num], old_reg & !0x8000);
            }
        }

        self.on_dma_finish_hook(channel_num);
    }
}

#[derive(Debug)] 
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
    pub irq: bool,
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
enum_from_primitive! {
#[derive(Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum IncrType {
    /// increment after each transfer
    Inc=0,
    /// decrement after each transfer
    Dec,
    /// address is fixed
    Fixed,
    /// increment during the transfer but then reset so that repeat DMA will
    /// always start at the same address. this is only valid for the dest addr
    Reload,
}
}

impl IncrType {
    pub fn update_addr(&self, addr: u32) -> u32 {
        match *self {
            IncrType::Inc |
            IncrType::Reload => addr + 1,
            IncrType::Dec => addr - 1,
            IncrType::Fixed => addr
        }
    }
}

/// Enum specifying when the DMA transfer should start
enum_from_primitive! {
#[derive(Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum TimingMode {
    /// start immediately
    Now=0,
    /// start at the next VBlank
    VBlank,
    /// start at the next HBlank
    HBlank,
    /// depends on the channel, but currently unimplemented
    Refresh,
}
}


#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn write() {
        let mut mem = Memory::new();
        mem.set_word(0x40000B0, 0x0F123456);
        assert_eq!(mem.dma.channels[0].src, 0x07123456);
        mem.set_word(0x40000D4, 0x08000300);
        assert_eq!(mem.dma.channels[3].src, 0x08000300);
        mem.set_word(0x40000CC, 0x08004FFF);
        assert_eq!(mem.dma.channels[2].dest, 0x00004FFF);
        mem.set_word(0x40000D8, 0x0910032A);
        assert_eq!(mem.dma.channels[3].dest, 0x0910032A);

        mem.set_halfword(0x40000B8, 0xABCD);
        assert_eq!(mem.dma.channels[0].count, 0x2BCD);
        mem.set_halfword(0x40000D0, 0x1234);
        assert_eq!(mem.dma.channels[2].count, 0x1234);
        mem.set_halfword(0x40000Dc, 0xFFFF);
        assert_eq!(mem.dma.channels[3].count, 0x3FFF);

        mem.set_halfword(0x40000BA, 0b1000_0010_1000_0001);
        {
            let channel = &mem.dma.channels[0];
            assert_eq!(channel.enabled, true);
            assert_eq!(channel.irq, false);
            assert_eq!(channel.timing, TimingMode::Now);
            assert_eq!(channel.word, false);
            assert_eq!(channel.repeat, true);
            assert_eq!(channel.src_incr, IncrType::Dec);
            assert_eq!(channel.dest_incr, IncrType::Inc);
        }
        mem.set_halfword(0x40000DE, 0b0001_1100_1101_1111);
        {
            let channel = &mem.dma.channels[3];
            assert_eq!(channel.enabled, false);
            assert_eq!(channel.irq, false);
            assert_eq!(channel.timing, TimingMode::VBlank);
            assert_eq!(channel.word, true);
            assert_eq!(channel.repeat, false);
            assert_eq!(channel.src_incr, IncrType::Dec);
            assert_eq!(channel.dest_incr, IncrType::Fixed);
        }
    }
}