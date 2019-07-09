//! Utilities for emulating the CPU's instruction pipeline
use self::Instruction::{
    DataProc,
    PSRTransfer,
    Multiply,
    MultiplyLong,
    SwapTransfer,
    SingleTransfer,
    SignedTransfer,
    BlockTransfer,
    Branch,
    BranchEx,
    SWInterrupt,
};
use ::cpu::arm::{
    block_trans,
    branch,
    branch_ex,
    data,
    mul,
    mul_long,
    psr,
    signed_trans,
    single_trans,
    swap,
    swi,
};
use ::cpu::thumb;
use ::cpu::status_reg::PSR;
use util;
use num::FromPrimitive;

/// An instruction in a specific stage of the ARM7's three stage pipeline
pub enum PipelineInstruction {
    /// A not yet fetched instruction. This is a placeholder for when the
    /// pipeline has just been flushed and the CPU is stalling waiting for the
    /// next instruction to be fetched
    Empty,
    /// A fetched ARM instruction
    RawARM(u32),
    /// A fetched THUMB instruction
    RawTHUMB(u16),
    // TODO: change the Option<u32> to an Option<CondField> instead since we
    // don't need the rest of the bits
    /// A decoded instruction, containing both the original raw instruction
    /// as well as the parsed Instruction
    Decoded(Option<u32>, Instruction)
}

/// Decode a raw ARM instruction
// NOTE: this will incorrectly parse some undefined instructions, but we assume
// that games will never run those
pub fn decode_arm(ins: u32) -> Option<Instruction> {
    let op0 = util::get_nibble(ins, 24);
    let op1 = util::get_nibble(ins, 20);
    let op2 = util::get_nibble(ins, 4);
    if op0 == 0 && op1 < 4 && op2 == 0b1001 {
        Some(Multiply(mul::Multiply::parse_instruction(ins)))
    } else if op0 == 0 && op1 > 7 && op2 == 0b1001 {
        Some(MultiplyLong(mul_long::MultiplyLong::parse_instruction(ins)))
    } else if op0 == 1 && op2 == 9 {
        Some(SwapTransfer(swap::SingleDataSwap::parse_instruction(ins)))
    } else if (ins & 0x0FFFFFF0) == 0x012FFF10 {
        Some(BranchEx(branch_ex::BranchAndExchange::parse_instruction(ins)))
    } else if op0 < 2 && (op2 == 9 || op2 == 11 || op2 == 13 || op2 == 15) {
        // if bits 4 and 7 are 1, this must be a signed/hw transfer
        Some(SignedTransfer(signed_trans::SignedDataTransfer::parse_instruction(ins)))
    } else if op0 < 4 {
        let data = data::DataProc::parse_instruction(ins);
        let op = data.opcode as u8;
        // PSR instructions are Data Processing operations with TST, TEQ, CMP,
        // or CMN, without the S flag set
        if !data.set_flags && op >= 8 && op <= 11 {
            Some(PSRTransfer(psr::PSRTransfer::parse_instruction(ins)))
        } else {
            Some(DataProc(data))
        }
    } else if op0 >= 4 && op0 < 8 {
        Some(SingleTransfer(single_trans::SingleDataTransfer::parse_instruction(ins)))
    } else if op0 == 8 || op0 == 9 {
        Some(BlockTransfer(block_trans::BlockDataTransfer::parse_instruction(ins)))
    } else if op0 == 10 || op0 == 11 {
        Some(Branch(branch::Branch::parse_instruction(ins)))
    } else if op0 == 15 {
        Some(SWInterrupt(swi::SWInterrupt::parse_instruction(ins)))
    } else {
        None
    }
}

/// Decode a raw thumb instruction
pub fn decode_thumb(ins: u16) -> Instruction {
    // this intermediate function exists to be able to test that the correct
    // THUMB format is identified
    _decode_thumb(ins)(ins)
}

// NOTE: this doesn't check for invalid instructions - it only looks at the minimum
// number of bits necessary to decide between valid THUMB formats
fn _decode_thumb(ins: u16) -> (fn(u16) -> Instruction) {
    // use binary on left to make it easier to compare to the reference doc
    match (ins >> 12) & 0xF {
        0b0000 => thumb::move_,
        0b0001 =>
            if util::get_bit_hw(ins, 11)
                { thumb::add_sub } else
                { thumb::move_ },
        0b0010 |
        0b0011 => thumb::data_imm,
        0b0100 => {
            let comp = (ins >> 10) & 0b11;
            match comp {
                0 => thumb::alu_op,
                1 => thumb::hi_reg_bex,
                _ => thumb::pc_rel_load,
            }
        },
        0b0101 => {
            if util::get_bit_hw(ins, 9)
                { thumb::signed_trans } else
                { thumb::reg_offset_trans }
        },
        0b0110 |
        0b0111 => thumb::imm_offset_trans,
        0b1000 => thumb::hw_trans,
        0b1001 => thumb::sp_rel_trans,
        0b1010 => thumb::load_addr,
        0b1011 => {
            if util::get_bit_hw(ins, 10)
                { thumb::push_pop } else
                { thumb::incr_sp }
        },
        0b1100 => thumb::block_trans,
        0b1101 => {
            if (ins >> 8) & 0xF == 0xF
                { thumb::swi } else
                { thumb::cond_branch }
        },
        0b1110 => thumb::branch,
        0b1111 => thumb::long_branch,
        _ => panic!("should not get here")
    }
}

