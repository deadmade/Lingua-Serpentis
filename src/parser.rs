use crate::ast::*;
use crate::error::{LinguaError, LResult};
use crate::token::{Token, TokenKind};

struct Parser<'a> {
    toks: &'a [Token],
    pos: usize,
}

impl<'a> Parser<'a> {
    fn peek(&self) -> Option<&TokenKind> {
        self.toks.get(self.pos).map(|t| &t.kind)
    }
    fn line(&self) -> usize {
        self.toks.get(self.pos).or_else(|| self.toks.last())
            .map(|t| t.line).unwrap_or(0)
    }
    fn advance(&mut self) -> &Token {
        let t = &self.toks[self.pos];
        self.pos += 1;
        t
    }
    fn eat(&mut self, kind: &TokenKind) -> LResult<()> {
        if self.peek() == Some(kind) {
            self.pos += 1;
            Ok(())
        } else {
            Err(self.err(&format!("expectatum: {:?}", kind)))
        }
    }
    fn err(&self, msg: &str) -> LinguaError {
        LinguaError { message: msg.to_string(), line: self.line() }
    }

    // --- expressions, lowest precedence first ---
    fn expr(&mut self) -> LResult<Expr> { self.or_expr() }

    fn or_expr(&mut self) -> LResult<Expr> {
        let mut left = self.and_expr()?;
        while self.peek() == Some(&TokenKind::Or) {
            self.advance();
            let right = self.and_expr()?;
            left = Expr::Binary { op: BinOp::Or, left: Box::new(left), right: Box::new(right) };
        }
        Ok(left)
    }
    fn and_expr(&mut self) -> LResult<Expr> {
        let mut left = self.cmp_expr()?;
        while self.peek() == Some(&TokenKind::And) {
            self.advance();
            let right = self.cmp_expr()?;
            left = Expr::Binary { op: BinOp::And, left: Box::new(left), right: Box::new(right) };
        }
        Ok(left)
    }
    fn cmp_expr(&mut self) -> LResult<Expr> {
        let mut left = self.add_expr()?;
        loop {
            let op = match self.peek() {
                Some(TokenKind::Eq) => BinOp::Eq,
                Some(TokenKind::Ne) => BinOp::Ne,
                Some(TokenKind::Gt) => BinOp::Gt,
                Some(TokenKind::Lt) => BinOp::Lt,
                _ => break,
            };
            self.advance();
            let right = self.add_expr()?;
            left = Expr::Binary { op, left: Box::new(left), right: Box::new(right) };
        }
        Ok(left)
    }
    fn add_expr(&mut self) -> LResult<Expr> {
        let mut left = self.mul_expr()?;
        loop {
            let op = match self.peek() {
                Some(TokenKind::Plus) => BinOp::Add,
                Some(TokenKind::Minus) => BinOp::Sub,
                _ => break,
            };
            self.advance();
            let right = self.mul_expr()?;
            left = Expr::Binary { op, left: Box::new(left), right: Box::new(right) };
        }
        Ok(left)
    }
    fn mul_expr(&mut self) -> LResult<Expr> {
        let mut left = self.unary()?;
        loop {
            let op = match self.peek() {
                Some(TokenKind::Star) => BinOp::Mul,
                Some(TokenKind::Slash) => BinOp::Div,
                Some(TokenKind::Percent) => BinOp::Mod,
                _ => break,
            };
            self.advance();
            let right = self.unary()?;
            left = Expr::Binary { op, left: Box::new(left), right: Box::new(right) };
        }
        Ok(left)
    }
    fn unary(&mut self) -> LResult<Expr> {
        match self.peek() {
            Some(TokenKind::Not) => { self.advance(); Ok(Expr::Unary { op: UnOp::Not, expr: Box::new(self.unary()?) }) }
            Some(TokenKind::Minus) => { self.advance(); Ok(Expr::Unary { op: UnOp::Neg, expr: Box::new(self.unary()?) }) }
            _ => self.primary(),
        }
    }
    fn primary(&mut self) -> LResult<Expr> {
        match self.peek().cloned() {
            Some(TokenKind::Int(i)) => { self.advance(); Ok(Expr::Int(i)) }
            Some(TokenKind::Float(f)) => { self.advance(); Ok(Expr::Float(f)) }
            Some(TokenKind::Text(s)) => { self.advance(); Ok(Expr::Text(s)) }
            Some(TokenKind::True) => { self.advance(); Ok(Expr::Bool(true)) }
            Some(TokenKind::False) => { self.advance(); Ok(Expr::Bool(false)) }
            Some(TokenKind::LParen) => {
                self.advance();
                let e = self.expr()?;
                self.eat(&TokenKind::RParen)?;
                Ok(e)
            }
            Some(TokenKind::Ident(name)) => {
                self.advance();
                if self.peek() == Some(&TokenKind::LParen) {
                    self.advance();
                    let mut args = Vec::new();
                    if self.peek() != Some(&TokenKind::RParen) {
                        args.push(self.expr()?);
                        while self.peek() == Some(&TokenKind::Comma) {
                            self.advance();
                            args.push(self.expr()?);
                        }
                    }
                    self.eat(&TokenKind::RParen)?;
                    Ok(Expr::Call { name, args })
                } else {
                    Ok(Expr::Ident(name))
                }
            }
            _ => Err(self.err("expressio expectata")),
        }
    }

    fn type_keyword(&mut self) -> Option<Type> {
        let ty = match self.peek()? {
            TokenKind::TyInt => Type::Int,
            TokenKind::TyFloat => Type::Float,
            TokenKind::TyText => Type::Text,
            TokenKind::TyChar => Type::Char,
            TokenKind::TyBool => Type::Bool,
            _ => return None,
        };
        self.advance();
        Some(ty)
    }

