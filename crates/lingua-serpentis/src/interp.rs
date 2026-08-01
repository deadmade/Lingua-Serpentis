use crate::ast::*;
use crate::error::{LinguaError, LResult, ProgramError};
use crate::fraction;
use crate::roman::Roman;
use crate::value::Value;
use std::collections::HashMap;

enum Flow { Normal, Break, Continue, Return(Value) }

struct Interp {
    scopes: Vec<HashMap<String, Value>>,
    funcs: HashMap<String, (FuncDecl, String)>, // qualified name -> (decl, source path)
    out: String,
    err_file: Option<String>, // set by the innermost frame that raised an error
    // The package namespace of the currently executing function's frame.
    // `None` for root and legacy (checker-less) mode; `Some(pkg)` while a
    // package function's body is running, so its own bare calls resolve
    // against `pkg`'s functions, mirroring the checker's namespace rule.
    current_ns: Option<String>,
}

impl Interp {
    fn new() -> Self {
        Interp {
            scopes: vec![HashMap::new()],
            funcs: HashMap::new(),
            out: String::new(),
            err_file: None,
            current_ns: None,
        }
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
            Expr::Int(r) => Ok(Value::Int(r.clone())),
            Expr::FloatLit { numer, den } => {
                // defensive: an unelaborated literal (checker skipped) defaults to nr
                Ok(Value::Double(fraction::from_rational(numer, den, &fraction::scripulum_grain())))
            }
            Expr::Float(c) => Ok(Value::Float(c.clone())),
            Expr::Double(c) => Ok(Value::Double(c.clone())),
            Expr::Text(s) => Ok(Value::Text(s.clone())),
            Expr::Char(c) => Ok(Value::Char(*c)),
            Expr::Bool(b) => Ok(Value::Bool(*b)),
            Expr::Ident(name) => self.get(name)
                .ok_or_else(|| LinguaError { message: format!("variabilis ignota: '{}'", name), line }),
            Expr::Unary { op, expr } => {
                let v = self.eval(expr, line)?;
                match (op, v) {
                    (UnOp::Not, Value::Bool(b)) => Ok(Value::Bool(!b)),
                    (UnOp::Neg, Value::Int(r)) => Ok(Value::Int(r.neg())),
                    (UnOp::Neg, Value::Float(c)) => Ok(Value::Float(c.neg())),
                    (UnOp::Neg, Value::Double(c)) => Ok(Value::Double(c.neg())),
                    _ => Err(LinguaError { message: "operatio unaria invalida".into(), line }),
                }
            }
            Expr::Binary { op, left, right } => {
                let l = self.eval(left, line)?;
                let r = self.eval(right, line)?;
                eval_binary(*op, l, r, line)
            }
            Expr::Call { package, name, args } => {
                let mut vals = Vec::new();
                for a in args { vals.push(self.eval(a, line)?); }
                if package.is_none() && name.as_str() == "longitudo" {
                    return match vals.into_iter().next() {
                        Some(Value::List(items)) => Ok(Value::Int(Roman::from_usize(items.len()))),
                        _ => Err(LinguaError { message: "'longitudo' seriem expectat".into(), line }),
                    };
                }
                if package.is_none() && name.as_str() == "adde" {
                    let mut it = vals.into_iter();
                    let (list, elem) = (it.next(), it.next());
                    return match (list, elem) {
                        (Some(Value::List(mut items)), Some(v)) => {
                            items.push(v);
                            Ok(Value::List(items))
                        }
                        _ => Err(LinguaError { message: "'adde' seriem et elementum expectat".into(), line }),
                    };
                }
                match package {
                    Some(p) => self.call(&format!("{}.{}", p, name), vals, line),
                    None => {
                        // A bare call resolves in the *executing* frame's own
                        // namespace (no fallback to the root), exactly like
                        // the checker: `check_file` validates a package
                        // file's unqualified calls against that package's
                        // own function table only.
                        match self.current_ns.clone() {
                            Some(ns) => self.call(&format!("{}.{}", ns, name), vals, line),
                            None => self.call(name, vals, line),
                        }
                    }
                }
            }
            Expr::ListLit(elems) => {
                let mut vals = Vec::new();
                for el in elems { vals.push(self.eval(el, line)?); }
                Ok(Value::List(vals))
            }
            Expr::Index { base, index } => {
                let b = self.eval(base, line)?;
                let i = self.eval(index, line)?;
                match (b, i) {
                    (Value::List(items), Value::Int(idx)) => {
                        match idx.to_usize() {
                            Some(i) if i >= 1 && i <= items.len() => Ok(items[i - 1].clone()),
                            _ => Err(LinguaError {
                                message: format!("index extra fines: {} (longitudo {})", idx.display(), items.len()),
                                line,
                            }),
                        }
                    }
                    _ => Err(LinguaError { message: "subscriptio invalida".into(), line }),
                }
            }
        }
    }

    fn call(&mut self, name: &str, args: Vec<Value>, line: usize) -> LResult<Value> {
        let (f, file) = self.funcs.get(name)
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
        // Package and function names cannot contain '.' (a package name must
        // lex as a single identifier), so this split is unambiguous: a
        // dotted lookup key's namespace is everything before the first '.'.
        let callee_ns = name.split_once('.').map(|(pkg, _)| pkg.to_string());
        // Swap in an isolated scope stack for the call, restore afterwards
        // (unconditionally, even if the body raises an error via `?`).
        let saved = std::mem::replace(&mut self.scopes, vec![frame]);
        let saved_ns = std::mem::replace(&mut self.current_ns, callee_ns);
        let result = (|| -> LResult<Value> {
            for s in &f.body {
                if let Flow::Return(v) = self.exec(s)? { return Ok(v); }
            }
            Ok(Value::Int(Roman::zero()))
        })();
        self.scopes = saved;
        self.current_ns = saved_ns;
        // Innermost frame first: the failing function's own file wins.
        if result.is_err() && self.err_file.is_none() && !file.is_empty() {
            self.err_file = Some(file);
        }
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
            StmtKind::IndexAssign { name, index, value } => {
                let idx = self.eval(index, s.line)?;
                let v = self.eval(value, s.line)?;
                let idx = match idx {
                    Value::Int(r) => r,
                    _ => return Err(LinguaError { message: "index seriei invalidus".into(), line: s.line }),
                };
                for scope in self.scopes.iter_mut().rev() {
                    if let Some(slot) = scope.get_mut(name) {
                        return match slot {
                            Value::List(items) => match idx.to_usize() {
                                Some(i) if i >= 1 && i <= items.len() => {
                                    items[i - 1] = v;
                                    Ok(Flow::Normal)
                                }
                                _ => Err(LinguaError {
                                    message: format!("index extra fines: {} (longitudo {})", idx.display(), items.len()),
                                    line: s.line,
                                }),
                            },
                            _ => Err(LinguaError { message: format!("'{}' non est series", name), line: s.line }),
                        };
                    }
                }
                Err(LinguaError { message: format!("variabilis ignota: '{}'", name), line: s.line })
            }
            StmtKind::Print(e) => {
                let v = self.eval(e, s.line)?;
                self.out.push_str(&v.display());
                self.out.push('\n');
                Ok(Flow::Normal)
            }
            StmtKind::ExprStmt(e) => { self.eval(e, s.line)?; Ok(Flow::Normal) }
            StmtKind::Func(f) => { self.funcs.insert(f.name.clone(), (f.clone(), String::new())); Ok(Flow::Normal) }
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
    match op {
        BinOp::Add => match (l, r) {
            (Int(a), Int(b)) => Ok(Int(a.add(&b))),
            (Float(a), Float(b)) => Ok(Float(a.add(&b))),
            (Double(a), Double(b)) => Ok(Double(a.add(&b))),
            (Text(a), Text(b)) => Ok(Text(a + &b)),
            _ => Err(bad()),
        },
        BinOp::Sub => match (l, r) {
            (Int(a), Int(b)) => Ok(Int(a.sub(&b))),
            (Float(a), Float(b)) => Ok(Float(a.sub(&b))),
            (Double(a), Double(b)) => Ok(Double(a.sub(&b))),
            _ => Err(bad()),
        },
        BinOp::Mul => match (l, r) {
            (Int(a), Int(b)) => Ok(Int(a.mul(&b))),
            (Float(a), Float(b)) => Ok(Float(fraction::mul(&a, &b, &fraction::uncia_grain()))),
            (Double(a), Double(b)) => Ok(Double(fraction::mul(&a, &b, &fraction::scripulum_grain()))),
            _ => Err(bad()),
        },
        BinOp::Div => match (l, r) {
            (Int(_), Int(ref b)) if b.is_zero() => Err(LinguaError { message: "divisio per nullum".into(), line }),
            (Int(a), Int(b)) => Ok(Int(a.div_rem(&b).0)),
            (Float(a), Float(b)) => match fraction::div(&a, &b, &fraction::uncia_grain()) {
                Some(q) => Ok(Float(q)),
                None => Err(LinguaError { message: "divisio per nullum".into(), line }),
            },
            (Double(a), Double(b)) => match fraction::div(&a, &b, &fraction::scripulum_grain()) {
                Some(q) => Ok(Double(q)),
                None => Err(LinguaError { message: "divisio per nullum".into(), line }),
            },
            _ => Err(bad()),
        },
        BinOp::Mod => match (l, r) {
            (Int(_), Int(ref b)) if b.is_zero() => Err(LinguaError { message: "modulus per nullum".into(), line }),
            (Int(a), Int(b)) => Ok(Int(a.div_rem(&b).1)),
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
        (Float(a), Float(b)) => a.cmp(&b),
        (Double(a), Double(b)) => a.cmp(&b),
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
            it.funcs.insert(f.name.clone(), (f.clone(), String::new()));
        }
    }
    for s in program {
        if !matches!(s.kind, StmtKind::Func(_)) {
            it.exec(s)?; // top-level Flow is ignored
        }
    }
    Ok(it.out)
}

