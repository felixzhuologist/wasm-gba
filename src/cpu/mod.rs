pub mod arm;
pub mod pipeline;
pub mod thumb;
pub mod status_reg;

use self::arm::RegOrImm;
use self::arm::data::apply_shift;
use self::status_reg::{InstructionSet, PSR, CPUMode};
use self::pipeline::{
    decode_arm,
    decode_thumb,
    Instruction,
    PipelineInstruction,
    satisfies_cond
};
use mem;
use util;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern {
    #[wasm_bindgen(js_namespace = console)]
    fn log(msg: &str);
}

// A macro to provide `println!(..)`-style syntax for `console.log` logging.
macro_rules! log {
    ($($t:tt)*) => (log(&format!($($t)*)))
}

/// A wrapper structs that keeps the inner CPU and pipeline in separate fields
/// to allow for splitting the borrow when executing an instruction
pub struct CPUWrapper {
    pub cpu: CPU,
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
    pub const fn new() -> CPUWrapper {
        CPUWrapper {
            cpu: CPU::new(),
            pipeline: [
                PipelineInstruction::Empty,
                PipelineInstruction::Empty,
                PipelineInstruction::Empty,
            ],
            idx: 0,
        }
    }

    /// Run a single instruction
    pub fn step(&mut self) {
        self.fetch();
        self.decode();
        self.execute();

        if self.cpu.should_flush {
            self.flush_pipeline();
            self.cpu.should_flush = false;
        } else {
            self.idx = (self.idx + 1) % 3;
            self.cpu.incr_pc();
        }

        self.cpu.mem.check_dma();
        self.cpu.check_interrupts();
    }

    pub fn fetch(&mut self) {
        let pc = self.cpu.get_reg(15);
        self.pipeline[self.idx] = if self.cpu.cpsr.isa == InstructionSet::THUMB {
            PipelineInstruction::RawTHUMB(self.cpu.mem.get_halfword(pc))
        } else {
            PipelineInstruction::RawARM(self.cpu.mem.get_word(pc))
        }
    }

    /// decode the next instruction and save the condition (if any) in the Decoded
    /// enum so that execute() can check if it is satisfied later
    pub fn decode(&mut self) {
        // index of the second element from the end
        let idx = ((self.idx + 2) % 3) as usize;
        match self.pipeline[idx] {
            PipelineInstruction::RawARM(n) => {
                let cond = util::get_nibble(n, 28);
                self.pipeline[idx] = PipelineInstruction::Decoded(
                    Some(cond), decode_arm(n).unwrap());
            },
            PipelineInstruction::RawTHUMB(n) => {
                self.pipeline[idx] =
                    PipelineInstruction::Decoded(None, decode_thumb(n))
            },
            _ => ()
        }
    }

