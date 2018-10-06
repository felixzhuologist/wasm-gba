//! THUMB instructions
//!
//! Almost all THUMB instructions can be implemented as ARM instructions, so
//! most functions here simply decode a raw instruction into the appropriate
//! ARM instruction. The exceptions are format 16 (conditional brancH) and
//! format 19 (long branch), which have their own Instruction enum branches.

use num::FromPrimitive;
use ::cpu::CPU;
use ::cpu::arm::RegOrImm;
use ::cpu::arm::data::{DataProc, Op};
use ::cpu::arm::branch::Branch;
use ::cpu::arm::branch_ex::BranchAndExchange;
use ::cpu::arm::mul::Multiply;
use ::cpu::arm::single_trans::SingleDataTransfer;
use ::cpu::arm::signed_trans::SignedDataTransfer;
use ::cpu::arm::block_trans::BlockDataTransfer;
use ::cpu::arm::swi::SWInterrupt;
use ::cpu::pipeline::{Instruction, satisfies_cond};
use ::util;

/// format 1:
/// 14 | 13 | 12  11 | 10 ... 6 | 5 .. 3 | 2 .. 0
/// 0  | 0  |   op   | offset 5 |   Rs   |   Rd
pub fn move_(raw: u16) -> Instruction {
    // dataproc expects shift of the format: offset 5 | op | 0
    let shift_op = (raw as u32>> 10) & 0b110;
    if shift_op == 0b110 {
        panic!("cannot RSR in THUMB mode")
    }
    let imm = (raw as u32 >> 3) & 0b11111000;
    let rs = (raw as u32 >> 3) & 0b111;
    Instruction::DataProc(DataProc{
        opcode: Op::MOV,
        set_flags: true,
        rn: 0, // this is unused for MOV instructions. should this be an Option?
        rd: (raw & 0b111) as usize,
        op2: RegOrImm::Reg { shift: imm | shift_op, reg: rs }
    })
}

/// format 2:
/// 15 | 14 | 13 | 12 | 11 | 10 | 9  | 8 .. 6      | 5 .. 3 | 2 .. 0
/// 0  | 0  | 0  | 1  | 1  | I  | op | Rn/offset 3 |   Rs   |   Rd
pub fn add_sub(raw: u16) -> Instruction {
    let opcode = if (raw >> 9) & 1 == 1 { Op::SUB } else { Op::ADD };
    let val = (raw as u32 >> 6) & 0b111;
    let op2 = if (raw >> 10) & 1 == 1 {
        RegOrImm::Imm { rotate: 0, value: val }
    } else {
        RegOrImm::Reg { shift: 0, reg: val }
    };

    Instruction::DataProc(DataProc{
        opcode,
        set_flags: true,
        rn: ((raw >> 3) & 0b111) as usize,
        rd: (raw & 0b111) as usize,
        op2,
    })
}

/// format 3:
/// 15 | 14 | 13 | 12 11 | 10 .. 8 | 7 .. 0
/// 0  | 0  | 1  |  op   |    Rd   | offset 8      
pub fn data_imm(raw: u16) -> Instruction {
    let opcode = match (raw >> 11) & 0b11 {
        0 => Op::MOV,
        1 => Op::CMP,
        2 => Op::ADD,
        3 => Op::SUB,
        _ => panic!("should not get here")
    };
    let rd = ((raw >> 8) & 0b111) as usize;
    Instruction::DataProc(DataProc{
        opcode,
        set_flags: true,
        rn: rd,
        rd,
        op2: RegOrImm::Imm { rotate: 0, value: raw as u32 & 0xFF },
    })
}