    fn ident(&mut self) -> LResult<String> {
        match self.peek().cloned() {
            Some(TokenKind::Ident(n)) => { self.advance(); Ok(n) }
            _ => Err(self.err("nomen expectatum")),
        }
    }

    fn statement(&mut self) -> LResult<Stmt> {
        let line = self.line();
        // declaration: <type> <ident> = <expr> ;
        if let Some(ty) = self.type_keyword() {
            let name = self.ident()?;
            self.eat(&TokenKind::Assign)?;
            let value = self.expr()?;
            self.eat(&TokenKind::Semicolon)?;
            return Ok(Stmt { kind: StmtKind::Let { ty, name, value }, line });
        }
        match self.peek() {
            Some(TokenKind::Print) => {
                self.advance();
                self.eat(&TokenKind::LParen)?;
                let e = self.expr()?;
                self.eat(&TokenKind::RParen)?;
                self.eat(&TokenKind::Semicolon)?;
                Ok(Stmt { kind: StmtKind::Print(e), line })
            }
            // assignment: <ident> = <expr> ;   (lookahead distinguishes from a call)
            Some(TokenKind::Ident(_))
                if self.toks.get(self.pos + 1).map(|t| &t.kind) == Some(&TokenKind::Assign) =>
            {
                let name = self.ident()?;
                self.eat(&TokenKind::Assign)?;
                let value = self.expr()?;
                self.eat(&TokenKind::Semicolon)?;
                Ok(Stmt { kind: StmtKind::Assign { name, value }, line })
            }
            _ => {
                let e = self.expr()?;
                self.eat(&TokenKind::Semicolon)?;
                Ok(Stmt { kind: StmtKind::ExprStmt(e), line })
            }
        }
    }
}

pub fn parse(tokens: &[Token]) -> LResult<Vec<Stmt>> {
    let mut p = Parser { toks: tokens, pos: 0 };
    let mut stmts = Vec::new();
    while p.pos < p.toks.len() {
        stmts.push(p.statement()?);
    }
    Ok(stmts)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::tokenize;

    fn parse_first_expr(src: &str) -> Expr {
        let toks = tokenize(src).unwrap();
        let stmts = parse(&toks).unwrap();
        match &stmts[0].kind {
            StmtKind::ExprStmt(e) => e.clone(),
            other => panic!("expected ExprStmt, got {:?}", other),
        }
    }

    fn parse_all(src: &str) -> Vec<Stmt> {
        parse(&tokenize(src).unwrap()).unwrap()
    }

    #[test]
    fn declaration_statement() {
        let s = parse_all("ni x = °V;");
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].kind, StmtKind::Let {
            ty: Type::Int, name: "x".into(), value: Expr::Int(5),
        });
    }

    #[test]
    fn assignment_and_print() {
        let s = parse_all("x = °I; scribe(x);");
        assert_eq!(s[0].kind, StmtKind::Assign { name: "x".into(), value: Expr::Int(1) });
        assert_eq!(s[1].kind, StmtKind::Print(Expr::Ident("x".into())));
    }

    #[test]
    fn arithmetic_precedence() {
        // °II + °III * °IV  ==  Add(2, Mul(3, 4))
        let e = parse_first_expr("°II + °III * °IV;");
        assert_eq!(e, Expr::Binary {
            op: BinOp::Add,
            left: Box::new(Expr::Int(2)),
            right: Box::new(Expr::Binary {
                op: BinOp::Mul,
                left: Box::new(Expr::Int(3)),
                right: Box::new(Expr::Int(4)),
            }),
        });
    }

    #[test]
    fn comparison_and_call() {
        let e = parse_first_expr("summa maior_est adde(a, °I);");
        match e {
            Expr::Binary { op: BinOp::Gt, right, .. } => match *right {
                Expr::Call { name, args } => {
                    assert_eq!(name, "adde");
                    assert_eq!(args.len(), 2);
                }
                other => panic!("expected call, got {:?}", other),
            },
            other => panic!("expected Gt, got {:?}", other),
        }
    }

    #[test]
    fn boolean_and_binds_tighter_than_or() {
        // a et b vel c  ==  Or(And(a,b), c)
        let e = parse_first_expr("a et b vel c;");
        assert_eq!(e, Expr::Binary {
            op: BinOp::Or,
            left: Box::new(Expr::Binary {
                op: BinOp::And,
                left: Box::new(Expr::Ident("a".into())),
                right: Box::new(Expr::Ident("b".into())),
            }),
            right: Box::new(Expr::Ident("c".into())),
        });
    }

    #[test]
    fn not_binds_tighter_than_and() {
        // non a et b  ==  And(Not(a), b)
        let e = parse_first_expr("non a et b;");
        assert_eq!(e, Expr::Binary {
            op: BinOp::And,
            left: Box::new(Expr::Unary { op: UnOp::Not, expr: Box::new(Expr::Ident("a".into())) }),
            right: Box::new(Expr::Ident("b".into())),
        });
    }

    #[test]
    fn subtraction_is_left_associative() {
        // °X - °III - °I  ==  Sub(Sub(10,3),1)
        let e = parse_first_expr("°X - °III - °I;");
        assert_eq!(e, Expr::Binary {
            op: BinOp::Sub,
            left: Box::new(Expr::Binary {
                op: BinOp::Sub,
                left: Box::new(Expr::Int(10)),
                right: Box::new(Expr::Int(3)),
            }),
            right: Box::new(Expr::Int(1)),
        });
    }

    #[test]
    fn zero_arg_call() {
        let e = parse_first_expr("adde();");
        assert_eq!(e, Expr::Call { name: "adde".into(), args: vec![] });
    }
}
