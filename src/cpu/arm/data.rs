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

#[derive(Clone, Debug, PartialEq, Eq)]
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

    pub fn run(&self, cpu: &mut CPU) -> u32 {
        let mut op1 = cpu.get_reg(self.rn);
        if cpu.cpsr.isa == InstructionSet::THUMB && self.rn == 15 {
            // TODO: this is probably only for the load_addr THUMB instruction...
            op1 &= !2;
        }
        let (op2, shift_carry) = match self.op2 {
            RegOrImm::Imm { rotate, value } => {
                let result = value.rotate_right(rotate * 2);
                // rotate 0 <=> LSL 0 which should preserve the carry flag
                let carry_out = if rotate == 0 {
                    cpu.cpsr.carry
                } else {
                    ((result >> 31) & 1) == 1
                };
                (result, carry_out)
            },
            RegOrImm::Reg { shift, reg } => {
                // when R15 is used as an operand and a register is used to specify
                // the shift amount, the PC will be 12 bytes ahead instead of 8
                let mut rm_val = cpu.get_reg(reg as usize);
                let reg_shift = util::get_bit(shift, 0);
                if self.rn == 15 && reg_shift {
                    op1 += 4;
                }
                if reg == 15 && reg_shift {
                    rm_val += 4;
                }
                let (mut op2, shift_carry) = apply_shift(cpu, shift, rm_val);
                (op2, shift_carry)
            }
        };

        let should_write = match self.opcode {
            Op::TST |
            Op::TEQ |
            Op::CMP |
            Op::CMN => false,
            _ => true
        };
        if !self.set_flags && !should_write {
            panic!("trying to use data instruction handler on a MRS/MSR instruction");
        }

        // all cases return a result, but the result is only saved in the destination
        // if should_write (above) is true. carry_out is always written, and overflow
        // is saved if it contains a value
        let (result, carry_out, overflow) = match self.opcode {
            Op::AND => (op1 & op2, shift_carry, None),
            Op::EOR => (op1 ^ op2, shift_carry, None),
            Op::SUB | Op::CMP => sub(op1, op2, 1),
            Op::RSB => sub(op2, op1, 1),
            Op::ADD | Op::CMN => add(op1, op2, 0),
            Op::ADC => add(op1, op2, cpu.cpsr.carry as u32),
            Op::SBC => sub(op1, op2, cpu.cpsr.carry as u32),
            Op::RSC => sub(op2, op1, cpu.cpsr.carry as u32),
            Op::TST => (op1 & op2, shift_carry, None),
            Op::TEQ => (op1 ^ op2, shift_carry, None),
            Op::ORR => (op1 | op2, shift_carry, None),
            Op::MOV => (op2, shift_carry, None),
            Op::BIC => (op1 & (!op2), shift_carry, None),
            Op::MVN => (!op2, shift_carry, None)
        };

        let old_pc = cpu.get_reg(15); // save PC in case we overwrite it here
        if should_write {
            cpu.set_reg(self.rd, result);
        }

        if self.set_flags || !should_write  {
            cpu.cpsr.zero = result == 0;
            cpu.cpsr.neg = util::get_bit(result, 31);
            cpu.cpsr.carry = carry_out;
            if let Some(val) = overflow {
                cpu.cpsr.overflow = val;
            }
        }

        if self.rd == 15 && self.set_flags {
            cpu.restore_cpsr();
        }

        let mut cycles = cpu.mem.access_time(old_pc, false);
        if let RegOrImm::Reg { shift: _, reg: _ } = self.op2 {
            cycles += 1;
        }
        if self.rd == 15 {
            cpu.should_flush = true;
            cycles += cpu.mem.access_time(cpu.r[15], true) +
                cpu.mem.access_time(cpu.r[15] + 4, false);
        }
        cycles
    }
}