/// format 4:
/// 15 | 14 | 13 | 12 | 11 | 10 | 9 .. 6 | 5 .. 3 | 2 .. 0
/// 0  | 0  | 1  |  0 | 0  | 0  |   op   |   Rs   |   Rd
pub fn alu_op(raw: u16) -> Instruction {
    let rd = (raw & 0b111) as usize;
    let rs = (raw >> 3) & 0b111;
    let op = (raw >> 6) & 0xF;

    // MOV instruction
    if op == 0b0010 || op == 0b0011 || op == 0b0100 || op == 0b0111 {
        // Rs4 0 op 1
        let rs = (raw << 1) & 0x70;
        let shift_nibble_lo = match op {
            0b0010 => 0b0001, // Rd := Rd << Rs
            0b0011 => 0b0011, // Rd := Rd >> Rs
            0b0100 => 0b0101, // Rd := Rd ASR Rs
            0b0111 => 0b0111, // Rd := Rd ROR Rs
            _ => panic!("should not get here")
        };
        let op2 = RegOrImm::Reg {
            shift: (rs | shift_nibble_lo) as u32,
            reg: rd as u32
        };
        Instruction::DataProc(DataProc {
            opcode: Op::MOV,
            set_flags: true,
            rn: 0, // unused for MOV
            rd,
            op2,
        })
    } else if op == 0b1001 { // RSBS Rd, Rs, #0 (Rd = -Rs)
        Instruction::DataProc(DataProc {
            opcode: Op::RSB,
            set_flags: true,
            rn: rs as usize,
            rd,
            op2: RegOrImm::Imm { rotate: 0, value: 0 }
        })
    } else if op == 0b1101 { // MUL instruction
        Instruction::Multiply(Multiply {
            accumulate: false,
            set_flags: true,
            rd,
            rn: 0, // unused when accumulate = false
            rs: rs as usize,
            rm: rd,
        })
    } else { // data instruction
        Instruction::DataProc(DataProc {
            opcode: Op::from_u16((raw >> 6) & 0xF).unwrap(),
            set_flags: true,
            rn: rd,
            rd,
            op2: RegOrImm::Reg { shift: 0, reg: rs as u32 }
        })
    }
}

/// format 5: allows ADD/CMP/MOV/BX on regs 8-15
/// 15 | 14 | 13 | 12 | 11 | 10 | 9 8 | 7 | 6 | 5 .. 3 | 2 .. 0
/// 0  | 1  | 0  | 0  | 0  | 1  | Op  |H1 |H2 | Rs/Hs  |  Rd/Hd
// TODO: ADD/CMP/MOV on both low regs should be undefined
pub fn hi_reg_bex(raw: u16) -> Instruction {
    let mut rd = raw & 0b111;
    let mut rs = (raw >> 3) & 0b111;
    if util::get_bit_hw(raw, 7) {
        rd += 8;
    }
    if util::get_bit_hw(raw, 6) {
        rs += 8;
    }
    
    match (raw >> 8) & 0b11 {
        0 => {
            Instruction::DataProc(DataProc {
                opcode: Op::ADD,
                set_flags: false,
                rn: rd as usize,
                rd: rd as usize,
                op2: RegOrImm::Reg { shift: 0, reg: rs as u32 }
            })
        },
        1 => {
            Instruction::DataProc(DataProc {
                opcode: Op::CMP,
                set_flags: true,
                rn: rd as usize,
                rd: 0, // unused for CMP,
                op2: RegOrImm::Reg { shift: 0, reg: rs as u32 }
            })
        },
        2 => {
            Instruction::DataProc(DataProc {
                opcode: Op::MOV,
                set_flags: false,
                rn: 0, // unused for MOV
                rd: rd as usize,
                op2: RegOrImm::Reg { shift: 0, reg: rs as u32 }
            })
        },
        3 => {
            Instruction::BranchEx(BranchAndExchange { reg: rs as usize })
        }
        _ => panic!("should not get here")
    }
}

/// format 6: pc relative load (LDR Rd, [R15, #Imm])
/// 15 | 14 | 13 | 12 | 11 | 10 .. 8 | 7 .. 0
/// 0  | 1  | 0  | 0  | 1  |    Rd   | Word8
pub fn pc_rel_load(raw: u16) -> Instruction {
    let rd = (raw as usize >> 8) & 0b111;
    Instruction::SingleTransfer(SingleDataTransfer {
        pre_index: true,
        offset_up: true,
        byte: false,
        write_back: false,
        load: true,
        rn: 15,
        rd,
        offset: RegOrImm::Imm { rotate: 0, value: (raw as u32 & 0xFF) << 2 }
    })
}

/// format 7: register offset transfer
/// 15 | 14 | 13 | 12 | 11 | 10 | 9 | 8 .. 6 | 5 .. 3 | 2 .. 0
/// 0  | 1  | 0  | 1  | L  | B  | 0 |  Ro    |   Rb   |   Rd
pub fn reg_offset_trans(raw: u16) -> Instruction {
    Instruction::SingleTransfer(SingleDataTransfer {
        pre_index: true,
        offset_up: true,
        byte: util::get_bit_hw(raw, 10),
        write_back: false,
        load: util::get_bit_hw(raw, 11),
        rn: (raw as usize >> 3) & 0b111,
        rd: raw as usize & 0b111,
        offset: RegOrImm::Reg { shift: 0, reg: (raw as u32 >> 6) & 0b111 }
    })
}

