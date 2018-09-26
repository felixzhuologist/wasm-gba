pub mod data;
pub mod branch_ex;
pub mod branch;
pub mod psr;
pub mod mul;
pub mod mul_long;
pub mod single_trans;
pub mod signed_trans;
pub mod block_trans;
pub mod swap;
pub mod swi;

pub enum Instruction {
    DataProc(data::DataProc),
    PSRTransfer(psr::PSRTransfer),
    Multiply(mul::Multiply),
    MultiplyLong(mul_long::MultiplyLong),
    SwapTransfer(swap::SingleDataSwap),
    SingleTransfer(single_trans::SingleDataTransfer),
    SignedTransfer(signed_trans::SignedDataTransfer),
    BlockTransfer(block_trans::BlockDataTransfer),
    Branch(branch::Branch),
    BranchEx(branch_ex::BranchAndExchange),
    SWInterrupt(swi::SWInterrupt),
    Noop
}

pub enum RegOrImm {
    Imm { rotate: u32, value: u32 },
    Reg { shift: u32, reg: u32 }
}
