use super::{Instruction, InstructionType, RegOrImm};
use ::cpu::{CPU, TransferParams, TransferSize};
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
    fn run(&self, cpu: &mut CPU) {
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

        cpu.transfer_reg(TransferParams {
            pre_index: self.pre_index,
            offset_up: self.offset_up,
            size: if self.byte { TransferSize::Byte } else { TransferSize::Word },
            write_back: self.write_back,
            load: self.load,
            base_reg: self.rn,
            data_reg: self.rd,
            signed: false,
            offset: &self.offset
        });
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
}