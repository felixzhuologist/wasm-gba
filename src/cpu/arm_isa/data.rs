use std;
use num::FromPrimitive;
use super::{InstructionType, Instruction, RegOrImm};
use ::cpu::CPU;
use ::util;

enum_from_primitive! {
#[derive(Copy, Clone)]
#[repr(u8)]
pub enum Op {
    AND = 0,
    EOR,
    SUB,
    RSB,
    ADD,
    ADC,
    SBC,
    RSC,
    TST,
    TEQ,
    CMP,
    CMN,
    ORR,
    MOV,
    BIC,
    MVN
}
}

pub struct DataProc {
    pub opcode: Op,
    pub set_flags: bool,
    rn: usize,
    rd: usize,
    op2: RegOrImm
}

impl DataProc {
    /// parses the following format:
    /// 27 .. 26 | 25 | 24 .. 21 | 20 | 19 .. 16 | 15 .. 12 | 11 .. 0
    ///    00    | I  |   opcode | S  |    Rn    |    Rd    |    op2
    pub fn parse_instruction(ins: u32) -> DataProc {
        let is_imm = util::get_bit(ins, 25);
        DataProc {
            rd: util::get_nibble(ins, 12) as usize,
            rn: util::get_nibble(ins, 16) as usize,
            set_flags: util::get_bit(ins, 20),
            opcode: Op::from_u32(util::get_nibble(ins, 21)).unwrap(),
            op2: if is_imm { 
                RegOrImm::Imm {
                    rotate: util::get_nibble(ins, 8),
                    value: util::get_byte(ins, 0)
                }
            } else {
                RegOrImm::Reg {
                    shift: util::get_byte(ins, 4),
                    reg: util::get_nibble(ins, 0)
                }
            }
        }
    }
}

impl Instruction for DataProc {
    fn get_type(&self) -> InstructionType { InstructionType::DataProc }
    fn process_instruction(&self, cpu: &mut CPU) {
        let op1 = cpu.get_reg(self.rn);
        let (op2, shift_carry) = match self.op2 {
            // TODO: what is carry flag set to when I=1 and a logical op is used?
            RegOrImm::Imm { rotate, value } => (value.rotate_right(rotate * 2), false),
            RegOrImm::Reg { shift, reg } => apply_shift(cpu, shift, reg)
        };

        let (result, carry_out) = match self.opcode {
            Op::AND => (op1 & op2, shift_carry),
            Op::EOR => (op1 ^ op2, shift_carry),
            Op::SUB => op1.overflowing_sub(op2),
            Op::RSB => op2.overflowing_sub(op1),
            Op::ADD => op1.overflowing_add(op2),
            Op::ADC => {
                let (r1, c1) = op1.overflowing_add(op2);
                let (r2, c2) = r1.overflowing_add(cpu.cpsr.c as u32);
                (r2, c1 || c2)
            },
            Op::SBC => {
                let (r1, c1) = op1.overflowing_sub(op2);
                let (r2, c2) = r1.overflowing_sub(1);
                let sub_overflow = c1 || c2;
                let (result, add_overflow) = r2.overflowing_add(cpu.cpsr.c as u32);
                // if we "underflowed" then overflowed, then they cancel out
                (result, sub_overflow ^ add_overflow)
            },
            Op::RSC => {
                let (r1, c1) = op2.overflowing_sub(op1);
                let (r2, c2) = r1.overflowing_sub(1);
                let sub_overflow = c1 || c2;
                let (result, add_overflow) = r2.overflowing_add(cpu.cpsr.c as u32);
                // if we "underflowed" then overflowed, then they cancel out
                (result, sub_overflow ^ add_overflow)
            },
            Op::TST => (op1 & op2, shift_carry),
            Op::TEQ => (op1 ^ op2, shift_carry),
            Op::CMP => op1.overflowing_sub(op2),
            Op::CMN => op1.overflowing_add(op2),
            Op::ORR => (op1 | op2, shift_carry),
            Op::MOV => (op2, shift_carry),
            Op::BIC => (op1 & (!op2), shift_carry),
            Op::MVN => (!op2, shift_carry)
        };

        let should_write = match self.opcode {
            Op::TST |
            Op::TEQ |
            Op::CMP |
            Op::CMN => true,
            _ => false
        };

        if should_write {
            cpu.set_reg(self.rd, result);
        }

        if !self.set_flags && should_write {
            panic!("trying to use data instruction handler on a MRS/MSR instruction");
        }
    
        if self.set_flags || !should_write  {
            // TODO: how are we supposed to know if the operands are signed?
            // and detect if the V flag should be set
            cpu.cpsr.c = carry_out;
            cpu.cpsr.z = result == 0;
            cpu.cpsr.n = ((result >> 31) & 1) == 1;
        }

        if self.rd == 15 && self.set_flags {
            cpu.restore_cpsr();
        }
    }
}

