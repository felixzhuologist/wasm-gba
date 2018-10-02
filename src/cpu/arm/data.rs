use num::FromPrimitive;
use super::RegOrImm;
use ::cpu::CPU;
use ::cpu::status_reg::InstructionSet;
use ::util;

enum_from_primitive! {
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
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

#[derive(Debug)]
pub struct DataProc {
    pub opcode: Op,
    pub set_flags: bool,
    pub rn: usize,
    pub rd: usize,
    pub op2: RegOrImm
}

const MAX: u32 = 0xFFFFFFFF;

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

    pub fn run(&self, cpu: &mut CPU) {
        let mut op1 = cpu.get_reg(self.rn);
        if cpu.cpsr.isa == InstructionSet::THUMB && self.rn == 15 {
            // TODO: this is probably only for the load_addr THUMB instruction...
            op1 &= !2;
        }
        let (op2, shift_carry) = match self.op2 {
            RegOrImm::Imm { rotate, value } => {
                let result = value.rotate_right(rotate * 2);
                // TODO: what is carry flag set to when I=1 and a logical op is used?
                (result, ((result >> 31) & 1) == 1)
            },
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
                let (r2, c2) = r1.overflowing_add(cpu.cpsr.carry as u32);
                (r2, c1 || c2)
            },
            Op::SBC => {
                let (r1, c1) = op1.overflowing_sub(op2);
                let (r2, c2) = r1.overflowing_sub(1);
                let sub_overflow = c1 || c2;
                let (result, add_overflow) = r2.overflowing_add(cpu.cpsr.carry as u32);
                // if we "underflowed" then overflowed, then they cancel out
                (result, sub_overflow ^ add_overflow)
            },
            Op::RSC => {
                let (r1, c1) = op2.overflowing_sub(op1);
                let (r2, c2) = r1.overflowing_sub(1);
                let sub_overflow = c1 || c2;
                let (result, add_overflow) = r2.overflowing_add(cpu.cpsr.carry as u32);
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
            Op::CMN => false,
            _ => true
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
            cpu.cpsr.carry = carry_out;
            cpu.cpsr.zero = result == 0;
            cpu.cpsr.neg = ((result >> 31) & 1) == 1;
        }

        if self.rd == 15 && self.set_flags {
            cpu.restore_cpsr();
        }
    }
}

/// Applies a shift to either a register value or an immediate value.
/// the shift parameter can either look like:
///  7 .. 3 | 2 .. 1 | 0                    7 .. 4 | 3 | 2 .. 1 | 0
///  --------------------        OR         ------------------------
///   val   | type   | 0                      reg  | 0 | type   | 1
/// where the left case uses a 5 bit immediate val as the shift amount, and the
/// right case uses the bottom byte of the contents of a registers.
/// The resulting val and the carry bit (which may be used to set the carry flag
/// for logical operations) are returned
pub fn apply_shift(cpu: &CPU, shift: u32, reg: u32) -> (u32, bool) {
    let shift_amount = get_shift_amount(cpu, shift);
    let val = cpu.get_reg(reg as usize);
    // TODO: use enum here?
    match (util::get_bit(shift, 2), util::get_bit(shift, 1)) {
        (false, false) => { // logical shift left
            if shift_amount == 0 {
                (val, cpu.cpsr.carry)
            } else if shift_amount > 32 {
                (0, false)
            } else if shift_amount == 32 {
                (0, (val & 1) == 1)
            } else {
                let carry_out = (val >> (32 - shift_amount)) & 1;
                ((val << shift_amount), carry_out == 1)
            }
        },
        (false, true) => { // logical shift right
            // LSR #0 is actually interpreted as LSR #32 since it is redundant
            // with LSL #0 
            if shift_amount == 0 {
                (0, ((val >> 31) & 1) == 1)
            } else if shift_amount > 32 {
                (0, false)
            } else {
                // otherwise use most significant discarded bit as the carry output
                let partial_shifted = val >> (shift_amount - 1);
                let carry_out = partial_shifted & 1;
                (partial_shifted >> 1, carry_out == 1)
            }
        },
        (true, false) => { // arithmetic shift right
            // As for LSR, ASR 0 is used to encode ASR 32
            if shift_amount == 0 || shift_amount > 32 {
                let carry_out = ((val >> 31) & 1) == 1;
                (if carry_out {MAX} else {0}, carry_out)
            } else {
                // convert to i32 to get arithmetic shifting
                let partial_shifted = (val as i32) >> (shift_amount - 1);
                let carry_out = partial_shifted & 1;
                ((partial_shifted >> 1) as u32, carry_out == 1)
            }
        },
        (true, true) => { // rotate right
            // RSR #0 is used to encode RRX
            if shift_amount == 0 {
                let carry_out = (val & 1) == 1;
                let result = (val >> 1) | ((cpu.cpsr.carry as u32) << 31);
                (result, carry_out)
            } else {
                let result = val.rotate_right(shift_amount);
                let carry_out = (result >> 31) & 1;
                (result, carry_out == 1)
            }
        }
    }
}