    pub fn execute(&mut self) {
        // index of the third element from the end
        let idx = ((self.idx + 1) % 3) as usize;
        if let PipelineInstruction::Decoded(cond, ref ins) = self.pipeline[idx] {
            log!("{:#X?}", ins);
            if cond.is_some() && !satisfies_cond(&self.cpu.cpsr, cond.unwrap()) {
                return;
            }
            match ins {
                Instruction::DataProc(ins) => ins.run(&mut self.cpu),
                Instruction::PSRTransfer(ins) => ins.run(&mut self.cpu),
                Instruction::Multiply(ins) => ins.run(&mut self.cpu),
                Instruction::MultiplyLong(ins) => ins.run(&mut self.cpu),
                Instruction::SwapTransfer(ins) => ins.run(&mut self.cpu),
                Instruction::SingleTransfer(ins) => ins.run(&mut self.cpu),
                Instruction::SignedTransfer(ins) => ins.run(&mut self.cpu),
                Instruction::BlockTransfer(ins) => ins.run(&mut self.cpu),
                Instruction::Branch(ins) => ins.run(&mut self.cpu),
                Instruction::BranchEx(ins) => ins.run(&mut self.cpu),
                Instruction::SWInterrupt(ins) => ins.run(&mut self.cpu),
                Instruction::CondBranch(ins) => ins.run(&mut self.cpu),
                Instruction::LongBranch(ins) => ins.run(&mut self.cpu),
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
    /// r13 is usually the stack pointer (to the top element of the stack, not
    /// the top element + 1), r14 is usually the link register,
    /// and r15 is the PC pointing to address + 8 of the current instruction
    pub r: [u32; 16],
    /// R8-R14 are banked in FIQ mode
    pub r_fiq: [u32; 7],
    /// R13-R14 are banked in IRQ mode
    pub r_irq: [u32; 2],
    /// R13-R14 are banked in UND mode
    pub r_und: [u32; 2],
    /// R13-R14 are banked in ABT mode
    pub r_abt: [u32; 2],
    /// R13-R14 are banked in SVC mode
    pub r_svc: [u32; 2],

    /// program state registers
    pub cpsr: PSR,
    /// banked SPSR registers
    pub spsr_svc: PSR,
    pub spsr_abt: PSR,
    pub spsr_und: PSR,
    pub spsr_irq: PSR,
    pub spsr_fiq: PSR,

    // flush the pipeline before the start of the next cycle
    should_flush: bool,

    pub mem: mem::Memory,
}

impl CPU {
    pub const fn new() -> CPU {
        CPU {
            r: [0; 16],
            r_fiq: [0; 7],
            r_irq: [0; 2],
            r_und: [0; 2],
            r_abt: [0; 2],
            r_svc: [0; 2],

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

    pub fn incr_pc(&mut self) {
        self.r[15] += self.instruction_size();
    }

    /// Add a signed offset to the PC
    pub fn modify_pc(&mut self, offset: i64) {
        // cast pc to i64 to avoid interpreting it as negative number
        self.r[15] = (self.r[15] as i64 + offset as i64) as u32;
        self.should_flush = true;
    }

    pub fn get_reg(&self, reg: usize) -> u32 {
        match reg {
            15 |
            0 ... 7 => self.r[reg],
            8 ... 12 => match self.cpsr.mode {
                CPUMode::FIQ => self.r_fiq[reg - 8],
                _ => self.r[reg]
            },
            13 ... 14 => match self.cpsr.mode {
                CPUMode::USR |
                CPUMode::SYS => self.r[reg],
                CPUMode::FIQ => self.r_fiq[reg - 8],
                CPUMode::IRQ => self.r_irq[reg - 13],
                CPUMode::UND => self.r_und[reg - 13],
                CPUMode::ABT => self.r_abt[reg - 13],
                CPUMode::SVC => self.r_svc[reg - 13],
                CPUMode::INVALID => panic!("invalid mode"),
            },
            _ => panic!("tried to access register {}", reg)
        }
    }

    pub fn set_reg(&mut self, reg: usize, val: u32) {
        match reg {
            15 |
            0 ... 7 => self.r[reg] = val,
            8 ... 12 => match self.cpsr.mode {
                CPUMode::FIQ => self.r_fiq[reg - 8] = val,
                _ => self.r[reg] = val
            },
            13 ... 14 => match self.cpsr.mode {
                CPUMode::USR |
                CPUMode::SYS => self.r[reg] = val,
                CPUMode::FIQ => self.r_fiq[reg - 8] = val,
                CPUMode::IRQ => self.r_irq[reg - 13] = val,
                CPUMode::UND => self.r_und[reg - 13] = val,
                CPUMode::ABT => self.r_abt[reg - 13] = val,
                CPUMode::SVC => self.r_svc[reg - 13] = val,
                CPUMode::INVALID => panic!("invalid mode"),
            },
            _ => panic!("tried to set register {}", reg)
        };
    }

    // TODO: merge params into struct?
    pub fn transfer_reg(&mut self, params: TransferParams) {
        // pre transfer
        let mut addr = self.get_reg(params.base_reg);
        let offset = self.get_offset(&params.offset);
        // word align the PC in THUMB mode if using as offset
        if self.cpsr.isa == InstructionSet::THUMB && params.base_reg == 15 {
            addr &= !3;
        }
        if params.pre_index {
            // It seems like addition on the CPU automatically overflows
            addr = if params.offset_up {
                addr.wrapping_add(offset)
            } else {
                addr.wrapping_sub(offset)
            };
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

    /// Return size of the current instruction set in bytes
    pub fn instruction_size(&self) -> u32 {
        if self.cpsr.isa == InstructionSet::THUMB { 2 } else { 4 }
    }

    /// restore CPSR to the SPSR for the current mode
    fn restore_cpsr(&mut self) {
        self.cpsr = self.get_spsr();
    }

    /// Set the CPSR
    fn set_cpsr(&mut self, val: u32, flags_only: bool) {
        self.cpsr.from_u32(val, flags_only);
    }

    /// Return the SPSR for the current mode
    fn get_spsr(&self) -> PSR {
        match self.cpsr.mode {
            // TODO: just return CPSR for USR mode?
            CPUMode::USR => panic!("USR mode has no SPSR"),
            CPUMode::FIQ => self.spsr_fiq,
            CPUMode::IRQ => self.spsr_irq,
            CPUMode::SVC => self.spsr_svc,
            CPUMode::ABT => self.spsr_abt,
            CPUMode::UND => self.spsr_und,
            CPUMode::SYS => self.cpsr,
            CPUMode::INVALID => panic!("invalid mode"),
        }
    }

    /// Set the SPSR for the current mode
    fn set_spsr(&mut self, val: u32, flags_only: bool) {
        match self.cpsr.mode {
            CPUMode::USR => panic!("USR mode has no SPSR"),
            CPUMode::FIQ => self.spsr_fiq.from_u32(val, flags_only),
            CPUMode::IRQ => self.spsr_irq.from_u32(val, flags_only),
            CPUMode::SVC => self.spsr_svc.from_u32(val, flags_only),
            CPUMode::ABT => self.spsr_abt.from_u32(val, flags_only),
            CPUMode::UND => self.spsr_und.from_u32(val, flags_only),
            CPUMode::SYS => (),
            CPUMode::INVALID => panic!("invalid mode"),
        }
    }

    /// Change CPU modes, saving or restoring the CPSR in the process
    fn change_mode(&mut self, new_mode: CPUMode) {
        match new_mode {
            CPUMode::USR => self.restore_cpsr(),
            CPUMode::FIQ => self.spsr_fiq = self.cpsr,
            CPUMode::IRQ => self.spsr_irq = self.cpsr,
            CPUMode::SVC => self.spsr_svc = self.cpsr,
            CPUMode::ABT => self.spsr_abt = self.cpsr,
            CPUMode::UND => self.spsr_und = self.cpsr,
            CPUMode::SYS => unimplemented!(),
            CPUMode::INVALID => panic!("invalid mode"),
        };
        self.cpsr.mode = new_mode;
    }

    fn set_isa(&mut self, thumb: bool) {
        self.cpsr.isa = if thumb { InstructionSet::THUMB } else { InstructionSet::ARM };
    }

    pub fn check_interrupts(&mut self) {
        if self.cpsr.irq && self.mem.int.pending_interrupts() {
            self.handle_interrupt(InterruptType::IRQ);
        }
    }

    /// Emulate a hardware interrupt being triggered
    ///   - CPU is switched to IRQ mode
    ///   - saves the CPSR in SPSR_irq and sets bit 7 (disable IRQ) in the CPSR
    ///   - saves the address of the next instruction in LR_irq, compensating for
    ///     THUMB/ARM instruction size
    ///   - branches to the appropriate hardware interrupt vector entry in the BIOS
    /// The following is done by the BIOS, so should be emulated here if the
    /// real BIOS is not loaded
    ///   - r0-r3, r12, LR are pushed onto the stack
    ///   - place address for the next instruction (in the BIOS) in LR
    ///   - branches to the address at 0x0300_7FFC
    fn handle_interrupt(&mut self, type_: InterruptType) {
        self.change_mode(type_.get_cpu_mode());
        if let InterruptType::IRQ = type_ {
            self.cpsr.irq = false;
        }

        let next_ins_addr = self.get_reg(15) - self.instruction_size();
        // TODO: is this correct? GBE does it differently
        self.set_reg(14, next_ins_addr);

        self.cpsr.isa = InstructionSet::ARM;
        self.set_reg(15, type_.get_handler_addr());
    }

    // TODO: this should probably be a function
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

pub enum InterruptType {
    Reset,
    Undefined,
    SWI,
    PrefetchAbort,
    DataAbort,
    IRQ,
    FIQ,
}

impl InterruptType {
    /// The address that the CPU jumps to for this specific interrupt type
    pub fn get_handler_addr(&self) -> u32 {
        match *self {
            InterruptType::SWI => 0x8,
            InterruptType::IRQ => 0x18,
            _ => unimplemented!()
        }
    }

    /// The mode that the CPU enters for this specific interrupt type
    pub fn get_cpu_mode(&self) -> CPUMode {
        match *self {
            InterruptType::SWI => CPUMode::SVC,
            InterruptType::IRQ => CPUMode::IRQ,
            _ => unimplemented!()
        }
    }
}

#[cfg(test)]
mod test {
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
    fn transfer_store() {
        let mut cpu = CPU::new();
        cpu.set_reg(1, 0xFFFF_FE00);
        cpu.set_reg(4, 0x0400_0000);
        cpu.transfer_reg(TransferParams {
            pre_index: true,
            offset_up: true,
            size: TransferSize::Word,
            write_back: false,
            load: false,
            base_reg: 4,
            data_reg: 0,
            signed: false,
            offset: &RegOrImm::Reg { shift: 0, reg: 1 }
        });
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