pub fn apply_shift(cpu: &mut CPU, shift: u32, reg: u32) -> (u32, bool) {
    let shift_amount = match (util::get_bit(shift, 3), util::get_bit(shift, 0)) {
        (false, true) => {
            let rs = util::get_nibble(shift, 4);
            if rs == 15 {
                panic!("cannot use R15 as shift amount");
            }
            cpu.get_reg(rs as usize) & 0xFF
        },
        (_, false) => shift & 0b11111000,
        _ => panic!("invalid sequence of bits for shift")
    };

    let rm_val = cpu.get_reg(reg as usize);
    // TODO: use enum here?
    match (util::get_bit(shift, 1), util::get_bit(shift, 0)) {
        (false, false) => { // logical shift left
            if shift_amount == 0 {
                (rm_val, cpu.cpsr.c)
            } else if shift_amount > 32 {
                (0, false)
            } else {
                let carry_out = (rm_val >> (32 - shift_amount)) & 1;
                ((rm_val << shift_amount), carry_out == 1)
            }
        },
        (false, true) => { // logical shift right
            // LSR #0 is actually interpreted as ASR #32 since it is redundant
            // with LSL #0 
            if shift_amount == 0 {
                let carry_out = (rm_val >> 31) & 1;
                (if carry_out == 1 {std::u32::MAX} else {0}, carry_out == 1)
            } else if shift_amount > 32 {
                (0, false)
            } else {
                // otherwise use most significant discarded bit as the carry output
                let partial_shifted = rm_val >> (shift_amount - 1);
                let carry_out = partial_shifted & 1;
                (partial_shifted >> 1, carry_out == 1)
            }
        },
        (true, false) => { // arithmetic shift right
            if shift_amount == 0 {
                (rm_val, cpu.cpsr.c)
            } else if shift_amount > 32 {
                let top_bit = (rm_val >> 31) & 1;
                (if top_bit == 1 {std::u32::MAX} else {0}, top_bit == 1)
            } else {
                // convert to i32 to get arithmetic shifting
                let partial_shifted = (rm_val as i32) >> (shift_amount - 1);
                let carry_out = partial_shifted & 1;
                return ((partial_shifted >> 1) as u32, carry_out == 1)
            }
        },
        (true, true) => {
            // RSR #0 is used to encode RRX
            if shift_amount == 0 {
                let result = (rm_val >> 1) | ((cpu.cpsr.c as u32) << 31);
                (result, (rm_val & 1) == 1)
            } else {
                let result = rm_val.rotate_right(shift_amount);
                let carry_out = (result >> 31) & 1;
                (result, carry_out == 1)
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn parse_reg() {
        let ins = DataProc::parse_instruction(
            0b0000_00_0_1010_1_0001_0010_10001000_1001);
        assert!(ins.set_flags);
        assert_eq!(ins.opcode as u8, Op::CMP as u8);
        assert_eq!(ins.rn, 1);
        assert_eq!(ins.rd, 2);
        assert!(match ins.op2 {
            RegOrImm::Reg { shift: 0x88, reg: 9 } => true,
            _ => false
        });
    }

    #[test]
    fn parse_imm() {
        let ins = DataProc::parse_instruction(
            0b0000_00_1_0101_0_1110_0111_0011_00000001);
        assert!(!ins.set_flags);
        assert_eq!(ins.opcode as u8, Op::ADC as u8);
        assert_eq!(ins.rn, 14);
        assert_eq!(ins.rd, 7);
        assert!(match ins.op2 {
            RegOrImm::Imm { rotate: 3, value: 1 } => true,
            _ => false
        });
    }
}