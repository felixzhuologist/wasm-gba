use ::cpu::CPU;

/// Cause a software interrupt trap to be taken, which switches to Supervisor mode,
/// changes the PC to a fixed value (0x08), and saves the CPSR
pub struct SWInterrupt { }

impl SWInterrupt {
    pub fn parse_instruction(ins: u32) -> SWInterrupt {
        SWInterrupt { }
    }

    pub fn run(&self, cpu: &mut CPU) {
        unimplemented!()
    }
}
