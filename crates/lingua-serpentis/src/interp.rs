use crate::ast::*;
use crate::error::{LinguaError, LResult};
use crate::value::Value;
use std::collections::HashMap;

enum Flow { Normal, Break, Continue, Return(Value) }

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
            Expr::FloatLit(f) => Ok(Value::Double(*f)), // defensive: an unelaborated literal defaults to nr
            Expr::Float(f) => Ok(Value::Float(*f)),
            Expr::Double(f) => Ok(Value::Double(*f)),
            Expr::Text(s) => Ok(Value::Text(s.clone())),
            Expr::Char(c) => Ok(Value::Char(*c)),
            Expr::Bool(b) => Ok(Value::Bool(*b)),
            Expr::Ident(name) => self.get(name)
                .ok_or_else(|| LinguaError { message: format!("variabilis ignota: '{}'", name), line }),
            Expr::Unary { op, expr } => {
                let v = self.eval(expr, line)?;
                match (op, v) {
                    (UnOp::Not, Value::Bool(b)) => Ok(Value::Bool(!b)),
                    (UnOp::Neg, Value::Int(i)) => Ok(Value::Int(-i)),
                    (UnOp::Neg, Value::Float(f)) => Ok(Value::Float(-f)),
                    (UnOp::Neg, Value::Double(f)) => Ok(Value::Double(-f)),
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
            Expr::ListLit(elems) => {
                let mut vals = Vec::new();
                for el in elems { vals.push(self.eval(el, line)?); }
                Ok(Value::List(vals))
            }
        }
    }

    fn call(&mut self, name: &str, args: Vec<Value>, line: usize) -> LResult<Value> {
        let f = self.funcs.get(name)
            .cloned()
            .ok_or_else(|| LinguaError { message: format!("functio ignota: '{}'", name), line })?;
        if f.params.len() != args.len() {
            return Err(LinguaError {
                message: format!("functio '{}' numerum argumentorum falsum accepit", name),
                line,
            });
        }
        let mut frame = HashMap::new();
        for ((_, pname), v) in f.params.iter().zip(args) {
            frame.insert(pname.clone(), v);
        }
        // Swap in an isolated scope stack for the call, restore afterwards
        // (unconditionally, even if the body raises an error via `?`).
        let saved = std::mem::replace(&mut self.scopes, vec![frame]);
        let result = (|| -> LResult<Value> {
            for s in &f.body {
                if let Flow::Return(v) = self.exec(s)? { return Ok(v); }
            }
            Ok(Value::Int(0))
        })();
        self.scopes = saved;
        result
    }

    fn exec(&mut self, s: &Stmt) -> LResult<Flow> {
        match &s.kind {
            StmtKind::Let { name, value, .. } => {
                let v = self.eval(value, s.line)?;
                self.declare(name.clone(), v);
                Ok(Flow::Normal)
            }
            StmtKind::Assign { name, value } => {
                let v = self.eval(value, s.line)?;
                self.assign(name, v, s.line)?;
                Ok(Flow::Normal)
            }
            StmtKind::Print(e) => {
                let v = self.eval(e, s.line)?;
                self.out.push_str(&v.display());
                self.out.push('\n');
                Ok(Flow::Normal)
            }
            StmtKind::ExprStmt(e) => { self.eval(e, s.line)?; Ok(Flow::Normal) }
            StmtKind::Func(f) => { self.funcs.insert(f.name.clone(), f.clone()); Ok(Flow::Normal) }
            StmtKind::Break => Ok(Flow::Break),
            StmtKind::Continue => Ok(Flow::Continue),
            StmtKind::Return(e) => {
                let v = self.eval(e, s.line)?;
                Ok(Flow::Return(v))
            }
            StmtKind::If { cond, then, else_ } => {
                if self.truthy(cond, s.line)? {
                    self.run_block(then)
                } else if let Some(eb) = else_ {
                    self.run_block(eb)
                } else {
                    Ok(Flow::Normal)
                }
            }
            StmtKind::While { cond, body } => {
                while self.truthy(cond, s.line)? {
                    match self.run_block(body)? {
                        Flow::Break => break,
                        Flow::Return(v) => return Ok(Flow::Return(v)),
                        _ => {}
                    }
                }
                Ok(Flow::Normal)
            }
            StmtKind::For { init, cond, step, body } => {
                self.scopes.push(HashMap::new()); // loop scope holds the init var
                let result = (|| {
                    self.exec(init)?;
                    while self.truthy(cond, s.line)? {
                        match self.run_block(body)? {
                            Flow::Break => break,
                            Flow::Return(v) => return Ok(Flow::Return(v)),
                            _ => {}
                        }
                        self.exec(step)?;
                    }
                    Ok(Flow::Normal)
                })();
                self.scopes.pop(); // pop unconditionally, even on error `?` exit
                result
            }
        }
    }

    fn run_block(&mut self, body: &[Stmt]) -> LResult<Flow> {
        self.scopes.push(HashMap::new());
        let result = (|| {
            for s in body {
                match self.exec(s)? {
                    Flow::Normal => {}
                    other => return Ok(other),
                }
            }
            Ok(Flow::Normal)
        })();
        self.scopes.pop(); // pop unconditionally, even on error `?` exit
        result
    }

    fn truthy(&mut self, cond: &Expr, line: usize) -> LResult<bool> {
        match self.eval(cond, line)? {
            Value::Bool(b) => Ok(b),
            _ => Err(LinguaError { message: "condicio non est veritas".into(), line }),
        }
    }
}

