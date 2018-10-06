use ::cpu::CPU;
use ::util;

/// This instruction specifies a jump of +/- 32Mbytes. The branch offset must take
/// account of the prefetch operation, which causes the PC to be 1/2 words ahead of
/// the current instruction (for THUMB/ARM)
#[derive(Clone, Debug)]
pub struct Branch {
    /// signed offset from the PC
    pub offset: i32,
    /// branch with link writes the old PC (adjusted for prefetch) into the link
    /// register and contains the address of the instruction following this
    /// instruction
    pub link: bool
}

impl Branch {
    /// parses the following format:
    /// 27 .. 25 | 24 | 23 .. 0
    ///    101   | L  | offset
    pub fn parse_instruction(ins: u32) -> Branch {
        // the offset is interpreted as a signed 2's complement 24 bit offset
        // which is shifted left two bits and then sign extended to 32 bits
        let mut offset = ins & 0xFFFFFF;
        if util::get_bit(offset, 23) {
            offset |= 0xFF000000;
        }

        Branch {
            offset: (offset as i32) << 2,
            link: util::get_bit(ins, 24)
        }
    }

    pub fn run(&self, cpu: &mut CPU) -> u32 {
        let old_pc = cpu.r[15];
        if self.link {
            let ret = old_pc - cpu.instruction_size();
            cpu.set_reg(14, ret);
        }

        cpu.modify_pc(self.offset as i64);

        // 1N + 2S
        cpu.mem.access_time(old_pc, false) +
            cpu.mem.access_time(cpu.r[15], true) +
            cpu.mem.access_time(cpu.r[15] + 4, false)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn parse() {
        let branch = Branch::parse_instruction(0xEA_00_00_18);
        assert_eq!(branch.offset, 0x60);
    }

    #[test]
    fn parse_with_link_signed() {
        let branch = Branch::parse_instruction(
            0b0000101_1_11001010_00001111_11010001);
        assert!(branch.link);
        assert_eq!(branch.offset, 0b111111_11001010_00001111_11010001_00u32 as i32);
    }

    #[test]
    fn parse_without_link_unsigned() {
        let branch = Branch::parse_instruction(
            0b0000101_0_01001010_00001111_11010001);
        assert!(!branch.link);
        assert_eq!(branch.offset, 0b000000_01001010_00001111_11010001_00u32 as i32);
    }

    #[test]
    fn parse_min() {
        let branch = Branch::parse_instruction(0x0A_800000);
        assert_eq!(branch.offset, -(1 << 25));
    }

    #[test]
    fn parse_max() {
        let branch = Branch::parse_instruction(0x0A_7FFFFF);
        // 4 because it gets shifted 2 so the rightmost 2 bits are 0
        assert_eq!(branch.offset, (1 << 25) - 4);
    }

    #[test]
    fn branch_down() {
        let mut cpu = CPU::new();
        cpu.set_reg(15, 64_000_000);
        let ins = Branch { offset: -100, link: true };
        ins.run(&mut cpu);

        assert_eq!(cpu.get_reg(15), 64_000_000 - 100);
        assert_eq!(cpu.get_reg(14), 64_000_000 - 4);
    }

    #[test]
    fn branch_up() {
        let mut cpu = CPU::new();
        cpu.set_reg(15, 64_000_000);
        let ins = Branch { offset: 113, link: false };
        ins.run(&mut cpu);

        assert_eq!(cpu.get_reg(15), 64_000_113);
    }
}
