#[derive(Debug, Clone, PartialEq)]
pub enum Type { Int, Float, Double, Text, Char, Bool, List(Box<Type>) }

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BinOp { Add, Sub, Mul, Div, Mod, Eq, Ne, Gt, Lt, And, Or }

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UnOp { Not, Neg }

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Int(i64),
    FloatLit(f64), // abstract fractional literal, pre-elaboration
    Float(f32),    // resolved mnr
    Double(f64),   // resolved nr
    Text(String),
    Char(char),
    Bool(bool),
    Ident(String),
    Unary { op: UnOp, expr: Box<Expr> },
    Binary { op: BinOp, left: Box<Expr>, right: Box<Expr> },
    Call { name: String, args: Vec<Expr> },
    ListLit(Vec<Expr>),
    Index { base: Box<Expr>, index: Box<Expr> },
}

#[derive(Debug, Clone, PartialEq)]
pub struct FuncDecl {
    pub name: String,
    pub params: Vec<(Type, String)>,
    pub ret: Type,
    pub body: Vec<Stmt>,
    /// Line of the header's parameter clause — or, when the function has no
    /// parameters, the token right after the name. Usually the same as the
    /// enclosing `Stmt.line`, but a header split across lines (`munus` on one
    /// line, the `capit (...)` clause on the next) puts it lower, and anything
    /// pointing at a parameter must use this instead of `Stmt.line`.
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