fn eval_binary(op: BinOp, l: Value, r: Value, line: usize) -> LResult<Value> {
    use Value::*;
    let bad = || LinguaError { message: "operatio binaria invalida".into(), line };
    let overflow = || LinguaError { message: "numerus nimis magnus (excessus arithmeticus)".into(), line };
    match op {
        BinOp::Add => match (l, r) {
            (Int(a), Int(b)) => a.checked_add(b).map(Int).ok_or_else(overflow),
            (Float(a), Float(b)) => Ok(Float(a + b)),
            (Double(a), Double(b)) => Ok(Double(a + b)),
            (Text(a), Text(b)) => Ok(Text(a + &b)),
            _ => Err(bad()),
        },
        BinOp::Sub => match (l, r) {
            (Int(a), Int(b)) => a.checked_sub(b).map(Int).ok_or_else(overflow),
            (Float(a), Float(b)) => Ok(Float(a - b)),
            (Double(a), Double(b)) => Ok(Double(a - b)),
            _ => Err(bad()),
        },
        BinOp::Mul => match (l, r) {
            (Int(a), Int(b)) => a.checked_mul(b).map(Int).ok_or_else(overflow),
            (Float(a), Float(b)) => Ok(Float(a * b)),
            (Double(a), Double(b)) => Ok(Double(a * b)),
            _ => Err(bad()),
        },
        BinOp::Div => match (l, r) {
            (Int(_), Int(0)) => Err(LinguaError { message: "divisio per nullum".into(), line }),
            (Int(a), Int(b)) => a.checked_div(b).map(Int).ok_or_else(overflow),
            (Float(a), Float(b)) => Ok(Float(a / b)),
            (Double(a), Double(b)) => Ok(Double(a / b)),
            _ => Err(bad()),
        },
        BinOp::Mod => match (l, r) {
            (Int(_), Int(0)) => Err(LinguaError { message: "modulus per nullum".into(), line }),
            (Int(a), Int(b)) => a.checked_rem(b).map(Int).ok_or_else(overflow),
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
        (Double(a), Double(b)) => a.partial_cmp(&b)
            .ok_or_else(|| LinguaError { message: "comparatio NaN".into(), line })?,
        (Text(a), Text(b)) => a.cmp(&b),
        (Char(a), Char(b)) => a.cmp(&b),
        (Bool(a), Bool(b)) => a.cmp(&b),
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
            it.exec(s)?; // top-level Flow is ignored
        }
    }
    Ok(it.out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::tokenize;
    use crate::parser::parse;

    // This helper parses and executes WITHOUT running the type checker,
    // so unelaborated float literals hit the interpreter's defensive `Expr::FloatLit`
    // arm rather than being type-refined. For precision assertions, use the full
    // pipeline via `tests/programs.rs` instead.
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

    #[test]
    fn while_loop_counts() {
        // NOTE: brief used "°nullus"; actual lexer/roman zero-literal is bare "nullus"
        // (see src/lexer.rs word_to_kind and src/roman.rs parse_roman) — adjusted spelling only.
        let src = "ni i = nullus; ni s = nullus; indem (i minor_est °V) { s = s + i; i = i + °I; } scribe(s);";
        assert_eq!(run(src), "10\n"); // 0+1+2+3+4
    }

    #[test]
    fn for_loop_with_break() {
        let src = "ni total = nullus; dum (ni i = nullus; i minor_est °X; i = i + °I) { si (i par_est °III) { abrumpe; } total = total + i; } scribe(total);";
        assert_eq!(run(src), "3\n"); // 0+1+2 then break at 3
    }

    #[test]
    fn function_call_returns() {
        let src = "munus adde capit (ni a, ni b) reddit ni { redde a + b; } scribe(adde(°IV, °VI));";
        assert_eq!(run(src), "10\n");
    }

    #[test]
    fn continue_skips_iteration() {
        // skip i==2: 0+1+3+4 = 8
        let src = "ni s = nullus; dum (ni i = nullus; i minor_est °V; i = i + °I) { si (i par_est °II) { continua; } s = s + i; } scribe(s);";
        assert_eq!(run(src), "8\n");
    }

    #[test]
    fn text_comparison() {
        assert_eq!(run("scribe(\"apple\" minor_est \"banana\");"), "verus\n");
    }

    #[test]
    fn char_literal_roundtrip() {
        assert_eq!(run("lit c = 'x'; scribe(c);"), "x\n");
    }

    #[test]
    fn arithmetic_overflow_errors() {
        // 3999^6 overflows i64; must be a graceful Err, not a panic
        let src = "ni x = °MMMCMXCIX; ni y = x * x * x * x * x * x; scribe(y);";
        let program = parse(&tokenize(src).unwrap()).unwrap();
        assert!(execute(&program).is_err());
    }

    #[test]
    fn list_literal_prints() {
        assert_eq!(run("scribe([°I, °II, °III]);"), "[1, 2, 3]\n");
        assert_eq!(run("scribe([]);"), "[]\n");
    }
}
