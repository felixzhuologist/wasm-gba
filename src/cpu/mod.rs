pub mod arm_isa;

use num::FromPrimitive;
use util;
use self::arm_isa::{
    branch,
    branch_ex,
    data,
    mul,
    mul_long,
    psr,
    single_trans,
    signed_trans,
    block_trans,
};

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

// TODO: what's the difference between a MUL/MULL/signed swap?
pub fn get_instruction_handler(ins: u32) -> Option<Box<arm_isa::Instruction>> {
    let op0 = util::get_nibble(ins, 24);
    let op1 = util::get_nibble(ins, 20);
    let op2 = util::get_nibble(ins, 4);
    if op0 == 0 && op1 < 4 && op2 == 0b1001 {
        Some(Box::new(mul::Multiply::parse_instruction(ins)))
    } else if op0 == 0 && op1 > 7 && op2 == 0b1001 {
        Some(Box::new(mul_long::MultiplyLong::parse_instruction(ins)))
    } else if op0 == 1 && op2 == 1 {
        Some(Box::new(branch_ex::BranchAndExchange::parse_instruction(ins)))
    } else if op0 < 4 {
        let data = data::DataProc::parse_instruction(ins);
        let op = data.opcode as u8;
        if !data.set_flags && op > 7 && op < 12 {
            Some(Box::new(psr::PSRTransfer::parse_instruction(ins)))
        } else {
            Some(Box::new(data))
        }
    } else if op0 >= 4 && op0 < 8 {
        Some(Box::new(single_trans::SingleDataTransfer::parse_instruction(ins)))
    } else if op0 == 8 || op0 == 9 {
        Some(Box::new(block_trans::BlockDataTransfer::parse_instruction(ins)))
    }else if op0 == 10 || op0 == 11 {
        Some(Box::new(branch::Branch::parse_instruction(ins)))
    } else {
        None
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

    // TODO: how should this function look? should we have an enum for ARM/THUMB?
    fn set_isa(&mut self, thumb: bool) {
        unimplemented!()
    }

    fn satisfies_cond(&self, cond: u32) -> bool {
        match CondField::from_u32(cond).unwrap() {
            CondField::EQ => self.get_z() == 1,
            CondField::NE => self.get_z() == 0,
            CondField::CS => self.get_c() == 1,
            CondField::CC => self.get_c() == 0,
            CondField::MI => self.get_n() == 1,
            CondField::PL => self.get_n() == 0,
            CondField::VS => self.get_v() == 1,
            CondField::VC => self.get_v() == 0,
            CondField::HI => self.get_c() == 1 && self.get_v() == 0,
            CondField::LS => self.get_c() == 0 || self.get_v() == 1,
            CondField::GE => self.get_n() == self.get_v(),
            CondField::LT => self.get_n() != self.get_v(),
            CondField::GT => self.get_z() == 0 && (self.get_n() == self.get_v()),
            CondField::LE => self.get_z() == 1 || (self.get_n() != self.get_v()),
            CondField::AL => true
        }
    }

    pub fn process_arm_instruction(&mut self, ins: u32) {
        let cond = util::get_nibble(ins, 28);
        if !self.satisfies_cond(cond) {
            return;
        }

        // it is redundant to pass the same instruction twice but separating
        // this out lets us test the two separate behaviours of picking the
        // right instruction handler, and that the given instruction handler
        // does the right thing.
        get_instruction_handler(ins).unwrap()
            .process_instruction(self);
    }
}

#[cfg(test)]
mod test {

    mod get_instruction_handler {
        use ::cpu::*;
        use ::cpu::arm_isa::InstructionType;
        #[test]
        fn branch() {
            assert_eq!(
                get_instruction_handler(0x0_A_123456).unwrap().get_type(),
                InstructionType::Branch);
            assert_eq!(
                get_instruction_handler(0x0_B_123456).unwrap().get_type(),
                InstructionType::Branch);
        }

        #[test]
        fn bex() {
            assert_eq!(
                get_instruction_handler(0x0_12FFF1_5).unwrap().get_type(),
                InstructionType::BranchAndExchange);
        }

        #[test]
        fn data() {
            assert_eq!(
                get_instruction_handler(0x03123456).unwrap().get_type(),
                InstructionType::DataProc);
            assert_eq!(
                get_instruction_handler(0xA3123456).unwrap().get_type(),
                InstructionType::DataProc);
            assert_eq!(
                get_instruction_handler(0x001A3D56).unwrap().get_type(),
                InstructionType::DataProc);
        }

        #[test]
        fn mul() {
            assert_eq!(
                get_instruction_handler(0x03_123_9_A).unwrap().get_type(),
                InstructionType::Multiply);
            assert_eq!(
                get_instruction_handler(0x02_ABC_9_0).unwrap().get_type(),
                InstructionType::Multiply);
        }

        #[test]
        fn mul_long() {
            assert_eq!(
                get_instruction_handler(0x08_123_9_A).unwrap().get_type(),
                InstructionType::MultiplyLong);
            assert_eq!(
                get_instruction_handler(0x0B_ABC_9_0).unwrap().get_type(),
                InstructionType::MultiplyLong);
        }

        #[test]
        fn psr() {
           assert_eq!(
                get_instruction_handler(0b0011_00010_1_001111_0000_000000000000)
                    .unwrap().get_type(),
                InstructionType::PSRTransfer);
           assert_eq!(
                get_instruction_handler(0b1111_00010_0_1010011111_00000000_1111)
                    .unwrap().get_type(),
                InstructionType::PSRTransfer);
        }

        #[test]
        fn single_trans() {
            assert_eq!(
                get_instruction_handler(0xA_4_123456).unwrap().get_type(),
                InstructionType::SingleDataTransfer);
            assert_eq!(
                get_instruction_handler(0xA_7_ABCDEF).unwrap().get_type(),
                InstructionType::SingleDataTransfer);
        }

        #[test]
        fn block_trans() {
            assert_eq!(
                get_instruction_handler(0x0_8_123456).unwrap().get_type(),
                InstructionType::BlockDataTransfer);
            assert_eq!(
                get_instruction_handler(0x0_9_1DFA10).unwrap().get_type(),
                InstructionType::BlockDataTransfer);
        }
    }
}
