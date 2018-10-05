use ::cpu::CPU;
use ::util;

/// The multiply and multiply-accumulate instructions perform integer multiplication
/// on the contents of two registers Rm and Rs and stores the lower 32 bits of the
/// result in Rd
#[derive(Debug)]
pub struct Multiply {
    /// if true, add contents of Rn to the product before storing in Rd
    pub accumulate: bool,
    pub set_flags: bool,
    pub rd: usize,
    pub rn: usize,
    pub rs: usize,
    pub rm: usize
}

impl Multiply {
    /// parses from the following format:
    /// 27 .. 22 | 21 | 20 | 19 .. 16 | 15 .. 12 | 11 .. 8 | 7 .. 4 | 3 .. 0
    ///   000000 | A  | S  |    Rd    |    Rn    |    Rs   |  1001  |  Rm 
    pub fn parse_instruction(ins: u32) -> Multiply {
        Multiply {
            accumulate: util::get_bit(ins, 21),
            set_flags: util::get_bit(ins, 20),
            rd: util::get_nibble(ins, 16) as usize,
            rn: util::get_nibble(ins, 12) as usize,
            rs: util::get_nibble(ins, 8) as usize,
            rm: util::get_nibble(ins, 0) as usize
        }
    }

    pub fn run(&self, cpu: &mut CPU) -> u32 {
        if self.rd == 15 || self.rm == 15 || self.rn == 15 {
            panic!("Can't use R15 as operand or dest in mul");
        }
        if self.rd == self.rm {
            panic!("Rd and Rm can't be the same in mul");
        }
        // since we only care about the bottom 32 bits, this will be the same
        // for both signed and unsigned integers
        let multiplier = cpu.get_reg(self.rs);
        let mut result: u64 = (cpu.get_reg(self.rm) as u64) * (multiplier as u64);
        if self.accumulate {
            result += cpu.get_reg(self.rn) as u64;
        }
        cpu.set_reg(self.rd, result as u32);
        if self.set_flags {
            cpu.cpsr.neg = ((result >> 31) & 1) == 1;
            cpu.cpsr.zero = result == 0;
        }

        cpu.mem.access_time(cpu.r[15], false) +
            mul_cycle_time(multiplier) +
            if self.accumulate { 1 } else { 0 }
    }
}

pub fn mul_cycle_time(multiplier: u32) -> u32 {
    let second_byte = (multiplier >> 8) as u8;
    let third_byte = (multiplier >> 16) as u8;
    let fourth_byte = (multiplier >> 24) as u8;
    1 + if second_byte == 0 || second_byte == 0xFF { 0 } else { 1 } +
        if third_byte == 0 || third_byte == 0xFF { 0 } else { 1 } +
        if fourth_byte == 0 || fourth_byte == 0xFF { 0 } else { 1 }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn parse() {
        let mul = Multiply::parse_instruction(
            0b0000_00000_1_1_0001_1000_1111_1001_0010);
        assert!(mul.accumulate);
        assert!(mul.set_flags);
        assert_eq!(mul.rd, 1);
        assert_eq!(mul.rn, 8);
        assert_eq!(mul.rs, 15);
        assert_eq!(mul.rm, 2);
    }
}
