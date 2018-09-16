use super::{Instruction, InstructionType, RegOrImm};
use super::data::apply_shift;
use ::cpu::CPU;
use ::util;

/// Load or store a single byte/word to/from memory. The memory address is
/// calculated by adding/subtracting an offset from a base register, which can
/// be written back into the base register if auto-indexing is required
pub struct SingleDataTransfer {
    /// if true, add offset before transfer else add after
    pre_index: bool,
    /// if true, add the offset to base, else subtract it
    offset_up: bool,
    /// if true, transfer byte, else transfer word
    byte: bool,
    /// if true, write address back to base reg, else do nothing
    write_back: bool,
    /// if true, load from memory, else write to memory
    load: bool,
    /// base register
    rn: usize,
    /// source/destination register
    rd: usize,
    /// offset which is either a 12 bit immediate value, or a shifted register.
    /// note that specifying shift amount using another register is not supported
    /// for this instruction
    offset: RegOrImm,
}

impl SingleDataTransfer {
    /// 25 | 24 | 23 | 22 | 21 | 20 | 19 .. 16 | 15 .. 12 | 11 .. 0
    /// I  | P  | U  | B  | W  | L  |    Rn    |    Rd    |  offset
    pub fn parse_instruction(ins: u32) -> SingleDataTransfer {
        let is_imm = !util::get_bit(ins, 25);
        SingleDataTransfer {
            pre_index: util::get_bit(ins, 24),
            offset_up: util::get_bit(ins, 23),
            byte: util::get_bit(ins, 22),
            write_back: util::get_bit(ins, 21),
            load: util::get_bit(ins, 20),
            rn: util::get_nibble(ins, 16) as usize,
            rd: util::get_nibble(ins, 12) as usize,
            offset: if is_imm {
                RegOrImm::Imm { rotate: 0, value: ins & 0xFFF }
            } else {
                RegOrImm::Reg {
                    shift: util::get_byte(ins, 4),
                    reg: util::get_nibble(ins, 0)
                }
            }
        }
    }
}

impl Instruction for SingleDataTransfer {
    fn get_type(&self) -> InstructionType { InstructionType::SingleDataTransfer }
    fn process_instruction(&self, cpu: &mut CPU) {
        if self.rn == 15 && self.write_back {
            panic!("cannot write back when R15 is the base register");
        }
        if let RegOrImm::Reg { shift: _, reg: rm } = self.offset {
            if rm == 15 {
                panic!("R15 cannot be used as the register offset");
            }
            if rm as usize == self.rn {
                panic!("Rm should not equal Rn");
            }
        }

        // pre transfer
        let mut addr = cpu.get_reg(self.rn);
        let offset = get_offset(cpu, &self.offset);
        if self.pre_index {
            if self.offset_up {
                addr += offset;
            } else {
                addr -= offset;
            }
        }

        // transfer
        if self.load {
            let val = if self.byte { cpu.mem.get_byte(addr) as u32 } else { cpu.mem.get_word(addr) };
            cpu.set_reg(self.rd, val);
        } else {
            let mut val = cpu.get_reg(self.rd);
            if self.rd == 15 {
                // when R15 is the source of a STR, the stored value will be the
                // addr of the current instruction + 12
                val += 4;
            }
            if self.byte {
                cpu.mem.set_byte(addr, val as u8);
            } else {
                cpu.mem.set_word(addr, val);
            }
        }

        // post transfer
        if !self.pre_index {
            if self.offset_up {
                addr += offset;
            } else {
                addr -= offset;
            }
        }

        // write back is assumed if post indexing
        if !self.pre_index || self.write_back {
            cpu.set_reg(self.rn, addr);
        }
    }
}

fn get_offset(cpu: &CPU, offset: &RegOrImm) -> u32 {
    match *offset {
        RegOrImm::Imm { rotate: _, value: n } => n,
        RegOrImm::Reg { shift: s, reg: r } => {
            if util::get_bit(s, 3) && util::get_bit(s, 0) {
                panic!("cannot use register value as shift amount for LDR/STR");
            }
            apply_shift(cpu, s, r).0
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn parse_imm() {
        let ins = SingleDataTransfer::parse_instruction(
            0b1111_01_0_1_0_1_0_1_0001_0010_100010001000);
        assert!(ins.pre_index);
        assert!(!ins.offset_up);
        assert!(ins.byte);
        assert!(!ins.write_back);
        assert!(ins.load);
        assert_eq!(ins.rn, 1);
        assert_eq!(ins.rd, 2);
        assert!(match ins.offset {
            RegOrImm::Imm { rotate: 0, value: 0x888 } => true,
            _ => false,
        });
    }

    #[test]
    fn parse_reg() {
        let ins = SingleDataTransfer::parse_instruction(
            0b1001_01_1_0_1_0_1_0_1110_0001_00111111_1001);
        assert!(!ins.pre_index);
        assert!(ins.offset_up);
        assert!(!ins.byte);
        assert!(ins.write_back);
        assert!(!ins.load);
        assert_eq!(ins.rn, 14);
        assert_eq!(ins.rd, 1);
        assert!(match ins.offset {
            RegOrImm::Reg { shift: 63, reg: 9 } => true,
            _ => false,
        });
    }

    #[test]
    fn load() {
        let ins = SingleDataTransfer {
            pre_index: true,
            offset_up: true,
            byte: true,
            write_back: false,
            load: true,
            rn: 0,
            rd: 1,
            offset: RegOrImm::Imm { rotate: 0, value: 20 }
        };
        let mut cpu = CPU::new();
        cpu.set_reg(0, 80);
        cpu.mem.set_byte(100, 77);
        ins.process_instruction(&mut cpu);
        assert_eq!(cpu.get_reg(1), 77);
    }

    #[test]
    fn store_autoindex() {
        let ins = SingleDataTransfer {
            pre_index: false,
            offset_up: false,
            byte: true,
            write_back: false,
            load: false,
            rn: 0,
            rd: 1,
            offset: RegOrImm::Imm { rotate: 0, value: 20 }
        };
        let mut cpu = CPU::new();
        cpu.set_reg(0, 100);
        cpu.set_reg(1, 77);
        ins.process_instruction(&mut cpu);
        assert_eq!(cpu.mem.get_byte(100), 77);
        assert_eq!(cpu.get_reg(0), 80);
    }
}