//! Interrupts are handled in 3 registers: IME, IE, and IF. Only the 0th bit
//! of IME is used, and is set to enable interrupts. IE and IF both have a bit
//! for each interrupt: if it's set in IE then that interrupt is enabled, and
//! if it's set in IF then that interrupt has been triggered and is waiting to
//! be handled. To acknowledge an interrupt, the game writes the bit back to IF.
//! IF and IE have the following format:
//! F E D C  B A 9 8  7 6 5 4  3 2 1 0 
//! X X T Y  G F E D  S L K J  I C H V
//! 0 (V) = VBlank Interrupt 
//! 1 (H) = HBlank Interrupt 
//! 2 (C) = VCount Interrupt 
//! 3 (I) = Timer 0 Interrupt 
//! 4 (J) = Timer 1 Interrupt 
//! 5 (K) = Timer 2 Interrupt 
//! 6 (L) = Timer 3 Interrupt 
//! 7 (S) = Serial Communication Interrupt 
//! 8 (D) = DMA0 Interrupt 
//! 9 (E) = DMA1 Interrupt 
//! A (F) = DMA2 Interrupt 
//! B (G) = DMA3 Interrupt 
//! C (Y) = Key Interrupt 
//! D (T) = Cassette Interrupt 

use super::addrs::*;
use mem::Memory;

pub struct Interrupt {
    pub master_enabled: bool,
    pub enabled: InterruptBitmap,
    pub triggered: InterruptBitmap,
}

impl Interrupt {
    pub const fn new() -> Interrupt {
        Interrupt {
            master_enabled: false,
            enabled: InterruptBitmap::new(),
            triggered: InterruptBitmap::new(),
        }
    }

    /// Return true if there is any pending interrupt
    pub fn pending_interrupts(&self) -> bool {
        if !self.master_enabled {
            return false;
        }

        self.enabled.as_array().iter()
            .zip(self.triggered.as_array().iter())
            .filter(|(enabled, triggered)| **enabled && **triggered)
            .peekable()
            .peek()
            .is_some()
    }
}

impl Memory {
    pub fn update_int_byte(&mut self, addr: u32, val: u8) {
        let enabled = &mut self.int.enabled;
        let triggered = &mut self.int.triggered;
        match addr {
            IME => { self.int.master_enabled = get_bit(val, 0); },
            IE_LO => {
                enabled.vblank = get_bit(val, 0);
                enabled.hblank = get_bit(val, 1);
                enabled.vcount = get_bit(val, 2);
                enabled.timer[0] = get_bit(val, 3);
                enabled.timer[1] = get_bit(val, 4);
                enabled.timer[2] = get_bit(val, 5);
                enabled.timer[3] = get_bit(val, 6);
                enabled.serial = get_bit(val, 7);
            },
            IE_HI => {
                enabled.dma[0] = get_bit(val, 0);
                enabled.dma[1] = get_bit(val, 1);
                enabled.dma[2] = get_bit(val, 2);
                enabled.dma[3] = get_bit(val, 3);
                enabled.keypad = get_bit(val, 4);
                enabled.gamepak = get_bit(val, 5);
            },
            // we XOR to emulate the fact that writing a 1 to a triggered
            // interrupt acknowledges/clears it
            IF_LO => {
                triggered.vblank ^= get_bit(val, 0);
                triggered.hblank ^= get_bit(val, 1);
                triggered.vcount ^= get_bit(val, 2);
                triggered.timer[0] ^= get_bit(val, 3);
                triggered.timer[1] ^= get_bit(val, 4);
                triggered.timer[2] ^= get_bit(val, 5);
                triggered.timer[3] ^= get_bit(val, 6);
                triggered.serial ^= get_bit(val, 7);
            },
            IF_HI => {
                triggered.dma[0] ^= get_bit(val, 0);
                triggered.dma[1] ^= get_bit(val, 1);
                triggered.dma[2] ^= get_bit(val, 2);
                triggered.dma[3] ^= get_bit(val, 3);
                triggered.keypad ^= get_bit(val, 4);
                triggered.gamepak ^= get_bit(val, 5);
            },
            WSCNT_LO => {
                self.rom_n_cycle = match (val >> 2) & 0b11 {
                    0 => 4,
                    1 => 3,
                    2 => 2,
                    3 => 8,
                    _ => panic!("should not get here")
                };
                self.rom_s_cycle_fast = (val >> 4) & 1 == 1;
            }
            _ => ()
        }
    }

    pub fn update_int_hw(&mut self, addr: u32, val: u32) {
        self.update_int_byte(addr, val as u8);
        self.update_int_byte(addr + 1, (val >> 8) as u8);
    }

    pub fn update_int_word(&mut self, addr: u32, val: u32) {
        self.update_int_hw(addr, val);
        self.update_int_hw(addr + 2, val >> 16);
    }
}

