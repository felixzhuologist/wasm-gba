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

    #[test]
    fn limit_min() {
        let mut regs = Registers::new();
        regs.set_reg(15, 64_000_000);
        let ins = Branch { offset: 1 << 23, link: true };
        ins.process_instruction(&mut regs);

        assert_eq!(regs.get_reg(15), 64_000_000 - (1<<25));
        assert_eq!(regs.get_reg(14), 64_000_000 - 4);
    }

    #[test]
    fn limit_max() {
        let mut regs = Registers::new();
        regs.set_reg(15, 64_000_000);
        let ins = Branch { offset : (1<<23) - 1, link: false };
        ins.process_instruction(&mut regs);

        // 4 because it gets shifted 2 so the rightmost 2 bits are 0
        assert_eq!(regs.get_reg(15), 64_000_000 + (1<<25) - 4);
        assert_eq!(regs.get_reg(14), 0);
    }
}
