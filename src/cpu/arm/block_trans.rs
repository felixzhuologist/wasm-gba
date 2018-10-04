use ::cpu::CPU;
use ::cpu::status_reg::{CPUMode, InstructionSet};
use ::util;

/// Load or store any subset of the currently visible registers
#[derive(Debug)]
pub struct BlockDataTransfer {
    /// if true, add offset before transfer else add after
    pub pre_index: bool,
    /// if true, add the offset to base, else subtract it
    pub offset_up: bool,
    /// if true, CPSR or force user mode (depending on other parameters)
    pub force: bool,
    /// if true, write address back to base reg, else do nothing
    pub write_back: bool,
    /// if true, load from memory, else write to memory
    pub load: bool,
    /// base register
    pub rn: usize,
    /// bit i of the register list being set means that register i should be transferred
    pub register_list: u16
}

impl BlockDataTransfer {
    /// 27 .. 25 | 24 | 23 | 22 | 21 | 20 | 19 .. 16 | 15 ...
    ///   100    | P  | U  | S  | W  | L  |    Rn    |  register list
    pub fn parse_instruction(ins: u32) -> BlockDataTransfer {
        BlockDataTransfer {
            pre_index: util::get_bit(ins, 24),
            offset_up: util::get_bit(ins, 23),
            force: util::get_bit(ins, 22),
            write_back: util::get_bit(ins, 21),
            load: util::get_bit(ins, 20),
            rn: util::get_nibble(ins, 16) as usize,
            register_list: ins as u16,
        }
    }