/// The possible instructions of the ARM instruction set
#[derive(Clone, Debug)]
pub enum Instruction {
    DataProc(data::DataProc),
    PSRTransfer(psr::PSRTransfer),
    Multiply(mul::Multiply),
    MultiplyLong(mul_long::MultiplyLong),
    SwapTransfer(swap::SingleDataSwap),
    SingleTransfer(single_trans::SingleDataTransfer),
    SignedTransfer(signed_trans::SignedDataTransfer),
    BlockTransfer(block_trans::BlockDataTransfer),
    Branch(branch::Branch),
    BranchEx(branch_ex::BranchAndExchange),
    SWInterrupt(swi::SWInterrupt),
    CondBranch(thumb::CondBranch),
    LongBranch(thumb::LongBranch),
}

/// Return whether the current state of the CPU's flags satisfies the condition
/// field of the given raw instruction
pub fn satisfies_cond(cpsr: &PSR, cond: u32) -> bool {
    match CondField::from_u32(cond).unwrap() {
        CondField::EQ => cpsr.zero,
        CondField::NE => !cpsr.zero,
        CondField::CS => cpsr.carry,
        CondField::CC => !cpsr.carry,
        CondField::MI => cpsr.neg,
        CondField::PL => !cpsr.neg,
        CondField::VS => cpsr.overflow,
        CondField::VC => !cpsr.overflow,
        CondField::HI => cpsr.carry && !cpsr.overflow,
        CondField::LS => !cpsr.carry || cpsr.overflow,
        CondField::GE => cpsr.neg == cpsr.overflow,
        CondField::LT => cpsr.neg != cpsr.overflow,
        CondField::GT => !cpsr.zero && (cpsr.neg == cpsr.overflow),
        CondField::LE => cpsr.zero || (cpsr.neg != cpsr.overflow),
        CondField::AL => true
    }
}

/// Each ARM instruction's most significant 4 bits contain a condition field
/// which is compared with the CPU's CPSR register to determine if the instruction
/// should be executed
enum_from_primitive! {
#[repr(u8)]
pub enum CondField {
    EQ = 0,
    NE,
    CS,
    CC,
    MI,
    PL,
    VS,
    VC,
    HI,
    LS,
    GE,
    LT,
    GT,
    LE,
    AL
}
}

#[cfg(test)]
mod test {
    mod decode_arm {
        use super::super::*;

        macro_rules! has_type {
            ($instr:expr, $instr_type: pat) => (
                assert!(match decode_arm($instr).unwrap() {
                    $instr_type => true,
                    result => {
                        println!("got: {:?}", result);
                        false
                    }
                })
            )
        }

        // for decode_arm()'s condition branches that are based exclusively on the op0, op1,
        // and op2 nibbles (which is all but 2), we can effectively test against all possible options
        // by enumerating all possible values for op0, op1, and op2. we do this
        // using generate_instructions(), which will generate all possible values
        // after possibly fixing any of the 12 bits (specified using a NibbleFilter).
        // we can then write tests for each instruction by copying fixed values
        // directly from the instruction table in the ARM manual (Figure 4-1)

        type NibbleFilter = (Option<u32>, Option<u32>, Option<u32>, Option<u32>);

        fn generate_instructions(f0: &NibbleFilter, f1: &NibbleFilter, f2: &NibbleFilter) -> Vec<u32> {
            let mut instructions = Vec::new();
            for op0 in generate_nibbles(f0) {
                for op1 in generate_nibbles(f1) {
                    for op2 in generate_nibbles(f2) {
                        instructions.push((op0 << 24) | (op1 << 20) | (op2 << 4))
                    }
                }
            }
            instructions
        }

        fn generate_nibbles(filter: &NibbleFilter) -> Vec<u32> {
            (0u32..16u32).filter(|num| matches_filter(num, filter)).collect()
        }

        fn matches_filter(num: &u32, filter: &NibbleFilter) -> bool {
            let (f3, f2, f1, f0) = filter;
            f3.map_or(true, |val| ((num >> 3) & 1u32) == val) &&
            f2.map_or(true, |val| ((num >> 2) & 1u32) == val) &&
            f1.map_or(true, |val| ((num >> 1) & 1u32) == val) &&
            f0.map_or(true, |val| ((num >> 0) & 1u32) == val)
        }

