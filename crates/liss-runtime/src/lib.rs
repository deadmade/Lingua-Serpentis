//! `liss-runtime` — the arithmetic and formatting the language shares between
//! the compiler and compiled programs.
//!
//! This crate is compiled **twice**: once for the host, inside the compiler,
//! because the typed IR needs `Roman` for integer constants; once for musl,
//! embedded in `liss-abi`'s staticlib. `liss-abi` is compiled once, only for
//! musl, and is never linked into the compiler — which is why the two crates
//! cannot be one (spec §11).
//!
//! Dependency-free by construction: nothing here may depend on any crate, in
//! either the host or the musl build. That is what makes compiling it twice,
//! for two different targets, straightforward.

pub mod exitus;
pub mod fraction;
pub mod roman;
pub mod scriptio;

pub use exitus::{exit_status, Warning};
pub use roman::Roman;

/// A binary operator. Carries no AST data of its own — just the operator's
/// identity — so it lives with the arithmetic it drives (`interp.rs`, and
/// later the checker's typed IR and codegen) rather than with the syntax
/// tree that names it. Moved down from `lingua-serpentis::ast` unchanged.
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

/// A unary operator. Moved down from `lingua-serpentis::ast` unchanged.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UnOp {
    Not,
    Neg,
}

/// `#[cfg(test)]`-only: a second, independent formatter kept purely as a
/// randomised oracle for `scriptio`'s tests. Spec §10.4 rules on why this
/// duplication is accepted rather than a defect — see the module doc.
#[cfg(test)]
mod speculum;

#[cfg(test)]
mod proptests;