    pub fn run(&self, cpu: &mut CPU) {
        if self.rn == 15 {
            panic!("can't use R15 as base in any LDM or STM instruction");
        }
        if self.force && cpu.cpsr.mode == CPUMode::USR {
            panic!("can't set S bit in a non privileged mode");
        }

        let is_pc_in_list = self.register_list >= (1 << 15); // is bit 15 set?
        let original_mode = cpu.cpsr.mode;
        let mut force_user_bank = false;
        if self.force {
            if is_pc_in_list && self.load {
                cpu.restore_cpsr();
            } else {
                force_user_bank = true;
                // temporarily switch to USR mode so that get/set reg refers
                // to the user bank registers
                cpu.cpsr.mode = CPUMode::USR;
            }
        }
        
        if force_user_bank && self.write_back {
            panic!("write back should not be used when forcing user bank");
        }
        if is_pc_in_list && self.load {
            cpu.should_flush = true;
        }

        let mut addr = cpu.get_reg(self.rn);
        let mut write_back = self.write_back;
        // start from larger regs if we are descending - this doesn't emulate
        // the CPU perfectly as it is always supposed to write lower registers
        // first, but that should only affect the case where we write the base
        // address and we check for that case explicitly (4.11.6 in the ARM7TDMI
        // data sheet)
        let bits = if self.offset_up { self.register_list } else { self.register_list.reverse_bits() };
        let mut is_first = true;
        for i in 0..16 {
            if bits & (1 << i) > 0 {
                if self.pre_index {
                    addr = if self.offset_up { addr + 4 } else { addr - 4 };
                }

                let reg = if self.offset_up { i } else { 15 - i };
                if self.load {
                    if reg == self.rn {
                        // a LDM should always overwrite the updated base register
                        // TODO: this is done differently in other emulators
                        write_back = false;
                    }
                    let memval = cpu.mem.get_word(addr);
                    cpu.set_reg(reg, memval);
                } else {
                    if reg == self.rn && !is_first {
                        // if we are storing the base register and this isn't
                        // the first register we are storing, store the updated
                        // value for the base register
                        // TODO: this is done differently in other emulators
                        // (they write back at the end of each loop)
                        cpu.mem.set_word(addr, addr);
                    } else {
                        let regval = cpu.get_reg(reg);
                        cpu.mem.set_word(addr, regval);
                    }
                }

                if !self.pre_index {
                    addr = if self.offset_up { addr + 4 } else { addr - 4 };
                }

                is_first = false;
            }
        }

        if write_back {
            cpu.set_reg(self.rn, addr);
        }
        if force_user_bank {
            cpu.cpsr.mode = original_mode;
        }
        let pc = cpu.get_reg(15);
        if is_pc_in_list && (pc & 1) == 1 {
            cpu.cpsr.isa = InstructionSet::THUMB;
            cpu.set_reg(15, pc & !1);
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn parse() {
        let ins = BlockDataTransfer::parse_instruction(
            0b1010_100_1_0_1_0_1_0101_1101100101100010);
        assert!(ins.pre_index);
        assert!(!ins.offset_up);
        assert!(ins.force);
        assert!(!ins.write_back);
        assert!(ins.load);
        assert_eq!(ins.rn, 5);
        assert_eq!(ins.register_list, 0b1101100101100010);
    }

    #[test]
    fn post_incr_up_store() {
        let mut cpu = CPU::new();
        cpu.set_reg(0, 0x03000000);
        cpu.set_reg(1, 0x123);
        cpu.set_reg(5, 0x321);
        cpu.set_reg(7, 0xABC);

        let ins = BlockDataTransfer {
            pre_index: false,
            offset_up: true,
            force: false,
            write_back: true,
            load: false,
            rn: 0,
            register_list: (1 << 1 | 1 << 5 | 1 << 7)
        };
        ins.run(&mut cpu);

        assert_eq!(cpu.mem.get_word(0x03000000), 0x123);
        assert_eq!(cpu.mem.get_word(0x03000004), 0x321);
        assert_eq!(cpu.mem.get_word(0x03000008), 0xABC);
        assert_eq!(cpu.get_reg(0), 0x0300000C);
    }

    #[test]
    fn pre_incr_up_load() {
        let mut cpu = CPU::new();
        cpu.set_reg(0, 0x03000000);
        cpu.mem.set_word(0x3000004, 0x123);
        cpu.mem.set_word(0x3000008, 0x321);
        cpu.mem.set_word(0x300000C, 0xABC);

        let ins = BlockDataTransfer {
            pre_index: true,
            offset_up: true,
            force: false,
            write_back: true,
            load: true,
            rn: 0,
            register_list: (1 << 1 | 1 << 5 | 1 << 7)
        };
        ins.run(&mut cpu);

        assert_eq!(cpu.get_reg(1), 0x123);
        assert_eq!(cpu.get_reg(5), 0x321);
        assert_eq!(cpu.get_reg(7), 0xABC);
        assert_eq!(cpu.get_reg(0), 0x0300000C);

        let mut cpu = CPU::new();
        cpu.set_reg(0, 0x3007EA0);
        cpu.set_reg(1, 0x4000220);
        cpu.set_reg(10, 0x4000220);
        cpu.set_reg(12, 0x3007EC0);
        cpu.set_reg(13, 0x3007E80);
        cpu.set_reg(14, 0xBD4);
        cpu.set_reg(15, 0xC28);

        cpu.mem.set_word(0x3007E80, 0x40_00_00_00);
        cpu.mem.set_word(0x3007E84, 0x85_00_00_00);
        cpu.mem.set_word(0x3007E88, 0x00_00_00_80);
        cpu.mem.set_word(0x3007E8C, 0x00_00_00_FF);
        cpu.mem.set_word(0x3007E90, 0x00_00_00_00);
        cpu.mem.set_word(0x3007E94, 0x00_00_00_00);
        cpu.mem.set_word(0x3007E98, 0x00_00_00_00);
        cpu.mem.set_word(0x3007E9C, 0x00_00_09_E7);

        // ldmia sp!, {r4-r10,lr}
        BlockDataTransfer {
            pre_index: false,
            offset_up: true,
            force: false,
            write_back: true,
            load: true,
            rn: 13,
            register_list: 0x47F0
        }.run(&mut cpu);

        assert_eq!(cpu.get_reg(0), 0x3007EA0);
        assert_eq!(cpu.get_reg(1), 0x4000220);
        assert_eq!(cpu.get_reg(4), 0x40_00_00_00);
        assert_eq!(cpu.get_reg(5), 0x85_00_00_00);
        assert_eq!(cpu.get_reg(6), 0x00_00_00_80);
        assert_eq!(cpu.get_reg(7), 0x00_00_00_FF);
        assert_eq!(cpu.get_reg(8), 0x00_00_00_00);
        assert_eq!(cpu.get_reg(9), 0x00_00_00_00);
        assert_eq!(cpu.get_reg(10), 0x00_00_00_00);
        assert_eq!(cpu.get_reg(14), 0x00_00_09_E7);
    }

    #[test]
    fn post_incr_down_load() {
        let mut cpu = CPU::new();
        cpu.set_reg(0, 0x0300000C);
        cpu.mem.set_word(0x300000C, 0x123);
        cpu.mem.set_word(0x3000008, 0x321);
        cpu.mem.set_word(0x3000004, 0xABC);

        let ins = BlockDataTransfer {
            pre_index: false,
            offset_up: false,
            force: false,
            write_back: true,
            load: true,
            rn: 0,
            register_list: (1 << 10 | 1 << 11 | 1 << 12)
        };
        ins.run(&mut cpu);

        assert_eq!(cpu.get_reg(12), 0x123);
        assert_eq!(cpu.get_reg(11), 0x321);
        assert_eq!(cpu.get_reg(10), 0xABC);
        assert_eq!(cpu.get_reg(0), 0x03000000);
    }

    #[test]
    fn pre_incr_down_store() {
        let mut cpu = CPU::new();
        cpu.set_reg(0, 0x3007EA0);
        cpu.set_reg(1, 0x4000200);
        cpu.set_reg(2, 0x85000008);
        cpu.set_reg(3, 0xBC4);
        cpu.set_reg(4, 0x4000000);
        cpu.set_reg(5, 0x85000000);
        cpu.set_reg(6, 0x80);
        cpu.set_reg(7, 0xFF);
        cpu.set_reg(13, 0x3007EA0);
        cpu.set_reg(14, 0x9E7);
        cpu.set_reg(15, 0xBD0);

        // stmdb sp!, {r4-r10,lr}
        let ins = BlockDataTransfer {
            pre_index: true,
            offset_up: false,
            force: false,
            write_back: true,
            load: false,
            rn: 13,
            register_list: 0x47F0
        };
        ins.run(&mut cpu);

        assert_eq!(cpu.mem.get_word(0x3007E80), 0x4000000);
        assert_eq!(cpu.mem.get_word(0x3007E84), 0x85000000);
        assert_eq!(cpu.mem.get_word(0x3007E88), 0x80);
        assert_eq!(cpu.mem.get_word(0x3007E8C), 0xFF);
        assert_eq!(cpu.mem.get_word(0x3007E90), 0);
        assert_eq!(cpu.mem.get_word(0x3007E94), 0);
        assert_eq!(cpu.mem.get_word(0x3007E98), 0);
        assert_eq!(cpu.mem.get_word(0x3007E9C), 0x9E7);
    }

    #[test]
    fn load_base_reg() {
        let mut cpu = CPU::new();
        cpu.set_reg(0, 0x03000000);
        cpu.mem.set_word(0x03000000, 0xDEF);
        cpu.mem.set_word(0x03000004, 0xFFF123);

        let ins = BlockDataTransfer {
            pre_index: false,
            offset_up: true,
            force: false,
            write_back: true,
            load: true,
            rn: 0,
            register_list: 0b11
        };
        ins.run(&mut cpu);

        assert_eq!(cpu.get_reg(0), 0xDEF);
        assert_eq!(cpu.get_reg(1), 0xFFF123);
    }

    #[test]
    fn store_base_reg() {
        let mut cpu = CPU::new();
        cpu.set_reg(0, 0x03000000);

        let ins = BlockDataTransfer {
            pre_index: true,
            offset_up: true,
            force: false,
            write_back: true,
            load: false,
            rn: 0,
            register_list: 1
        };
        ins.run(&mut cpu);
        assert_eq!(cpu.mem.get_word(0x03000004), 0x03000000);
    }
}