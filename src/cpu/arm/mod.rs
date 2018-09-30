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

#[derive(Debug, PartialEq, Eq)]
pub enum RegOrImm {
    Imm { rotate: u32, value: u32 },
    Reg { shift: u32, reg: u32 }
}
