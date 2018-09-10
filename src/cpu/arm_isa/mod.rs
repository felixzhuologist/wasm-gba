pub mod data;
pub mod branch_ex;
pub mod branch;
pub mod psr;
pub mod mul;
pub mod mul_long;
pub mod single_trans;
pub mod signed_trans;
pub mod block_trans;

#[derive(Debug, Eq, PartialEq)]
pub enum InstructionType {
    DataProc,
    PSRTransfer,
    Multiply,
    MultiplyLong,
    SingleDataSwap,
    BranchAndExchange,
    SingleDataTransfer,
    SignedDataTransfer,
    BlockDataTransfer,
    Branch
}

pub enum RegOrImm {
    Imm { rotate: u32, value: u32 },
    Reg { shift: u32, reg: u32 }
}

pub trait Instruction {
    fn process_instruction(&self, cpu: &mut super::CPU);
    /// return an enum indicating the instruction type. Used during testing
    /// to recover the instruction type 
    fn get_type(&self) -> InstructionType;
}