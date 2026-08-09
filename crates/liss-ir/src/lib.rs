//! The typed IR the checker emits and codegen consumes: types only, no producer and no consumer.

use liss_runtime::{BinOp, Roman, UnOp};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Grain {
    /// `mnr` -- 1/12.
    Uncia,
    /// `nr` -- 1/288.
    Scripulum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ty {
    Bool,
    Char,
    Int,
    Frac(Grain),
    Text,
    List(Box<Ty>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Const {
    Bool(bool),
    Char(char),
    Int(Roman),
    /// The grain resolved to, plus the count of that grain.
    Frac(Grain, Roman),
    Text(String),
}

/// One local in a function's flat `slots` (§4.2); parameters are `0..params`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SlotId(pub u32);

/// The index of one function in the owning `Programma::funcs`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FuncId(pub u32);

#[derive(Debug, Clone, PartialEq)]
pub struct Expr {
    pub ty: Ty,
    pub kind: ExprKind,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExprKind {
    Const(Const),
    Load(SlotId),
    Unary {
        op: UnOp,
        arg: Box<Expr>,
    },
    /// `operand_ty` is separate because a comparison yields `Bool` from
    /// operands that are not, and codegen needs to know which `cmp` to call.
    Binary {
        op: BinOp,
        operand_ty: Ty,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },
    Call {
        func: FuncId,
        args: Vec<Expr>,
    },
    ListLit {
        items: Vec<Expr>,
    },
    Index {
        base: Box<Expr>,
        index: Box<Expr>,
    },
    /// `longitudo` -- desugared at lowering time (§4.4), so it has no symbol.
    Len(Box<Expr>),
    /// `adde` -- desugared at lowering time (§4.4), so it has no symbol.
    Append {
        list: Box<Expr>,
        item: Box<Expr>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Stmt {
    pub line: usize,
    pub kind: StmtKind,
}

#[derive(Debug, Clone, PartialEq)]
pub enum StmtKind {
    Let { slot: SlotId, value: Expr },
    /// Evaluate -> free old -> store (§7.3.3).
    Assign { slot: SlotId, value: Expr },
    IndexAssign { slot: SlotId, index: Expr, value: Expr },
    Print(Expr),
    /// The root is freed if nothing consumed it (§7.3.1).
    ExprStmt(Expr),
    /// `si` / `aut si`, the latter already flattened by the parser (§4.4).
    If { cond: Expr, then: Block, else_: Option<Block> },
    /// `indem` -- a condition-only loop, not desugared.
    While { cond: Expr, body: Block },
    /// `dum` -- not desugared, and `header` is a `Block` because it is a scope;
    /// see `StmtKind::For` in `docs/internals/abi.md`.
    For { header: Block, cond: Expr, step: Box<Stmt>, body: Block },
    /// `redde` -- frees every enclosing block, innermost first (§7.3.4).
    Return(Expr),
    /// `abrumpe`.
    Break,
    /// `continua`.
    Continue,
}

/// Statements plus the slots it introduces, freed in reverse declaration order
/// (§7.3.2): an owned slot in no `declares` is a slot nothing frees.
#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    pub declares: Vec<SlotId>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Func {
    pub symbol: String,
    /// `None` in single-file mode; lowers to a zero-length name at the ABI.
    pub file: Option<String>,
    pub is_entry: bool,
    /// Slots `0..params`. Not a free list: `body.declares` already holds them,
    /// and freeing them again here double-frees every heap parameter.
    pub params: u32,
    pub slots: Vec<Ty>,
    pub ret: Ty,
    pub body: Block,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Programma {
    pub funcs: Vec<Func>,
}

/// Any character outside `[A-Za-z0-9_]` becomes `_uXXXX_` -- one escape per
/// `char`, not per UTF-8 byte.
fn escape(s: &str) -> String {
    let mut out = String::new();
    for ch in s.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            out.push(ch);
        } else {
            out.push_str(&format!("_u{:x}_", ch as u32));
        }
    }
    out
}

/// `liss_<n>_<ns>_<m>_<name>`, where `n` and `m` are the byte lengths of the
/// *escaped* components (§4.3) -- length-prefixing is what keeps this injective.
pub fn symbol(ns: Option<&str>, name: &str) -> String {
    let ns = escape(ns.unwrap_or(""));
    let name = escape(name);
    format!("liss_{}_{}_{}_{}", ns.len(), ns, name.len(), name)
}

#[cfg(test)]
mod tests;