/// format 8: sign-extended transfer
/// 15 | 14 | 13 | 12 | 11 | 10 | 9 | 8 .. 6 | 5 .. 3 | 2 .. 0
/// 0  | 1  | 0  | 1  | H  | S  | 1 |  Ro    |   Rb   |   Rd
pub fn signed_trans(raw: u16) -> Instruction {
    let hflag = util::get_bit_hw(raw, 11);
    let signed = util::get_bit_hw(raw, 10);
    let (load, halfword) = match (signed, hflag) {
        (false, false) => (false, true), // store halfword
        (false, true) => (true, true), // load halfword
        (true, false) => (true, false), // load sign extended byte
        (true, true) => (true, true), // load sign extended halfword
    };
    Instruction::SignedTransfer(SignedDataTransfer {
        pre_index: true,
        offset_up: true,
        halfword,
        write_back: false,
        load,
        rn: (raw as usize >> 3) & 0b111,
        rd: raw as usize & 0b111,
        signed,
        offset: RegOrImm::Reg { shift: 0, reg: (raw as u32 >> 6) & 0b111 }
    })
}


/// format 9: immediate offset transfer
/// 15 | 14 | 13 | 12 | 11 | 10 .. 6 | 5 .. 3 | 2 .. 0
/// 0  | 1  | 1  | B  | L  | offset5 |   Rb   |   Rd
pub fn imm_offset_trans(raw: u16) -> Instruction {
    let imm = (raw as u32 >> 6) & 0b11111;
    let byte = util::get_bit_hw(raw, 12);
    Instruction::SingleTransfer(SingleDataTransfer {
        pre_index: true,
        offset_up: true,
        byte,
        write_back: false,
        load: util::get_bit_hw(raw, 11),
        rn: (raw as usize >> 3) & 0b111,
        rd: raw as usize & 0b111,
        offset: RegOrImm::Imm {
            rotate: 0,
            value: if byte { imm } else { imm << 2 }
        }
    })
}

/// format 10: hw transfer
/// 15 | 14 | 13 | 12 | 11 | 10 .. 6 | 5 .. 3 | 2 .. 0
/// 1  | 0  | 0  | 0  | L  | offset5 |   Rb   |   Rd
pub fn hw_trans(raw: u16) -> Instruction {
    let imm = (raw as u32 >> 5) & 0b111110;
    Instruction::SignedTransfer(SignedDataTransfer {
        pre_index: true,
        offset_up: true,
        halfword: true,
        write_back: false,
        load: util::get_bit_hw(raw, 11),
        rn: (raw as usize >> 3) & 0b111,
        rd: raw as usize & 0b111,
        signed: false,
        offset: RegOrImm::Imm { rotate: 0, value: imm }
    })
}

/// format 11: sp rel transfer
/// 15 | 14 | 13 | 12 | 11 | 10 .. 8 | 7 .. 0
/// 1  | 0  | 0  | 1  | L  |    Rd   | offset8
pub fn sp_rel_trans(raw: u16) -> Instruction {
    Instruction::SingleTransfer(SingleDataTransfer {
        pre_index: true,
        offset_up: true,
        byte: false,
        write_back: false,
        load: util::get_bit_hw(raw, 11),
        rn: 13,
        rd: (raw as usize >> 8) & 0b111,
        offset: RegOrImm::Imm { rotate: 0, value: raw as u32 & 0xFF }
    })
}

/// format 12: load addr: adds a 10 bit constant to either the PC or SP
/// 15 | 14 | 13 | 12 | 11 | 10 .. 8 | 7 .. 0
/// 1  | 0  | 1  | 0  | SP |    Rd   | word8
pub fn load_addr(raw: u16) -> Instruction {
    let rn: usize = if util::get_bit_hw(raw, 11) { 13 } else { 15 };
    Instruction::DataProc(DataProc{
        opcode: Op::ADD,
        set_flags: false,
        rn,
        rd: (raw as usize >> 8) & 0b111,
        op2: RegOrImm::Imm { rotate: 0, value: (raw as u32 & 0xFF) << 2}
    })
}

