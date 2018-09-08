pub mod data;

pub enum InstructionType {
    DataProc,
    PSRTransfer,
    Multiply,
    MultiplyLong,
    SingleDataSwap,
    BranchAndExchange,
    SingleDataTransfer,
    BlockDataTransfer,
    Branch
}

pub trait Instruction {
    fn process_instruction(&self, cpu: &mut super::CPU, ins: u32);
    /// return an enum indicating the instruction type. Used during testing
    /// to recover the instruction type 
    fn get_type(&self) -> InstructionType;
}