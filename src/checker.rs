use crate::ast::*;
use crate::error::{LinguaError, LResult};
use std::collections::HashMap;

struct Checker {
    scopes: Vec<HashMap<String, Type>>,
    funcs: HashMap<String, (Vec<Type>, Type)>, // name -> (param types, return type)
}

impl Checker {
    fn new() -> Self {
        Checker { scopes: vec![HashMap::new()], funcs: HashMap::new() }
    }

    fn lookup(&self, name: &str) -> Option<Type> {
        self.scopes.iter().rev().find_map(|s| s.get(name).copied())
    }
    fn declare(&mut self, name: String, ty: Type) {
        self.scopes.last_mut().unwrap().insert(name, ty);
    }

    fn type_name(t: Type) -> &'static str {
        match t {
            Type::Int => "ni", Type::Float => "mnr",
            Type::Text => "voc", Type::Char => "lit", Type::Bool => "veritas",
        }
    }

    fn check_expr(&self, e: &Expr, line: usize) -> LResult<Type> {
        match e {
            Expr::Int(_) => Ok(Type::Int),
            Expr::Float(_) => Ok(Type::Float),
            Expr::Text(_) => Ok(Type::Text),
            Expr::Bool(_) => Ok(Type::Bool),
            Expr::Ident(name) => self.lookup(name)
                .ok_or_else(|| LinguaError { message: format!("variabilis ignota: '{}'", name), line }),
            Expr::Unary { op, expr } => {
                let t = self.check_expr(expr, line)?;
                match op {
                    UnOp::Not if t == Type::Bool => Ok(Type::Bool),
                    UnOp::Neg if t == Type::Int || t == Type::Float => Ok(t),
                    _ => Err(LinguaError { message: "operatio unaria typo non convenit".into(), line }),
                }
            }
            Expr::Binary { op, left, right } => {
                let lt = self.check_expr(left, line)?;
                let rt = self.check_expr(right, line)?;
                self.check_binary(*op, lt, rt, line)
            }
            Expr::Call { name, args } => {
                let (params, ret) = self.funcs.get(name)
                    .cloned()
                    .ok_or_else(|| LinguaError { message: format!("functio ignota: '{}'", name), line })?;
                if params.len() != args.len() {
                    return Err(LinguaError { message: format!("functio '{}' numerum argumentorum falsum accepit", name), line });
                }
                for (want, arg) in params.iter().zip(args) {
                    let got = self.check_expr(arg, line)?;
                    if got != *want {
                        return Err(LinguaError {
                            message: format!("argumentum typum '{}' expectat, sed '{}' datum est",
                                Self::type_name(*want), Self::type_name(got)),
                            line,
                        });
                    }
                }
                Ok(ret)
            }
        }
    }

    fn check_binary(&self, op: BinOp, lt: Type, rt: Type, line: usize) -> LResult<Type> {
        let mismatch = || LinguaError {
            message: format!("operatio typis '{}' et '{}' non convenit",
                Self::type_name(lt), Self::type_name(rt)),
            line,
        };
        match op {
            BinOp::Add => match (lt, rt) {
                (Type::Int, Type::Int) => Ok(Type::Int),
                (Type::Float, Type::Float) => Ok(Type::Float),
                (Type::Text, Type::Text) => Ok(Type::Text),
                _ => Err(mismatch()),
            },
            BinOp::Sub | BinOp::Mul | BinOp::Div => match (lt, rt) {
                (Type::Int, Type::Int) => Ok(Type::Int),
                (Type::Float, Type::Float) => Ok(Type::Float),
                _ => Err(mismatch()),
            },
            BinOp::Mod => match (lt, rt) {
                (Type::Int, Type::Int) => Ok(Type::Int),
                _ => Err(mismatch()),
            },
            BinOp::Eq | BinOp::Ne | BinOp::Gt | BinOp::Lt => {
                if lt == rt { Ok(Type::Bool) } else { Err(mismatch()) }
            }
            BinOp::And | BinOp::Or => match (lt, rt) {
                (Type::Bool, Type::Bool) => Ok(Type::Bool),
                _ => Err(mismatch()),
            },
        }
    }

    fn check_stmt(&mut self, s: &Stmt) -> LResult<()> {
        match &s.kind {
            StmtKind::Let { ty, name, value } => {
                let vt = self.check_expr(value, s.line)?;
                if vt != *ty {
                    return Err(LinguaError {
                        message: format!("'{}' typum '{}' expectat, sed '{}' datum est",
                            name, Self::type_name(*ty), Self::type_name(vt)),
                        line: s.line,
                    });
                }
                self.declare(name.clone(), *ty);
                Ok(())
            }
            StmtKind::Assign { name, value } => {
                let target = self.lookup(name)
                    .ok_or_else(|| LinguaError { message: format!("variabilis ignota: '{}'", name), line: s.line })?;
                let vt = self.check_expr(value, s.line)?;
                if vt != target {
                    return Err(LinguaError {
                        message: format!("'{}' typum '{}' expectat, sed '{}' datum est",
                            name, Self::type_name(target), Self::type_name(vt)),
                        line: s.line,
                    });
                }
                Ok(())
            }
            StmtKind::Print(e) | StmtKind::ExprStmt(e) => {
                self.check_expr(e, s.line)?;
                Ok(())
            }
            _ => Err(LinguaError { message: "sententia nondum tractata".into(), line: s.line }),
        }
    }
}

pub fn check(program: &[Stmt]) -> LResult<()> {
    let mut c = Checker::new();
    for s in program {
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
        check(&parse(&tokenize(src).unwrap()).unwrap())
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
}
