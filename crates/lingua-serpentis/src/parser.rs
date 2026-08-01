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
            _ => self.postfix(),
        }
    }
    fn postfix(&mut self) -> LResult<Expr> {
        let mut e = self.primary()?;
        while self.peek() == Some(&TokenKind::LBracket) {
            self.advance(); // [
            let index = self.expr()?;
            self.eat(&TokenKind::RBracket)?;
            e = Expr::Index { base: Box::new(e), index: Box::new(index) };
        }
        Ok(e)
    }
    // argument list: consumes '(' … ')'
    fn call_args(&mut self) -> LResult<Vec<Expr>> {
        self.eat(&TokenKind::LParen)?;
        let mut args = Vec::new();
        if self.peek() != Some(&TokenKind::RParen) {
            args.push(self.expr()?);
            while self.peek() == Some(&TokenKind::Comma) {
                self.advance();
                args.push(self.expr()?);
            }
        }
        self.eat(&TokenKind::RParen)?;
        Ok(args)
    }

    fn primary(&mut self) -> LResult<Expr> {
        match self.peek().cloned() {
            Some(TokenKind::Int(i)) => { self.advance(); Ok(Expr::Int(i)) }
            Some(TokenKind::Frac { numer, den }) => { self.advance(); Ok(Expr::FloatLit { numer, den }) }
            Some(TokenKind::Text(s)) => { self.advance(); Ok(Expr::Text(s)) }
            Some(TokenKind::Char(c)) => { self.advance(); Ok(Expr::Char(c)) }
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
                if self.peek() == Some(&TokenKind::Dot) {
                    // qualified call: the only thing a package name can do
                    self.advance(); // .
                    let fname = self.ident()?;
                    if self.peek() != Some(&TokenKind::LParen) {
                        return Err(self.err("vocatio functionis post nomen fasciculi expectata"));
                    }
                    let args = self.call_args()?;
                    return Ok(Expr::Call { package: Some(name), name: fname, args });
                }
                if self.peek() == Some(&TokenKind::LParen) {
                    let args = self.call_args()?;
                    Ok(Expr::Call { package: None, name, args })
                } else {
                    Ok(Expr::Ident(name))
                }
            }
            Some(TokenKind::LBracket) => {
                self.advance(); // [
                let mut elems = Vec::new();
                if self.peek() != Some(&TokenKind::RBracket) {
                    elems.push(self.expr()?);
                    while self.peek() == Some(&TokenKind::Comma) {
                        self.advance();
                        elems.push(self.expr()?);
                    }
                }
                self.eat(&TokenKind::RBracket)?;
                Ok(Expr::ListLit(elems))
            }
            _ => Err(self.err("expressio expectata")),
        }
    }

    fn type_keyword(&mut self) -> Option<Type> {
        let mut ty = match self.peek()? {
            TokenKind::TyInt => Type::Int,
            TokenKind::TyFloat => Type::Float,
            TokenKind::TyDouble => Type::Double,
            TokenKind::TyText => Type::Text,
            TokenKind::TyChar => Type::Char,
            TokenKind::TyBool => Type::Bool,
            _ => return None,
        };
        self.advance();
        while self.peek() == Some(&TokenKind::Lista) {
            self.advance();
            ty = Type::List(Box::new(ty));
        }
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

        // Old inline form `munus ni nomen(...)`: a type keyword sits where the
        // name belongs. Caught here so it gets real advice, not `nomen expectatum`.
        if matches!(
            self.peek(),
            Some(TokenKind::TyInt | TokenKind::TyFloat | TokenKind::TyDouble
                 | TokenKind::TyText | TokenKind::TyChar | TokenKind::TyBool)
        ) {
            return Err(self.err("typus reditus post 'reddit' ponendus, non ante nomen"));
        }

        let name = self.ident()?;

        // `capit (...)` introduces a non-empty parameter clause; its absence
        // means the function takes nothing. `def_line` marks where the
        // parameters are written, which for a header split across lines differs
        // from the `munus` line.
        let def_line = self.line();
        let mut params: Vec<(Type, String)> = Vec::new();
        if self.peek() == Some(&TokenKind::Takes) {
            self.advance(); // capit
            self.eat(&TokenKind::LParen)?;
            if self.peek() == Some(&TokenKind::RParen) {
                return Err(self.err("munus sine parametris 'capit' omittat"));
            }
            loop {
                let ty = self.type_keyword()
                    .ok_or_else(|| self.err("typus parametri expectatus"))?;
                let pname = self.ident()?;
                params.push((ty, pname));
                if self.peek() == Some(&TokenKind::Comma) { self.advance(); } else { break; }
            }
            self.eat(&TokenKind::RParen)?;
        } else if self.peek() == Some(&TokenKind::LParen) {
            return Err(self.err("'capit' ante parametra expectatum"));
        }

        // `reddit <type>` — always present, even for an `ni` return.
        if self.peek() != Some(&TokenKind::Yields) {
            return Err(self.err("'reddit' et typus reditus expectati"));
        }
        self.advance(); // reddit
        let ret = self.type_keyword()
            .ok_or_else(|| self.err("typus post 'reddit' expectatus"))?;

        let body = self.block()?;
        Ok(Stmt {
            kind: StmtKind::Func(FuncDecl { name, params, ret, body, def_line }),
            line,
        })
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
                // index assignment: <ident>[<expr>] = <expr> ;
                if matches!(e, Expr::Index { .. }) && self.peek() == Some(&TokenKind::Assign) {
                    let (base, index) = match e {
                        Expr::Index { base, index } => (base, index),
                        _ => unreachable!(),
                    };
                    let name = match *base {
                        Expr::Ident(n) => n,
                        _ => return Err(self.err("sinistra subscriptionis variabilis esse debet")),
                    };
                    self.advance(); // =
                    let value = self.expr()?;
                    self.eat(&TokenKind::Semicolon)?;
                    return Ok(Stmt { kind: StmtKind::IndexAssign { name, index: *index, value }, line });
                }
                // A call followed by `{` is a function definition missing its
                // `munus … reddit` header — the only way that shape arises.
                if matches!(e, Expr::Call { .. }) && self.peek() == Some(&TokenKind::LBrace) {
                    return Err(self.err("munus ante definitionem expectatum"));
                }
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

/// Parse one file under the package model (§4.4–4.5 of the design):
/// `affer` lines first, then `munus` declarations, nothing else.
pub fn parse_file(tokens: &[Token]) -> LResult<FileAst> {
    let mut p = Parser { toks: tokens, pos: 0 };
    let mut imports: Vec<FileImport> = Vec::new();
    while p.peek() == Some(&TokenKind::Import) {
        let line = p.line();
        p.advance(); // affer
        let package = p.ident()?;
        p.eat(&TokenKind::Semicolon)?;
        if imports.iter().any(|i| i.package == package) {
            return Err(LinguaError {
                message: format!("fasciculus '{}' bis allatus", package),
                line,
            });
        }
        imports.push(FileImport { package, line });
    }
    let mut funcs = Vec::new();
    while p.pos < p.toks.len() {
        match p.peek() {
            Some(TokenKind::Func) => funcs.push(p.func_stmt()?),
            Some(TokenKind::Import) => {
                return Err(p.err("'affer' ante omnia munera ponendum"));
            }
            _ => return Err(p.err("extra munera nihil nisi declarationes licet")),
        }
    }
    Ok(FileAst { imports, funcs })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::tokenize;

    fn ri(n: i64) -> crate::roman::Roman { crate::roman::Roman::from_i64(n) }

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

    fn parse_err(src: &str) -> String {
        parse(&tokenize(src).unwrap())
            .expect_err("expected a parse error")
            .message
    }

    #[test]
    fn declaration_statement() {
        let s = parse_all("ni x = °V;");
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].kind, StmtKind::Let {
            ty: Type::Int, name: "x".into(), value: Expr::Int(ri(5)),
        });
    }

    #[test]
    fn assignment_and_print() {
        let s = parse_all("x = °I; scribe(x);");
        assert_eq!(s[0].kind, StmtKind::Assign { name: "x".into(), value: Expr::Int(ri(1)) });
        assert_eq!(s[1].kind, StmtKind::Print(Expr::Ident("x".into())));
    }

    #[test]
    fn arithmetic_precedence() {
        // °II + °III * °IV  ==  Add(2, Mul(3, 4))
        let e = parse_first_expr("°II + °III * °IV;");
        assert_eq!(e, Expr::Binary {
            op: BinOp::Add,
            left: Box::new(Expr::Int(ri(2))),
            right: Box::new(Expr::Binary {
                op: BinOp::Mul,
                left: Box::new(Expr::Int(ri(3))),
                right: Box::new(Expr::Int(ri(4))),
            }),
        });
    }

    #[test]
    fn comparison_and_call() {
        let e = parse_first_expr("summa maior_est adde(a, °I);");
        match e {
            Expr::Binary { op: BinOp::Gt, right, .. } => match *right {
                Expr::Call { name, args, .. } => {
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
                left: Box::new(Expr::Int(ri(10))),
                right: Box::new(Expr::Int(ri(3))),
            }),
            right: Box::new(Expr::Int(ri(1))),
        });
    }

    #[test]
    fn zero_arg_call() {
        let e = parse_first_expr("adde();");
        assert_eq!(e, Expr::Call { package: None, name: "adde".into(), args: vec![] });
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
        let s = parse_all("munus adde capit (ni a, ni b) reddit ni { redde a + b; }");
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

    #[test]
    fn zero_argument_function() {
        let s = parse_all("munus duo reddit ni { redde °II; }");
        match &s[0].kind {
            StmtKind::Func(f) => {
                assert_eq!(f.name, "duo");
                assert_eq!(f.ret, Type::Int);
                assert!(f.params.is_empty());
            }
            other => panic!("expected func, got {:?}", other),
        }
    }

    #[test]
    fn mixed_parameter_types() {
        let s = parse_all("munus test capit (voc v, ni n) reddit veritas { redde verus; }");
        match &s[0].kind {
            StmtKind::Func(f) => {
                assert_eq!(f.ret, Type::Bool);
                assert_eq!(f.params, vec![(Type::Text, "v".into()), (Type::Int, "n".into())]);
            }
            other => panic!("expected func, got {:?}", other),
        }
    }

    #[test]
    fn empty_capit_is_rejected() {
        // With no parameters, `capit ()` must be omitted entirely.
        let m = parse_err("munus f capit () reddit ni { redde °I; }");
        assert!(m.contains("sine parametris"), "got: {m}");
    }

    #[test]
    fn parameters_without_capit_are_rejected() {
        let m = parse_err("munus f(ni a) reddit ni { redde a; }");
        assert!(m.contains("ante parametra"), "got: {m}");
    }

    #[test]
    fn missing_reddit_is_rejected() {
        let m = parse_err("munus f capit (ni a) { redde a; }");
        assert!(m.contains("typus reditus"), "got: {m}");
    }

    #[test]
    fn missing_return_type_is_rejected() {
        let m = parse_err("munus f reddit { redde °I; }");
        assert!(m.contains("post 'reddit'"), "got: {m}");
    }

    #[test]
    fn parameter_without_a_type_is_rejected() {
        let m = parse_err("munus f capit (a) reddit ni { redde °I; }");
        assert!(m.contains("typus parametri"), "got: {m}");
    }

    // Load-bearing: the old syntaxes were removed deliberately. Without this,
    // a future change could quietly restore a dual-syntax parser.
    #[test]
    fn old_inline_form_is_rejected() {
        let m = parse_err("munus ni adde(ni a, ni b) { redde a + b; }");
        assert!(m.contains("ante nomen"), "got: {m}");
    }

    #[test]
    fn definition_without_header_gets_real_advice() {
        let m = parse_err("adde(a, b) { redde a + b; }");
        assert!(m.contains("munus ante definitionem"), "got: {m}");
    }

    #[test]
    fn list_literal_parses() {
        let e = parse_first_expr("[°I, °II, °III];");
        assert_eq!(e, Expr::ListLit(vec![Expr::Int(ri(1)), Expr::Int(ri(2)), Expr::Int(ri(3))]));
        assert_eq!(parse_first_expr("[];"), Expr::ListLit(vec![]));
    }

    #[test]
    fn list_type_declaration_parses() {
        let s = parse_all("ni lista xs = [°I];");
        assert_eq!(s[0].kind, StmtKind::Let {
            ty: Type::List(Box::new(Type::Int)),
            name: "xs".into(),
            value: Expr::ListLit(vec![Expr::Int(ri(1))]),
        });
    }

    #[test]
    fn index_expression_parses() {
        let e = parse_first_expr("xs[°I];");
        assert_eq!(e, Expr::Index {
            base: Box::new(Expr::Ident("xs".into())),
            index: Box::new(Expr::Int(ri(1))),
        });
        // chained indexing (nested lists): xs[°I][°II]
        let e2 = parse_first_expr("xs[°I][°II];");
        assert!(matches!(e2, Expr::Index { base, .. } if matches!(*base, Expr::Index { .. })));
    }

    #[test]
    fn index_assignment_parses() {
        let s = parse_all("xs[°II] = °X;");
        assert_eq!(s[0].kind, StmtKind::IndexAssign {
            name: "xs".into(),
            index: Expr::Int(ri(2)),
            value: Expr::Int(ri(10)),
        });
    }

    #[test]
    fn bare_index_is_still_an_expr_statement() {
        // `xs[°I];` with no `=` must remain an ExprStmt, not become an assign.
        let s = parse_all("xs[°I];");
        assert!(matches!(s[0].kind, StmtKind::ExprStmt(Expr::Index { .. })));
    }

    fn parse_file_ok(src: &str) -> FileAst {
        parse_file(&tokenize(src).unwrap()).expect("expected a valid file")
    }

    fn parse_file_err(src: &str) -> String {
        parse_file(&tokenize(src).unwrap())
            .expect_err("expected a file-level parse error")
            .message
    }

    #[test]
    fn file_with_imports_then_functions() {
        let f = parse_file_ok(
            "affer mathematica;\naffer fractio;\nmunus initium reddit ni { redde nullus; }",
        );
        assert_eq!(f.imports.len(), 2);
        assert_eq!(f.imports[0], FileImport { package: "mathematica".into(), line: 1 });
        assert_eq!(f.imports[1], FileImport { package: "fractio".into(), line: 2 });
        assert_eq!(f.funcs.len(), 1);
        assert!(matches!(f.funcs[0].kind, StmtKind::Func(_)));
    }

    #[test]
    fn empty_file_parses_to_nothing() {
        let f = parse_file_ok("// solum commentarium\n");
        assert!(f.imports.is_empty() && f.funcs.is_empty());
    }

    #[test]
    fn duplicate_import_is_rejected() {
        let m = parse_file_err("affer mathematica;\naffer mathematica;");
        assert!(m.contains("bis allatus"), "got: {m}");
    }

    #[test]
    fn import_after_a_function_is_rejected() {
        let m = parse_file_err("munus f reddit ni { redde nullus; }\naffer mathematica;");
        assert!(m.contains("ante omnia munera"), "got: {m}");
    }

    #[test]
    fn top_level_statement_is_rejected() {
        for src in ["scribe(°I);", "ni x = °V;", "si (verus) { scribe(°I); }"] {
            let m = parse_file_err(src);
            assert!(m.contains("nihil nisi declarationes"), "src {src:?} got: {m}");
        }
    }

    #[test]
    fn qualified_call_parses() {
        let e = parse_first_expr("mathematica.summa(°I, °II);");
        assert_eq!(e, Expr::Call {
            package: Some("mathematica".into()),
            name: "summa".into(),
            args: vec![Expr::Int(ri(1)), Expr::Int(ri(2))],
        });
    }

    #[test]
    fn qualified_name_must_be_called() {
        let toks = tokenize("ni x = mathematica.summa;").unwrap();
        let m = parse(&toks).expect_err("expected error").message;
        assert!(m.contains("vocatio"), "got: {m}");
    }

    #[test]
    fn dot_without_a_function_name_is_rejected() {
        let toks = tokenize("mathematica.(°I);").unwrap();
        assert!(parse(&toks).is_err());
    }
}
