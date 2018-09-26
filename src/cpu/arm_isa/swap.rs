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

    pub fn run(&self, cpu: &mut CPU) {
        if self.rn == 15 || self.rd == 15 || self.rm == 15 {
            panic!("can't use R15 as an operand");
        }
    
        let addr = cpu.get_reg(self.rn);
        let memval = if self.byte {
            cpu.mem.get_byte(addr) as u32
        } else {
            cpu.mem.get_word(addr)
        };

        let regval = cpu.get_reg(self.rm);
        if self.byte {
            cpu.mem.set_byte(addr, regval as u8);
        } else {
            cpu.mem.set_word(addr, regval);
        }

        cpu.set_reg(self.rd, memval);
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

    #[test]
    fn swap_byte() {
        let mut cpu = CPU::new();
        let addr = 0x02000001;
        cpu.set_reg(0, addr);
        cpu.set_reg(2, 0xFF);
        cpu.mem.set_byte(addr, 0x3A);

        let ins = SingleDataSwap {
            byte: true,
            rn: 0,
            rd: 1,
            rm: 2,
        };

        ins.run(&mut cpu);

        assert_eq!(cpu.mem.get_byte(addr), 0xFF);
        assert_eq!(cpu.get_reg(1), 0x3A);
    }

    #[test]
    fn swap_word() {
        let mut cpu = CPU::new();
        let addr = 0x02000001;
        cpu.set_reg(0, addr);
        cpu.set_reg(1, 0xFE41);
        cpu.mem.set_word(addr, 0x3AFF001);

        let ins = SingleDataSwap {
            byte: false,
            rn: 0,
            rd: 1,
            rm: 1,
        };

        ins.run(&mut cpu);

        assert_eq!(cpu.mem.get_word(addr), 0xFE41);
        assert_eq!(cpu.get_reg(1), 0x3AFF001); 
    }
}