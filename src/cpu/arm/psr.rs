use super::RegOrImm;
use ::cpu::CPU;
use ::cpu::status_reg::CPUMode;
use ::util;

#[derive(Debug)]
pub enum StateRegType {
    /// CPSR
    Current,
    /// SPSR of the current mode
    Saved
}

#[derive(Debug)]
pub enum TransferType {
    Read { stype: StateRegType, dest: usize },
    Write { stype: StateRegType, source: RegOrImm, flag_only: bool }
}

/// These instructions are TEQ/TST/CMN/CMP Data Processing operations but
/// without the S flag set. They allow access to the CPSR/SPSR registers, i.e.
/// reading CPSR/SPSR of the current mode to a register, or writing a reg/immediate
/// value to the CPSR/SPSR of the current mode.
#[derive(Debug)]
pub struct PSRTransfer {
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
        let is_write = util::get_bit(ins, 21);
        let stype =  if util::get_bit(ins, 22) { StateRegType::Saved } 
            else { StateRegType::Current };
        PSRTransfer {
            trans: if is_write {
                let is_imm = util::get_bit(ins, 25);
                TransferType::Write {
                    stype,
                    source: if is_imm {
                        RegOrImm::Imm {
                            rotate: util::get_nibble(ins, 8),
                            value: util::get_byte(ins, 0)
                        }
                    } else {
                        RegOrImm::Reg { reg: util::get_byte(ins, 0), shift: 0 }
                    },
                    flag_only: !util::get_bit(ins, 16),
                }
            } else {
                TransferType::Read { stype, dest: util::get_nibble(ins, 12) as usize }
            }
        }
    }

    pub fn run(&self, cpu: &mut CPU) {
        match self.trans {
            TransferType::Read { ref stype, dest } => {
                if dest == 15 {
                    panic!("can't read/write PSR with R15");
                }
                let val = match stype {
                    StateRegType::Current => cpu.cpsr.to_u32(),
                    StateRegType::Saved => cpu.get_spsr().to_u32()
                };
                cpu.set_reg(dest, val);
            },
            TransferType::Write { ref stype, ref source, flag_only } => {
                let mut val = match source {
                    RegOrImm::Imm { rotate, ref value } => value.rotate_right(*rotate),
                    RegOrImm::Reg { shift: _, reg } => {
                        if *reg == 15 {
                            panic!("can't read/write PSR with R15");
                        }
                        cpu.get_reg(*reg as usize)
                    }
                };
                match stype {
                    StateRegType::Current => {
                        cpu.set_cpsr(val, flag_only);
                        if let CPUMode::INVALID = cpu.cpsr.mode {
                            panic!("setting CPSR to an invalid mode")
                        }
                    },
                    StateRegType::Saved => cpu.set_spsr(val, flag_only)
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use ::cpu::status_reg::{InstructionSet, CPUMode};

    #[test]
    fn parse_read() {
        let ins = PSRTransfer::parse_instruction(
            0b0000_00010_0_001111_0001_000000000000);
        assert!(match ins.trans {
            TransferType::Read { stype: StateRegType::Current, dest: 1 } => true,
            _ => false
        })
    }

    #[test]
    fn parse_write_reg() {
        let ins = PSRTransfer::parse_instruction(
            0b0000_00010_1_1010001111_00000000_1000);
        assert!(match ins.trans {
            TransferType::Write {
                stype: StateRegType::Saved,
                source: RegOrImm::Reg { shift: 0, reg: 8 },
                flag_only: true,
            } => true,
            _ => false
        });

        let ins = PSRTransfer::parse_instruction(
            0b0000_00010_1_1010011111_00000000_1000);
        assert!(match ins.trans {
            TransferType::Write {
                stype: StateRegType::Saved,
                source: RegOrImm::Reg { shift: 0, reg: 8 },
                flag_only: false,
            } => true,
            _ => false
        });
    }

    #[test]
    fn parse_write_imm() {
        let ins = PSRTransfer::parse_instruction(
            0b0000_00110_0_1010001111_1010_10000000);
        assert!(match ins.trans {
            TransferType::Write {
                stype: StateRegType::Current,
                source: RegOrImm::Imm { rotate: 10, value: 128 },
                flag_only: true,
            } => true,
            _ => false
        });
    }

    #[test]
    fn read_cpsr() {
        let mut cpu = CPU::new();
        cpu.cpsr.carry = true;
        cpu.cpsr.isa = InstructionSet::THUMB;
        cpu.cpsr.mode = CPUMode::FIQ;

        let ins = PSRTransfer {
            trans: TransferType::Read { stype: StateRegType::Current, dest: 0 }
        };

        ins.run(&mut cpu);

        assert_eq!(cpu.cpsr.to_u32(), cpu.get_reg(0));
    }

    #[test]
    fn write_spsr_invalid() {
        let mut cpu = CPU::new();
        let ins = PSRTransfer {
            trans: TransferType::Write {
                stype: StateRegType::Saved,
                source: RegOrImm::Reg { shift: 0, reg: 14 },
                flag_only: false
            }
        };
        ins.run(&mut cpu);
        let spsr = cpu.get_spsr();
        assert_eq!(spsr.neg, false);
        assert_eq!(spsr.zero, false);
        assert_eq!(spsr.carry, false);
        assert_eq!(spsr.overflow, false);
        assert_eq!(spsr.irq, false);
        assert_eq!(spsr.fiq, false);
        assert_eq!(spsr.isa, InstructionSet::ARM);
        assert_eq!(spsr.mode, CPUMode::INVALID);
    }

    #[test]
    #[should_panic]
    fn write_cpsr_invalid() {
        let mut cpu = CPU::new();
        let ins = PSRTransfer {
            trans: TransferType::Write {
                stype: StateRegType::Current,
                source: RegOrImm::Reg { shift: 0, reg: 14 },
                flag_only: false
            }
        };
        ins.run(&mut cpu);
    }

    #[test]
    fn write_flagonly() {
        let mut cpu = CPU::new();
        let ins = PSRTransfer {
            trans: TransferType::Write {
                stype: StateRegType::Current,
                source: RegOrImm::Imm { rotate: 0, value: 0xFFFFFFFF },
                flag_only: true
            }
        };
        ins.run(&mut cpu);
        assert_eq!(cpu.cpsr.neg, true);
        assert_eq!(cpu.cpsr.zero, true);
        assert_eq!(cpu.cpsr.carry, true);
        assert_eq!(cpu.cpsr.overflow, true);
        assert_eq!(cpu.cpsr.irq, true);
        assert_eq!(cpu.cpsr.fiq, true);
        assert_eq!(cpu.cpsr.isa, InstructionSet::ARM);
        assert_eq!(cpu.cpsr.mode, CPUMode::SVC);
    }

    #[test]
    #[should_panic]
    fn use_r15() {
        let ins = PSRTransfer {
            trans: TransferType::Read { stype: StateRegType::Saved, dest: 15 }
        };

        ins.run(&mut CPU::new());
    }
}
