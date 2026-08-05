use crate::ast::*;
use crate::error::{LinguaError, LResult, ProgramError};
use std::collections::{HashMap, HashSet};

/// Synthesized type of an expression during checking. `FloatLit` is the
/// not-yet-resolved type of a fractional literal — and of arithmetic that
/// combines only such literals; every other expression is `Known`.
#[derive(Clone, PartialEq)]
enum InferTy {
    Known(Type),
    FloatLit,
}

type Sig = (Vec<Type>, Type); // (param types, return type)

struct Checker {
    scopes: Vec<HashMap<String, Type>>,
    funcs: HashMap<String, Sig>,                       // current namespace (folder)
    packages: HashMap<String, HashMap<String, Sig>>,   // program-wide package signatures
    imports: HashSet<String>,                          // current file's affer set
    current_ret: Option<Type>, // Some(_) while checking a function body
    loop_depth: usize,         // how many `indem`/`dum` bodies enclose the current statement
}

impl Checker {
    fn new() -> Self {
        Checker {
            scopes: vec![HashMap::new()],
            funcs: HashMap::new(),
            packages: HashMap::new(),
            imports: HashSet::new(),
            current_ret: None,
            loop_depth: 0,
        }
    }

    /// Check a loop body with `abrumpe`/`continua` enabled inside it.
    fn check_loop_body(&mut self, body: &mut [Stmt]) -> LResult<()> {
        self.loop_depth += 1;
        let r = self.check_block(body);
        self.loop_depth -= 1;
        r
    }

    fn lookup(&self, name: &str) -> Option<Type> {
        self.scopes.iter().rev().find_map(|s| s.get(name).cloned())
    }
    /// `lookup`, raising the out-of-scope diagnostic when the name is unknown.
    ///
    /// One site rather than the three that used to spell this message out
    /// (the `Ident` expression, `Assign` and `IndexAssign`). Three identical
    /// literals meant the catalogue could not tell which one a message came
    /// from, and only the expression form was ever provoked — see D-24.
    fn lookup_or_err(&self, name: &str, line: usize) -> LResult<Type> {
        self.lookup(name).ok_or_else(|| LinguaError {
            message: format!("variabilis ignota: '{}'", name),
            line,
        })
    }
    fn declare(&mut self, name: String, ty: Type) {
        self.scopes.last_mut().unwrap().insert(name, ty);
    }

    fn check_args(&self, fname: &str, params: &[Type], args: &mut [Expr], line: usize) -> LResult<()> {
        if params.len() != args.len() {
            return Err(LinguaError {
                message: format!("functio '{}' numerum argumentorum falsum accepit", fname),
                line,
            });
        }
        for (want, arg) in params.iter().zip(args.iter_mut()) {
            let got = self.resolve_expr(arg, Some(want.clone()), line)?;
            if got != *want {
                return Err(LinguaError {
                    message: format!("argumentum typum '{}' expectat, sed '{}' datum est",
                        Self::type_name(want), Self::type_name(&got)),
                    line,
                });
            }
        }
        Ok(())
    }

    fn type_name(t: &Type) -> String {
        match t {
            Type::Int => "ni".into(), Type::Float => "mnr".into(), Type::Double => "nr".into(),
            Type::Text => "voc".into(), Type::Char => "lit".into(), Type::Bool => "veritas".into(),
            Type::List(inner) => format!("{} lista", Self::type_name(inner)),
        }
    }

    /// Name used for an `InferTy` in error messages; a bare fractional literal
    /// reports as `nr`, its default type.
    fn infer_name(t: &InferTy) -> String {
        match t {
            InferTy::Known(ty) => Self::type_name(ty),
            InferTy::FloatLit => "nr".into(),
        }
    }

