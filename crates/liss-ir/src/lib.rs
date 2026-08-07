//! `liss-ir` -- the typed intermediate representation the checker emits and
//! codegen consumes. Types only: no producer (the checker, Task 7) and no
//! consumer (codegen, later tasks) live here. See spec §4.
//!
//! Every `Expr` node carries its own `Ty` so codegen never consults an
//! environment and never infers -- that is the entire purpose of this crate.

use liss_runtime::{BinOp, Roman, UnOp};

/// The two fixed-point grains the language's `mnr` and `nr` types round to:
/// twelfths (`Uncia`) and 1/288ths (`Scripulum`). `Type::Float`/`Double`
/// collapse into `Ty::Frac(Grain)` here -- every operation on them is
/// identical modulo one constant (12 or 288) -- but the *checker* keeps them
/// distinct in its own `Type`, because its diagnostics say `mnr` and `nr`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Grain {
    /// `mnr` -- 1/12.
    Uncia,
    /// `nr` -- 1/288.
    Scripulum,
}

/// The IR's type system: six variants, no more. Every `Expr` node carries
/// one of these directly (§4.1) -- codegen reads it off the node and never
/// infers or consults an environment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ty {
    Bool,
    Char,
    Int,
    Frac(Grain),
    Text,
    List(Box<Ty>),
}

/// A compile-time-known literal value. One variant per `Ty` that can be
/// spelled as a literal -- `List` has no `Const` arm because list literals
/// are always built from `ExprKind::ListLit`, never a single constant.
#[derive(Debug, Clone, PartialEq)]
pub enum Const {
    Bool(bool),
    Char(char),
    Int(Roman),
    /// A fixed-point literal: the grain it was resolved to, plus its count
    /// of that grain (e.g. `°I.VI mnr` is six unciae, `Roman::from_i64(6)`).
    Frac(Grain, Roman),
    Text(String),
}

/// The index of one local in a function's flat `slots: Vec<Ty>` (§4.2).
/// Slots are allocated per function and never reused: leaving a block pops
/// the name binding, not the number, so shadowing needs no special case.
/// Parameters are slots `0..params`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SlotId(pub u32);

/// The index of one function in the owning `Programma::funcs`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FuncId(pub u32);

/// One expression node. Every node in the tree carries its own `ty` --
/// that is the entire purpose of this crate (§4.1, property 1).
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
    /// `operand_ty` is carried separately from the node's own `ty` because
    /// comparisons (`Eq`, `Ne`, `Gt`, `Lt`) yield `Bool` from operands that
    /// are not `Bool`, and codegen needs to know which `cmp` to call (§4.1,
    /// property 2).
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
    /// `longitudo` -- desugared at lowering time (§4.4); it is not a
    /// function and has no symbol.
    Len(Box<Expr>),
    /// `adde` -- desugared at lowering time (§4.4); it is not a function
    /// and has no symbol.
    Append {
        list: Box<Expr>,
        item: Box<Expr>,
    },
}

/// One statement. Carries `line` and nothing finer -- exactly the
/// interpreter's granularity (`exec` passes `s.line` into every `eval`), so
/// it is exactly the attribution the corpus is blessed against.
#[derive(Debug, Clone, PartialEq)]
pub struct Stmt {
    pub line: usize,
    pub kind: StmtKind,
}