/// format 13: add 9 bit signed constant to the SP
/// 15 .. 8  | 7 | 6 .. 0
/// 10110000 | S | SWord7
pub fn incr_sp(raw: u16) -> Instruction {
    let opcode = match util::get_bit_hw(raw, 7) {
        false => Op::ADD,
        true => Op::SUB,
    };
    Instruction::DataProc(DataProc{
        opcode,
        set_flags: false,
        rn: 13,
        rd: 13,
        op2: RegOrImm::Imm { rotate: 0, value: (raw as u32 & 0x7F) << 2}
    })
}

/// format 14: push R0-7/LR to the stack, or pop R0-7/PC from the stack
/// 15 | 14 | 13 | 12 | 11 | 10 | 9 | 8 | 7 .. 0
/// 1  | 0  | 1  | 1  | L  | 1  | 0 | R |  Rlist
pub fn push_pop(raw: u16) -> Instruction {
    let pop = util::get_bit_hw(raw, 11);
    let mut register_list = raw & 0xFF;
    if util::get_bit_hw(raw, 8) {
        let extra_reg = if pop { 15 } else { 14 };
        register_list |= 1 << extra_reg;
    }

    // pre decr store if pushing, post incr load if popping
    Instruction::BlockTransfer(BlockDataTransfer {
        pre_index: !pop,
        offset_up: pop,
        force: false,
        write_back: true,
        load: pop,
        rn: 13,
        register_list,
    })
}

/// format 15: postincr store/load - STMIA/LDMIA Rb!,{ RList }
/// 15 | 14 | 13 | 12 | 11 | 10 .. 8 | 7 .. 0
/// 1  | 1  | 0  | 0  | L  |    Rb   | Rlist
pub fn block_trans(raw: u16) -> Instruction {
    Instruction::BlockTransfer(BlockDataTransfer {
        pre_index: false,
        offset_up: true,
        force: false,
        write_back: true,
        load: util::get_bit_hw(raw, 11),
        rn: (raw as usize >> 8) & 0b111,
        register_list: raw & 0xFF
    })
}

/// format 16:
/// 15 | 14 | 13 | 12 | 11 .. 8 | 7 .. 0
/// 1  | 1  | 0  | 1  |  cond   | soffset8
pub fn cond_branch(raw: u16) -> Instruction {
    let mut offset = raw & 0xFF;
    if util::get_bit_hw(offset, 7) {
        offset |= 0xFF00;
    }

    Instruction::CondBranch(CondBranch {
        cond: (raw >> 8) & 0xF,
        offset: (offset as i16) << 1,
    })
}

// TODO: this extra instruction probably isn't necessary if decode_thumb returns
// an (Option<Cond>, Instruction) that gets passed to Decoded()
#[derive(Clone, Debug)]
pub struct CondBranch { pub cond: u16, offset: i16 }

// for ARM instructions the condition is checked while decoding but for THUMB
// instructions they are checked during execution, since only one THUMB
// instruction is executed conditionally
impl CondBranch {
    pub fn run(&self, cpu: &mut CPU) -> u32 {
        if satisfies_cond(&cpu.cpsr, self.cond as u32) {
            let old_pc = cpu.r[15];
            cpu.modify_pc(self.offset as i64);
            cpu.mem.access_time(old_pc, false) +
                cpu.mem.access_time(cpu.r[15], true) +
                cpu.mem.access_time(cpu.r[15] + 4, false)
        } else {
            1
        }
    }
}

/// format 17: SWI
/// 15 .. 8  | 7 .. 0
/// 11011111 | value8
pub fn swi(raw: u16) -> Instruction {
    Instruction::SWInterrupt(SWInterrupt { comment: raw as u32 & 0xFF })
}

/// format 18: unconditional branch
/// 15 .. 11 | 10 .. 0
///  11100   | offset11
pub fn branch(raw: u16) -> Instruction {
    let mut offset = raw & 0x7FF;
    if util::get_bit_hw(offset, 10) {
        offset |= 0xF800;
    }

    Instruction::Branch(Branch {
        offset: ((offset as i16) << 1) as i32,
        link: false // docs say BAL but GBE does not link for this ins
    })
}