    fn check_expr(&self, e: &mut Expr, line: usize) -> LResult<InferTy> {
        match e {
            Expr::Int(_) => Ok(InferTy::Known(Type::Int)),
            Expr::FloatLit { .. } => Ok(InferTy::FloatLit),
            Expr::Float(_) => Ok(InferTy::Known(Type::Float)),
            Expr::Double(_) => Ok(InferTy::Known(Type::Double)),
            Expr::Text(_) => Ok(InferTy::Known(Type::Text)),
            Expr::Char(_) => Ok(InferTy::Known(Type::Char)),
            Expr::Bool(_) => Ok(InferTy::Known(Type::Bool)),
            Expr::Ident(name) => self.lookup_or_err(name, line).map(InferTy::Known),
            Expr::Unary { op, expr } => {
                let t = self.check_expr(expr, line)?;
                match op {
                    UnOp::Not if t == InferTy::Known(Type::Bool) => Ok(InferTy::Known(Type::Bool)),
                    UnOp::Neg => match t {
                        InferTy::FloatLit => Ok(InferTy::FloatLit),
                        InferTy::Known(Type::Int) => Ok(InferTy::Known(Type::Int)),
                        InferTy::Known(Type::Float) => Ok(InferTy::Known(Type::Float)),
                        InferTy::Known(Type::Double) => Ok(InferTy::Known(Type::Double)),
                        _ => Err(LinguaError { message: "operatio unaria typo non convenit".into(), line }),
                    },
                    _ => Err(LinguaError { message: "operatio unaria typo non convenit".into(), line }),
                }
            }
            Expr::Binary { op, left, right } => {
                let lt = self.check_expr(left, line)?;
                let rt = self.check_expr(right, line)?;
                self.check_binary(*op, lt, rt, left, right, line)
            }
            Expr::Call { package: Some(p), name, args } => {
                if !self.imports.contains(p) {
                    return Err(LinguaError {
                        message: format!("fasciculus '{}' non allatus est", p),
                        line,
                    });
                }
                let sigs = self.packages.get(p).ok_or_else(|| LinguaError {
                    message: format!("fasciculus '{}' non inventus", p),
                    line,
                })?;
                let (params, ret) = sigs.get(name).cloned().ok_or_else(|| LinguaError {
                    message: format!("functio '{}' in fasciculo '{}' non inventa", name, p),
                    line,
                })?;
                self.check_args(name, &params, args, line)?;
                Ok(InferTy::Known(ret))
            }
            Expr::Call { package: None, name, args } => {
                if name.as_str() == "longitudo" {
                    if args.len() != 1 {
                        return Err(LinguaError { message: "'longitudo' unum argumentum accipit".into(), line });
                    }
                    let t = self.resolve_expr(&mut args[0], None, line)?;
                    return match t {
                        Type::List(_) => Ok(InferTy::Known(Type::Int)),
                        other => Err(LinguaError {
                            message: format!("'longitudo' seriem expectat, non '{}'", Self::type_name(&other)),
                            line,
                        }),
                    };
                }
                if name.as_str() == "adde" {
                    if args.len() != 2 {
                        return Err(LinguaError { message: "'adde' duo argumenta accipit".into(), line });
                    }
                    // If the list argument is a bare empty literal, the element
                    // type comes from the value argument; otherwise the list
                    // argument fixes it and the value must match.
                    let empty_list_arg = matches!(&args[0], Expr::ListLit(v) if v.is_empty());
                    if empty_list_arg {
                        let vt = self.resolve_expr(&mut args[1], None, line)?;
                        self.resolve_expr(&mut args[0], Some(Type::List(Box::new(vt.clone()))), line)?;
                        return Ok(InferTy::Known(Type::List(Box::new(vt))));
                    }
                    let lt = self.resolve_expr(&mut args[0], None, line)?;
                    let elem = match lt {
                        Type::List(inner) => *inner,
                        other => return Err(LinguaError {
                            message: format!("'adde' seriem expectat, non '{}'", Self::type_name(&other)),
                            line,
                        }),
                    };
                    let vt = self.resolve_expr(&mut args[1], Some(elem.clone()), line)?;
                    if vt != elem {
                        return Err(LinguaError {
                            message: format!("'adde': series '{}' elementum '{}' non accipit",
                                Self::type_name(&elem), Self::type_name(&vt)),
                            line,
                        });
                    }
                    return Ok(InferTy::Known(Type::List(Box::new(elem))));
                }
                let (params, ret) = self.funcs.get(name)
                    .cloned()
                    .ok_or_else(|| LinguaError { message: format!("functio ignota: '{}'", name), line })?;
                self.check_args(name, &params, args, line)?;
                Ok(InferTy::Known(ret))
            }
            Expr::ListLit(elems) => {
                let elem = self.check_list_elems(elems, None, line)?;
                Ok(InferTy::Known(Type::List(Box::new(elem))))
            }
            Expr::Index { base, index } => {
                let base_ty = self.resolve_expr(base, None, line)?;
                let elem = match base_ty {
                    Type::List(inner) => *inner,
                    other => return Err(LinguaError {
                        message: format!("res typi '{}' subscribi non potest (non est series)",
                            Self::type_name(&other)),
                        line,
                    }),
                };
                let it = self.resolve_expr(index, Some(Type::Int), line)?;
                if it != Type::Int {
                    return Err(LinguaError {
                        message: "index seriei 'ni' esse debet".into(), line });
                }
                Ok(InferTy::Known(elem))
            }
        }
    }

    /// Resolve a list literal's elements to a single element type. `expected`
    /// is the demanded element type: `Some` when the context fixes it (element
    /// float literals then adapt to it), `None` to infer from the first element
    /// (floats then default to `nr`). Errors on an empty slice with no expected
    /// type, or on heterogeneous elements. Returns the ELEMENT type; callers
    /// wrap it in `Type::List`.
    fn check_list_elems(&self, elems: &mut [Expr], expected: Option<Type>, line: usize) -> LResult<Type> {
        if elems.is_empty() {
            return match expected {
                Some(t) => Ok(t),
                None => Err(LinguaError {
                    message: "typus elementorum seriei vacuae ignotus".into(), line }),
            };
        }
        let (first_el, rest) = elems.split_first_mut().unwrap();
        let first = self.resolve_expr(first_el, expected.clone(), line)?;
        for el in rest {
            let want = expected.clone().or_else(|| Some(first.clone()));
            let t = self.resolve_expr(el, want, line)?;
            if t != first {
                return Err(LinguaError {
                    message: format!("elementa seriei discrepant: '{}' et '{}'",
                        Self::type_name(&first), Self::type_name(&t)),
                    line,
                });
            }
        }
        Ok(first)
    }

    /// Type a list literal against an optional expected list type (the primary
    /// path, since it has the expected element type in context).
    fn resolve_list(&self, e: &mut Expr, expected: Option<Type>, line: usize) -> LResult<Type> {
        let elem_expected = match &expected {
            Some(Type::List(inner)) => Some((**inner).clone()),
            _ => None,
        };
        let elems = match e {
            Expr::ListLit(v) => v,
            _ => unreachable!("resolve_list called on non-list"),
        };
        let elem = self.check_list_elems(elems, elem_expected, line)?;
        Ok(Type::List(Box::new(elem)))
    }

