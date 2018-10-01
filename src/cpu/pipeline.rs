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
    /// A decoded instruction
    Decoded(Instruction)
}

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
    } else if op0 == 1 && op2 == 1 {
        Some(BranchEx(branch_ex::BranchAndExchange::parse_instruction(ins)))
    } else if op0 < 2 && (op2 == 9 || op2 == 11 || op2 == 13 || op2 == 15) {
        // if bits 4 and 7 are 1, this must be a signed/hw transfer
        Some(SignedTransfer(signed_trans::SignedDataTransfer::parse_instruction(ins)))
    } else if op0 < 4 {
        let data = data::DataProc::parse_instruction(ins);
        let op = data.opcode as u8;
        if !data.set_flags && op > 7 && op < 12 {
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
    Noop
}



/// Return whether the current state of the CPU's flags matches the given condition
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
                    _ => false
                })
            )
        }

        #[test]
        fn branch() {
            has_type!(0x0_A_123456, Instruction::Branch(_));
            has_type!(0x0_B_123456, Instruction::Branch(_));
        }

        #[test]
        fn bex() {
            has_type!(0x0_12FFF1_5, Instruction::BranchEx(_));
        }

        #[test]
        fn data() {
            has_type!(0x03123456, Instruction::DataProc(_));
            has_type!(0xA3123456, Instruction::DataProc(_));
            has_type!(0x001A3D56, Instruction::DataProc(_));
        }

        #[test]
        fn mul() {
            has_type!(0x03_123_9_A, Instruction::Multiply(_));
            has_type!(0x02_ABC_9_0, Instruction::Multiply(_));
        }

        #[test]
        fn mul_long() {
            has_type!(0x08_123_9_A, Instruction::MultiplyLong(_));
            has_type!(0x0B_ABC_9_0, Instruction::MultiplyLong(_));
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
        fn single_trans() {
            has_type!(0xA_4_123456, Instruction::SingleTransfer(_));
            has_type!(0xA_7_ABCDEF, Instruction::SingleTransfer(_));
        }

        #[test]
        fn block_trans() {
            has_type!(0x0_8_123456, Instruction::BlockTransfer(_));
            has_type!(0x0_9_1DFA10, Instruction::BlockTransfer(_));
        }

        #[test]
        fn sw_interrupt() {
            has_type!(0xFF_123ABC, Instruction::SWInterrupt(_));
        }

        #[test]
        fn swap() {
            has_type!(0xF_1_0_123_9_5, Instruction::SwapTransfer(_));
            has_type!(0xF_1_8_ABC_9_E, Instruction::SwapTransfer(_));
        }

        #[test]
        fn signed_halfword_transfer() {
            has_type!(0xF_1_0BE0_B_3, Instruction::SignedTransfer(_));
            has_type!(0xF_0_FABC_D_3, Instruction::SignedTransfer(_));
            has_type!(0xF_0_7123_F_3, Instruction::SignedTransfer(_));
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