/// Unix keeps an exit status's low 8 bits: the value mod CCLVI, shifted
/// non-negative (a negative remainder gains one modulus).
fn exit_status(r: &Roman) -> u8 {
    let rem = r.div_rem(&Roman::from_u64(256)).1;
    let x = rem.to_i64().expect("|rem| < 256 fits i64");
    ((x + 256) % 256) as u8
}

/// Run a checked program (§4.3 of the design): register every function under
/// its qualified name, call the root's `initium`, and turn its returned ni
/// into the process exit status.
pub fn execute_program(prog: &ParsedProgram) -> Result<(String, u8), ProgramError> {
    let mut it = Interp::new();
    for file in &prog.root {
        for s in &file.ast.funcs {
            if let StmtKind::Func(f) = &s.kind {
                it.funcs.insert(f.name.clone(), (f.clone(), file.path.clone()));
            }
        }
    }
    for (pkg, files) in &prog.packages {
        for file in files {
            for s in &file.ast.funcs {
                if let StmtKind::Func(f) = &s.kind {
                    it.funcs.insert(format!("{}.{}", pkg, f.name), (f.clone(), file.path.clone()));
                }
            }
        }
    }
    match it.call("initium", Vec::new(), 0) {
        Ok(v) => {
            let code = match v {
                Value::Int(r) => exit_status(&r),
                _ => 0, // checker guarantees Int; defensive only
            };
            Ok((it.out, code))
        }
        Err(e) => Err(ProgramError {
            file: it.err_file.take(),
            line: Some(e.line),
            message: e.message,
        }),
    }
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
        assert_eq!(run("scribe(°V + °III * °II);"), "XI\n");
    }

    #[test]
    fn variables() {
        assert_eq!(run("ni x = °X; x = x + °II; scribe(x);"), "XII\n");
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
        assert_eq!(run(src), "X\n"); // 0+1+2+3+4
    }

    #[test]
    fn for_loop_with_break() {
        let src = "ni total = nullus; dum (ni i = nullus; i minor_est °X; i = i + °I) { si (i par_est °III) { abrumpe; } total = total + i; } scribe(total);";
        assert_eq!(run(src), "III\n"); // 0+1+2 then break at 3
    }

    #[test]
    fn function_call_returns() {
        let src = "munus iunge capit (ni a, ni b) reddit ni { redde a + b; } scribe(iunge(°IV, °VI));";
        assert_eq!(run(src), "X\n");
    }

    #[test]
    fn continue_skips_iteration() {
        // skip i==2: 0+1+3+4 = 8
        let src = "ni s = nullus; dum (ni i = nullus; i minor_est °V; i = i + °I) { si (i par_est °II) { continua; } s = s + i; } scribe(s);";
        assert_eq!(run(src), "VIII\n");
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
    fn unbounded_arithmetic_no_longer_overflows() {
        // 3999^6 overflowed i64 and used to be an error; the Roman engine grows
        // another tier instead, so this now succeeds.
        let src = "ni x = °MMMCMXCIX; ni y = x * x * x * x * x * x; scribe(y);";
        let program = parse(&tokenize(src).unwrap()).unwrap();
        assert!(execute(&program).is_ok());
    }

    #[test]
    fn list_literal_prints() {
        assert_eq!(run("scribe([°I, °II, °III]);"), "[I, II, III]\n");
        assert_eq!(run("scribe([]);"), "[]\n");
    }

    #[test]
    fn index_read_is_one_based() {
        assert_eq!(run("ni lista xs = [°X, °XX, °XXX]; scribe(xs[°I]);"), "X\n");
        assert_eq!(run("ni lista xs = [°X, °XX, °XXX]; scribe(xs[°III]);"), "XXX\n");
    }

    #[test]
    fn index_out_of_range_errors() {
        let bad_hi = "ni lista xs = [°I, °II]; scribe(xs[°III]);";
        assert!(execute(&parse(&tokenize(bad_hi).unwrap()).unwrap()).is_err());
        // index 0 (bare `nullus`) is below the 1-based floor
        let bad_lo = "ni lista xs = [°I, °II]; scribe(xs[nullus]);";
        assert!(execute(&parse(&tokenize(bad_lo).unwrap()).unwrap()).is_err());
    }

    #[test]
    fn index_write_mutates_in_place() {
        assert_eq!(
            run("ni lista xs = [°I, °II, °III]; xs[°II] = °X; scribe(xs);"),
            "[I, X, III]\n"
        );
    }

    #[test]
    fn index_write_out_of_range_errors() {
        let src = "ni lista xs = [°I]; xs[°II] = °X; scribe(xs);";
        assert!(execute(&parse(&tokenize(src).unwrap()).unwrap()).is_err());
    }

    #[test]
    fn longitudo_and_adde_run() {
        assert_eq!(run("ni lista xs = [°I, °II, °III]; scribe(longitudo(xs));"), "III\n");
        assert_eq!(run("ni lista xs = [°I]; xs = adde(xs, °II); scribe(xs);"), "[I, II]\n");
    }

    #[test]
    fn list_equality_runs() {
        assert_eq!(run("scribe([°I, °II] par_est [°I, °II]);"), "verus\n");
        assert_eq!(run("scribe([°I] par_est [°II]);"), "falsus\n");
    }

    #[test]
    fn nested_list_read() {
        assert_eq!(
            run("ni lista lista m = [[°I, °II], [°III]]; scribe(m[°I][°II]);"),
            "II\n"
        );
        assert_eq!(
            run("ni lista lista m = [[°I, °II], [°III]]; scribe(m[°II][°I]);"),
            "III\n"
        );
    }

    use crate::ast::{ParsedFile, ParsedProgram};
    use crate::parser::parse_file;
    use std::collections::BTreeMap;

    fn pfile(path: &str, src: &str) -> ParsedFile {
        ParsedFile { path: path.into(), ast: parse_file(&tokenize(src).unwrap()).unwrap() }
    }

    fn run_prog(root: &[(&str, &str)], pkgs: &[(&str, &[(&str, &str)])])
        -> Result<(String, u8), crate::error::ProgramError>
    {
        let mut packages = BTreeMap::new();
        for (name, files) in pkgs {
            packages.insert(name.to_string(), files.iter().map(|(p, s)| pfile(p, s)).collect());
        }
        let prog = ParsedProgram {
            root: root.iter().map(|(p, s)| pfile(p, s)).collect(),
            packages,
        };
        execute_program(&prog)
    }

    #[test]
    fn initium_runs_and_exits_zero() {
        let r = run_prog(&[("a.liss", "munus initium reddit ni { scribe(°V); redde nullus; }")], &[]);
        assert_eq!(r.unwrap(), ("V\n".to_string(), 0));
    }

    #[test]
    fn exit_status_is_the_return_mod_256() {
        let r = run_prog(&[("a.liss", "munus initium reddit ni { redde °CCLVII; }")], &[]);
        assert_eq!(r.unwrap().1, 1); // 257 mod 256
        let r = run_prog(&[("a.liss", "munus initium reddit ni { redde °CCLVI; }")], &[]);
        assert_eq!(r.unwrap().1, 0); // 256 mod 256
        let r = run_prog(&[("a.liss", "munus initium reddit ni { redde °V; }")], &[]);
        assert_eq!(r.unwrap().1, 5);
    }

    #[test]
    fn negative_return_wraps_like_unix() {
        let r = run_prog(&[("a.liss", "munus initium reddit ni { redde -°I; }")], &[]);
        assert_eq!(r.unwrap().1, 255);
    }

    #[test]
    fn falling_off_initium_exits_zero() {
        // The existing fall-through semantics return Int zero.
        let r = run_prog(&[("a.liss", "munus initium reddit ni { scribe(°I); }")], &[]);
        assert_eq!(r.unwrap(), ("I\n".to_string(), 0));
    }

    #[test]
    fn qualified_call_reaches_the_package() {
        let r = run_prog(
            &[("principale.liss",
               "affer mathematica;\nmunus initium reddit ni { scribe(mathematica.duplum(°XXI)); redde nullus; }")],
            &[("mathematica", &[("mathematica/m.liss",
               "munus duplum capit (ni n) reddit ni { redde n * °II; }")])],
        );
        assert_eq!(r.unwrap().0, "XLII\n");
    }

    #[test]
    fn intra_package_bare_call_same_file() {
        // A package function's bare call to a sibling in the *same file*
        // must resolve inside the package, not against the root namespace.
        let r = run_prog(
            &[("principale.liss",
               "affer mathematica;\nmunus initium reddit ni { scribe(mathematica.decem()); redde nullus; }")],
            &[("mathematica", &[("mathematica/m.liss",
               "munus quinque reddit ni { redde °V; }\nmunus decem reddit ni { redde quinque() + quinque(); }")])],
        );
        assert_eq!(r.unwrap().0, "X\n");
    }

    #[test]
    fn intra_package_bare_call_across_files() {
        // Same as above, but `quinque` and `decem` live in two different files
        // of the same package — still the same namespace.
        let r = run_prog(
            &[("principale.liss",
               "affer mathematica;\nmunus initium reddit ni { scribe(mathematica.decem()); redde nullus; }")],
            &[("mathematica", &[
                ("mathematica/quinque.liss", "munus quinque reddit ni { redde °V; }"),
                ("mathematica/decem.liss", "munus decem reddit ni { redde quinque() + quinque(); }"),
            ])],
        );
        assert_eq!(r.unwrap().0, "X\n");
    }

    #[test]
    fn package_bare_call_ignores_root_shadow() {
        // The root defines its own `quinque` (returning C). The package's bare
        // call to `quinque()` must still bind to the package's own `quinque`
        // (returning V), not silently fall through to the root's — this is
        // the soundness half of the finding.
        let r = run_prog(
            &[("principale.liss",
               "affer mathematica;\nmunus quinque reddit ni { redde °C; }\nmunus initium reddit ni { scribe(mathematica.decem()); redde nullus; }")],
            &[("mathematica", &[("mathematica/m.liss",
               "munus quinque reddit ni { redde °V; }\nmunus decem reddit ni { redde quinque() + quinque(); }")])],
        );
        assert_eq!(r.unwrap().0, "X\n"); // not "CC\n"
    }

    #[test]
    fn same_folder_files_share_a_namespace() {
        let r = run_prog(
            &[
                ("auxilia.liss", "munus quinque reddit ni { redde °V; }"),
                ("principale.liss", "munus initium reddit ni { scribe(quinque()); redde nullus; }"),
            ],
            &[],
        );
        assert_eq!(r.unwrap().0, "V\n");
    }

    #[test]
    fn runtime_error_names_the_file_of_the_failing_function() {
        let r = run_prog(
            &[("principale.liss",
               "affer mathematica;\nmunus initium reddit ni { redde mathematica.divide(°I, nullus); }")],
            &[("mathematica", &[("mathematica/m.liss",
               "munus divide capit (ni a, ni b) reddit ni { redde a / b; }")])],
        );
        let e = r.unwrap_err();
        assert_eq!(e.file.as_deref(), Some("mathematica/m.liss"));
        assert_eq!(e.line, Some(1));
        assert!(e.message.contains("divisio"), "got: {}", e.message);
    }

    #[test]
    fn runtime_error_in_initium_names_the_root_file() {
        let r = run_prog(
            &[("principale.liss", "munus initium reddit ni { redde °I / nullus; }")], &[]);
        let e = r.unwrap_err();
        assert_eq!(e.file.as_deref(), Some("principale.liss"));
    }
}