    /// Synthesize `e`'s type, resolving a bare fractional literal to a concrete
    /// float type. `expected` is the type the context wants (`None` for an
    /// untyped sink such as `scribe`). A `FloatLit` becomes the expected float
    /// type; with no float context it defaults to `nr`; demanded as a non-float
    /// type it is an error.
    fn resolve_expr(&self, e: &mut Expr, expected: Option<Type>, line: usize) -> LResult<Type> {
        if matches!(e, Expr::ListLit(_)) {
            return self.resolve_list(e, expected, line);
        }
        match self.check_expr(e, line)? {
            InferTy::Known(t) => Ok(t),
            InferTy::FloatLit => {
                let ty = match expected {
                    Some(Type::Float) => Type::Float,
                    Some(Type::Double) => Type::Double,
                    Some(other) => return Err(LinguaError {
                        message: format!("numerus fractus typum 'mnr' vel 'nr' requirit, non '{}'",
                            Self::type_name(&other)),
                        line,
                    }),
                    None => Type::Double,
                };
                resolve_floatlit(e, ty.clone());
                Ok(ty)
            }
        }
    }

    fn check_binary(&self, op: BinOp, lt: InferTy, rt: InferTy,
                    left: &mut Expr, right: &mut Expr, line: usize) -> LResult<InferTy> {
        use InferTy::*;
        let mismatch = || LinguaError {
            message: format!("operatio typis '{}' et '{}' non convenit",
                Self::infer_name(&lt), Self::infer_name(&rt)),
            line,
        };
        match op {
            BinOp::Add => match (&lt, &rt) {
                (FloatLit, FloatLit) => Ok(FloatLit),
                (FloatLit, Known(Type::Float)) => { resolve_floatlit(left, Type::Float); Ok(Known(Type::Float)) }
                (FloatLit, Known(Type::Double)) => { resolve_floatlit(left, Type::Double); Ok(Known(Type::Double)) }
                (Known(Type::Float), FloatLit) => { resolve_floatlit(right, Type::Float); Ok(Known(Type::Float)) }
                (Known(Type::Double), FloatLit) => { resolve_floatlit(right, Type::Double); Ok(Known(Type::Double)) }
                (Known(Type::Int), Known(Type::Int)) => Ok(Known(Type::Int)),
                (Known(Type::Float), Known(Type::Float)) => Ok(Known(Type::Float)),
                (Known(Type::Double), Known(Type::Double)) => Ok(Known(Type::Double)),
                (Known(Type::Text), Known(Type::Text)) => Ok(Known(Type::Text)),
                _ => Err(mismatch()),
            },
            BinOp::Sub | BinOp::Mul | BinOp::Div => match (&lt, &rt) {
                (FloatLit, FloatLit) => Ok(FloatLit),
                (FloatLit, Known(Type::Float)) => { resolve_floatlit(left, Type::Float); Ok(Known(Type::Float)) }
                (FloatLit, Known(Type::Double)) => { resolve_floatlit(left, Type::Double); Ok(Known(Type::Double)) }
                (Known(Type::Float), FloatLit) => { resolve_floatlit(right, Type::Float); Ok(Known(Type::Float)) }
                (Known(Type::Double), FloatLit) => { resolve_floatlit(right, Type::Double); Ok(Known(Type::Double)) }
                (Known(Type::Int), Known(Type::Int)) => Ok(Known(Type::Int)),
                (Known(Type::Float), Known(Type::Float)) => Ok(Known(Type::Float)),
                (Known(Type::Double), Known(Type::Double)) => Ok(Known(Type::Double)),
                _ => Err(mismatch()),
            },
            BinOp::Mod => match (&lt, &rt) {
                (Known(Type::Int), Known(Type::Int)) => Ok(Known(Type::Int)),
                _ => Err(mismatch()),
            },
            BinOp::Eq | BinOp::Ne => match (&lt, &rt) {
                (FloatLit, FloatLit) => {
                    resolve_floatlit(left, Type::Double);
                    resolve_floatlit(right, Type::Double);
                    Ok(Known(Type::Bool))
                }
                (FloatLit, Known(Type::Float)) => { resolve_floatlit(left, Type::Float); Ok(Known(Type::Bool)) }
                (FloatLit, Known(Type::Double)) => { resolve_floatlit(left, Type::Double); Ok(Known(Type::Bool)) }
                (Known(Type::Float), FloatLit) => { resolve_floatlit(right, Type::Float); Ok(Known(Type::Bool)) }
                (Known(Type::Double), FloatLit) => { resolve_floatlit(right, Type::Double); Ok(Known(Type::Bool)) }
                (Known(a), Known(b)) if a == b => Ok(Known(Type::Bool)),
                _ => Err(mismatch()),
            },
            BinOp::Gt | BinOp::Lt => match (&lt, &rt) {
                (FloatLit, FloatLit) => {
                    resolve_floatlit(left, Type::Double);
                    resolve_floatlit(right, Type::Double);
                    Ok(Known(Type::Bool))
                }
                (FloatLit, Known(Type::Float)) => { resolve_floatlit(left, Type::Float); Ok(Known(Type::Bool)) }
                (FloatLit, Known(Type::Double)) => { resolve_floatlit(left, Type::Double); Ok(Known(Type::Bool)) }
                (Known(Type::Float), FloatLit) => { resolve_floatlit(right, Type::Float); Ok(Known(Type::Bool)) }
                (Known(Type::Double), FloatLit) => { resolve_floatlit(right, Type::Double); Ok(Known(Type::Bool)) }
                (Known(Type::List(_)), Known(Type::List(_))) => Err(LinguaError {
                    message: "series ordine comparari non potest (par_est/par_non_est tantum)".into(),
                    line,
                }),
                (Known(a), Known(b)) if a == b => Ok(Known(Type::Bool)),
                _ => Err(mismatch()),
            },
            BinOp::And | BinOp::Or => match (&lt, &rt) {
                (Known(Type::Bool), Known(Type::Bool)) => Ok(Known(Type::Bool)),
                _ => Err(mismatch()),
            },
        }
    }

