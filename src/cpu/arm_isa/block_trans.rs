use super::{Instruction, InstructionType, RegOrImm};
use ::cpu::CPU;
use ::util;

/// Load or store any subset of the currently visible registers
pub struct BlockDataTransfer {
    /// if true, add offset before transfer else add after
    pre_index: bool,
    /// if true, add the offset to base, else subtract it
    offset_up: bool,
    /// if true, load PSR or force user mode
    force: bool,
    /// if true, write address back to base reg, else do nothing
    write_back: bool,
    /// if true, load from memory, else write to memory
    load: bool,
    /// base register
    rn: usize,
    /// bit i of the register list being set means that register i should be transferred
    register_list: u16
}

impl BlockDataTransfer {
    /// 27 .. 25 | 24 | 23 | 22 | 21 | 20 | 19 .. 16 | 15 ...
    ///   100    | P  | U  | S  | W  | L  |    Rn    |  register list
    pub fn parse_instruction(ins: u32) -> BlockDataTransfer {
        BlockDataTransfer {
            pre_index: util::get_bit(ins, 24),
            offset_up: util::get_bit(ins, 23),
            force: util::get_bit(ins, 22),
            write_back: util::get_bit(ins, 21),
            load: util::get_bit(ins, 20),
            rn: util::get_nibble(ins, 16) as usize,
            register_list: ins as u16,
        }
    }
}

impl Instruction for BlockDataTransfer {
    fn get_type(&self) -> InstructionType { InstructionType::BlockDataTransfer }
    fn process_instruction(&self, cpu: &mut CPU) {
        unimplemented!()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn parse() {
        let ins = BlockDataTransfer::parse_instruction(
            0b1010_100_1_0_1_0_1_0101_1101100101100010);
        assert!(ins.pre_index);
        assert!(!ins.offset_up);
        assert!(ins.force);
        assert!(!ins.write_back);
        assert!(ins.load);
        assert_eq!(ins.rn, 5);
        assert_eq!(ins.register_list, 0b1101100101100010);
    }
}