/// Applies a either an instruction specified or a register specified shift to
/// the provided value. The shift parameter can either look like:
///  7 .. 3 | 2 .. 1 | 0                    7 .. 4 | 3 | 2 .. 1 | 0
///  --------------------        OR         ------------------------
///   val   | type   | 0                      reg  | 0 | type   | 1
/// where the left case uses a 5 bit immediate val as the shift amount, and the
/// right case uses the bottom byte of the contents of a registers.
/// The resulting val and the carry bit (which may be used to set the carry flag
/// for logical operations) are returned
pub fn apply_shift(cpu: &CPU, shift: u32, val: u32) -> (u32, bool) {
    let (is_shift_immediate, shift_amount) = get_shift_amount(cpu, shift);

    // the special encodings for LSR/ASR/RSR 0 only apply to immediate shifts,
    // so return early (and perform LSL 0) if we shift by a reg amount that is 0
    if !is_shift_immediate && shift_amount == 0 {
        return (val, cpu.cpsr.carry);
    }

    // TODO: use enum here?
    match (util::get_bit(shift, 2), util::get_bit(shift, 1)) {
        (false, false) => { // logical shift left
            if shift_amount == 0 {
                (val, cpu.cpsr.carry)
            } else if shift_amount > 32 {
                (0, false)
            } else if shift_amount == 32 {
                (0, util::get_bit(val, 0))
            } else {
                let carry_out = util::get_bit(val, (32 - shift_amount) as u8);
                ((val << shift_amount), carry_out)
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
            if shift_amount == 0 || shift_amount >= 32 {
                let carry_out = util::get_bit(val, 31);
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
                let carry_out = util::get_bit(val, 0);
                let result = (val >> 1) | ((cpu.cpsr.carry as u32) << 31);
                (result, carry_out)
            } else {
                let result = val.rotate_right(shift_amount);
                (result, util::get_bit(result, 31))
            }
        }
    }
}

/// Parse the shift bits (4 - 11) and return whether the shift amount was an
/// immediate, and the actual shift amount
fn get_shift_amount(cpu: &CPU, shift: u32) -> (bool, u32) {
    match (util::get_bit(shift, 3), util::get_bit(shift, 0)) {
        // shift by register amount
        (false, true) => {
            let rs = util::get_nibble(shift, 4);
            if rs == 15 {
                panic!("cannot use R15 as shift amount");
            }
            (false, cpu.get_reg(rs as usize) & 0xFF)
        },
        // shift by immediate amount
        (_, false) => (true, (shift >> 3) & 0b11111),
        _ => panic!("invalid sequence of bits for shift")
    }
}

/// Return the sum, carry, and overflow of the two operands
fn add(op1: u32, op2: u32, carry: u32) -> (u32, bool, Option<bool>) {
    let (r1, c1) = op1.overflowing_add(op2);
    let (r2, c2) = r1.overflowing_add(carry);
    // there's an overflow for addition when both operands are positive and the
    // result is negative, or both operands are negative and the result is positive.
    let overflow = (!(op1 ^ op2)) & (op1 ^ r2);
    (r2, c1 || c2, Some(util::get_bit(overflow, 31)))
}

/// Return the difference, carry, and overflow of the two operands
fn sub(op1: u32, op2: u32, carry: u32) -> (u32, bool, Option<bool>) {
    add(op1, !op2, carry)
}

// TODO: use eq trait for DataProc instead of comparing each field individually
#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn parse_move() {
        let ins = DataProc::parse_instruction(
            0b11100001101000000110100100010110);
        assert_eq!(ins, DataProc {
            opcode: Op::MOV,
            set_flags: false,
            rn: 0,
            rd: 6,
            op2: RegOrImm::Reg { shift: 0b10010001, reg: 6 }
        });
    }

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
        assert_eq!(get_shift_amount(&cpu, 0b11011_000), (true, 0b11011));
        assert_eq!(get_shift_amount(&cpu, 0b00001_010), (true, 0b00001));
        assert_eq!(get_shift_amount(&cpu, 0b10000_100), (true, 0b10000));
        assert_eq!(get_shift_amount(&cpu, 0b11111_110), (true, 0b11111));
        assert_eq!(get_shift_amount(&cpu, 0), (true, 0));
    }

    #[test]
    fn shift_amt_reg() {
        let mut cpu = CPU::new();

        cpu.set_reg(0, 0xFFFFFF_03);
        assert_eq!(get_shift_amount(&cpu, 0b0000_0001), (false, 0x03));

        cpu.set_reg(3, 0x00_FF);
        assert_eq!(get_shift_amount(&cpu, 0b0011_0011), (false, 0xFF));

        cpu.set_reg(4, 0xAB_09);
        assert_eq!(get_shift_amount(&cpu, 0b0100_0101), (false, 0x09));

        cpu.set_reg(14, 0x99_A1);
        assert_eq!(get_shift_amount(&cpu, 0b1110_0111), (false, 0xA1));

        assert_eq!(get_shift_amount(&cpu, 0b0001_0111), (false, 0));
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
        assert_eq!(apply_shift(&cpu, 0b00101_000, cpu.get_reg(5)), (0xFF123456 << 5, true));

        // check least significant discarded bit = 0
        cpu.set_reg(3, 0xF7123455);
        assert_eq!(apply_shift(&cpu, 0b00101_000, cpu.get_reg(3)), (0xF7123455 << 5, false));

        // check that LSL by 0 retains the current carry flag
        cpu.cpsr.carry = true;
        assert_eq!(apply_shift(&cpu, 0, cpu.get_reg(0)), (0, true));

        // lsl 32 with low bit = 0
        cpu.set_reg(10, 32);
        assert_eq!(apply_shift(&cpu, 0b1010_0001, cpu.get_reg(5)), (0, false));
        // lsl 32 with low bit = 1
        assert_eq!(apply_shift(&cpu, 0b1010_0001, cpu.get_reg(3)), (0, true));

        // lsl by more than 32
        cpu.set_reg(11, 33);
        assert_eq!(apply_shift(&cpu, 0b1011_0001, cpu.get_reg(11)), (0, false));
        assert_eq!(apply_shift(&cpu, 0b1011_0001, cpu.get_reg(11)), (0, false));
    }

    #[test]
    fn shift_lsr() {
        let mut cpu = CPU::new();
        // check most significant discarded bit = 1
        cpu.set_reg(15, 0xABCDEF3F);
        assert_eq!(apply_shift(&cpu, 0b00101_010, cpu.get_reg(15)), (0xABCDEF3F >> 5, true));

        // check most significant discarded bit = 0
        cpu.set_reg(10, 0x123456A8);
        assert_eq!(apply_shift(&cpu, 0b00101_010, cpu.get_reg(10)), (0x123456A8 >> 5, false));

        // check lsr 0/32 with high bit = 1
        cpu.set_reg(0, 0xFFFFFFFF);
        cpu.set_reg(8, 32);
        assert_eq!(apply_shift(&cpu, 0b1000_0011, cpu.get_reg(0)), (0, true));
        assert_eq!(apply_shift(&cpu, 0b00000_010, cpu.get_reg(0)), (0, true));

        // check lsr 0/32 with high bit = 0
        cpu.set_reg(1, 0x7FFFFFF);
        assert_eq!(apply_shift(&cpu, 0b1000_0011, cpu.get_reg(1)), (0, false));
        assert_eq!(apply_shift(&cpu, 0b00000_010, cpu.get_reg(1)), (0, false));

        // lsr by more than 32
        cpu.set_reg(9, 33);
        assert_eq!(apply_shift(&cpu, 0b1001_0011, cpu.get_reg(15)), (0, false));
        assert_eq!(apply_shift(&cpu, 0b1001_0011, cpu.get_reg(10)), (0, false));
    }

    #[test]
    fn shift_asr() {
        let mut cpu = CPU::new();

        // check positive, msdb = 1
        cpu.set_reg(0, 0x3123453F);
        assert_eq!(apply_shift(&cpu, 0b00101_100, cpu.get_reg(0)), (0x3123453F >> 5, true));

        // check negative, msdb = 0
        cpu.set_reg(1, 0xF12345A8);
        assert_eq!(
            apply_shift(&cpu, 0b00101_100, cpu.get_reg(1)),
            (((0xF12345A8u32 as i32) >> 5) as u32, false));

        // check ASR 0 (32)
        assert_eq!(apply_shift(&cpu, 0b00000_100, cpu.get_reg(0)), (0, false));
        assert_eq!(apply_shift(&cpu, 0b00000_100, cpu.get_reg(1)), (MAX, true));

        // check ASR > 32
        cpu.set_reg(14, 33);
        assert_eq!(apply_shift(&cpu, 0b1110_0101, cpu.get_reg(0)), (0, false));
        assert_eq!(apply_shift(&cpu, 0b1110_0101, cpu.get_reg(1)), (MAX, true));
    }

    #[test]
    fn shift_ror() {
        let mut cpu = CPU::new();

        // ROR 0/RRX
        cpu.set_reg(0, 0x3123453F);
        assert_eq!(apply_shift(&cpu, 0b00000_110, cpu.get_reg(0)), (0x3123453F >> 1, true));

        cpu.cpsr.carry = true;
        cpu.set_reg(1, 0xFFFFFFFE);
        assert_eq!(apply_shift(&cpu, 0b00000_110, cpu.get_reg(1)), (0xFFFFFFFF, false));

        // ROR 5 with bit 4 = 1
        assert_eq!(
            apply_shift(&cpu, 0b00101_110, cpu.get_reg(0)),
            (0x3123453Fu32.rotate_right(5), true));
        // ROR 5 with bit 4 = 0
        cpu.set_reg(2, 0x12345608);
        assert_eq!(
            apply_shift(&cpu, 0b00101_110, cpu.get_reg(2)),
            (0x12345608u32.rotate_right(5), false));

        // ROR >= 32
        cpu.set_reg(14, 32);
        assert_eq!(
            apply_shift(&cpu, 0b1110_0111, cpu.get_reg(0)),
            (0x3123453F, false));
        cpu.set_reg(14, 37);
        assert_eq!(
            apply_shift(&cpu, 0b1110_0111, cpu.get_reg(2)),
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
    fn sbc() {
        // subtract two large numbers and check for overflow
        let mut cpu = CPU::new();
        cpu.cpsr.carry = true;
        cpu.set_reg(2, 0xD1234567);

        let ins = DataProc {
            opcode: Op::SBC,
            set_flags: true,
            rn: 2,
            rd: 3,
            // this will get rotated to 0xEF_000000
            op2: RegOrImm::Imm { rotate: 2, value: 0xEF }
        };
        ins.run(&mut cpu);

        assert_eq!(cpu.get_reg(3), 0xE1234559);
        assert_eq!(cpu.cpsr.carry, false);
        assert_eq!(cpu.cpsr.zero, false);
        assert_eq!(cpu.cpsr.neg, true);
        assert_eq!(cpu.cpsr.overflow, false);
    }

    #[test]
    fn add_wrapped() {
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
        assert_eq!(cpu.cpsr.overflow, false);
    }

    #[test]
    fn add_overflow() {
        let mut cpu = CPU::new();
        cpu.set_reg(0, MAX/2 - 100);
        cpu.set_reg(1, MAX/2 - 13135);

        let ins = DataProc {
            opcode: Op::ADD,
            set_flags: true,
            rn: 0,
            rd: 3,
            op2: RegOrImm::Reg { shift: 0, reg: 1 }
        };
        ins.run(&mut cpu);

        assert_eq!(cpu.get_reg(3), 0xFFFFCC4B);
        assert_eq!(cpu.cpsr.carry, false);
        assert_eq!(cpu.cpsr.zero, false);
        assert_eq!(cpu.cpsr.neg, true);
        assert_eq!(cpu.cpsr.overflow, true);
    }

    #[test]
    fn mov() {
        let mut cpu = CPU::new();
        DataProc {
            opcode: Op::MOV,
            set_flags: false,
            rn: 0,
            rd: 12,
            op2: RegOrImm::Imm { rotate: 3, value: 1 }
        }.run(&mut cpu);
        assert_eq!(cpu.get_reg(12), 0x4000000);

        DataProc {
            opcode: Op::MOV,
            set_flags: false,
            rn: 0,
            rd: 14,
            op2: RegOrImm::Imm { rotate: 0, value: 4 }
        }.run(&mut cpu);
        assert_eq!(cpu.get_reg(14), 4);
    }

    #[test]
    fn cmp() {
        let mut cpu = CPU::new();
        cpu.set_reg(12, 0x20);
        DataProc {
            opcode: Op::CMP,
            set_flags: true,
            rn: 12,
            rd: 0,
            op2: RegOrImm::Imm { rotate: 0, value: 0 }
        }.run(&mut cpu);
        assert_eq!(cpu.cpsr.zero, false);
        assert_eq!(cpu.cpsr.carry, true);
        assert_eq!(cpu.cpsr.overflow, false);
        assert_eq!(cpu.cpsr.neg, false);
    }


    #[test]
    fn move_carry() {
        // check that an immediate op2 preserves carry for logical ops
        let mut cpu = CPU::new();
        cpu.cpsr.carry = true;
        DataProc {
            opcode: Op::MOV,
            set_flags: true,
            rn: 0,
            rd: 0,
            op2: RegOrImm::Imm { rotate: 0, value: 0 }
        }.run(&mut cpu);
        assert_eq!(cpu.cpsr.carry, true);
    }

    #[test]
    fn shift_reg() {
        // check that LSR by a register with value 0 is the same as LSL 0
        let mut cpu = CPU::new();
        cpu.set_reg(4, 0);
        cpu.set_reg(11, 1);
        cpu.set_reg(12, 0);
        cpu.cpsr.neg = true;
        DataProc {
            opcode: Op::MOV,
            set_flags: true,
            rn: 0,
            rd: 12,
            op2: RegOrImm::Reg { shift: 0b0100_0011, reg: 11 }
        }.run(&mut cpu);
        assert!(!cpu.cpsr.neg);
        assert!(!cpu.cpsr.zero);
        assert_eq!(cpu.get_reg(12), 1);
    }

    #[test]
    fn pc_op() {
        // check that if R15 is used as an operand AND a shift by register
        // amount is specified that the operand(s) get incremented appropriately
        let mut cpu = CPU::new();
        cpu.set_reg(15, 8);
        DataProc {
            opcode: Op::ADD,
            set_flags: false,
            rn: 15,
            rd: 0,
            op2: RegOrImm::Reg { shift: 0b0001_0001, reg: 15 }
        }.run(&mut cpu);
        assert_eq!(cpu.get_reg(0), 24);

        // doesn't happen for shift by immediate
        DataProc {
            opcode: Op::ADD,
            set_flags: false,
            rn: 15,
            rd: 1,
            op2: RegOrImm::Reg { shift: 0, reg: 15 }
        }.run(&mut cpu);
        assert_eq!(cpu.get_reg(1), 16);
    }

    #[test]
    fn tst() {
        let mut cpu = CPU::new();
        cpu.cpsr.carry = true;
        cpu.set_reg(0, 0x3000564);
        DataProc {
            opcode: Op::TST,
            set_flags: true,
            rn: 0,
            rd: 0,
            op2: RegOrImm::Imm { rotate: 4, value: 0b1110}
        }.run(&mut cpu);
        assert!(!cpu.cpsr.carry)
    }
}
