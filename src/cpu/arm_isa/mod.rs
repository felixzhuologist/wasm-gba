pub mod data;
pub mod branch_ex;
pub mod branch;
pub mod psr;
pub mod mul;
pub mod mul_long;

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
    fn process_instruction(&self, cpu: &mut super::CPU);
    /// return an enum indicating the instruction type. Used during testing
    /// to recover the instruction type 
    fn get_type(&self) -> InstructionType;
}