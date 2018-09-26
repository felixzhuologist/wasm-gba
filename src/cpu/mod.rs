pub mod arm_isa;
pub mod status_reg;

use num::FromPrimitive;
use mem;
use util;
use self::arm_isa::{
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
    Instruction,
    RegOrImm};
use self::arm_isa::Instruction::{
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
    Noop
};
use self::arm_isa::data::apply_shift;
use self::status_reg::{CPUMode, PSR, ProcessorMode};

/// A wrapper structs that keeps the inner CPU and pipeline in separate fields
/// to allow for splitting the borrow when executing an instruction
pub struct CPUWrapper {
    cpu: CPU,
    // since we only need to keep track of the last 3 elements
    // of the pipeline at a time (the latest fetched instruction, the latest
    // decoded instruction, and the next decoded instruction to execute), we
    // use a circular buffer of size 3
    pipeline: [PipelineInstruction; 3],
    // index into the circular buffer
    idx: usize,
}

impl CPUWrapper {
    /// Initialize CPU values assuming boot from GBA BIOS. In particular, with
    /// all regs zeroed out, and with CPSR in ARM and SVC modes with IRQ/FIQ bits
    /// set.
    pub fn new() -> CPUWrapper {
        let mut cpu = CPUWrapper {
            cpu: CPU::new(),
            pipeline: [
                PipelineInstruction::Empty,
                PipelineInstruction::Empty,
                PipelineInstruction::Empty,
            ],
            idx: 0,
        };
        cpu.cpu.cpsr.i = false;
        cpu.cpu.cpsr.f = false;
        cpu.cpu.cpsr.mode = ProcessorMode:: SVC;

        cpu
    }

    fn satisfies_cond(&self, cond: u32) -> bool {
        let cpsr = &self.cpu.cpsr;
        match CondField::from_u32(cond).unwrap() {
            CondField::EQ => cpsr.z,
            CondField::NE => !cpsr.z,
            CondField::CS => cpsr.c,
            CondField::CC => !cpsr.c,
            CondField::MI => cpsr.n,
            CondField::PL => !cpsr.n,
            CondField::VS => cpsr.v,
            CondField::VC => !cpsr.v,
            CondField::HI => cpsr.c && !cpsr.v,
            CondField::LS => !cpsr.c || cpsr.v,
            CondField::GE => cpsr.n == cpsr.v,
            CondField::LT => cpsr.n != cpsr.v,
            CondField::GT => !cpsr.z && (cpsr.n == cpsr.v),
            CondField::LE => cpsr.z || (cpsr.n != cpsr.v),
            CondField::AL => true
        }
    }

    pub fn fetch(&mut self) {
        // self.pipeline[idx] = ...
        // self.idx = (self.idx + 1) % 3;
        unimplemented!()
    }

    /// decode the next instruction. if the condition of the instruction isn't met,
    /// the raw instruction is decoded as a Noop
    pub fn decode(&mut self) {
        // index of the second element from the end
        let idx = ((self.idx as i8 - 2 as i8) % 3) as usize;
        if let PipelineInstruction::Raw(n) = self.pipeline[idx] {
            let cond = util::get_nibble(n, 28);
            self.pipeline[idx] = PipelineInstruction::Decoded(
                if self.satisfies_cond(cond) {
                    get_instruction_handler(n).unwrap()
                } else {
                    Noop
                })
        }
    }

    pub fn execute(&mut self) {
        // index of the third element from the end
        let idx = ((self.idx as i8 - 3 as i8) % 3) as usize;
        if let PipelineInstruction::Decoded(ref ins) = self.pipeline[idx] {
            match ins {
                DataProc(ins) => ins.run(&mut self.cpu),
                PSRTransfer(ins) => ins.run(&mut self.cpu),
                Multiply(ins) => ins.run(&mut self.cpu),
                MultiplyLong(ins) => ins.run(&mut self.cpu),
                SwapTransfer(ins) => ins.run(&mut self.cpu),
                SingleTransfer(ins) => ins.run(&mut self.cpu),
                SignedTransfer(ins) => ins.run(&mut self.cpu),
                BlockTransfer(ins) => ins.run(&mut self.cpu),
                Branch(ins) => ins.run(&mut self.cpu),
                BranchEx(ins) => ins.run(&mut self.cpu),
                SWInterrupt(ins) => ins.run(&mut self.cpu),
                Noop => (),
            }
        }
    }

    pub fn flush_pipeline(&mut self) {
        for i in 0..3 {
            self.pipeline[i] = PipelineInstruction::Empty;
        }
        self.idx = 0;
    }
}

pub struct CPU {
    /// r0-r12 are general purpose registers,
    /// r13 is usually the stack pointer, r14 is usually the link register,
    /// and r15 is the PC pointing to address + 8 of the current instruction
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
    spsr_fiq: PSR,

    // flush the pipeline before the start of the next cycle
    should_flush: bool,

    mem: mem::Memory,
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

            should_flush: false,

            mem: mem::Memory::new(),
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

