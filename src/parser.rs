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

    fn block(&mut self) -> LResult<Vec<Stmt>> {
        self.eat(&TokenKind::LBrace)?;
        let mut stmts = Vec::new();
        while self.peek() != Some(&TokenKind::RBrace) {
            if self.peek().is_none() {
                return Err(self.err("'}' expectatum"));
            }
            stmts.push(self.statement()?);
        }
        self.eat(&TokenKind::RBrace)?;
        Ok(stmts)
    }

    fn if_stmt(&mut self) -> LResult<Stmt> {
        let line = self.line();
        self.advance(); // si
        self.eat(&TokenKind::LParen)?;
        let cond = self.expr()?;
        self.eat(&TokenKind::RParen)?;
        let then = self.block()?;
        let else_ = if self.peek() == Some(&TokenKind::Else) {
            self.advance(); // aut
            if self.peek() == Some(&TokenKind::If) {
                Some(vec![self.if_stmt()?]) // "aut si" → nested if
            } else {
                Some(self.block()?)
            }
        } else {
            None
        };
        Ok(Stmt { kind: StmtKind::If { cond, then, else_ }, line })
    }

    fn while_stmt(&mut self) -> LResult<Stmt> {
        let line = self.line();
        self.advance(); // indem
        self.eat(&TokenKind::LParen)?;
        let cond = self.expr()?;
        self.eat(&TokenKind::RParen)?;
        let body = self.block()?;
        Ok(Stmt { kind: StmtKind::While { cond, body }, line })
    }

    fn for_stmt(&mut self) -> LResult<Stmt> {
        let line = self.line();
        self.advance(); // dum
        self.eat(&TokenKind::LParen)?;
        let init = Box::new(self.statement()?);      // consumes its own ';'
        let cond = self.expr()?;
        self.eat(&TokenKind::Semicolon)?;
        let step = Box::new(self.assign_no_semi()?); // step has no trailing ';'
        self.eat(&TokenKind::RParen)?;
        let body = self.block()?;
        Ok(Stmt { kind: StmtKind::For { init, cond, step, body }, line })
    }

    // for-loop step: <ident> = <expr>  (no trailing semicolon)
    fn assign_no_semi(&mut self) -> LResult<Stmt> {
        let line = self.line();
        let name = self.ident()?;
        self.eat(&TokenKind::Assign)?;
        let value = self.expr()?;
        Ok(Stmt { kind: StmtKind::Assign { name, value }, line })
    }

    fn func_stmt(&mut self) -> LResult<Stmt> {
        let line = self.line();
        self.advance(); // munus
        let ret = self.type_keyword().ok_or_else(|| self.err("typus reditus expectatus"))?;
        let name = self.ident()?;
        self.eat(&TokenKind::LParen)?;
        let mut params = Vec::new();
        if self.peek() != Some(&TokenKind::RParen) {
            loop {
                let pty = self.type_keyword().ok_or_else(|| self.err("typus parametri expectatus"))?;
                let pname = self.ident()?;
                params.push((pty, pname));
                if self.peek() == Some(&TokenKind::Comma) { self.advance(); } else { break; }
            }
        }
        self.eat(&TokenKind::RParen)?;
        let body = self.block()?;
        Ok(Stmt { kind: StmtKind::Func(FuncDecl { name, params, ret, body }), line })
    }

    fn statement(&mut self) -> LResult<Stmt> {
        let line = self.line();
        match self.peek() {
            Some(TokenKind::If) => return self.if_stmt(),
            Some(TokenKind::While) => return self.while_stmt(),
            Some(TokenKind::For) => return self.for_stmt(),
            Some(TokenKind::Func) => return self.func_stmt(),
            Some(TokenKind::Return) => {
                self.advance();
                let e = self.expr()?;
                self.eat(&TokenKind::Semicolon)?;
                return Ok(Stmt { kind: StmtKind::Return(e), line });
            }
            Some(TokenKind::Break) => {
                self.advance();
                self.eat(&TokenKind::Semicolon)?;
                return Ok(Stmt { kind: StmtKind::Break, line });
            }
            Some(TokenKind::Continue) => {
                self.advance();
                self.eat(&TokenKind::Semicolon)?;
                return Ok(Stmt { kind: StmtKind::Continue, line });
            }
            _ => {}
        }
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

    #[test]
    fn if_else_if_else() {
        let s = parse_all("si (x) { scribe(°I); } aut si (y) { scribe(°II); } aut { scribe(°III); }");
        match &s[0].kind {
            StmtKind::If { then, else_, .. } => {
                assert_eq!(then.len(), 1);
                assert!(else_.is_some());
                // else branch is a single nested If (the "aut si")
                match &else_.as_ref().unwrap()[0].kind {
                    StmtKind::If { else_: inner, .. } => assert!(inner.is_some()),
                    other => panic!("expected nested if, got {:?}", other),
                }
            }
            other => panic!("expected if, got {:?}", other),
        }
    }

    #[test]
    fn while_for_break_continue() {
        let s = parse_all("indem (x) { abrumpe; }");
        assert!(matches!(s[0].kind, StmtKind::While { .. }));
        let f = parse_all("dum (ni i = °I; i minor_est °X; i = i + °I) { continua; }");
        assert!(matches!(f[0].kind, StmtKind::For { .. }));
    }

    #[test]
    fn function_declaration_and_return() {
        let s = parse_all("munus ni adde(ni a, ni b) { redde a + b; }");
        match &s[0].kind {
            StmtKind::Func(f) => {
                assert_eq!(f.name, "adde");
                assert_eq!(f.ret, Type::Int);
                assert_eq!(f.params, vec![(Type::Int, "a".into()), (Type::Int, "b".into())]);
                assert!(matches!(f.body[0].kind, StmtKind::Return(_)));
            }
            other => panic!("expected func, got {:?}", other),
        }
    }
}