#[derive(Debug, Clone, PartialEq)]
pub enum StmtKind {
    /// A fresh binding into a freshly allocated slot.
    Let { slot: SlotId, value: Expr },
    /// Rebinds an existing slot. Ownership is evaluate -> free old -> store
    /// (§7.3.3): `x = x + °I` must read `x` before the old value dies.
    Assign { slot: SlotId, value: Expr },
    /// Writes one element of a list held in `slot`. `IndexAssign` frees the
    /// displaced element inside `liss_list_put` (§7.3).
    IndexAssign { slot: SlotId, index: Expr, value: Expr },
    Print(Expr),
    /// An expression evaluated for effect; its root is freed if nothing
    /// consumed it (§7.3.1).
    ExprStmt(Expr),
    /// `si` / `aut si`. `aut si` is already flattened by the parser into a
    /// one-statement else block before lowering (§4.4), so this carries no
    /// separate "else if" arm.
    If { cond: Expr, then: Block, else_: Option<Block> },
    /// `indem` -- a condition-only loop. Not desugared: trivially a CFG
    /// (§4.4).
    While { cond: Expr, body: Block },
    /// `dum` -- a C-style init/cond/step loop. Not desugared: `continua`
    /// must still run the step, so codegen builds the CFG directly rather
    /// than lowering to a statement tree (§4.4).
    For { init: Box<Stmt>, cond: Expr, step: Box<Stmt>, body: Block },
    /// `redde`. Evaluates, then frees every slot of every enclosing block
    /// of the function, innermost first, including the parameter frame; the
    /// returned temporary itself is never among them (§7.3.4).
    Return(Expr),
    /// `abrumpe`.
    Break,
    /// `continua`.
    Continue,
}

/// A lexical block: its statements, plus the slots it introduces. Freed in
/// reverse declaration order on scope exit (§7.3.2).
///
/// `declares` comes from the checker's scope stack, not from codegen
/// re-deriving scopes -- it is the ownership machinery (§7.3), and
/// re-deriving it would be the same mistake as re-deriving types.
#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub stmts: Vec<Stmt>,
    pub declares: Vec<SlotId>,
}

/// One function, lowered and fully typed.
#[derive(Debug, Clone, PartialEq)]
pub struct Func {
    /// The mangled linker symbol -- see `symbol` (§4.3).
    pub symbol: String,
    /// `None` in single-file mode, where diagnostics read `Error in linea
    /// N:` rather than `Error in 'x.liss', linea N:`. Lowers to a
    /// zero-length file name at the ABI (§4.1, property 4).
    pub file: Option<String>,
    /// Whether this is the root `initium`, the program's entry point.
    pub is_entry: bool,
    /// Slots `0..params` are the parameters; `slots[params..]` are locals.
    pub params: u32,
    /// The function's locals, flat and indexed by `SlotId` -- exactly the
    /// shape `cranelift_frontend::Variable` wants (§4.2).
    pub slots: Vec<Ty>,
    pub ret: Ty,
    pub body: Block,
}

/// The whole lowered program: every function, root and package, in one flat
/// list addressed by `FuncId`.
#[derive(Debug, Clone, PartialEq)]
pub struct Programma {
    pub funcs: Vec<Func>,
}

/// Escapes one identifier component for `symbol`: any byte outside
/// `[A-Za-z0-9_]` becomes `_uXXXX_`, the lowercase hex of its Unicode scalar
/// value. Identifiers are Unicode-alphabetic (`lexer.rs` uses
/// `is_alphabetic()`), so a passthrough is not an option -- non-ASCII
/// identifiers are legal in this language, and this is what keeps the
/// mapping injective once combined with `symbol`'s length prefix.
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

/// Mangles a function's namespace (`None` for the root, `Some(pkg)` for a
/// package) and name into a linker symbol (§4.3):
///
/// ```text
/// liss_<n>_<ns>_<m>_<name>
/// ```
///
/// where `n` and `m` are the byte lengths of the *escaped* components.
/// Length-prefixing the escaped form is what keeps the mapping injective --
/// without it, `symbol(None, "aê")` and `symbol(None, "a_u00ea_")` would
/// collide, since the first's escape is a literal prefix of the second's
/// spelling.
///
/// The character immediately after `liss_` is always a digit (the first
/// digit of `n`), which is the collision proof against runtime entry
/// points such as `liss_int_add` and `liss_panic`, whose next character is
/// always a letter.
pub fn symbol(ns: Option<&str>, name: &str) -> String {
    let ns = escape(ns.unwrap_or(""));
    let name = escape(name);
    format!("liss_{}_{}_{}_{}", ns.len(), ns, name.len(), name)
}

#[cfg(test)]
mod tests;