    // TODO: merge params into struct?
    pub fn transfer_reg(&mut self, params: TransferParams) {
        // pre transfer
        let mut addr = self.get_reg(params.base_reg);
        let offset = self.get_offset(&params.offset);
        if params.pre_index {
            addr = if params.offset_up { addr + offset } else { addr - offset };
        }

        // transfer
        if params.load {
            let val = match params.size {
                TransferSize::Byte => {
                    if params.signed {
                        (self.mem.get_byte(addr) as i8) as u32
                    } else {
                        self.mem.get_byte(addr) as u32
                    }
                },
                TransferSize::Halfword => {
                    if params.signed {
                        (self.mem.get_halfword(addr) as i16) as u32
                    } else {
                        self.mem.get_halfword(addr) as u32
                    }
                },
                TransferSize::Word => self.mem.get_word(addr),
            };
            self.set_reg(params.data_reg, val);
        } else {
            let mut val = self.get_reg(params.data_reg);
            if params.data_reg == 15 {
                // when R15 is the source of a STR, the stored value will be the
                // addr of the current instruction + 12
                val += 4;
            }
            match params.size {
                TransferSize::Byte => self.mem.set_byte(addr, val as u8),
                TransferSize::Halfword => self.mem.set_halfword(addr, val),
                TransferSize::Word => self.mem.set_word(addr, val),
            }
        }

        // post transfer
        if !params.pre_index {
            addr = if params.offset_up { addr + offset } else { addr - offset };
        }

        // write back is assumed if post indexing
        if !params.pre_index || params.write_back {
            self.set_reg(params.base_reg, addr);
        } 
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

    fn set_isa(&mut self, thumb: bool) {
        self.cpsr.t = if thumb { CPUMode::THUMB } else { CPUMode::ARM };
    }

    fn get_offset(&self, offset: &RegOrImm) -> u32 {
        match *offset {
            RegOrImm::Imm { rotate: _, value: n } => n,
            RegOrImm::Reg { shift: s, reg: r } => {
                if util::get_bit(s, 3) && util::get_bit(s, 0) {
                    panic!("cannot use register value as shift amount for LDR/STR");
                }
                apply_shift(self, s, r).0
            }
        } 
    }
}

/// An instruction in a specific stage of the ARM7's three stage pipeline
pub enum PipelineInstruction {
    /// A not yet fetched instruction. This is a placeholder for when the
    /// pipeline has just been flushed and the CPU is stalling waiting for the
    /// next instruction to be fetched
    Empty,
    /// A fetched instruction. This could either be a 32bit ARM instruction or
    /// a 16 bit THUMB instruction
    Raw(u32),
    /// A decoded instruction
    Decoded(Instruction)
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

pub fn get_instruction_handler(ins: u32) -> Option<Instruction> {
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

pub struct TransferParams<'a> {
    pre_index: bool,
    offset_up: bool,
    size: TransferSize,
    write_back: bool,
    load: bool,
    base_reg: usize,
    data_reg: usize,
    signed: bool,
    offset: &'a RegOrImm
}

pub enum TransferSize {
    Byte,
    Halfword,
    Word,
}

#[cfg(test)]
mod test {

    mod cpu {
        use ::cpu::*;

        #[test]
        fn transfer_load() {
            let mut cpu = CPU::new();
            cpu.set_reg(0, 80);
            cpu.mem.set_byte(100, 77);
            cpu.transfer_reg(TransferParams {
                pre_index: true,
                offset_up: true,
                size: TransferSize::Byte,
                write_back: false,
                load: true,
                base_reg: 0,
                data_reg: 1,
                signed: false,
                offset: &RegOrImm::Imm { rotate: 0, value: 20 }
            });
            assert_eq!(cpu.get_reg(1), 77);
        }

        #[test]
        fn transfer_store_autoindex() {
            let mut cpu = CPU::new();
            cpu.set_reg(0, 100);
            cpu.set_reg(1, 77);
            cpu.transfer_reg(TransferParams {
                pre_index: false,
                offset_up: false,
                size: TransferSize::Byte,
                write_back: false,
                load: false,
                base_reg: 0,
                data_reg: 1,
                signed: false,
                offset: &RegOrImm::Imm { rotate: 0, value: 20 }
            });
            assert_eq!(cpu.mem.get_byte(100), 77);
            assert_eq!(cpu.get_reg(0), 80);
        }

        #[test]
        fn transfer_load_signed() {
            let mut cpu = CPU::new();
            cpu.set_reg(0, 100);
            cpu.mem.set_word(100, 0xA10B);
            cpu.transfer_reg(TransferParams {
                pre_index: false,
                offset_up: false,
                size: TransferSize::Halfword,
                write_back: true,
                load: true,
                base_reg: 0,
                data_reg: 14,
                signed: true,
                offset: &RegOrImm::Imm { rotate: 0, value: 20 }
            });
            assert_eq!(cpu.get_reg(14), 0xFFFFA10B);
            assert_eq!(cpu.get_reg(0), 80);
        }
    }

    mod get_instruction_handler {
        use ::cpu::*;
        use ::cpu::arm_isa::Instruction;

        macro_rules! has_type {
            ($instr:expr, $instr_type: pat) => (
                assert!(match get_instruction_handler($instr).unwrap() {
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
}
