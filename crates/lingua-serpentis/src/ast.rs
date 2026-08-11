use crate::spatium::Spatium;
use crate::roman::Roman;

pub use runtime::{BinOp, UnOp};

#[derive(Debug, Clone, PartialEq)]
pub enum Type { Int, Float, Double, Text, Char, Bool, List(Box<Type>) }

/// An expression and where it is written. The span is what a caret underlines, so
/// every expression carries one — a diagnostic about a subexpression names *it*,
/// not the statement enclosing it.
#[derive(Debug, Clone)]
pub struct Expr {
    pub kind: ExprKind,
    pub spatium: Spatium,
}

impl Expr {
    pub fn novum(kind: ExprKind, spatium: Spatium) -> Expr {
        Expr { kind, spatium }
    }
}

/// **Structural, and deliberately blind to the span.** Two expressions are equal
/// when they say the same thing, whatever line they say it on — which is what
/// every comparison in the tree wants, and what keeps a position from changing
/// the meaning of `==`. Where a span is the subject, it is asserted directly:
/// `lexer.rs` for the tokens, `fragmentum.rs` for the extraction, and the corpus
/// for the caret that results.
impl PartialEq for Expr {
    fn eq(&self, other: &Self) -> bool {
        self.kind == other.kind
    }
}

/// So an expected shape can be written as the bare `ExprKind` it is about.
#[cfg(test)]
impl PartialEq<ExprKind> for Expr {
    fn eq(&self, other: &ExprKind) -> bool {
        self.kind == *other
    }
}

/// A span for an expression built without a source to place it in — tests, and the
/// `ExprKind` half of a comparison. `linea` is 0, so `fragmentum::extract` declines
/// to draw a snippet for it rather than inventing one.
pub const NUSQUAM: Spatium = Spatium { principium: 0, finis: 0, linea: 0 };

#[cfg(test)]
pub fn ex(kind: ExprKind) -> Expr {
    Expr::novum(kind, NUSQUAM)
}

#[cfg(test)]
pub fn bx(kind: ExprKind) -> Box<Expr> {
    Box::new(ex(kind))
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExprKind {
    Int(Roman),
    FloatLit { numer: Roman, den: Roman }, // abstract, unresolved
    Text(String),
    Char(char),
    Bool(bool),
    Ident(String),
    Unary { op: UnOp, expr: Box<Expr> },
    Binary { op: BinOp, left: Box<Expr>, right: Box<Expr> },
    Call { package: Option<String>, name: String, args: Vec<Expr> },
    ListLit(Vec<Expr>),
    Index { base: Box<Expr>, index: Box<Expr> },
}

#[derive(Debug, Clone, PartialEq)]
pub struct FuncDecl {
    pub name: String,
    pub name_span: Spatium,
    /// Each parameter's type, name, and where the name is written. The span replaces
    /// the old `def_line`, which existed only so a parameter's warning could land on
    /// the line it was written rather than on `munus`.
    pub params: Vec<(Type, String, Spatium)>,
    pub ret: Type,
    pub body: Vec<Stmt>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Stmt {
    pub kind: StmtKind,
    /// First token through last, for the diagnostics that are about a statement
    /// rather than about an expression inside it.
    pub spatium: Spatium,
}

#[derive(Debug, Clone, PartialEq)]
pub enum StmtKind {
    Let { ty: Type, name: String, name_span: Spatium, value: Expr },
    Assign { name: String, value: Expr },
    IndexAssign { name: String, index: Expr, value: Expr },
    Print(Expr),
    ExprStmt(Expr),
    If { cond: Expr, then: Vec<Stmt>, else_: Option<Vec<Stmt>> },
    While { cond: Expr, body: Vec<Stmt> },
    For { init: Box<Stmt>, cond: Expr, step: Box<Stmt>, body: Vec<Stmt> },
    Func(FuncDecl),
    Return(Expr),
    Break,
    Continue,
}

/// One `affer` line at the top of a file.
#[derive(Debug, Clone, PartialEq)]
pub struct FileImport {
    pub package: String,
    pub line: usize,
}

/// Every element of `funcs` is a `StmtKind::Func`.
#[derive(Debug, Clone, PartialEq)]
pub struct FileAst {
    pub imports: Vec<FileImport>,
    pub funcs: Vec<Stmt>,
}

/// `path` is relative to the program root.
#[derive(Debug, Clone, PartialEq)]
pub struct ParsedFile {
    pub path: String,
    /// The file as read, kept beside its path so a diagnostic can take the line it
    /// points at while the text is still in hand.
    pub src: String,
    pub ast: FileAst,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedProgram {
    pub root: Vec<ParsedFile>,
    pub packages: std::collections::BTreeMap<String, Vec<ParsedFile>>,
}
