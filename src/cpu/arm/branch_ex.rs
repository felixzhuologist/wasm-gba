use ::cpu::CPU;
use ::cpu::status_reg::CPUMode;
use ::util;

/// This instruction performs a branch by copying the contents of a single register
/// into the program counter, and causes a pipeline flush and refill.
pub struct BranchAndExchange {
    /// contents of this register are written to the PC
    pub reg: usize,
}

impl BranchAndExchange {
    /// parses the following format:
    /// cond_0001_0010_1111_1111_1111_0001_Rn
    pub fn parse_instruction(ins: u32) -> BranchAndExchange {
        BranchAndExchange {
            reg: util::get_nibble(ins, 0) as usize,
        }
    }

    pub fn run(&self, cpu: &mut CPU) {
        let switch_to_thumb = util::get_bit(self.reg as u32, 0);
        cpu.set_isa(switch_to_thumb);
        let jump_dest = cpu.get_reg(self.reg);
        cpu.set_reg(15, jump_dest);
        cpu.should_flush = true;
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn parse() {
        let bx = BranchAndExchange::parse_instruction(
            0b0000_0001_0010_1111_1111_1111_0001_1011);
        assert_eq!(bx.reg, 0b1011);
    }

    #[test]
    fn process() {
        let mut cpu = CPU::new();
        cpu.set_reg(3, 0x1123);

        let ins = BranchAndExchange { reg: 3 };
        ins.run(&mut cpu);

        assert_eq!(cpu.get_reg(15), 0x1123);
        assert_eq!(cpu.cpsr.t, CPUMode::THUMB);
        assert!(cpu.should_flush);
    }

    #[test]
    fn process_noop() {
        let mut cpu = CPU::new();
        cpu.set_reg(15, 5);

        let ins = BranchAndExchange { reg: 2 };
        ins.run(&mut cpu);

        assert_eq!(cpu.get_reg(15), 0);
        assert_eq!(cpu.cpsr.t, CPUMode::ARM);
        assert!(cpu.should_flush);
    }
}