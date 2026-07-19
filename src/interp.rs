use crate::ast::*;
use crate::error::{LinguaError, LResult};
use crate::value::Value;
use std::collections::HashMap;

struct Interp {
    scopes: Vec<HashMap<String, Value>>,
    funcs: HashMap<String, FuncDecl>,
    out: String,
}

impl Interp {
    fn new() -> Self {
        Interp { scopes: vec![HashMap::new()], funcs: HashMap::new(), out: String::new() }
    }

    fn get(&self, name: &str) -> Option<Value> {
        for scope in self.scopes.iter().rev() {
            if let Some(v) = scope.get(name) {
                return Some(v.clone());
            }
        }
        None
    }
    fn declare(&mut self, name: String, v: Value) {
        self.scopes.last_mut().unwrap().insert(name, v);
    }
    fn assign(&mut self, name: &str, v: Value, line: usize) -> LResult<()> {
        for scope in self.scopes.iter_mut().rev() {
            if scope.contains_key(name) {
                scope.insert(name.to_string(), v);
                return Ok(());
            }
        }
        Err(LinguaError { message: format!("variabilis ignota: '{}'", name), line })
    }

    fn eval(&mut self, e: &Expr, line: usize) -> LResult<Value> {
        match e {
            Expr::Int(i) => Ok(Value::Int(*i)),
            Expr::Float(f) => Ok(Value::Float(*f)),
            Expr::Text(s) => Ok(Value::Text(s.clone())),
            Expr::Bool(b) => Ok(Value::Bool(*b)),
            Expr::Ident(name) => self.get(name)
                .ok_or_else(|| LinguaError { message: format!("variabilis ignota: '{}'", name), line }),
            Expr::Unary { op, expr } => {
                let v = self.eval(expr, line)?;
                match (op, v) {
                    (UnOp::Not, Value::Bool(b)) => Ok(Value::Bool(!b)),
                    (UnOp::Neg, Value::Int(i)) => Ok(Value::Int(-i)),
                    (UnOp::Neg, Value::Float(f)) => Ok(Value::Float(-f)),
                    _ => Err(LinguaError { message: "operatio unaria invalida".into(), line }),
                }
            }
            Expr::Binary { op, left, right } => {
                let l = self.eval(left, line)?;
                let r = self.eval(right, line)?;
                eval_binary(*op, l, r, line)
            }
            Expr::Call { name, args } => {
                let mut vals = Vec::new();
                for a in args { vals.push(self.eval(a, line)?); }
                self.call(name, vals, line)
            }
        }
    }

    // Placeholder until Task 10 fills in real function calls.
    fn call(&mut self, name: &str, _args: Vec<Value>, line: usize) -> LResult<Value> {
        Err(LinguaError { message: format!("functio nondum vocari potest: '{}'", name), line })
    }

    fn exec(&mut self, s: &Stmt) -> LResult<()> {
        match &s.kind {
            StmtKind::Let { name, value, .. } => {
                let v = self.eval(value, s.line)?;
                self.declare(name.clone(), v);
                Ok(())
            }
            StmtKind::Assign { name, value } => {
                let v = self.eval(value, s.line)?;
                self.assign(name, v, s.line)
            }
            StmtKind::Print(e) => {
                let v = self.eval(e, s.line)?;
                self.out.push_str(&v.display());
                self.out.push('\n');
                Ok(())
            }
            StmtKind::ExprStmt(e) => { self.eval(e, s.line)?; Ok(()) }
            StmtKind::Func(f) => { self.funcs.insert(f.name.clone(), f.clone()); Ok(()) }
            _ => Err(LinguaError { message: "sententia nondum tractata".into(), line: s.line }),
        }
    }
}

fn eval_binary(op: BinOp, l: Value, r: Value, line: usize) -> LResult<Value> {
    use Value::*;
    let bad = || LinguaError { message: "operatio binaria invalida".into(), line };
    match op {
        BinOp::Add => match (l, r) {
            (Int(a), Int(b)) => Ok(Int(a + b)),
            (Float(a), Float(b)) => Ok(Float(a + b)),
            (Text(a), Text(b)) => Ok(Text(a + &b)),
            _ => Err(bad()),
        },
        BinOp::Sub => match (l, r) {
            (Int(a), Int(b)) => Ok(Int(a - b)),
            (Float(a), Float(b)) => Ok(Float(a - b)),
            _ => Err(bad()),
        },
        BinOp::Mul => match (l, r) {
            (Int(a), Int(b)) => Ok(Int(a * b)),
            (Float(a), Float(b)) => Ok(Float(a * b)),
            _ => Err(bad()),
        },
        BinOp::Div => match (l, r) {
            (Int(_), Int(0)) => Err(LinguaError { message: "divisio per nullum".into(), line }),
            (Int(a), Int(b)) => Ok(Int(a / b)),
            (Float(a), Float(b)) => Ok(Float(a / b)),
            _ => Err(bad()),
        },
        BinOp::Mod => match (l, r) {
            (Int(_), Int(0)) => Err(LinguaError { message: "modulus per nullum".into(), line }),
            (Int(a), Int(b)) => Ok(Int(a % b)),
            _ => Err(bad()),
        },
        BinOp::Eq => Ok(Bool(l == r)),
        BinOp::Ne => Ok(Bool(l != r)),
        BinOp::Gt => cmp(l, r, line, |o| o == std::cmp::Ordering::Greater),
        BinOp::Lt => cmp(l, r, line, |o| o == std::cmp::Ordering::Less),
        BinOp::And => match (l, r) {
            (Bool(a), Bool(b)) => Ok(Bool(a && b)),
            _ => Err(bad()),
        },
        BinOp::Or => match (l, r) {
            (Bool(a), Bool(b)) => Ok(Bool(a || b)),
            _ => Err(bad()),
        },
    }
}

fn cmp(l: Value, r: Value, line: usize, keep: impl Fn(std::cmp::Ordering) -> bool) -> LResult<Value> {
    use Value::*;
    let ord = match (l, r) {
        (Int(a), Int(b)) => a.cmp(&b),
        (Float(a), Float(b)) => a.partial_cmp(&b)
            .ok_or_else(|| LinguaError { message: "comparatio NaN".into(), line })?,
        _ => return Err(LinguaError { message: "comparatio invalida".into(), line }),
    };
    Ok(Value::Bool(keep(ord)))
}

pub fn execute(program: &[Stmt]) -> LResult<String> {
    let mut it = Interp::new();
    // Register functions first so order doesn't matter.
    for s in program {
        if let StmtKind::Func(f) = &s.kind {
            it.funcs.insert(f.name.clone(), f.clone());
        }
    }
    for s in program {
        if !matches!(s.kind, StmtKind::Func(_)) {
            it.exec(s)?;
        }
    }
    Ok(it.out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::tokenize;
    use crate::parser::parse;

    fn run(src: &str) -> String {
        let program = parse(&tokenize(src).unwrap()).unwrap();
        execute(&program).unwrap()
    }

    #[test]
    fn arithmetic_and_print() {
        assert_eq!(run("scribe(°V + °III * °II);"), "11\n");
    }

    #[test]
    fn variables() {
        assert_eq!(run("ni x = °X; x = x + °II; scribe(x);"), "12\n");
    }

    #[test]
    fn booleans_print_in_latin() {
        assert_eq!(run("scribe(°V maior_est °III);"), "verus\n");
    }

    #[test]
    fn string_concatenation() {
        assert_eq!(run("scribe(\"sal\" + \"ve\");"), "salve\n");
    }
}
