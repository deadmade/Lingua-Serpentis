//! The arithmetic and formatting the language shares between the compiler and compiled programs.

pub mod dispositio;
pub mod exitus;
pub mod fraction;
pub mod genus;
pub mod roman;
pub mod scriptio;

pub use exitus::{exit_status, Warning};
pub use roman::Roman;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    Ne,
    Gt,
    Lt,
    And,
    Or,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UnOp {
    Not,
    Neg,
}

#[cfg(test)]
mod speculum;

#[cfg(test)]
mod proptests;
