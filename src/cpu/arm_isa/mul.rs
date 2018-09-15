use super::{Instruction, InstructionType};
use ::cpu::Registers;
use ::util;

/// The multiply and multiply-accumulate instructions perform integer multiplication
/// on the contents of two registers Rm and Rs and stores the lower 32 bits of the
/// result in Rd
pub struct Multiply {
    /// if true, add contents of Rn to the product before storing in Rd
    accumulate: bool,
    set_flags: bool,
    rd: usize,
    rn: usize,
    rs: usize,
    rm: usize
}

impl Multiply {
    /// parses from the following format:
    /// 27 .. 22 | 21 | 20 | 19 .. 16 | 15 .. 12 | 11 .. 8 | 7 .. 4 | 3 .. 0
    ///   000000 | A  | S  |    Rd    |    Rn    |    Rs   |  1001  |  Rm 
    pub fn parse_instruction(ins: u32) -> Multiply {
        Multiply {
            accumulate: util::get_bit(ins, 21),
            set_flags: util::get_bit(ins, 20),
            rd: util::get_nibble(ins, 16) as usize,
            rn: util::get_nibble(ins, 12) as usize,
            rs: util::get_nibble(ins, 8) as usize,
            rm: util::get_nibble(ins, 0) as usize
        }
    }
}

impl Instruction for Multiply {
    fn get_type(&self) -> InstructionType { InstructionType::Multiply }
    fn process_instruction(&self, regs: &mut Registers) {
        if self.rd == 15 || self.rm == 15 || self.rn == 15 {
            panic!("Can't use R15 as operand or dest in mul");
        }
        if self.rd == self.rm {
            panic!("Rd and Rm can't be the same in mul");
        }
        // since we only care about the bottom 32 bits, this will be the same
        // for both signed and unsigned integers
        let mut result: u64 = (regs.get_reg(self.rm) as u64) * (regs.get_reg(self.rn) as u64);
        if self.accumulate {
            result += regs.get_reg(self.rs) as u64;
        }
        regs.set_reg(self.rd, result as u32);
        if self.set_flags {
            regs.cpsr.n = ((result >> 31) & 1) == 1;
            regs.cpsr.z = result == 0;
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn parse() {
        let mul = Multiply::parse_instruction(
            0b0000_00000_1_1_0001_1000_1111_1001_0010);
        assert!(mul.accumulate);
        assert!(mul.set_flags);
        assert_eq!(mul.rd, 1);
        assert_eq!(mul.rn, 8);
        assert_eq!(mul.rs, 15);
        assert_eq!(mul.rm, 2);
    }
}