        #[test]
        fn data() {
            has_type!(0x03123456, Instruction::DataProc(_));
            has_type!(0xA3123456, Instruction::DataProc(_));
            has_type!(0x001A3D56, Instruction::DataProc(_));
            has_type!(0b11100001101000000110100100010110, Instruction::DataProc(_));
        }

        #[test]
        fn psr() {
            has_type!(
                0b0011_00010_1_001111_0000_000000000000,
                Instruction::PSRTransfer(_));
            has_type!(
                0b1111_00010_0_1010011111_00000000_1111,
                Instruction::PSRTransfer(_));
        }

        #[test]
        fn mul() {
            let f0 = (Some(0), Some(0), Some(0), Some(0));
            let f1 = (Some(0), Some(0), None, None);
            let f2 = (Some(1), Some(0), Some(0), Some(1));
            for ins in generate_instructions(&f0, &f1, &f2) {
                has_type!(ins, Instruction::Multiply(_));
            }
        }

        #[test]
        fn mul_long() {
            let f0 = (Some(0), Some(0), Some(0), Some(0));
            let f1 = (Some(1), None, None, None);
            let f2 = (Some(1), Some(0), Some(0), Some(1));
            for ins in generate_instructions(&f0, &f1, &f2) {
                has_type!(ins, Instruction::MultiplyLong(_));
            }
        }

        #[test]
        fn single_data_swap() {
            let f0 = (Some(0), Some(0), Some(0), Some(1));
            let f1 = (Some(0), None, Some(0), Some(0));
            let f2 = (Some(1), Some(0), Some(0), Some(1));
            for ins in generate_instructions(&f0, &f1, &f2) {
                has_type!(ins, Instruction::SwapTransfer(_));
            }
        }

        #[test]
        fn bex() {
            has_type!(0x0_12FFF1_5, Instruction::BranchEx(_));
        }

        #[test]
        fn signed_halfword_transfer() {
            let f0 = (Some(0), Some(0), Some(0), None);
            let f1 = (None, None, None, None);
            let f2 = (Some(1), None, None, Some(1));
            for ins in generate_instructions(&f0, &f1, &f2) {
                let op0 = util::get_nibble(ins, 24);
                let op1 = util::get_nibble(ins, 20);
                let op2 = util::get_nibble(ins, 4);
                // mul instruction takes precedence
                if op2 == 9 && op0 == 0 && (op1 <= 3 || op1 >= 8) {
                    continue;
                }
                // why ????
                if op2 == 9 && op0 == 1 {
                    continue;
                }
                has_type!(ins, Instruction::SignedTransfer(_));
            }
        }

        #[test]
        fn single_trans() {
            let f0 = (Some(0), Some(1), None, None);
            let f1 = (None, None, None, None);
            let f2 = (None, None, None, None);
            for ins in generate_instructions(&f0, &f1, &f2) {
                has_type!(ins, Instruction::SingleTransfer(_));
            }
        }

        #[test]
        fn block_trans() {
            let f0 = (Some(1), Some(0), Some(0), None);
            let f1 = (None, None, None, None);
            let f2 = (None, None, None, None);
            for ins in generate_instructions(&f0, &f1, &f2) {
                has_type!(ins, Instruction::BlockTransfer(_));
            }
        }

        #[test]
        fn branch() {
            let f0 = (Some(1), Some(0), Some(1), None);
            let f1 = (None, None, None, None);
            let f2 = (None, None, None, None);
            for ins in generate_instructions(&f0, &f1, &f2) {
                has_type!(ins, Instruction::Branch(_));
            }
        }

        #[test]
        fn sw_interrupt() {
            has_type!(0xFF_123ABC, Instruction::SWInterrupt(_));
        }
    }

    mod decode_thumb {
        use super::super::*;
        use ::cpu::thumb::*;

        macro_rules! has_format {
            ($instr:expr, $thumb_format: ident) => (
                assert!(_decode_thumb($instr) == $thumb_format))
        }

        #[test]
        fn sanity() {
            has_format!(0x0123, move_);
            has_format!(0x1012, move_);
            has_format!(0x1F12, add_sub);
            has_format!(0x2000, data_imm);
            has_format!(0x3FFF, data_imm);
            has_format!(0x4A00, pc_rel_load);
            has_format!(0x42FA, alu_op);
            has_format!(0x451A, hi_reg_bex);
            has_format!(0x51AB, reg_offset_trans);
            has_format!(0x5700, signed_trans);
            has_format!(0x6123, imm_offset_trans);
            has_format!(0x700F, imm_offset_trans);
            has_format!(0x8FFF, hw_trans);
            has_format!(0x9001, sp_rel_trans);
            has_format!(0xAAAB, load_addr);
            has_format!(0xB00A, incr_sp);
            has_format!(0xBD00, push_pop);
            has_format!(0xCEEA, block_trans);
            has_format!(0xDE01, cond_branch);
            has_format!(0xDF01, swi);
            has_format!(0xE590, branch);
            has_format!(0xF3C7, long_branch);
        }
    }
}