    fn check_stmt(&mut self, s: &mut Stmt) -> LResult<()> {
        let line = s.line;
        match &mut s.kind {
            StmtKind::Let { ty, name, value } => {
                let vt = self.resolve_expr(value, Some(ty.clone()), line)?;
                if vt != *ty {
                    return Err(LinguaError {
                        message: format!("'{}' typum '{}' expectat, sed '{}' datum est",
                            name, Self::type_name(ty), Self::type_name(&vt)),
                        line,
                    });
                }
                self.declare(name.clone(), ty.clone());
                Ok(())
            }
            StmtKind::Assign { name, value } => {
                let target = self.lookup_or_err(name, line)?;
                let vt = self.resolve_expr(value, Some(target.clone()), line)?;
                if vt != target {
                    return Err(LinguaError {
                        message: format!("'{}' typum '{}' expectat, sed '{}' datum est",
                            name, Self::type_name(&target), Self::type_name(&vt)),
                        line,
                    });
                }
                Ok(())
            }
            StmtKind::Print(e) | StmtKind::ExprStmt(e) => {
                self.resolve_expr(e, None, line)?;
                Ok(())
            }
            StmtKind::If { cond, then, else_ } => {
                self.expect_bool(cond, line)?;
                self.check_block(then)?;
                if let Some(eb) = else_ { self.check_block(eb)?; }
                Ok(())
            }
            StmtKind::While { cond, body } => {
                self.expect_bool(cond, line)?;
                self.check_loop_body(body)
            }
            StmtKind::For { init, cond, step, body } => {
                self.scopes.push(HashMap::new());
                let r = (|| {
                    // `init`, `cond` and `step` are the loop's header, not its
                    // body: an `abrumpe` there is as loose as one outside.
                    self.check_stmt(init)?;
                    self.expect_bool(cond, line)?;
                    self.check_stmt(step)?;
                    self.check_loop_body(body)
                })();
                self.scopes.pop();
                r
            }
            StmtKind::Func(f) => {
                self.scopes.push(HashMap::new());
                for (t, n) in &f.params { self.declare(n.clone(), t.clone()); }
                let saved = self.current_ret.clone();
                self.current_ret = Some(f.ret.clone());
                // A function body is not inside any loop, whatever encloses
                // the declaration. Nested functions are rejected, so this can
                // only matter for the legacy top-level entry point — but the
                // invariant is worth stating rather than relying on.
                let saved_depth = std::mem::take(&mut self.loop_depth);
                let r = self.check_block_inner(&mut f.body);
                self.loop_depth = saved_depth;
                self.current_ret = saved;
                self.scopes.pop();
                r
            }
            StmtKind::Return(e) => {
                let want = self.current_ret.clone()
                    .ok_or_else(|| LinguaError { message: "'redde' extra functionem".into(), line })?;
                let got = self.resolve_expr(e, Some(want.clone()), line)?;
                if got != want {
                    return Err(LinguaError {
                        message: format!("reditus typum '{}' expectat, sed '{}' datum est",
                            Self::type_name(&want), Self::type_name(&got)),
                        line,
                    });
                }
                Ok(())
            }
            StmtKind::IndexAssign { name, index, value } => {
                let target = self.lookup_or_err(name, line)?;
                let elem = match target {
                    Type::List(inner) => *inner,
                    other => return Err(LinguaError {
                        message: format!("'{}' non est series (typus '{}')", name, Self::type_name(&other)),
                        line,
                    }),
                };
                let it = self.resolve_expr(index, Some(Type::Int), line)?;
                if it != Type::Int {
                    return Err(LinguaError { message: "index seriei 'ni' esse debet".into(), line });
                }
                let vt = self.resolve_expr(value, Some(elem.clone()), line)?;
                if vt != elem {
                    return Err(LinguaError {
                        message: format!("elementum typum '{}' expectat, sed '{}' datum est",
                            Self::type_name(&elem), Self::type_name(&vt)),
                        line,
                    });
                }
                Ok(())
            }
            // Defectus D-07: these were accepted unconditionally. Outside a
            // loop the interpreter yields Flow::Break, and the function-body
            // loop matches only Flow::Return — so it was dropped on the floor
            // and execution carried on, with no diagnostic and no clue.
            StmtKind::Break if self.loop_depth == 0 => {
                Err(LinguaError { message: "'abrumpe' extra iterationem".into(), line })
            }
            StmtKind::Continue if self.loop_depth == 0 => {
                Err(LinguaError { message: "'continua' extra iterationem".into(), line })
            }
            StmtKind::Break | StmtKind::Continue => Ok(()),
        }
    }

    /// A block that opens a scope of its own: a `si`, `aut`, `indem` or `dum`
    /// body. It is `check_block_inner` between a push and a pop, and the pop
    /// happens on every path — which is what the two hand-written error exits
    /// it replaced were doing. Delegating also leaves one nested-function
    /// guard instead of two identical ones (D-24).
    fn check_block(&mut self, body: &mut [Stmt]) -> LResult<()> {
        self.scopes.push(HashMap::new());
        let r = self.check_block_inner(body);
        self.scopes.pop();
        r
    }