/// format 19: allows for a branch and link with a full 23 bit offset
/// a long branch with H = 1 followed by one with H = 0 is equivalent to one BL
/// 15 .. 12 | 11 | 10 .. 0
///   1111   | H  | offset
pub fn long_branch(raw: u16) -> Instruction {
    Instruction::LongBranch(LongBranch {
        first: !util::get_bit_hw(raw, 11),
        offset: raw & 0x7FF
    })
}

// long_branch is implemented as one instruction to keep the Instruction enum
// minimal
#[derive(Clone, Debug)]
pub struct LongBranch { pub first: bool, offset: u16 }

impl LongBranch {
    pub fn run(&self, cpu: &mut CPU) -> u32 {
        if self.first {
            let mut offset = (self.offset as u32) << 12;
            if util::get_bit(offset, 22) {
                offset |= 0xFF800000;
            }
            let upper = cpu.get_reg(15) as i64 + (offset as i32) as i64;
            cpu.set_reg(14, upper as u32);
            0 // incur the cycle cost when the second half is run
        } else {
            let next_ins = (cpu.get_reg(15) - 2) | 1;
            let pc = cpu.get_reg(14).wrapping_add((self.offset as u32) << 1);
            let old_pc = cpu.r[15];
            cpu.set_reg(14, next_ins);
            cpu.set_reg(15, pc & !1);
            cpu.should_flush = true;
            cpu.mem.access_time(old_pc, false) +
                cpu.mem.access_time(pc, true) +
                cpu.mem.access_time(pc + 4, false)
        }
    }
}

#[cfg(test)]
mod test {
    use ::cpu::status_reg::InstructionSet;
    use super::*;

    #[test]
    fn test_move() {
        match move_(0b000_01_11011_011_110) {
            Instruction::DataProc(ins) => {
                assert_eq!(ins.rd, 0b110);
                assert_eq!(ins.op2, RegOrImm::Reg { shift: 0b11011_01_0, reg: 0b011 });      
            },
            _ => panic!()
        }
    }

    #[test]
    fn test_add_sub() {
        match add_sub(0b00011_1_0_001_110_101) {
            Instruction::DataProc(ins) => {
                assert_eq!(ins.rd, 0b101);
                assert_eq!(ins.rn, 0b110);
                assert_eq!(ins.opcode, Op::ADD);
                assert_eq!(ins.op2, RegOrImm::Imm { rotate: 0, value: 0b001 });              
            },
            _ => panic!()
        }
    }

    #[test]
    fn test_data_imm() {
        match data_imm(0b001_01_110_11110001) {
            Instruction::DataProc(ins) => {
                assert_eq!(ins.opcode, Op::CMP);
                assert_eq!(ins.rd, 0b110);
                assert_eq!(ins.rn, 0b110);
                assert_eq!(ins.op2, RegOrImm::Imm { rotate: 0, value: 0b11110001 });
            },
            _ => panic!()
        }
    }

    #[test]
    fn test_alu() {
        match alu_op(0b010000_1010_001_010) {
            Instruction::DataProc(ins) => {
                assert_eq!(ins.opcode, Op::CMP);
                assert_eq!(ins.rd, 0b010);
                assert_eq!(ins.rn, 0b010);
                assert_eq!(ins.op2, RegOrImm::Reg { shift: 0, reg: 0b001 });
            },
            _ => panic!()
        };

        match alu_op(0b010000_0111_001_010) {
            Instruction::DataProc(ins) => {
                assert_eq!(ins.opcode, Op::MOV);
                assert_eq!(ins.rd, 0b010);
                assert_eq!(ins.op2, RegOrImm::Reg { shift: 0b0001_0111, reg: 0b010 });
            },
            _ => panic!()
        };

        match alu_op(0b010000_1001_001_010) {
            Instruction::DataProc(ins) => {
                assert_eq!(ins.opcode, Op::RSB);
                assert_eq!(ins.rn, 0b001);
                assert_eq!(ins.rd, 0b010);
                assert_eq!(ins.op2, RegOrImm::Imm { rotate: 0, value: 0 });
            },
            _ => panic!()
        };
    }

