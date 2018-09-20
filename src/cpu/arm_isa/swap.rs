use super::{Instruction, InstructionType};
use ::cpu::CPU;
use ::util;

/// Swap a byte or word between a register and external memory "atomically"
pub struct SingleDataSwap {
    /// if true, swap byte else swap word
    byte: bool,
    /// base register
    rn: usize,
    /// destination register
    rd: usize,
    /// source register
    rm: usize
}

impl SingleDataSwap {
    /// 27 .. 23 | 22 | 21 | 20 | 19 .. 16 | 15 .. 12 | 11 .. 4  | 3 .. 0
    ///  00010   | B  | 0  | 0  |    Rn    |    Rd    | 00001001 |   Rm
    pub fn parse_instruction(ins: u32) -> SingleDataSwap {
        SingleDataSwap {
            byte: util::get_bit(ins, 22),
            rn: util::get_nibble(ins, 16) as usize,
            rd: util::get_nibble(ins, 12) as usize,
            rm: util::get_nibble(ins, 0) as usize
        }
    }
}

impl Instruction for SingleDataSwap {
    fn get_type(&self) -> InstructionType { InstructionType::SingleDataSwap }
    fn run(&self, cpu: &mut CPU) {
        unimplemented!()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn parse() {
        let ins = SingleDataSwap::parse_instruction(
            0b1111_00010_1_00_1000_0001_00001001_1100);
        assert!(ins.byte);
        assert_eq!(ins.rn, 8);
        assert_eq!(ins.rd, 1);
        assert_eq!(ins.rm, 12);
    }
}