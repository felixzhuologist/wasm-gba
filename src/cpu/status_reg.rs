use util;
use num::FromPrimitive;

/// The 7 modes of operation of the ARM7TDMI processor, which is in user mode
/// by default
enum_from_primitive! {
#[derive(Copy, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum ProcessorMode {
    USR = 0b10000,
    /// fast interrupt mode supports a data transfer or channel process
    FIQ = 0b10001,
    /// interrupt mode is used for general purpose interrupt handling
    IRQ = 0b10010,
    /// supervisor mode is a protected mode for the OS
    SVC = 0b10011,
    /// abort mode is entered after a data or instruction prefetch abort
    ABT = 0b10111,
    /// undefined mode is entered when an undefined instruction is executed
    UND = 0b11011,
    /// system mode is a privileged user mode for the OS
    SYS = 0b11111
}
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum CPUMode {
    ARM,
    THUMB
}

/// A program status register. On real hardware, this is a 32 bit register with
/// 16 defined bits and 16 reserved bits.
/// 
///  | 31 | 30 | 29 | 28 | ... | 7 | 6 | 5 | 4 ... 0
///  | N  | Z  | C  | V  | ... | I | F | T | processsor mode
///
/// where N, Z, C, V are condition flags:
/// flag | logical instruction     | arithmetic instruction
/// ------------------------------------------------------------------
///  N   | none                    | bit 31 of the result has been set
///  Z   | result is 0             | result is 0
///  C   | carry flag after shift  | result was > than 32 bits
///  V   | none                    | result was > 31 bits 
///
/// I = 1 disables the IRQ
/// F = 1 disables the FIQ
/// T = 0 means the processor is in ARM state, and in THUMB state otherwise
/// M[4:0] is the processor mode, as defined above
///
/// in this implementation we unpack the 32 bits to avoid having to do bit
/// manipulation each time we want to get a specific flag, at the expense of
/// more space
pub struct PSR {
    pub n: bool,
    pub z: bool,
    pub c: bool,
    pub v: bool,
    pub i: bool,
    pub f: bool,
    pub t: CPUMode,
    pub mode: ProcessorMode
}

impl PSR {
    pub fn new() -> PSR {
        PSR {
            n: false,
            z: false,
            c: false,
            v: false,
            i: false,
            f: false,
            t: CPUMode::ARM,
            mode: ProcessorMode::USR
        }
    }

    pub fn to_u32(&self) -> u32 {
        ((self.n as u32) << 31) |
        ((self.z as u32) << 30) |
        ((self.c as u32) << 29) |
        ((self.v as u32) << 28) |
        ((self.i as u32) << 7) |
        ((self.f as u32) << 6) |
        ((match self.t { CPUMode::ARM => 0, CPUMode::THUMB => 1 }) << 5) |
        (self.mode as u32)
    }

    pub fn from_u32(&mut self, val: u32) {
        self.n = util::get_bit(val, 31);
        self.z = util::get_bit(val, 30);
        self.c = util::get_bit(val, 29);
        self.v = util::get_bit(val, 28);
        self.i = util::get_bit(val, 7);
        self.f = util::get_bit(val, 6);
        self.t = if util::get_bit(val, 5) { CPUMode::THUMB } else { CPUMode::ARM };
        self.mode = ProcessorMode::from_u32(val & 0b11111).unwrap();
    }
}