    #[test]
    fn test_hi_reg_bex() {
        match hi_reg_bex(0b010001_11_00_001_110) {
            Instruction::BranchEx(ins) => {
                assert_eq!(ins.reg, 0b001);
            },
            _ => panic!()
        }

        match hi_reg_bex(0b010001_00_11_001_110) {
            Instruction::DataProc(ins) => {
                assert_eq!(ins.set_flags, false);
                assert_eq!(ins.opcode, Op::ADD);
                assert_eq!(ins.rn, 0b1110);
                assert_eq!(ins.rd, 0b1110);
                assert_eq!(ins.op2, RegOrImm::Reg { shift: 0, reg: 0b1001 });
            },
            _ => panic!()
        }
    }

    #[test]
    fn test_pc_rel_load() {
        match pc_rel_load(0b01001_101_10100101) {
            Instruction::SingleTransfer(ins) => {
                assert_eq!(ins.pre_index, true);
                assert_eq!(ins.offset_up, true);
                assert_eq!(ins.byte, false);
                assert_eq!(ins.write_back, false);
                assert_eq!(ins.load, true);
                assert_eq!(ins.rn, 15);
                assert_eq!(ins.rd, 0b101);
                assert_eq!(
                    ins.offset,
                    RegOrImm::Imm { rotate: 0, value: 0b1010010100 });
            },
            _ => panic!()
        }
    }

    #[test]
    fn test_reg_offset_trans() {
        match reg_offset_trans(0b0101_0_1_0_100_010_001) {
            Instruction::SingleTransfer(ins) => {
                assert_eq!(ins.pre_index, true);
                assert_eq!(ins.offset_up, true);
                assert_eq!(ins.byte, true);
                assert_eq!(ins.write_back, false);
                assert_eq!(ins.load, false);
                assert_eq!(ins.rn, 0b010);
                assert_eq!(ins.rd, 0b001);
                assert_eq!(ins.offset, RegOrImm::Reg { shift: 0, reg: 0b100 });
            },
            _ => panic!()
        }
    }

    #[test]
    fn test_signed_trans() {
        match signed_trans(0b0101_1_1_1_100_010_001) {
            Instruction::SignedTransfer(ins) => {
                assert_eq!(ins.pre_index, true);
                assert_eq!(ins.offset_up, true);
                assert_eq!(ins.halfword, true);
                assert_eq!(ins.write_back, false);
                assert_eq!(ins.load, true);
                assert_eq!(ins.rn, 0b010);
                assert_eq!(ins.rd, 0b001);
                assert_eq!(ins.signed, true);
                assert_eq!(ins.offset, RegOrImm::Reg { shift: 0, reg: 0b100 });
            },
            _ => panic!()
        }
    }

    #[test]
    fn test_imm_offset_trans() {
        match imm_offset_trans(0b011_0_0_11011_010_001) {
            Instruction::SingleTransfer(ins) => {
                assert_eq!(ins.pre_index, true);
                assert_eq!(ins.offset_up, true);
                assert_eq!(ins.byte, false);
                assert_eq!(ins.write_back, false);
                assert_eq!(ins.load, false);
                assert_eq!(ins.rn, 0b010);
                assert_eq!(ins.rd, 0b001);
                assert_eq!(
                    ins.offset,
                    RegOrImm::Imm { rotate: 0, value: 0b1101100 });
            },
            _ => panic!()
        }
    }

    #[test]
    fn test_hw_trans() {
        match hw_trans(0b1000_1_10101_111_000) {
            Instruction::SignedTransfer(ins) => {
                assert_eq!(ins.pre_index, true);
                assert_eq!(ins.offset_up, true);
                assert_eq!(ins.halfword, true);
                assert_eq!(ins.write_back, false);
                assert_eq!(ins.load, true);
                assert_eq!(ins.rn, 0b111);
                assert_eq!(ins.rd, 0b000);
                assert_eq!(ins.signed, false);
                assert_eq!(
                    ins.offset,
                    RegOrImm::Imm { rotate: 0, value: 0b101010 });
            },
            _ => panic!()
        }
    }

    #[test]
    fn test_sp_rel_trans() {
        match sp_rel_trans(0b1001_0_111_10110001) {
            Instruction::SingleTransfer(ins) => {
                assert_eq!(ins.pre_index, true);
                assert_eq!(ins.offset_up, true);
                assert_eq!(ins.byte, false);
                assert_eq!(ins.write_back, false);
                assert_eq!(ins.load, false);
                assert_eq!(ins.rn, 13);
                assert_eq!(ins.rd, 0b111);
                assert_eq!(
                    ins.offset,
                    RegOrImm::Imm { rotate: 0, value: 0b10110001 });
            },
            _ => panic!()
        }
    }