    /// A block that runs in a scope the caller already pushed — a function
    /// body, whose scope is the parameter frame.
    fn check_block_inner(&mut self, body: &mut [Stmt]) -> LResult<()> {
        for s in body.iter_mut() {
            if let StmtKind::Func(f) = &s.kind {
                return Err(LinguaError {
                    message: format!("functio '{}' intra corpus definiri non potest; functiones nidificatae non sustinentur", f.name),
                    line: s.line,
                });
            }
            self.check_stmt(s)?;
        }
        Ok(())
    }

    fn expect_bool(&self, cond: &mut Expr, line: usize) -> LResult<()> {
        if self.check_expr(cond, line)? == InferTy::Known(Type::Bool) {
            Ok(())
        } else {
            Err(LinguaError { message: "condicio 'veritas' esse debet".into(), line })
        }
    }
}

/// Rewrite every still-abstract fractional literal reachable through negation
/// and arithmetic into a concrete-precision node, narrowing the exact rational
/// onto the type's grain grid (uncia for mnr, scripulum for nr) by truncation.
/// Only ever called on an expression whose synthesized type was `FloatLit`.
fn resolve_floatlit(e: &mut Expr, ty: Type) {
    match e {
        Expr::FloatLit { numer, den } => {
            let (numer, den) = (numer.clone(), den.clone());
            *e = match ty {
                Type::Float => Expr::Float(
                    crate::fraction::from_rational(&numer, &den, &crate::fraction::uncia_grain())),
                Type::Double => Expr::Double(
                    crate::fraction::from_rational(&numer, &den, &crate::fraction::scripulum_grain())),
                _ => return,
            };
        }
        Expr::Unary { expr, .. } => resolve_floatlit(expr, ty),
        Expr::Binary { left, right, .. } => {
            resolve_floatlit(left, ty.clone());
            resolve_floatlit(right, ty);
        }
        _ => {}
    }
}

/// The names the language keeps for its own builtins, and the diagnostic for
/// taking one. Both entry points enforce this rule; they used to do it from
/// byte-identical copies, which is two implementations of one rule that can
/// drift apart, and only one of which a user ever runs (D-24).
fn reserved_name_error(name: &str, line: usize) -> Option<LinguaError> {
    matches!(name, "longitudo" | "adde").then(|| LinguaError {
        message: format!("nomen '{}' reservatum est", name),
        line,
    })
}

pub fn check(program: &mut [Stmt]) -> LResult<()> {
    let mut c = Checker::new();
    for s in program.iter() {
        if let StmtKind::Func(f) = &s.kind {
            if let Some(e) = reserved_name_error(&f.name, s.line) {
                return Err(e);
            }
            let ptypes = f.params.iter().map(|(t, _)| t.clone()).collect();
            c.funcs.insert(f.name.clone(), (ptypes, f.ret.clone()));
        }
    }
    for s in program.iter_mut() {
        c.check_stmt(s)?;
    }
    Ok(())
}

/// Collect one folder's function signatures across all its files, rejecting
/// reserved names and duplicates (§4.2 — two `initium` in the root land here
/// as the ordinary duplicate error).
fn collect_sigs(files: &[ParsedFile]) -> Result<HashMap<String, Sig>, ProgramError> {
    let mut sigs: HashMap<String, Sig> = HashMap::new();
    for file in files {
        for s in &file.ast.funcs {
            let StmtKind::Func(f) = &s.kind else { continue };
            if let Some(e) = reserved_name_error(&f.name, s.line) {
                return Err(ProgramError::in_file(&file.path, e));
            }
            if sigs.contains_key(&f.name) {
                return Err(ProgramError::in_file(&file.path, LinguaError {
                    message: format!("functio '{}' bis definita", f.name),
                    line: s.line,
                }));
            }
            let ptypes = f.params.iter().map(|(t, _)| t.clone()).collect();
            sigs.insert(f.name.clone(), (ptypes, f.ret.clone()));
        }
    }
    Ok(sigs)
}

/// Check one file's bodies against its folder's namespace and its own imports.
fn check_file(
    file: &mut ParsedFile,
    ns_sigs: &HashMap<String, Sig>,
    pkg_sigs: &HashMap<String, HashMap<String, Sig>>,
) -> Result<(), ProgramError> {
    for imp in &file.ast.imports {
        if !pkg_sigs.contains_key(&imp.package) {
            return Err(ProgramError::in_file(&file.path, LinguaError {
                message: format!("fasciculus '{}' non inventus", imp.package),
                line: imp.line,
            }));
        }
    }
    let mut c = Checker::new();
    c.funcs = ns_sigs.clone();
    c.packages = pkg_sigs.clone();
    c.imports = file.ast.imports.iter().map(|i| i.package.clone()).collect();
    for s in file.ast.funcs.iter_mut() {
        c.check_stmt(s).map_err(|e| ProgramError::in_file(&file.path, e))?;
    }
    Ok(())
}

