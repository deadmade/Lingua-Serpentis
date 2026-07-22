use crate::ast::*;
use crate::error::{LinguaError, LResult};
use std::collections::HashMap;

/// Synthesized type of an expression during checking. `FloatLit` is the
/// not-yet-resolved type of a fractional literal — and of arithmetic that
/// combines only such literals; every other expression is `Known`.
#[derive(Clone, Copy, PartialEq)]
enum InferTy {
    Known(Type),
    FloatLit,
}

struct Checker {
    scopes: Vec<HashMap<String, Type>>,
    funcs: HashMap<String, (Vec<Type>, Type)>, // name -> (param types, return type)
    current_ret: Option<Type>, // Some(_) while checking a function body
}

impl Checker {
    fn new() -> Self {
        Checker { scopes: vec![HashMap::new()], funcs: HashMap::new(), current_ret: None }
    }

    fn lookup(&self, name: &str) -> Option<Type> {
        self.scopes.iter().rev().find_map(|s| s.get(name).copied())
    }
    fn declare(&mut self, name: String, ty: Type) {
        self.scopes.last_mut().unwrap().insert(name, ty);
    }

    fn type_name(t: Type) -> &'static str {
        match t {
            Type::Int => "ni", Type::Float => "mnr", Type::Double => "nr",
            Type::Text => "voc", Type::Char => "lit", Type::Bool => "veritas",
        }
    }

    /// Name used for an `InferTy` in error messages; a bare fractional literal
    /// reports as `nr`, its default type.
    fn infer_name(t: InferTy) -> &'static str {
        match t {
            InferTy::Known(ty) => Self::type_name(ty),
            InferTy::FloatLit => "nr",
        }
    }

    fn check_expr(&self, e: &mut Expr, line: usize) -> LResult<InferTy> {
        match e {
            Expr::Int(_) => Ok(InferTy::Known(Type::Int)),
            Expr::FloatLit(_) => Ok(InferTy::FloatLit),
            Expr::Float(_) => Ok(InferTy::Known(Type::Float)),
            Expr::Double(_) => Ok(InferTy::Known(Type::Double)),
            Expr::Text(_) => Ok(InferTy::Known(Type::Text)),
            Expr::Char(_) => Ok(InferTy::Known(Type::Char)),
            Expr::Bool(_) => Ok(InferTy::Known(Type::Bool)),
            Expr::Ident(name) => self.lookup(name)
                .map(InferTy::Known)
                .ok_or_else(|| LinguaError { message: format!("variabilis ignota: '{}'", name), line }),
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
            Expr::Call { name, args } => {
                let (params, ret) = self.funcs.get(name)
                    .cloned()
                    .ok_or_else(|| LinguaError { message: format!("functio ignota: '{}'", name), line })?;
                if params.len() != args.len() {
                    return Err(LinguaError { message: format!("functio '{}' numerum argumentorum falsum accepit", name), line });
                }
                for (want, arg) in params.iter().zip(args.iter_mut()) {
                    let got = self.resolve_expr(arg, Some(*want), line)?;
                    if got != *want {
                        return Err(LinguaError {
                            message: format!("argumentum typum '{}' expectat, sed '{}' datum est",
                                Self::type_name(*want), Self::type_name(got)),
                            line,
                        });
                    }
                }
                Ok(InferTy::Known(ret))
            }
        }
    }

    /// Synthesize `e`'s type, resolving a bare fractional literal to a concrete
    /// float type. `expected` is the type the context wants (`None` for an
    /// untyped sink such as `scribe`). A `FloatLit` becomes the expected float
    /// type; with no float context it defaults to `nr`; demanded as a non-float
    /// type it is an error.
    fn resolve_expr(&self, e: &mut Expr, expected: Option<Type>, line: usize) -> LResult<Type> {
        match self.check_expr(e, line)? {
            InferTy::Known(t) => Ok(t),
            InferTy::FloatLit => {
                let ty = match expected {
                    Some(Type::Float) => Type::Float,
                    Some(Type::Double) => Type::Double,
                    Some(other) => return Err(LinguaError {
                        message: format!("numerus fractus typum 'mnr' vel 'nr' requirit, non '{}'",
                            Self::type_name(other)),
                        line,
                    }),
                    None => Type::Double,
                };
                resolve_floatlit(e, ty);
                Ok(ty)
            }
        }
    }

    fn check_binary(&self, op: BinOp, lt: InferTy, rt: InferTy,
                    left: &mut Expr, right: &mut Expr, line: usize) -> LResult<InferTy> {
        use InferTy::*;
        let mismatch = || LinguaError {
            message: format!("operatio typis '{}' et '{}' non convenit",
                Self::infer_name(lt), Self::infer_name(rt)),
            line,
        };
        match op {
            BinOp::Add => match (lt, rt) {
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
            BinOp::Sub | BinOp::Mul | BinOp::Div => match (lt, rt) {
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
            BinOp::Mod => match (lt, rt) {
                (Known(Type::Int), Known(Type::Int)) => Ok(Known(Type::Int)),
                _ => Err(mismatch()),
            },
            BinOp::Eq | BinOp::Ne | BinOp::Gt | BinOp::Lt => match (lt, rt) {
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
            BinOp::And | BinOp::Or => match (lt, rt) {
                (Known(Type::Bool), Known(Type::Bool)) => Ok(Known(Type::Bool)),
                _ => Err(mismatch()),
            },
        }
    }

    fn check_stmt(&mut self, s: &mut Stmt) -> LResult<()> {
        let line = s.line;
        match &mut s.kind {
            StmtKind::Let { ty, name, value } => {
                let vt = self.resolve_expr(value, Some(*ty), line)?;
                if vt != *ty {
                    return Err(LinguaError {
                        message: format!("'{}' typum '{}' expectat, sed '{}' datum est",
                            name, Self::type_name(*ty), Self::type_name(vt)),
                        line,
                    });
                }
                self.declare(name.clone(), *ty);
                Ok(())
            }
            StmtKind::Assign { name, value } => {
                let target = self.lookup(name)
                    .ok_or_else(|| LinguaError { message: format!("variabilis ignota: '{}'", name), line })?;
                let vt = self.resolve_expr(value, Some(target), line)?;
                if vt != target {
                    return Err(LinguaError {
                        message: format!("'{}' typum '{}' expectat, sed '{}' datum est",
                            name, Self::type_name(target), Self::type_name(vt)),
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
                self.check_block(body)
            }
            StmtKind::For { init, cond, step, body } => {
                self.scopes.push(HashMap::new());
                let r = (|| {
                    self.check_stmt(init)?;
                    self.expect_bool(cond, line)?;
                    self.check_stmt(step)?;
                    self.check_block(body)
                })();
                self.scopes.pop();
                r
            }
            StmtKind::Func(f) => {
                self.scopes.push(HashMap::new());
                for (t, n) in &f.params { self.declare(n.clone(), *t); }
                let saved = self.current_ret;
                self.current_ret = Some(f.ret);
                let r = self.check_block_inner(&mut f.body);
                self.current_ret = saved;
                self.scopes.pop();
                r
            }
            StmtKind::Return(e) => {
                let want = self.current_ret
                    .ok_or_else(|| LinguaError { message: "'redde' extra functionem".into(), line })?;
                let got = self.resolve_expr(e, Some(want), line)?;
                if got != want {
                    return Err(LinguaError {
                        message: format!("reditus typum '{}' expectat, sed '{}' datum est",
                            Self::type_name(want), Self::type_name(got)),
                        line,
                    });
                }
                Ok(())
            }
            StmtKind::Break | StmtKind::Continue => Ok(()),
        }
    }

    fn check_block(&mut self, body: &mut [Stmt]) -> LResult<()> {
        self.scopes.push(HashMap::new());
        for s in body.iter_mut() {
            if let StmtKind::Func(f) = &s.kind {
                self.scopes.pop();
                return Err(LinguaError {
                    message: format!("functio '{}' intra corpus definiri non potest; functiones nidificatae non sustinentur", f.name),
                    line: s.line,
                });
            }
            if let Err(e) = self.check_stmt(s) {
                self.scopes.pop();
                return Err(e);
            }
        }
        self.scopes.pop();
        Ok(())
    }

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
/// and arithmetic into a concrete-precision node. Narrows to `f32` for `mnr`.
/// Only ever called on an expression whose synthesized type was `FloatLit`, so
/// there are no concrete float leaves to clobber.
fn resolve_floatlit(e: &mut Expr, ty: Type) {
    match e {
        Expr::FloatLit(f) => {
            *e = match ty {
                Type::Float => Expr::Float(*f as f32),
                Type::Double => Expr::Double(*f),
                _ => return,
            };
        }
        Expr::Unary { expr, .. } => resolve_floatlit(expr, ty),
        Expr::Binary { left, right, .. } => {
            resolve_floatlit(left, ty);
            resolve_floatlit(right, ty);
        }
        _ => {}
    }
}

pub fn check(program: &mut [Stmt]) -> LResult<()> {
    let mut c = Checker::new();
    for s in program.iter() {
        if let StmtKind::Func(f) = &s.kind {
            let ptypes = f.params.iter().map(|(t, _)| *t).collect();
            c.funcs.insert(f.name.clone(), (ptypes, f.ret));
        }
    }
    for s in program.iter_mut() {
        c.check_stmt(s)?;
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
        assert!(check_src("munus adde capit (ni a, ni b) reddit ni { redde a + b; } scribe(adde(°I, °II));").is_ok());
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
}
