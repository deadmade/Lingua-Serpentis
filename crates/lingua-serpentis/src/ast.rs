#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Type { Int, Float, Text, Char, Bool }

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BinOp { Add, Sub, Mul, Div, Mod, Eq, Ne, Gt, Lt, And, Or }

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UnOp { Not, Neg }

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Int(i64),
    Float(f64),
    Text(String),
    Char(char),
    Bool(bool),
    Ident(String),
    Unary { op: UnOp, expr: Box<Expr> },
    Binary { op: BinOp, left: Box<Expr>, right: Box<Expr> },
    Call { name: String, args: Vec<Expr> },
}

#[derive(Debug, Clone, PartialEq)]
pub struct FuncDecl {
    pub name: String,
    pub params: Vec<(Type, String)>,
    pub ret: Type,
    pub body: Vec<Stmt>,
    /// Line of the *definition* header (`nomen(params) {`). A declaration
    /// spans two lines — a `::` signature line and a definition line — and the
    /// enclosing `Stmt.line` is the signature line, so anything that needs to
    /// point at a parameter must use this instead.
    pub def_line: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Stmt {
    pub kind: StmtKind,
    pub line: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum StmtKind {
    Let { ty: Type, name: String, value: Expr },
    Assign { name: String, value: Expr },
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