fn get_shift_amount(cpu: &CPU, shift: u32) -> u32 {
    match (util::get_bit(shift, 3), util::get_bit(shift, 0)) {
        (false, true) => {
            let rs = util::get_nibble(shift, 4);
            if rs == 15 {
                panic!("cannot use R15 as shift amount");
            }
            cpu.get_reg(rs as usize) & 0xFF
        },
        (_, false) => (shift >> 3) & 0b11111,
        _ => panic!("invalid sequence of bits for shift")
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

    #[test]
    fn shift_amt_imm() {
        let cpu = CPU::new();
        assert_eq!(get_shift_amount(&cpu, 0b11011_000), 0b11011);
        assert_eq!(get_shift_amount(&cpu, 0b00001_010), 0b00001);
        assert_eq!(get_shift_amount(&cpu, 0b10000_100), 0b10000);
        assert_eq!(get_shift_amount(&cpu, 0b11111_110), 0b11111);
        assert_eq!(get_shift_amount(&cpu, 0), 0);
    }

    #[test]
    fn shift_amt_reg() {
        let mut cpu = CPU::new();

        cpu.set_reg(0, 0xFFFFFF_03);
        assert_eq!(get_shift_amount(&cpu, 0b0000_0001), 0x03);

        cpu.set_reg(3, 0x00_FF);
        assert_eq!(get_shift_amount(&cpu, 0b0011_0011), 0xFF);

        cpu.set_reg(4, 0xAB_09);
        assert_eq!(get_shift_amount(&cpu, 0b0100_0101), 0x09);

        cpu.set_reg(14, 0x99_A1);
        assert_eq!(get_shift_amount(&cpu, 0b1110_0111), 0xA1);

        assert_eq!(get_shift_amount(&cpu, 0b0001_0111), 0);
    }

    #[test]
    #[should_panic]
    fn shift_amt_reg_15() {
        let cpu = CPU::new();
        get_shift_amount(&cpu, 0b1111_0_00_1);
    }

    #[test]
    fn shift_lsl() {
        let mut cpu = CPU::new();
        // check least significant discarded bit = 1
        cpu.set_reg(5, 0xFF123456);
        assert_eq!(apply_shift(&cpu, 0b00101_000, 5), (0xFF123456 << 5, true));

        // check least significant discarded bit = 0
        cpu.set_reg(3, 0xF7123455);
        assert_eq!(apply_shift(&cpu, 0b00101_000, 3), (0xF7123455 << 5, false));

        // check that LSL by 0 retains the current carry flag
        cpu.cpsr.carry = true;
        assert_eq!(apply_shift(&cpu, 0, 0), (0, true));

        // lsl 32 with low bit = 0
        cpu.set_reg(10, 32);
        assert_eq!(apply_shift(&cpu, 0b1010_0001, 5), (0, false));
        // lsl 32 with low bit = 1
        assert_eq!(apply_shift(&cpu, 0b1010_0001, 3), (0, true));

        // lsl by more than 32
        cpu.set_reg(11, 33);
        assert_eq!(apply_shift(&cpu, 0b1011_0001, 11), (0, false));
        assert_eq!(apply_shift(&cpu, 0b1011_0001, 11), (0, false));
    }

    #[test]
    fn shift_lsr() {
        let mut cpu = CPU::new();
        // check most significant discarded bit = 1
        cpu.set_reg(15, 0xABCDEF3F);
        assert_eq!(apply_shift(&cpu, 0b00101_010, 15), (0xABCDEF3F >> 5, true));

        // check most significant discarded bit = 0
        cpu.set_reg(10, 0x123456A8);
        assert_eq!(apply_shift(&cpu, 0b00101_010, 10), (0x123456A8 >> 5, false));

        // check lsr 0/32 with high bit = 1
        cpu.set_reg(0, 0xFFFFFFFF);
        cpu.set_reg(8, 32);
        assert_eq!(apply_shift(&cpu, 0b1000_0011, 0), (0, true));
        assert_eq!(apply_shift(&cpu, 0b00000_010, 0), (0, true));

        // check lsr 0/32 with high bit = 0
        cpu.set_reg(1, 0x7FFFFFF);
        assert_eq!(apply_shift(&cpu, 0b1000_0011, 1), (0, false));
        assert_eq!(apply_shift(&cpu, 0b00000_010, 1), (0, false));

        // lsr by more than 32
        cpu.set_reg(9, 33);
        assert_eq!(apply_shift(&cpu, 0b1001_0011, 15), (0, false));
        assert_eq!(apply_shift(&cpu, 0b1001_0011, 10), (0, false));
    }

    #[test]
    fn shift_asr() {
        let mut cpu = CPU::new();

        // check positive, msdb = 1
        cpu.set_reg(0, 0x3123453F);
        assert_eq!(apply_shift(&cpu, 0b00101_100, 0), (0x3123453F >> 5, true));

        // check negative, msdb = 0
        cpu.set_reg(1, 0xF12345A8);
        assert_eq!(
            apply_shift(&cpu, 0b00101_100, 1),
            (((0xF12345A8u32 as i32) >> 5) as u32, false));

        // check ASR 0 (32)
        assert_eq!(apply_shift(&cpu, 0b00000_100, 0), (0, false));
        assert_eq!(apply_shift(&cpu, 0b00000_100, 1), (MAX, true));

        // check ASR > 32
        cpu.set_reg(14, 33);
        assert_eq!(apply_shift(&cpu, 0b1110_0101, 0), (0, false));
        assert_eq!(apply_shift(&cpu, 0b1110_0101, 1), (MAX, true));
    }

    #[test]
    fn shift_ror() {
        let mut cpu = CPU::new();

        // ROR 0/RRX
        cpu.set_reg(0, 0x3123453F);
        assert_eq!(apply_shift(&cpu, 0b00000_110, 0), (0x3123453F >> 1, true));

        cpu.cpsr.carry = true;
        cpu.set_reg(1, 0xFFFFFFFE);
        assert_eq!(apply_shift(&cpu, 0b00000_110, 1), (0xFFFFFFFF, false));

        // ROR 5 with bit 4 = 1
        assert_eq!(
            apply_shift(&cpu, 0b00101_110, 0),
            (0x3123453Fu32.rotate_right(5), true));
        // ROR 5 with bit 4 = 0
        cpu.set_reg(2, 0x12345608);
        assert_eq!(
            apply_shift(&cpu, 0b00101_110, 2),
            (0x12345608u32.rotate_right(5), false));

        // ROR >= 32
        cpu.set_reg(14, 32);
        assert_eq!(
            apply_shift(&cpu, 0b1110_0111, 0),
            (0x3123453F, false));
        cpu.set_reg(14, 37);
        assert_eq!(
            apply_shift(&cpu, 0b1110_0111, 2),
            (0x12345608u32.rotate_right(5), false));
    }

    #[test]
    fn add() {
        let mut cpu = CPU::new();
        cpu.set_reg(0, 11);

        let ins = DataProc {
            opcode: Op::ADD,
            set_flags: true,
            rn: 0,
            rd: 3,
            op2: RegOrImm::Imm { rotate: 0, value: 10 }
        };
        ins.run(&mut cpu);

        assert_eq!(cpu.get_reg(3), 21);
        assert_eq!(cpu.cpsr.carry, false);
        assert_eq!(cpu.cpsr.zero, false);
        assert_eq!(cpu.cpsr.neg, false);
    }

    #[test]
    fn add_overflow() {
        let mut cpu = CPU::new();
        cpu.set_reg(0, MAX);
        cpu.set_reg(1, 5);

        let ins = DataProc {
            opcode: Op::ADD,
            set_flags: true,
            rn: 0,
            rd: 3,
            op2: RegOrImm::Reg { shift: 0, reg: 1 }
        };
        ins.run(&mut cpu);

        assert_eq!(cpu.get_reg(3), 4);
        assert_eq!(cpu.cpsr.carry, true);
        assert_eq!(cpu.cpsr.zero, false);
        assert_eq!(cpu.cpsr.neg, false);
    }
}
