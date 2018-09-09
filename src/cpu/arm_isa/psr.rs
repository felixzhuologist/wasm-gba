use super::{Instruction, InstructionType};
use ::cpu::CPU;
use ::util;
use super::data::RegOrImm;

pub enum StateRegType {
    /// CPSR
    Current,
    /// SPSR of the current mode
    Saved
}

pub enum TransferType {
    Read { stype: StateRegType, dest: u32 },
    Write { stype: StateRegType, source: RegOrImm }
}

/// These instructions are TEQ/TST/CMN/CMP Data Processing operations but
/// without the S flag set. They allow access to the CPSR/SPSR registers, i.e.
/// reading CPSR/SPSR of the current mode to a register, or writing a reg/immediate
/// value to the CPSR/SPSR of the current mode.
struct PSRTransfer {
    trans: TransferType
}

impl PSRTransfer {
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
    pub fn parse_instruction(ins: u32) -> PSRTransfer {
        // TODO: should we check the other differentiating bits?
        let is_read = util::get_bit(ins, 21);
        let stype =  if util::get_bit(ins, 22) { StateRegType::Saved } 
            else { StateRegType::Current };
        PSRTransfer {
            trans: if is_read {
                TransferType::Read { stype, dest: util::get_nibble(ins, 12) }
            } else {
                let is_imm = util::get_bit(ins, 26);
                TransferType::Write {
                    stype,
                    source: if is_imm {
                        RegOrImm::Imm {
                            rotate: util::get_nibble(ins, 8),
                            value: util::get_byte(ins, 0)
                        }
                    } else {
                        RegOrImm::Reg { reg: util::get_byte(ins, 0), shift: 0 }
                    }
                }
            }
        }
    }
}

impl Instruction for PSRTransfer {
    fn get_type(&self) -> InstructionType { InstructionType::PSRTransfer }
    fn process_instruction(&self, cpu: &mut CPU) {
        match self.trans {
            TransferType::Read { ref stype, ref dest } => {
                let val = match stype {
                    StateRegType::Current => cpu.get_cpsr(),
                    StateRegType::Saved => cpu.get_spsr()
                };
                cpu.r[*dest as usize] = val;
            },
            TransferType::Write { ref stype, ref source } => {
                let val = match source {
                    RegOrImm::Imm { ref rotate, ref value } => value.rotate_right(*rotate),
                    RegOrImm::Reg { shift: _, ref reg } => cpu.r[*reg as usize]
                };
                match stype {
                    StateRegType::Current => cpu.set_cpsr(val),
                    StateRegType::Saved => cpu.set_spsr(val)
                }
            }
        }
    }
}