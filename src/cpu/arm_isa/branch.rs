use super::{Instruction, InstructionType};
use ::cpu::Registers;
use ::util;

/// This instruction specifies a jump of +/- 32Mbytes. The branch offset must take
/// account of the prefetch operation, which causes the PC to be 2 words ahead of
/// the current instruction
pub struct Branch {
    /// the offset is interpreted as a signed 2's complement 24 bit offset which
    /// is shifted left two bits and then sign extended to 32 bits
    offset: u32,
    /// branch with link writes the old PC (adjusted for prefetch) into the link
    /// register and contains the address of the instruction following this
    /// instruction
    link: bool
}

impl Branch {
    /// parses the following format:
    /// 27 .. 25 | 24 | 23 .. 0
    ///    101   | L  | offset
    pub fn parse_instruction(ins: u32) -> Branch {
        Branch {
            offset: ins & 0xFFFFFF,
            link: util::get_bit(ins, 24)
        }
    }
}

impl Instruction for Branch {
    fn get_type(&self) -> InstructionType { InstructionType::Branch }
    fn process_instruction(&self, regs: &mut Registers) {
        if self.link {
            let ret = regs.get_reg(15) - 4;
            regs.set_reg(14, ret);
        }
        let sign_extended = if util::get_bit(self.offset, 23) {
            self.offset | 0xFF000000
        } else {
            self.offset
        };

        // TODO: is i64 necessary?
        let pc = (regs.get_reg(15) as i64) + (sign_extended << 2) as i64;
        regs.set_reg(15, pc as u32);
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn parse_with_link() {
        let branch = Branch::parse_instruction(0x0_B_ABC123);
        assert!(branch.link);
        assert_eq!(branch.offset, 0xABC123);
    }

    #[test]
    fn parse_without_link() {
        let branch = Branch::parse_instruction(0x0_A_ABCDEF);
        assert!(!branch.link);
        assert_eq!(branch.offset, 0xABCDEF);
    }
}