    #[test]
    fn test_load_addr() {
        match load_addr(0b1010_0_001_11110001) {
            Instruction::DataProc(ins) => {
                assert_eq!(ins.opcode, Op::ADD);
                assert_eq!(ins.set_flags, false);
                assert_eq!(ins.rn, 15);
                assert_eq!(ins.rd, 0b001);
                assert_eq!(
                    ins.op2,
                    RegOrImm::Imm { rotate: 0, value: 0b1111000100 });
            },
            _ => panic!()
        }
    }

    #[test]
    fn test_incr_sp() {
        match incr_sp(0b10110000_1_1010011) {
            Instruction::DataProc(ins) => {
                assert_eq!(ins.opcode, Op::SUB);
                assert_eq!(ins.set_flags, false);
                assert_eq!(ins.rn, 13);
                assert_eq!(ins.rd, 13);
                assert_eq!(
                    ins.op2,
                    RegOrImm::Imm { rotate: 0, value: 0b101001100 });
            },
            _ => panic!()
        }
    }

    #[test]
    fn test_push_pop() {
        match push_pop(0b1011_1_10_0_01011010) {
            Instruction::BlockTransfer(ins) => {
                assert_eq!(ins.pre_index, false);
                assert_eq!(ins.offset_up, true);
                assert_eq!(ins.force, false);
                assert_eq!(ins.write_back, true);
                assert_eq!(ins.rn, 13);
                assert_eq!(ins.register_list, 0b00000000_01011010);
            },
            _ => panic!()
        }

        match push_pop(0b1011_0_10_1_10110001) {
            Instruction::BlockTransfer(ins) => {
                assert_eq!(ins.pre_index, true);
                assert_eq!(ins.offset_up, false);
                assert_eq!(ins.force, false);
                assert_eq!(ins.write_back, true);
                assert_eq!(ins.rn, 13);
                assert_eq!(ins.register_list, 0b01000000_10110001);
            },
            _ => panic!()
        }
    }

    #[test]
    fn test_block_trans() {
        match block_trans(0b1100_1_101_11001010) {
            Instruction::BlockTransfer(ins) => {
                assert_eq!(ins.pre_index, false);
                assert_eq!(ins.offset_up, true);
                assert_eq!(ins.force, false);
                assert_eq!(ins.write_back, true);
                assert_eq!(ins.rn, 0b101);
                assert_eq!(ins.register_list, 0b00000000_11001010);
            },
            _ => panic!()
        }
    }

    #[test]
    fn test_branch() {
        // min possible offset
        match branch(0b11100_10000000000) {
            Instruction::Branch(ins) => { assert_eq!(ins.offset, -(1 << 11)); }
            _ => panic!()
        }
        // max possible offset
        match branch(0b11100_01111111111) {
            Instruction::Branch(ins) => { assert_eq!(ins.offset, (1 << 11) - 2); }
            _ => panic!()
        }
        match branch(0b11100_00000000011) {
            Instruction::Branch(ins) => { assert_eq!(ins.offset, 0b110); }
            _ => panic!()
        }
    }

    #[test]
    fn test_cond_branch() {
        match cond_branch(0b1101101111111100) {
            Instruction::CondBranch(ins) => { assert_eq!(ins.offset, -8); },
            _ => panic!()
        }
    }

    #[test]
    fn test_long_branch() {
        let mut cpu = CPU::new();
        cpu.set_reg(14, 0x942);
        cpu.set_reg(15, 0x1942);
        cpu.cpsr.isa = InstructionSet::THUMB;
        match long_branch(0xF7FF) {
            Instruction::LongBranch(ins) => {
                assert_eq!(ins.first, true);
                ins.run(&mut cpu);
                assert_eq!(cpu.get_reg(14), 0x942);
            },
            _ => panic!()
        }
        cpu.incr_pc();
        match long_branch(0xF840) {
            Instruction::LongBranch(ins) => {
                assert_eq!(ins.first, false);
                ins.run(&mut cpu);
            },
            _ => panic!()
        }
        assert_eq!(cpu.get_reg(14), 0x1943);
        assert_eq!(cpu.get_reg(15), 0x9C2);
    }
}
