use ::cpu::{CPU, InterruptType};

/// Cause a software interrupt trap to be taken, which switches to Supervisor mode,
/// changes the PC to a fixed value (0x08), and saves the CPSR
#[derive(Debug)]
pub struct SWInterrupt { pub comment: u32 }

impl SWInterrupt {
    pub fn parse_instruction(ins: u32) -> SWInterrupt {
        SWInterrupt { comment: ins & 0xFFFFFF }
    }

    pub fn run(&self, cpu: &mut CPU) {
        cpu.handle_interrupt(InterruptType::SWI);
    }
}