/// Check a whole parsed program (§4 of the design): per-namespace signatures,
/// the `initium` entry point, import validity, then every file's bodies.
pub fn check_program(prog: &mut ParsedProgram) -> Result<(), ProgramError> {
    let root_sigs = collect_sigs(&prog.root)?;
    let mut pkg_sigs: HashMap<String, HashMap<String, Sig>> = HashMap::new();
    for (name, files) in &prog.packages {
        pkg_sigs.insert(name.clone(), collect_sigs(files)?);
    }

    match root_sigs.get("initium") {
        None => return Err(ProgramError::structural(
            "munus 'initium' in programmate non inventum")),
        Some((params, ret)) => {
            if !params.is_empty() || *ret != Type::Int {
                // find the declaration to attribute the error
                for file in &prog.root {
                    for s in &file.ast.funcs {
                        if let StmtKind::Func(f) = &s.kind {
                            if f.name == "initium" {
                                return Err(ProgramError::in_file(&file.path, LinguaError {
                                    message: "munus 'initium' nihil capere et 'ni' reddere debet".into(),
                                    line: s.line,
                                }));
                            }
                        }
                    }
                }
                unreachable!("initium was in root_sigs, so its declaration exists");
            }
        }
    }

    for file in prog.root.iter_mut() {
        check_file(file, &root_sigs, &pkg_sigs)?;
    }
    for (name, files) in prog.packages.iter_mut() {
        let ns = &pkg_sigs[name];
        for file in files.iter_mut() {
            check_file(file, ns, &pkg_sigs)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::tokenize;
    use crate::parser::parse;

    fn check_src(src: &str) -> LResult<()> {
        let mut program = parse(&tokenize(src).unwrap()).unwrap();
        check(&mut program)
    }

    #[test]
    fn well_typed_program_passes() {
        assert!(check_src("ni x = °V; x = x + °I; scribe(x);").is_ok());
    }

    #[test]
    fn break_outside_a_loop_is_rejected() {
        // Defectus D-07. The checker accepted `abrumpe` anywhere, and the
        // interpreter's function-body loop matches only Flow::Return — so the
        // Break was dropped on the floor and execution simply carried on.
        // Silently wrong behaviour with no diagnostic at all.
        let e = check_src("scribe(°I); abrumpe; scribe(°II);").unwrap_err();
        assert_eq!(e.message, "'abrumpe' extra iterationem");
    }

    #[test]
    fn continue_outside_a_loop_is_rejected() {
        let e = check_src("scribe(°I); continua; scribe(°II);").unwrap_err();
        assert_eq!(e.message, "'continua' extra iterationem");
    }

    #[test]
    fn break_and_continue_inside_a_loop_are_fine() {
        assert!(check_src("ni i = °I; indem (i minor_est °X) { i = i + °I; abrumpe; }").is_ok());
        assert!(check_src("ni i = °I; indem (i minor_est °X) { i = i + °I; continua; }").is_ok());
        // Nested one level deeper, inside an `si` within the loop body.
        assert!(check_src(
            "ni i = °I; indem (i minor_est °X) { si (i par_est °II) { abrumpe; } i = i + °I; }"
        )
        .is_ok());
    }

    #[test]
    fn a_loop_does_not_license_a_break_after_it() {
        // The depth must come back down: a loop earlier in the same block
        // must not make a later top-level `abrumpe` legal.
        let e = check_src(
            "ni i = °I; indem (i minor_est °X) { i = i + °I; } abrumpe;"
        )
        .unwrap_err();
        assert_eq!(e.message, "'abrumpe' extra iterationem");
    }

    #[test]
    fn assigning_text_to_int_fails() {
        let err = check_src("ni x = \"salve\";").unwrap_err();
        assert!(err.message.contains("typ")); // "typum ... expectat"
        assert_eq!(err.line, 1);
    }

    #[test]
    fn comparison_yields_bool() {
        assert!(check_src("veritas b = °V maior_est °III;").is_ok());
    }

    #[test]
    fn adding_int_and_text_fails() {
        assert!(check_src("ni x = °V + \"a\";").is_err());
    }

    #[test]
    fn using_undeclared_variable_fails() {
        assert!(check_src("scribe(y);").is_err());
    }

    #[test]
    fn non_bool_condition_fails() {
        let err = check_src("si (°V) { scribe(°I); }").unwrap_err();
        assert!(err.message.contains("veritas") || err.message.contains("condicio"));
    }

    #[test]
    fn function_return_type_checked() {
        assert!(check_src("munus iunge capit (ni a, ni b) reddit ni { redde a + b; } scribe(iunge(°I, °II));").is_ok());
    }

    #[test]
    fn returning_wrong_type_fails() {
        assert!(check_src("munus f reddit ni { redde \"salve\"; }").is_err());
    }

    #[test]
    fn calling_with_wrong_arg_type_fails() {
        assert!(check_src("munus f capit (ni a) reddit ni { redde a; } scribe(f(\"x\"));").is_err());
    }

    #[test]
    fn nested_function_rejected() {
        let err = check_src("munus outer reddit ni { munus inner reddit ni { redde °I; } redde °I; }").unwrap_err();
        assert!(err.message.contains("nidificat"));
    }

    #[test]
    fn fractional_literal_adapts_to_mnr_and_nr() {
        assert!(check_src("mnr x = °I:I/II;").is_ok());
        assert!(check_src("nr y = °I:I/II;").is_ok());
    }

    #[test]
    fn unconstrained_fraction_defaults_ok() {
        assert!(check_src("scribe(°I:I/II);").is_ok());
    }

    #[test]
    fn fraction_arithmetic_adapts_to_declared_double() {
        assert!(check_src("nr z = °I:I/II + °I:I/IV;").is_ok());
    }

    #[test]
    fn mixing_mnr_and_nr_is_rejected() {
        assert!(check_src("mnr a = °I:I/II; nr b = °I:I/IV; scribe(a + b);").is_err());
        assert!(check_src("mnr a = °I:I/II; nr b = a;").is_err());
        assert!(check_src("mnr a = °I:I/II; nr b = °I:I/IV; scribe(a maior_est b);").is_err());
    }

    #[test]
    fn fractional_literal_into_int_slot_is_rejected() {
        let err = check_src("ni n = °I:I/II;").unwrap_err();
        assert!(err.message.contains("fractus"), "got: {}", err.message);
    }

    #[test]
    fn function_returning_fraction_adapts_to_nr() {
        assert!(check_src("munus tau reddit nr { redde °I:I/III; }").is_ok());
    }

    #[test]
    fn function_accepts_fraction_argument_as_mnr() {
        // The literal argument adapts to the declared mnr parameter type.
        assert!(check_src("munus f capit (mnr a) reddit mnr { redde a; } scribe(f(°I:I/II));").is_ok());
    }

    #[test]
    fn list_type_names_render_recursively() {
        assert_eq!(Checker::type_name(&Type::List(Box::new(Type::Int))), "ni lista");
        assert_eq!(
            Checker::type_name(&Type::List(Box::new(Type::List(Box::new(Type::Text))))),
            "voc lista lista"
        );
    }

    #[test]
    fn homogeneous_list_literal_checks() {
        assert!(check_src("ni lista xs = [°I, °II, °III];").is_ok());
        assert!(check_src("voc lista ns = [\"a\", \"b\"];").is_ok());
    }

    #[test]
    fn mixed_element_list_is_rejected() {
        let err = check_src("ni lista xs = [°I, \"a\"];").unwrap_err();
        assert!(err.message.contains("discrepant") || err.message.contains("expectat"),
            "got: {}", err.message);
    }

    #[test]
    fn empty_list_needs_a_declared_element_type() {
        assert!(check_src("ni lista xs = [];").is_ok());          // annotation supplies it
        let err = check_src("scribe([]);").unwrap_err();          // no expected type
        assert!(err.message.contains("vacuae"), "got: {}", err.message);
    }

    #[test]
    fn list_of_fractions_adopts_declared_precision() {
        // Element float literals must adapt to the declared element type,
        // not default to nr.
        assert!(check_src("mnr lista xs = [°I:I/II, °I:I/IV];").is_ok());
        assert!(check_src("nr lista ys = [°I:I/II];").is_ok());
    }

    #[test]
    fn indexing_a_list_yields_element_type() {
        assert!(check_src("ni lista xs = [°I, °II]; ni y = xs[°I];").is_ok());
    }

    #[test]
    fn indexing_requires_int_index_and_a_list() {
        // mnr index rejected
        assert!(check_src("ni lista xs = [°I]; ni y = xs[°I:I/II];").is_err());
        // indexing a non-list rejected
        assert!(check_src("ni n = °V; ni y = n[°I];").is_err());
    }

    #[test]
    fn index_assignment_checks_element_type() {
        assert!(check_src("ni lista xs = [°I, °II]; xs[°I] = °X;").is_ok());
        // wrong element type
        assert!(check_src("ni lista xs = [°I]; xs[°I] = \"a\";").is_err());
        // assigning through a non-list variable
        assert!(check_src("ni n = °V; n[°I] = °X;").is_err());
        // non-int index
        assert!(check_src("ni lista xs = [°I]; xs[°I:I/II] = °X;").is_err());
    }

    #[test]
    fn longitudo_returns_int() {
        assert!(check_src("ni lista xs = [°I, °II]; ni n = longitudo(xs);").is_ok());
        assert!(check_src("ni n = longitudo(°V);").is_err()); // not a list
    }

    #[test]
    fn adde_appends_same_element_type() {
        assert!(check_src("ni lista xs = [°I]; xs = adde(xs, °II);").is_ok());
        assert!(check_src("ni lista xs = [°I]; xs = adde(xs, \"a\");").is_err()); // wrong elem
        // empty-literal list argument takes its element type from the value arg
        assert!(check_src("ni lista xs = adde([], °I);").is_ok());
    }

    #[test]
    fn reserved_builtin_names_cannot_be_user_functions() {
        assert!(check_src("munus adde capit (ni a) reddit ni { redde a; }").is_err());
        assert!(check_src("munus longitudo reddit ni { redde °I; }").is_err());
    }

    #[test]
    fn list_ordering_is_a_type_error() {
        let gt = check_src("ni lista xs = [°I]; ni lista ys = [°II]; scribe(xs maior_est ys);");
        assert!(gt.is_err());
        assert!(gt.unwrap_err().message.contains("ordine"));
        let lt = check_src("ni lista xs = [°I]; ni lista ys = [°II]; scribe(xs minor_est ys);");
        assert!(lt.is_err());
        assert!(lt.unwrap_err().message.contains("ordine"));
    }

    #[test]
    fn list_equality_is_allowed() {
        assert!(check_src("scribe([°I] par_est [°I]);").is_ok());
        assert!(check_src("scribe([°I] par_non_est [°I]);").is_ok());
    }

    use crate::ast::{ParsedFile, ParsedProgram};
    use crate::parser::parse_file;
    use std::collections::BTreeMap;

    fn pfile(path: &str, src: &str) -> ParsedFile {
        ParsedFile { path: path.into(), ast: parse_file(&tokenize(src).unwrap()).unwrap() }
    }

    fn pprog(root: &[(&str, &str)], pkgs: &[(&str, &[(&str, &str)])]) -> ParsedProgram {
        let mut packages = BTreeMap::new();
        for (name, files) in pkgs {
            packages.insert(
                name.to_string(),
                files.iter().map(|(p, s)| pfile(p, s)).collect(),
            );
        }
        ParsedProgram { root: root.iter().map(|(p, s)| pfile(p, s)).collect(), packages }
    }

    #[test]
    fn program_with_import_and_qualified_call_checks() {
        let mut p = pprog(
            &[("principale.liss",
               "affer mathematica;\nmunus initium reddit ni { scribe(mathematica.summa(°I, °II)); redde nullus; }")],
            &[("mathematica",
               &[("mathematica/summa.liss",
                  "munus summa capit (ni a, ni b) reddit ni { redde a + b; }")])],
        );
        assert!(check_program(&mut p).is_ok());
    }

    #[test]
    fn missing_initium_is_a_structural_error() {
        let mut p = pprog(&[("a.liss", "munus f reddit ni { redde nullus; }")], &[]);
        let e = check_program(&mut p).unwrap_err();
        assert!(e.message.contains("initium"), "got: {}", e.message);
        assert_eq!(e.file, None);
        assert_eq!(e.line, None);
    }

    #[test]
    fn initium_with_wrong_signature_is_rejected() {
        let mut with_params = pprog(
            &[("a.liss", "munus initium capit (ni n) reddit ni { redde n; }")], &[]);
        let e = check_program(&mut with_params).unwrap_err();
        assert!(e.message.contains("nihil capere"), "got: {}", e.message);
        assert_eq!(e.file.as_deref(), Some("a.liss"));

        let mut wrong_ret = pprog(
            &[("a.liss", "munus initium reddit voc { redde \"salve\"; }")], &[]);
        let e = check_program(&mut wrong_ret).unwrap_err();
        assert!(e.message.contains("reddere"), "got: {}", e.message);
    }

    #[test]
    fn duplicate_function_across_files_of_one_folder_is_rejected() {
        let mut p = pprog(
            &[("a.liss", "munus initium reddit ni { redde nullus; }")],
            &[("mathematica", &[
                ("mathematica/a.liss", "munus summa reddit ni { redde °I; }"),
                ("mathematica/b.liss", "munus summa reddit ni { redde °II; }"),
            ])],
        );
        let e = check_program(&mut p).unwrap_err();
        assert!(e.message.contains("bis definita"), "got: {}", e.message);
        assert_eq!(e.file.as_deref(), Some("mathematica/b.liss"));
    }

    #[test]
    fn importing_an_unknown_package_is_rejected() {
        let mut p = pprog(
            &[("a.liss", "affer ignota;\nmunus initium reddit ni { redde nullus; }")], &[]);
        let e = check_program(&mut p).unwrap_err();
        assert!(e.message.contains("non inventus"), "got: {}", e.message);
        assert_eq!(e.file.as_deref(), Some("a.liss"));
        assert_eq!(e.line, Some(1));
    }

    #[test]
    fn qualified_call_without_import_is_rejected() {
        let mut p = pprog(
            &[("a.liss", "munus initium reddit ni { redde mathematica.summa(°I, °II); }")],
            &[("mathematica", &[("mathematica/s.liss",
                "munus summa capit (ni a, ni b) reddit ni { redde a + b; }")])],
        );
        let e = check_program(&mut p).unwrap_err();
        assert!(e.message.contains("non allatus"), "got: {}", e.message);
    }

    #[test]
    fn unknown_function_in_a_real_package_is_rejected() {
        let mut p = pprog(
            &[("a.liss", "affer mathematica;\nmunus initium reddit ni { redde mathematica.ignota(); }")],
            &[("mathematica", &[("mathematica/s.liss",
                "munus summa capit (ni a, ni b) reddit ni { redde a + b; }")])],
        );
        let e = check_program(&mut p).unwrap_err();
        assert!(e.message.contains("in fasciculo"), "got: {}", e.message);
    }

    #[test]
    fn import_cycles_are_allowed() {
        let mut p = pprog(
            &[("a.liss", "affer par;\nmunus initium reddit ni { redde par.aequum(°IV); }")],
            &[
                ("par", &[("par/p.liss",
                    "affer impar;\nmunus aequum capit (ni n) reddit ni { si (n par_est nullus) { redde °I; } redde impar.imparum(n - °I); }")]),
                ("impar", &[("impar/i.liss",
                    "affer par;\nmunus imparum capit (ni n) reddit ni { si (n par_est nullus) { redde nullus; } redde par.aequum(n - °I); }")]),
            ],
        );
        assert!(check_program(&mut p).is_ok());
    }

    #[test]
    fn local_and_imported_functions_may_share_a_name() {
        // root's own `summa` returns voc; the package's returns ni — the call
        // sites type-check against the right one, so resolution is provably
        // by qualifier, not by accident.
        let mut p = pprog(
            &[("a.liss",
               "affer mathematica;\nmunus summa reddit voc { redde \"summa\"; }\nmunus initium reddit ni { scribe(summa()); redde mathematica.summa(°I, °II); }")],
            &[("mathematica", &[("mathematica/s.liss",
                "munus summa capit (ni a, ni b) reddit ni { redde a + b; }")])],
        );
        assert!(check_program(&mut p).is_ok());
    }

    #[test]
    fn initium_in_a_package_is_an_ordinary_function() {
        let mut p = pprog(
            &[("a.liss", "affer alia;\nmunus initium reddit ni { redde alia.initium(°V); }")],
            &[("alia", &[("alia/x.liss",
                "munus initium capit (ni n) reddit ni { redde n; }")])],
        );
        assert!(check_program(&mut p).is_ok());
    }
}
