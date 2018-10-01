use util;
use num::FromPrimitive;

/// The 7 modes of operation of the ARM7TDMI processor, which is in user mode
/// by default
enum_from_primitive! {
#[derive(Copy, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum CPUMode {
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
pub enum InstructionSet {
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
/// M[4..0] is the processor mode, as defined above
///
/// in this implementation we unpack the 32 bits to avoid having to do bit
/// manipulation each time we want to get a specific flag, at the expense of
/// more space
pub struct PSR {
    pub neg: bool,
    pub zero: bool,
    pub carry: bool,
    pub overflow: bool,
    pub irq: bool,
    pub fiq: bool,
    pub isa: InstructionSet,
    pub mode: CPUMode
}

impl PSR {
    pub const fn new() -> PSR {
        PSR {
            neg: false,
            zero: false,
            carry: false,
            overflow: false,
            irq: false,
            fiq: false,
            isa: InstructionSet::ARM,
            mode: CPUMode::SVC
        }
    }

    pub fn to_u32(&self) -> u32 {
        ((self.neg as u32) << 31) |
        ((self.zero as u32) << 30) |
        ((self.carry as u32) << 29) |
        ((self.overflow as u32) << 28) |
        ((self.irq as u32) << 7) |
        ((self.fiq as u32) << 6) |
        ((match self.isa { InstructionSet::ARM => 0, InstructionSet::THUMB => 1 }) << 5) |
        (self.mode as u32)
    }

    pub fn from_u32(&mut self, val: u32) {
        self.neg = util::get_bit(val, 31);
        self.zero = util::get_bit(val, 30);
        self.carry = util::get_bit(val, 29);
        self.overflow = util::get_bit(val, 28);
        self.irq = util::get_bit(val, 7);
        self.fiq = util::get_bit(val, 6);
        self.isa = if util::get_bit(val, 5) { InstructionSet::THUMB } else { InstructionSet::ARM };
        self.mode = CPUMode::from_u32(val & 0b11111).unwrap();
    }
}

