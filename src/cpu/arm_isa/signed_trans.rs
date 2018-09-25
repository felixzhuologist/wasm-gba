use super::{Instruction, InstructionType, RegOrImm};
use ::cpu::{CPU, TransferParams, TransferSize};
use ::util;

/// Load or store a half words of data from memory and also load sign-extended
/// bytes/halfwords. The memory address is calculated by adding/subtracting an
/// offset from a base register, which can be written back into the base register
/// if auto-indexing is required
pub struct SignedDataTransfer {
    /// if true, add offset before transfer else add after
    pre_index: bool,
    /// if true, add the offset to base, else subtract it
    offset_up: bool,
    /// if true, transfer halfword, else byte
    halfword: bool,
    /// if true, write address back to base reg, else do nothing
    write_back: bool,
    /// if true, load from memory, else write to memory
    load: bool,
    /// base register
    rn: usize,
    /// source/destination register
    rd: usize,
    /// if true, treat as signed, otherwise as unsigned
    signed: bool,
    /// offset register
    offset: RegOrImm
}

impl SignedDataTransfer {
    /// for register offset, then immediate offset:
    /// 27 .. 25 | 24 | 23 | 22 | 21 | 20 | 19 .. 16 | 15 .. 12 | 11 .. 8 | 7 | 6 | 5 | 4 | 3 .. 0
    ///   000    | P  | U  | 0  | W  | L  |    Rn    |    Rd    |  0000   | 1 | S | H | 1 |   Rm
    ///   000    | P  | U  | 1  | W  | L  |    Rn    |    Rd    |  hi     | 1 | S | H | 1 |   lo
    pub fn parse_instruction(ins: u32) -> SignedDataTransfer {
        let is_imm = util::get_bit(ins, 22);
        SignedDataTransfer {
            pre_index: util::get_bit(ins, 24),
            offset_up: util::get_bit(ins, 23),
            write_back: util::get_bit(ins, 21),
            load: util::get_bit(ins, 20),
            rn: util::get_nibble(ins, 16) as usize,
            rd: util::get_nibble(ins, 12) as usize,
            signed: util::get_bit(ins, 6),
            halfword: util::get_bit(ins, 5),
            offset: if is_imm {
                RegOrImm::Imm {
                    rotate: 0,
                    value: (util::get_nibble(ins, 8) << 4) | util::get_nibble(ins, 0)
                }
            } else {
                RegOrImm::Reg { shift: 0, reg: util::get_nibble(ins, 0) }
            }
        }
    }
}

impl Instruction for SignedDataTransfer {
    fn get_type(&self) -> InstructionType { InstructionType::SignedDataTransfer }
    fn run(&self, cpu: &mut CPU) {
        if !self.load && self.signed {
            panic!("should not store when signed operations have been selected");
        }

        // all the same, except you can load as signed (which means that when
        // you sign extended the value before you store in register, and with
        // different quantities)
        cpu.transfer_reg(TransferParams {
            pre_index: self.pre_index,
            offset_up: self.offset_up,
            size: if self.halfword { TransferSize::Halfword } else { TransferSize::Byte },
            write_back: self.write_back,
            load: self.signed || self.load, // always load if S bit is 1
            base_reg: self.rn,
            data_reg: self.rd,
            signed: self.signed,
            offset: &self.offset
        });
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn parse_reg() {
        let ins = SignedDataTransfer::parse_instruction(
            0b1111_000_0_1_0_1_0_0001_0010_00001_1_1_1_0011);
        assert!(!ins.pre_index);
        assert!(ins.offset_up);
        assert!(ins.write_back);
        assert!(!ins.load);
        assert_eq!(ins.rn, 1);
        assert_eq!(ins.rd, 2);
        assert!(ins.signed);
        assert!(ins.halfword);
        assert!(match ins.offset {
            RegOrImm::Reg { shift: 0, reg: 3 } => true,
            _ => false,
        });
    }

    #[test]
    fn parse_imm() {
        let ins = SignedDataTransfer::parse_instruction(
            0b1111_000_1_0_1_1_0_0001_0010_1100_1_1_0_1_0011);
        assert!(ins.pre_index);
        assert!(!ins.offset_up);
        assert!(ins.write_back);
        assert!(!ins.load);
        assert_eq!(ins.rn, 1);
        assert_eq!(ins.rd, 2);
        assert!(ins.signed);
        assert!(!ins.halfword);
        assert!(match ins.offset {
            RegOrImm::Imm { rotate: 0, value: 0xC3 } => true,
            _ => false,
        });
    }
}