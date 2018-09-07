#[macro_use]
extern crate enum_primitive;
extern crate num;

use num::FromPrimitive;

enum_from_primitive! {
#[repr(u8)]
pub enum DataOp {
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

#[derive(PartialEq)]
#[repr(u8)]
pub enum ProcessorMode {
    USR = 0b10000,
    FIQ = 0b10001,
    IRQ = 0b10010,
    SVC = 0b10011,
    ABT = 0b10111,
    UND = 0b11011,
    SYS = 0b11111
}

enum_from_primitive! {
#[repr(u8)]
pub enum ShiftType {
    LSL = 0,
    LSR,
    ASR,
    RSR
}
}

pub struct CPU {
    /// r0-r12 are general purpose registers
    r: [u32; 13],
    /// R8-R14 are banked in FIQ mode
    r_fiq: [u32; 7],
    /// R13-R14 are banked in IRQ mode
    r_irq: [u32; 2],
    /// R13-R14 are banked in UND mode
    r_und: [u32; 2],
    /// R13-R14 are banked in ABT mode
    r_abt: [u32; 2],
    /// R13-R14 are banked in SVC mode
    r_svc: [u32; 2],
    /// r13 is typically the stack pointer, but can be used as a general purpose
    /// register if the stack pointer isn't necessary 
    r13: u32,
    /// link register
    r14: u32,
    /// pc pointing to address + 8 of the current instruction
    r15: u32,
    /// current processor status register
    /// bits: [N, Z, C, V, ... I, F, T, M4, M3, M2, M1, M0]
    /// flag | logical instruction     | arithmetic instruction
    ///  N   | none                    | bit 31 of the result has been set
    ///  Z   | result is 0             | result is 0
    ///  C   | carry flag after shift  | result was > than 32 bits
    ///  V   | none                    | result was > 31 bits 
    cpsr: u32,
    /// banked SPSR registers
    spsr_svc: u32,
    spsr_abt: u32,
    spsr_und: u32,
    spsr_irq: u32,
    spsr_fiq: u32
}

impl CPU {
    /// negative result from ALU flag
    fn get_n(&self) -> u32 {
        self.cpsr >> 31
    }

    fn set_n(&mut self, val: bool) {
        unimplemented!()
    }

    fn get_mode(&self) -> ProcessorMode {
        unimplemented!()
    }

    /// zero result from ALU flag
    fn get_z(&self) -> u32 {
        (self.cpsr >> 30) & 1
    }

    fn set_z(&mut self, val: bool) {
        unimplemented!()
    }

    /// ALU operation carried out
    fn get_c(&self) -> u32 {
        (self.cpsr >> 29) & 1
    }

    fn set_c(&mut self, val: bool) {
        // TODO: separate out cspr values?
        unimplemented!()
    }

    /// ALU operation overflowed
    fn get_v(&self) -> u32 {
        (self.cpsr >> 28) & 1
    }

    fn set_v(&mut self, val: bool) {
        unimplemented!()
    }

    fn get_cpsr(&self) -> u32 {
        unimplemented!()
    }

    /// restore CPSR to the SPSR for the current mode
    fn restore_cpsr(&mut self) {
        unimplemented!()
    }

    fn set_cpsr(&mut self, val: u32) {
        unimplemented!()
    }

    fn get_spsr(&self) -> u32 {
        unimplemented!()
    }

    /// Set the SPSR for the current mode
    fn set_spsr(&mut self, val: u32) {
        unimplemented!()
    }

    fn satisfies_cond(&self, cond: u8) -> bool {
        match CondField::from_u8(cond) {
            Some(CondField::EQ) => self.get_z() == 1,
            Some(CondField::NE) => self.get_z() == 0,
            Some(CondField::CS) => self.get_c() == 1,
            Some(CondField::CC) => self.get_c() == 0,
            Some(CondField::MI) => self.get_n() == 1,
            Some(CondField::PL) => self.get_n() == 0,
            Some(CondField::VS) => self.get_v() == 1,
            Some(CondField::VC) => self.get_v() == 0,
            Some(CondField::HI) => self.get_c() == 1 && self.get_v() == 0,
            Some(CondField::LS) => self.get_c() == 0 || self.get_v() == 1,
            Some(CondField::GE) => self.get_n() == self.get_v(),
            Some(CondField::LT) => self.get_n() != self.get_v(),
            Some(CondField::GT) => self.get_z() == 0 && (self.get_n() == self.get_v()),
            Some(CondField::LE) => self.get_z() == 1 || (self.get_n() != self.get_v()),
            Some(CondField::AL) => true,
            None => false
        }
    }

    pub fn process_arm_instruction(&mut self, ins: u32) {
        let cond = (ins >> 28) as u8;
        if !self.satisfies_cond(cond) {
            return;
        }
    }

    pub fn branch_and_exchange(&mut self, ins: u32) {
        let register = (ins & 0x0000000F) as u8;
        let thumb = (register & 1) == 1;
        self.r15 = self.r[register as usize];
    }

    pub fn branch(&mut self, ins: u32) {
        let link = (ins >> 24) == 1;
        if link {
            // PC - 4 to adjust for prefetch
            self.r14 = self.r15 - 4;
        }
        let mut offset = ins & 0x00FFFFFF; // 24 lower bits
        // TODO: use build in arithmetic shift by converting to signed?
        if (offset >> 23) == 1 {
            offset |= 0xFF000000; // sign extend
        }
        // TODO: group registers and create PC method?
        self.r15 = ((self.r15 as i64) + ((offset << 2) as i64)) as u32;
    }

    /// Data processing instruction, which has the following format:
    /// | 31 ... 28 | 27 | 26 | 25 | 24 ... 21 | 20 | 19 ... 16 | 15 ... 12 | 11 ... 0  |
    /// |    cond   | 0  | 0  | I  |  opcode   | S  |     Rn    |    Rd     | operand 2 |
    pub fn data_proc(&mut self, ins: u32) -> Result<(), &'static str> {
        let opcode = DataOp::from_u32(((ins >> 1) & 0x00F00000) >> 19).unwrap();
        let dest = (((ins >> 12) as u8) & 0x0F) as usize;
        let op1 = self.r[(((ins >> 16) as u8) & 0x0F) as usize];
        let (op2, shift_carry) = if (ins >> 25) == 1 {
            // immediate operand rotate field is a 4 bit unsigned int which specifies
            // a shift operation on the 8 bit immediate value
            let rotate = (ins >> 8) & 0xF;
            // the imm value i szero extended to 32 bits then subject to a rotate
            // right by twice the value in the rotate field
            ((ins & 0x0000000F).rotate_right(rotate * 2), false)
            // TODO: what is carry flag set to when I=1 and a logical op is used?
        } else {
            self.apply_shift(ins)?
        };
        
        let (result, carry_out) = match opcode {
            DataOp::AND => (op1 & op2, shift_carry),
            DataOp::EOR => (op1 ^ op2, shift_carry),
            DataOp::SUB => op1.overflowing_sub(op2),
            DataOp::RSB => op2.overflowing_sub(op1),
            DataOp::ADD => op1.overflowing_add(op2),
            DataOp::ADC => {
                let (r1, c1) = op1.overflowing_add(op2);
                let (r2, c2) = r1.overflowing_add(self.get_c());
                (r2, c1 || c2)
            },
            DataOp::SBC => {
                let (r1, c1) = op1.overflowing_sub(op2);
                let (r2, c2) = r1.overflowing_sub(1);
                let sub_overflow = c1 || c2;
                let (result, add_overflow) = r2.overflowing_add(self.get_c());
                // if we "underflowed" then overflowed, then they cancel out
                (result, sub_overflow ^ add_overflow)
            },
            DataOp::RSC => {
                let (r1, c1) = op2.overflowing_sub(op1);
                let (r2, c2) = r1.overflowing_sub(1);
                let sub_overflow = c1 || c2;
                let (result, add_overflow) = r2.overflowing_add(self.get_c());
                // if we "underflowed" then overflowed, then they cancel out
                (result, sub_overflow ^ add_overflow)
            },
            DataOp::TST => (op1 & op2, shift_carry),
            DataOp::TEQ => (op1 ^ op2, shift_carry),
            DataOp::CMP => op1.overflowing_sub(op2),
            DataOp::CMN => op1.overflowing_add(op2),
            DataOp::ORR => (op1 | op2, shift_carry),
            DataOp::MOV => (op2, shift_carry),
            DataOp::BIC => (op1 & (!op2), shift_carry),
            DataOp::MVN => (!op2, shift_carry)
        };

        let should_write = match opcode {
            DataOp::TST |
            DataOp::TEQ |
            DataOp::CMP |
            DataOp::CMN => true,
            _ => false
        };

        if should_write {
            self.r[dest] = result;
        }

        let set_status_bit = ((ins >> 20) & 1) == 1;
        if !set_status_bit && should_write {
            return Err("trying to use data instruction handler on a MRS/MSR instruction");
        }
    
        if set_status_bit || !should_write  {
            // TODO: how are we supposed to know if the operands are signed?
            // and detect if the V flag should be set
            self.set_c(carry_out);
            self.set_z(result == 0);
            self.set_n(((result >> 31) & 1) == 1);
        }

        if dest == 15 && set_status_bit {
            self.restore_cpsr();
        }
        Ok(())
    }

    /// uses the rightmost 12 bits to get the second operand from a register and
    /// the carry out from the barrel shifter for the data process instruction.
    /// The 12 bits follows one of the following formats:
    /// ```
    /// | 11      ...       7 | 6   ...  5 | 4 | Rm
    /// |    shift amount     | shift type | 0 |
    /// 
    /// | 11      ...   8 | 7 | 6   ...  5 | 4 | Rm
    /// |  shift register | 0 | shift type | 1 |
    /// ```
    /// where:
    ///   - Rm is the register whose contents we want to shift
    ///   - shift amount is a 5 bit field which indicates how much to shift
    ///   - shift type
    ///     - 00 = logical left
    ///     - 01 = logical right
    ///     - 10 = arithmetic right
    ///     - 11 = rotate right
    ///   - the least significant byte of the contents of the shift register are
    ///     used to determine the shift amount
    pub fn apply_shift(&mut self, ins: u32) -> Result<(u32, bool), &'static str> {
        let shift_amount = if ((ins >> 4) & 1) == 0 {
            (ins >> 7) & 0b11111             
        } else if ((ins >> 4) & 1) == 1 && ((ins >> 7) & 1) == 0 {
            let rs = ((ins >> 8) & 0xF) as usize;
            if rs == 15 {
                return Err("cannot use R15 as shift amount");
            }
            self.r[rs] & 0xFF
        } else {
            return Err("invalid input in apply_shift");
        };

        let rm_val = self.r[(ins & 0xF) as usize];
        match ShiftType::from_u8((ins >> 5) as u8) {
            Some(ShiftType::LSL) => {
                if shift_amount == 0 {
                    return Ok((rm_val, self.get_c() == 1));
                } else if shift_amount > 32 {
                    return Ok((0, false));
                }
                // save the least significant discarded bit as the carry output
                let carry_out = (rm_val >> (32 - shift_amount)) & 1;
                return Ok((rm_val << shift_amount, carry_out == 1));
            },
            Some(ShiftType::LSR) => {
                // LSR #0 is actually interpreted as ASR #32 since it is redundant
                // with LSL #0 
                if shift_amount == 0 {
                    let carry_out = (rm_val >> 31) & 1;
                    return Ok((
                        if carry_out == 1 {std::u32::MAX} else {0},
                        carry_out == 1))
                } else if shift_amount > 32 {
                    return Ok((0, false));
                } else {
                    // otherwise use most significant discarded bit as the carry output
                    let partial_shifted = rm_val >> (shift_amount - 1);
                    let carry_out = partial_shifted & 1;
                    return Ok((partial_shifted >> 1, carry_out == 1));
                }
            },
            Some(ShiftType::ASR) => {
                if shift_amount == 0 {
                    return Ok((rm_val, self.get_c() == 1));
                } else if shift_amount > 32 {
                    let top_bit = (rm_val >> 31) & 1;
                    return Ok((
                        if top_bit == 1 {std::u32::MAX} else {0},
                        top_bit == 1));
                }
                // convert to i32 to get arithmetic shifting
                let partial_shifted = (rm_val as i32) >> (shift_amount - 1);
                let carry_out = partial_shifted & 1;
                return Ok(((partial_shifted >> 1) as u32, carry_out == 1));
            },
            Some(ShiftType::RSR) => {
                // RSR #0 is used to encode RRX
                if shift_amount == 0 {
                    let result = (rm_val >> 1) | (self.get_c() << 31);
                    return Ok((result, (rm_val & 1) == 1));
                }
                let result = rm_val.rotate_right(shift_amount);
                let carry_out = (result >> 31) & 1;
                return Ok((result, carry_out == 1));
            },
            None => Err("invalid shift type")
        }
    }

    /// MRS/MSR instructions are TEQ/TST/CMN/CMP Data Processing operations but
    /// without the S flag set and allow access to the CPSR/SPSR registers. The
    /// instruction formats are:
    /// 
    /// transfer PSR contents to register
    /// 31 .. 28 | 27 .. 23 | 22 | 21 .. 16 | 15 .. 12 | 11 .. 0 |
    ///   cond   |   00010  | Ps |  001111  |    Rd    | 00....0 |
    /// 
    /// transfer Rm contents to PSR
    /// 31 .. 28 | 27 .. 23 | 22 | 21  ..  12 | 11 .. 4 | 3 .. 0 |
    ///   cond   |   00010  | Pd | 1010011111 | 00....0 | Rm     |
    /// 
    /// transfer reg or immediate contents to PSR flag bits only
    /// 31 .. 28 | 27 | 26 | 25 | 24 | 23 | 22 | 21  ..  12 | 11  .. 0 |
    ///    cond  | 0  | 0  | I  | 1  | 0  | Pd | 1010001111 | operand
    /// 
    /// where Px is 0 for the CPSR and 1 for the SPSR of the current mode
    pub fn psr_transfer(&mut self, ins: u32) {
        if ((ins >> 21) & 1) == 0 { // read
            let val = if ((ins >> 22) & 1) == 1 {
                self.get_spsr()
            } else {
                self.get_cpsr()
            };
            self.r[((ins >> 12) & 0xF) as usize] = val;
        } else {
            let val = if ((ins >> 25) & 1) == 1 {
                // TODO: refactor immediate + rotate logic
                let rotate = (ins >> 8) & 0xF;
                (ins & 0x0000000F).rotate_right(rotate * 2)
            } else { // reg
                self.r[(ins & 0xF) as usize]
            };

            if ((ins >> 22) & 1) == 1 {
                self.set_spsr(val);
            } else {
                self.set_cpsr(val);
            }
        }
    }

    /// multiply instruction, which is a data processing instruction where both 
    /// bits 4 and 7 are 1
    /// 
    /// 27 .. 22 | 21 | 20 | 19 .. 16 | 15 .. 12 | 11 .. 8 | 7 .. 4 | 3 .. 0
    ///   000000 | A  | S  |    Rd    |    Rn    |    Rs   |  1001  |  Rm 
    /// 
    pub fn multiply(&mut self, ins: u32) -> Result<(), &'static str> {
        let rm = ins & 0xF;
        let rn = (ins >> 12) & 0xF;
        let rd = (ins >> 16) & 0xF;
        if rd == 15 || rm == 15 || rn == 15 {
            return Err("Can't use R15 as operand or dest in mul");
        }
        if rd == rm {
            return Err("Rd and Rm can't be the same in mul");
        }
        let mut result = (self.r[rm as usize] as u64) * (self.r[rn as usize] as u64);
        if (ins >> 21) & 1 == 1 {
            result += self.r[((ins >> 8) & 0xF) as usize] as u64;
        }
        self.r[rd as usize] = result as u32;
        if (ins >> 20) & 1 == 1 {
            self.set_n(((result >> 31) & 1) == 1);
            self.set_z(result == 0);
        }
        Ok(())
    }

    /// multiply into a u64 that is stored into a hi and lo register
    /// 27 .. 23 | 22 | 21 | 20 | 19 .. 16 | 15 .. 12 | 11 .. 8 | 7 .. 4 | 3 .. 0
    ///   00001  | U  | A  | S  |   Rd hi  |   Rd lo  |   Rs    |  1001  |  Rm
    pub fn multiply_long(&mut self, ins: u32) -> Result<(), &'static str> {
        let rm = ins & 0xF;
        let rs = (ins >> 8) & 0xF;
        let rdhi = (ins >> 16) & 0xF;
        let rdlo = (ins >> 12) & 0xF;
        if rm == 15 || rs == 15 || rdhi == 15 || rdlo == 15 {
            return Err("Can't use R15 as operand or dest in mul");
        }
        if rdhi == rdlo || rdhi == rm || rdlo == rm {
            return Err("RdHi, RdLo, and Rm must all specify different registers");
        }

        let mut result = (self.r[rm as usize] as u64) * (self.r[rs as usize] as u64);
        if (ins >> 21) & 1 == 1 {
            result *= 2;
        }

        let top = (result >> 32) as u32;
        let bot = result as u32;
        self.r[rdhi as usize] = top;
        self.r[rdlo as usize] = bot;
        if (ins >> 20) & 1 == 1 {
            self.set_n(((top >> 31) & 1) == 1);
            self.set_z(result == 0);
        }
        Ok(())
    }
}