pub struct InterruptBitmap {
    pub vblank: bool,
    pub hblank: bool,
    pub vcount: bool,
    pub timer: [bool; 4],
    pub serial: bool,
    pub dma: [bool; 4],
    pub keypad: bool,
    pub gamepak: bool,
}

impl InterruptBitmap {
    pub const fn new() -> InterruptBitmap {
        InterruptBitmap {
            vblank: false,
            hblank: false,
            vcount: false,
            timer: [false; 4],
            serial: false,
            dma: [false; 4],
            keypad: false,
            gamepak: false,      
        }
    }

    pub fn as_array(&self) -> [bool; 14] {
        [
            self.vblank,
            self.hblank,
            self.vcount,
            self.timer[0],
            self.timer[1],
            self.timer[2],
            self.timer[3],
            self.serial,
            self.dma[0],
            self.dma[1],
            self.dma[2],
            self.dma[3],
            self.keypad,
            self.gamepak,
        ]
    }
}

fn get_bit(val: u8, i: u8) -> bool {
    ((val >> i) & 1) == 1
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn write() {
        let mut mem = Memory::new();
        mem.set_byte(0x4000208, 1);
        assert_eq!(mem.int.master_enabled, true);

        mem.set_halfword(0x4000200, 0b0010_1110_1111_0001);
        {
            let enabled = &mem.int.enabled;
            assert_eq!(enabled.vblank, true);
            assert_eq!(enabled.hblank, false);
            assert_eq!(enabled.vcount, false);
            assert_eq!(enabled.timer[0], false);
            assert_eq!(enabled.timer[1], true);
            assert_eq!(enabled.timer[2], true);
            assert_eq!(enabled.timer[3], true);
            assert_eq!(enabled.serial, true);
            assert_eq!(enabled.dma[0], false);
            assert_eq!(enabled.dma[1], true);
            assert_eq!(enabled.dma[2], true);
            assert_eq!(enabled.dma[3], true);
            assert_eq!(enabled.keypad, false);
            assert_eq!(enabled.gamepak, true);
        }

        mem.set_halfword(0x4000202, 0b0000_1100_0000_1010);
        {
            let triggered = &mem.int.triggered;
            assert_eq!(triggered.vblank, false);
            assert_eq!(triggered.hblank, true);
            assert_eq!(triggered.vcount, false);
            assert_eq!(triggered.timer[0], true);
            assert_eq!(triggered.timer[1], false);
            assert_eq!(triggered.timer[2], false);
            assert_eq!(triggered.timer[3], false);
            assert_eq!(triggered.serial, false);
            assert_eq!(triggered.dma[0], false);
            assert_eq!(triggered.dma[1], false);
            assert_eq!(triggered.dma[2], true);
            assert_eq!(triggered.dma[3], true);
            assert_eq!(triggered.keypad, false);
            assert_eq!(triggered.gamepak, false);
        }

        mem.set_byte(0x4000204, 0b1011_0100);
        assert_eq!(mem.rom_n_cycle, 3);
        assert_eq!(mem.rom_s_cycle_fast, true);
    }

    #[test]
    fn acknowledge_int() {
        let mut mem = Memory::new();
        mem.set_halfword(0x4000202, 0b0000_1100_0000_1010);
        {
            let triggered = &mem.int.triggered;
            assert_eq!(triggered.vblank, false);
            assert_eq!(triggered.hblank, true);
            assert_eq!(triggered.vcount, false);
            assert_eq!(triggered.timer[0], true);
            assert_eq!(triggered.timer[1], false);
            assert_eq!(triggered.timer[2], false);
            assert_eq!(triggered.timer[3], false);
            assert_eq!(triggered.serial, false);
            assert_eq!(triggered.dma[0], false);
            assert_eq!(triggered.dma[1], false);
            assert_eq!(triggered.dma[2], true);
            assert_eq!(triggered.dma[3], true);
            assert_eq!(triggered.keypad, false);
            assert_eq!(triggered.gamepak, false);
        }

        // acknowledge hblank and timer[0]
        mem.set_halfword(0x4000202, 0b0000_0000_0000_1010);
        {
            let triggered = &mem.int.triggered;
            assert_eq!(triggered.vblank, false);
            assert_eq!(triggered.hblank, false);
            assert_eq!(triggered.vcount, false);
            assert_eq!(triggered.timer[0], false);
            assert_eq!(triggered.timer[1], false);
            assert_eq!(triggered.timer[2], false);
            assert_eq!(triggered.timer[3], false);
            assert_eq!(triggered.serial, false);
            assert_eq!(triggered.dma[0], false);
            assert_eq!(triggered.dma[1], false);
            assert_eq!(triggered.dma[2], true);
            assert_eq!(triggered.dma[3], true);
            assert_eq!(triggered.keypad, false);
            assert_eq!(triggered.gamepak, false);
        }
    }
}
