use ::cpu::CPU;
use ::cpu::arm::mul::mul_cycle_time;
use ::util;

/// The multiply and multiply-accumulate instructions perform integer multiplication
/// on the contents of two registers Rm and Rs and stores the lower 32 bits of the
/// result in RdLo and the high 32 bits in RdHi
#[derive(Clone, Debug)]
pub struct MultiplyLong {
    /// if true, add contents of RdHi,RdLo (as a 64 bit integer) to the product
    /// before storing it
    accumulate: bool,
    /// if true, treat operands as two's complement signed numbers and write a
    /// two's complement signed 64 bit result
    is_signed: bool,
    set_flags: bool,
    rdhi: usize,
    rdlo: usize,
    rs: usize,
    rm: usize
}

impl MultiplyLong {
    /// parses from the following format:
    /// 27 .. 23 | 22 | 21 | 20 | 19 .. 16 | 15 .. 12 | 11 .. 8 | 7 .. 4 | 3 .. 0
    ///   00001  | U  | A  | S  |   Rd hi  |   Rd lo  |   Rs    |  1001  |  Rm
    pub fn parse_instruction(ins: u32) -> MultiplyLong {
        MultiplyLong {
            is_signed: util::get_bit(ins, 22),
            accumulate: util::get_bit(ins, 21),
            set_flags: util::get_bit(ins, 20),
            rdhi: util::get_nibble(ins, 16) as usize,
            rdlo: util::get_nibble(ins, 12) as usize,
            rs: util::get_nibble(ins, 8) as usize,
            rm: util::get_nibble(ins, 0) as usize,
        }
    }

    pub fn run(&self, cpu: &mut CPU) -> u32 {
        if self.rm == 15 || self.rs == 15 || self.rdhi == 15 || self.rdlo == 15 {
            panic!("Can't use R15 as operand or dest in mul");
        }
        if self.rdhi == self.rdlo ||  self.rdhi == self.rm || self.rdlo == self.rm {
            panic!("RdHi, RdLo, and Rm must all specify different registers");
        }

        let summand =
            ((cpu.get_reg(self.rdhi) as u64) << 32) |
            (cpu.get_reg(self.rdlo) as u64);

        let multiplier = cpu.get_reg(self.rs);
        let result = if self.is_signed {
            let mut prod: i64 =
                (cpu.get_reg(self.rm) as i32 as i64) *
                multiplier as i32 as i64;
            if self.accumulate {
                prod += summand as i64
            }
            prod as u64
        } else {
            let mut prod: u64 =
                (cpu.get_reg(self.rm) as u64) *
                multiplier as u64;
            if self.accumulate {
                prod += summand
            }
            prod
        };

        let top = (result >> 32) as u32;
        let bot = result as u32;
        cpu.set_reg(self.rdhi, top);
        cpu.set_reg(self.rdlo, bot);
        if self.set_flags {
            cpu.cpsr.neg = ((top >> 31) & 1) == 1;
            cpu.cpsr.zero = result == 0;
        }

        // TODO: this isn't quite accurate for signed mull, see docs
        cpu.mem.access_time(cpu.r[15], false) +
            mul_cycle_time(multiplier) +
            if self.accumulate { 1 } else { 0 }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn parse() {
        let mul = MultiplyLong::parse_instruction(
            0b0000_00001_1_1_1_0001_0010_0011_1001_1000);
        assert!(mul.is_signed);
        assert!(mul.accumulate);
        assert!(mul.set_flags);
        assert_eq!(mul.rdhi, 1);
        assert_eq!(mul.rdlo, 2);
        assert_eq!(mul.rs, 3);
        assert_eq!(mul.rm, 8);
    }
}