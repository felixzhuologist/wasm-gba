pub mod arm_isa;
pub mod status_reg;

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
    swi,
    swap
};
use self::status_reg::{CPUMode, PSR, ProcessorMode};

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

pub fn get_instruction_handler(ins: u32) -> Option<Box<arm_isa::Instruction>> {
    let op0 = util::get_nibble(ins, 24);
    let op1 = util::get_nibble(ins, 20);
    let op2 = util::get_nibble(ins, 4);
    if op0 == 0 && op1 < 4 && op2 == 0b1001 {
        Some(Box::new(mul::Multiply::parse_instruction(ins)))
    } else if op0 == 0 && op1 > 7 && op2 == 0b1001 {
        Some(Box::new(mul_long::MultiplyLong::parse_instruction(ins)))
    } else if op0 == 1 && op2 == 9 {
        Some(Box::new(swap::SingleDataSwap::parse_instruction(ins)))
    } else if op0 == 1 && op2 == 1 {
        Some(Box::new(branch_ex::BranchAndExchange::parse_instruction(ins)))
    } else if op0 < 2 && (op2 == 9 || op2 == 11 || op2 == 13 || op2 == 15) {
        // if bits 4 and 7 are 1, this must be a signed/hw transfer
        Some(Box::new(signed_trans::SignedDataTransfer::parse_instruction(ins)))
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
    } else if op0 == 10 || op0 == 11 {
        Some(Box::new(branch::Branch::parse_instruction(ins)))
    } else if op0 == 15 {
        Some(Box::new(swi::SWInterrupt::parse_instruction(ins)))
    } else {
        None
    }
}

pub struct CPU {
    /// r0-r12 are general purpose registers,
    /// r13 is typically the stack pointer, but can be used as a general purpose
    /// register if the stack pointer isn't necessary,
    /// r14 is the link register (for storing addressses to jump back to)/a
    /// general purpose register, and r15 is the PC pointing to address + 8 of
    /// the current instruction
    r: [u32; 16],
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

    /// program state registers
    cpsr: PSR,
    /// banked SPSR registers
    spsr_svc: PSR,
    spsr_abt: PSR,
    spsr_und: PSR,
    spsr_irq: PSR,
    spsr_fiq: PSR
}

impl CPU {
    pub fn new() -> CPU {
        CPU {
            r: [0; 16],
            r_fiq: [0; 7],
            r_irq: [0; 2],
            r_und: [0, 2],
            r_abt: [0, 2],
            r_svc: [0, 2],

            cpsr: PSR::new(),
            spsr_svc: PSR::new(),
            spsr_abt: PSR::new(),
            spsr_und: PSR::new(),
            spsr_irq: PSR::new(),
            spsr_fiq: PSR::new(),
        }
    }
    pub fn get_reg(&self, reg: usize) -> u32 {
        match reg {
            15 |
            0 ... 7 => self.r[reg],
            8 ... 12 => match self.cpsr.mode {
                ProcessorMode::FIQ => self.r_fiq[reg - 8],
                _ => self.r[reg]
            },
            13 ... 14 => match self.cpsr.mode {
                ProcessorMode::USR |
                ProcessorMode::SYS => self.r[reg],
                ProcessorMode::FIQ => self.r_fiq[reg - 8],
                ProcessorMode::IRQ => self.r_irq[reg - 13],
                ProcessorMode::UND => self.r_und[reg - 13],
                ProcessorMode::ABT => self.r_abt[reg - 13],
                ProcessorMode::SVC => self.r_svc[reg - 13],
            },
            _ => panic!("tried to access register {}", reg)
        }
    }

    pub fn set_reg(&mut self, reg: usize, val: u32) {
        match reg {
            15 |
            0 ... 7 => self.r[reg] = val,
            8 ... 12 => match self.cpsr.mode {
                ProcessorMode::FIQ => self.r_fiq[reg - 8] = val,
                _ => self.r[reg] = val
            },
            13 ... 14 => match self.cpsr.mode {
                ProcessorMode::USR |
                ProcessorMode::SYS => self.r[reg] = val,
                ProcessorMode::FIQ => self.r_fiq[reg - 8] = val,
                ProcessorMode::IRQ => self.r_irq[reg - 13] = val,
                ProcessorMode::UND => self.r_und[reg - 13] = val,
                ProcessorMode::ABT => self.r_abt[reg - 13] = val,
                ProcessorMode::SVC => self.r_svc[reg - 13] = val,
            },
            _ => panic!("tried to set register {}", reg)
        };
    }

    /// restore CPSR to the SPSR for the current mode
    fn restore_cpsr(&mut self) {
        unimplemented!()
    }

    fn set_cpsr(&mut self, val: u32) {
        self.cpsr.from_u32(val);
    }

    fn get_spsr(&self) -> PSR {
        unimplemented!()
    }

    /// Set the SPSR for the current mode
    fn set_spsr(&mut self, val: u32) {
        unimplemented!()
    }

    // TODO: how should this function look? should we have an enum for ARM/THUMB?
    fn set_isa(&mut self, thumb: bool) {
        self.cpsr.t = if thumb { CPUMode::THUMB } else { CPUMode::ARM };
    }

    fn satisfies_cond(&self, cond: u32) -> bool {
        match CondField::from_u32(cond).unwrap() {
            CondField::EQ => self.cpsr.z,
            CondField::NE => !self.cpsr.z,
            CondField::CS => self.cpsr.c,
            CondField::CC => !self.cpsr.c,
            CondField::MI => self.cpsr.n,
            CondField::PL => !self.cpsr.n,
            CondField::VS => self.cpsr.v,
            CondField::VC => !self.cpsr.v,
            CondField::HI => self.cpsr.c && !self.cpsr.v,
            CondField::LS => !self.cpsr.c || self.cpsr.v,
            CondField::GE => self.cpsr.n == self.cpsr.v,
            CondField::LT => self.cpsr.n != self.cpsr.v,
            CondField::GT => !self.cpsr.z && (self.cpsr.n == self.cpsr.v),
            CondField::LE => self.cpsr.z || (self.cpsr.n != self.cpsr.v),
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

        #[test]
        fn sw_interrupt() {
            assert_eq!(
                get_instruction_handler(0xFF_123ABC).unwrap().get_type(),
                InstructionType::SWInterrupt);
        }

        #[test]
        fn swap() {
            assert_eq!(
                get_instruction_handler(0xF_1_0_123_9_5).unwrap().get_type(),
                InstructionType::SingleDataSwap);
            assert_eq!(
                get_instruction_handler(0xF_1_8_ABC_9_E).unwrap().get_type(),
                InstructionType::SingleDataSwap);
        }

        #[test]
        fn signed_halfword_transfer() {
            assert_eq!(
                get_instruction_handler(0xF_1_0BE0_B_3).unwrap().get_type(),
                InstructionType::SignedDataTransfer);
            assert_eq!(
                get_instruction_handler(0xF_0_FABC_D_3).unwrap().get_type(),
                InstructionType::SignedDataTransfer);
            assert_eq!(
                get_instruction_handler(0xF_0_7123_F_3).unwrap().get_type(),
                InstructionType::SignedDataTransfer);
        }
    }
}
