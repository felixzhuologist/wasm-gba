use super::{Instruction, InstructionType};
use ::cpu::CPU;
use ::util;

/// This instruction specifies a jump of +/- 32Mbytes. The branch offset must take
/// account of the prefetch operation, which causes the PC to be 2 words ahead of
/// the current instruction
struct Branch {
    /// the offset is interpreted as a signed 2's complement 24 bit offset which
    /// is shifted left two bits and then sign extended to 32 bits
    offset: u32,
    /// branch with link writes the old PC (adjusted for prefetch) into the link
    /// register and contains the address of the instruction following this
    /// instruction
    link: bool
}

impl Branch {
    pub fn parse_instruction(ins: u32) -> Branch {
        Branch {
            offset: ins & 0xFFFFFF,
            link: util::get_bit(ins, 24)
        }
    }
}

impl Instruction for Branch {
    fn get_type(&self) -> InstructionType { InstructionType::Branch }
    fn process_instruction(&self, cpu: &mut CPU) {
        if self.link {
            cpu.r14 = cpu.r15 - 4;
        }
        let sign_extended = if util::get_bit(self.offset, 23) {
            self.offset | 0xFF000000
        } else {
            self.offset
        };

        // TODO: is i64 necessary?
        cpu.r15 = ((cpu.r15 as i64) + (sign_extended << 2) as i64) as u32;
    }
}