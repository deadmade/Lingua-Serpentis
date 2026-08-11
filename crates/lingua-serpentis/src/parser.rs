use crate::ast::*;
use crate::error::{LinguaError, LResult};
use crate::spatium::Spatium;
use crate::token::{Token, TokenKind};

/// A binary node spans its two operands, from the start of the left to the end
/// of the right.
fn binaria(op: BinOp, left: Expr, right: Expr) -> Expr {
    let spatium = left.spatium.ad(right.spatium);
    Expr::novum(ExprKind::Binary { op, left: Box::new(left), right: Box::new(right) }, spatium)
}

/// Half the measured ceiling on a spawned 2 MiB thread, which is 128 (D-05).
const ALTITUDO_MAXIMA: usize = 64;

struct Parser<'a> {
    toks: &'a [Token],
    pos: usize,
    depth: usize,
}

impl<'a> Parser<'a> {
    fn peek(&self) -> Option<&TokenKind> {
        self.toks.get(self.pos).map(|t| &t.kind)
    }
    /// The token about to be read, or the last one when the cursor is past the end:
    /// three accessors asked for it separately and had to agree on the fallback.
    fn hic(&self) -> Option<&Token> {
        self.toks.get(self.pos).or_else(|| self.toks.last())
    }
    /// Kept for `FileImport.line`, which has no span of its own.
    fn line(&self) -> usize {
        self.hic().map(|t| t.spatium.linea()).unwrap_or(0)
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
            // Quoted: the robustness suite strips quoted spans before its lexical checks.
            Err(self.err(&format!("expectatum: '{}'", kind.name())))
        }
    }
    /// Every parser diagnostic comes through here, so the offending token's span
    /// reaches all of them at once.
    fn err(&self, msg: &str) -> LinguaError {
        LinguaError {
            message: msg.to_string(),
            line: self.line(),
            spatium: self.hic().map(|t| t.spatium),
        }
    }

    /// One level deeper, refusing rather than overflowing. `expr`, `unary` and
    /// `block` are three independent routes and share this one counter (D-05).
    fn descend<T>(&mut self, f: impl FnOnce(&mut Self) -> LResult<T>) -> LResult<T> {
        if self.depth >= ALTITUDO_MAXIMA {
            return Err(self.err("expressio nimis alte implicata"));
        }
        self.depth += 1;
        let r = f(self);
        self.depth -= 1;
        r
    }

    /// The span of the token about to be read, which is where a production begins.
    fn spatium_hic(&self) -> Spatium {
        self.hic().map(|t| t.spatium).unwrap_or(Spatium::novum(0, 0, 0))
    }

    /// The span of the token just consumed, which is where a production ends.
    fn ultimum(&self) -> Spatium {
        self.toks
            .get(self.pos.wrapping_sub(1))
            .or_else(|| self.toks.last())
            .map(|t| t.spatium)
            .unwrap_or(Spatium::novum(0, 0, 0))
    }

    fn expr(&mut self) -> LResult<Expr> { self.descend(|p| p.or_expr()) }

    fn or_expr(&mut self) -> LResult<Expr> {
        let mut left = self.and_expr()?;
        while self.peek() == Some(&TokenKind::Or) {
            self.advance();
            let right = self.and_expr()?;
            left = binaria(BinOp::Or, left, right);
        }
        Ok(left)
    }
    fn and_expr(&mut self) -> LResult<Expr> {
        let mut left = self.cmp_expr()?;
        while self.peek() == Some(&TokenKind::And) {
            self.advance();
            let right = self.cmp_expr()?;
            left = binaria(BinOp::And, left, right);
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
            left = binaria(op, left, right);
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
            left = binaria(op, left, right);
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
            left = binaria(op, left, right);
        }
        Ok(left)
    }
    fn unary(&mut self) -> LResult<Expr> {
        let op = match self.peek() {
            Some(TokenKind::Not) => UnOp::Not,
            Some(TokenKind::Minus) => UnOp::Neg,
            _ => return self.postfix(),
        };
        // From the operator through the operand: `-x` is underlined whole.
        let signum = self.advance().spatium;
        let expr = self.descend(|p| p.unary())?;
        let spatium = signum.ad(expr.spatium);
        Ok(Expr::novum(ExprKind::Unary { op, expr: Box::new(expr) }, spatium))
    }
    fn postfix(&mut self) -> LResult<Expr> {
        let mut e = self.primary()?;
        while self.peek() == Some(&TokenKind::LBracket) {
            self.advance();
            let index = self.expr()?;
            self.eat(&TokenKind::RBracket)?;
            let spatium = e.spatium.ad(self.ultimum());
            e = Expr::novum(ExprKind::Index { base: Box::new(e), index: Box::new(index) }, spatium);
        }
        Ok(e)
    }
    /// A comma-separated list of expressions up to `finis`, which is consumed.
    /// Empty is allowed; a trailing comma is not, because a comma is followed by an
    /// element and nothing else.
    fn separata(&mut self, finis: &TokenKind) -> LResult<Vec<Expr>> {
        let mut out = Vec::new();
        if self.peek() != Some(finis) {
            out.push(self.expr()?);
            while self.peek() == Some(&TokenKind::Comma) {
                self.advance();
                out.push(self.expr()?);
            }
        }
        self.eat(finis)?;
        Ok(out)
    }

    fn call_args(&mut self) -> LResult<Vec<Expr>> {
        self.eat(&TokenKind::LParen)?;
        self.separata(&TokenKind::RParen)
    }

    fn primary(&mut self) -> LResult<Expr> {
        // A literal or a name spans exactly its own token; the composite forms
        // below close from that token through the one that ends them.
        let unum = |p: &mut Self, kind: ExprKind| {
            let spatium = p.advance().spatium;
            Ok(Expr::novum(kind, spatium))
        };
        match self.peek().cloned() {
            Some(TokenKind::Int(i)) => unum(self, ExprKind::Int(i)),
            Some(TokenKind::Frac { numer, den }) => unum(self, ExprKind::FloatLit { numer, den }),
            Some(TokenKind::Text(s)) => unum(self, ExprKind::Text(s)),
            Some(TokenKind::Char(c)) => unum(self, ExprKind::Char(c)),
            Some(TokenKind::True) => unum(self, ExprKind::Bool(true)),
            Some(TokenKind::False) => unum(self, ExprKind::Bool(false)),
            Some(TokenKind::LParen) => {
                self.advance();
                let e = self.expr()?;
                self.eat(&TokenKind::RParen)?;
                // The parenthesised expression keeps its own span: a caret should
                // point at the expression, not at the punctuation around it.
                Ok(e)
            }
            Some(TokenKind::Ident(name)) => {
                let nomen = self.advance().spatium;
                if self.peek() == Some(&TokenKind::Dot) {
                    self.advance();
                    let (fname, _) = self.ident()?;
                    if self.peek() != Some(&TokenKind::LParen) {
                        return Err(self.err("vocatio functionis post nomen fasciculi expectata"));
                    }
                    let args = self.call_args()?;
                    let spatium = nomen.ad(self.ultimum());
                    return Ok(Expr::novum(
                        ExprKind::Call { package: Some(name), name: fname, args },
                        spatium,
                    ));
                }
                if self.peek() == Some(&TokenKind::LParen) {
                    let args = self.call_args()?;
                    let spatium = nomen.ad(self.ultimum());
                    Ok(Expr::novum(ExprKind::Call { package: None, name, args }, spatium))
                } else {
                    Ok(Expr::novum(ExprKind::Ident(name), nomen))
                }
            }
            Some(TokenKind::LBracket) => {
                let aperit = self.advance().spatium;
                let elems = self.separata(&TokenKind::RBracket)?;
                let spatium = aperit.ad(self.ultimum());
                Ok(Expr::novum(ExprKind::ListLit(elems), spatium))
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

    fn ident(&mut self) -> LResult<(String, Spatium)> {
        match self.peek().cloned() {
            Some(TokenKind::Ident(n)) => { let s = self.advance().spatium; Ok((n, s)) }
            _ => Err(self.err("nomen expectatum")),
        }
    }

    fn block(&mut self) -> LResult<Vec<Stmt>> {
        self.descend(|p| {
            p.eat(&TokenKind::LBrace)?;
            let mut stmts = Vec::new();
            while p.peek() != Some(&TokenKind::RBrace) {
                if p.peek().is_none() {
                    return Err(p.err("'}' expectatum"));
                }
                stmts.push(p.statement()?);
            }
            p.eat(&TokenKind::RBrace)?;
            Ok(stmts)
        })
    }

    fn if_stmt(&mut self) -> LResult<Stmt> {
        let initium = self.spatium_hic();
        self.advance();
        self.eat(&TokenKind::LParen)?;
        let cond = self.expr()?;
        self.eat(&TokenKind::RParen)?;
        let then = self.block()?;
        let else_ = if self.peek() == Some(&TokenKind::Else) {
            self.advance();
            if self.peek() == Some(&TokenKind::If) {
                Some(vec![self.if_stmt()?])
            } else {
                Some(self.block()?)
            }
        } else {
            None
        };
        Ok(Stmt { kind: StmtKind::If { cond, then, else_ }, spatium: initium.ad(self.ultimum()) })
    }

    fn while_stmt(&mut self) -> LResult<Stmt> {
        let initium = self.spatium_hic();
        self.advance();
        self.eat(&TokenKind::LParen)?;
        let cond = self.expr()?;
        self.eat(&TokenKind::RParen)?;
        let body = self.block()?;
        Ok(Stmt { kind: StmtKind::While { cond, body }, spatium: initium.ad(self.ultimum()) })
    }

    fn for_stmt(&mut self) -> LResult<Stmt> {
        let initium = self.spatium_hic();
        self.advance();
        self.eat(&TokenKind::LParen)?;
        let init = Box::new(self.statement()?); // consumes its own ';'
        let cond = self.expr()?;
        self.eat(&TokenKind::Semicolon)?;
        let step = Box::new(self.assign_no_semi()?);
        self.eat(&TokenKind::RParen)?;
        let body = self.block()?;
        Ok(Stmt { kind: StmtKind::For { init, cond, step, body }, spatium: initium.ad(self.ultimum()) })
    }

    fn assign_no_semi(&mut self) -> LResult<Stmt> {
        let initium = self.spatium_hic();
        let (name, _) = self.ident()?;
        self.eat(&TokenKind::Assign)?;
        let value = self.expr()?;
        Ok(Stmt { kind: StmtKind::Assign { name, value }, spatium: initium.ad(self.ultimum()) })
    }

    fn func_stmt(&mut self) -> LResult<Stmt> {
        let initium = self.spatium_hic();
        self.advance();

        // Old inline form `munus ni nomen(...)`, caught here for real advice.
        if matches!(
            self.peek(),
            Some(TokenKind::TyInt | TokenKind::TyFloat | TokenKind::TyDouble
                 | TokenKind::TyText | TokenKind::TyChar | TokenKind::TyBool)
        ) {
            return Err(self.err("typus reditus post 'reddit' ponendus, non ante nomen"));
        }

        let (name, name_span) = self.ident()?;

        let mut params: Vec<(Type, String, Spatium)> = Vec::new();
        if self.peek() == Some(&TokenKind::Takes) {
            self.advance();
            self.eat(&TokenKind::LParen)?;
            if self.peek() == Some(&TokenKind::RParen) {
                return Err(self.err("munus sine parametris 'capit' omittat"));
            }
            loop {
                let ty = self.type_keyword()
                    .ok_or_else(|| self.err("typus parametri expectatus"))?;
                let (pname, pspan) = self.ident()?;
                // D-16, and rejected here and nowhere else.
                if params.iter().any(|(_, existing, _)| *existing == pname) {
                    return Err(self.err(&format!("parametrum '{}' bis nominatum", pname)));
                }
                params.push((ty, pname, pspan));
                if self.peek() == Some(&TokenKind::Comma) { self.advance(); } else { break; }
            }
            self.eat(&TokenKind::RParen)?;
        } else if self.peek() == Some(&TokenKind::LParen) {
            return Err(self.err("'capit' ante parametra expectatum"));
        }

        if self.peek() != Some(&TokenKind::Yields) {
            return Err(self.err("'reddit' et typus reditus expectati"));
        }
        self.advance();
        let ret = self.type_keyword()
            .ok_or_else(|| self.err("typus post 'reddit' expectatus"))?;

        let body = self.block()?;
        Ok(Stmt {
            kind: StmtKind::Func(FuncDecl { name, name_span, params, ret, body }),
            spatium: initium.ad(self.ultimum()),
        })
    }

    fn statement(&mut self) -> LResult<Stmt> {
        let initium = self.spatium_hic();
        match self.peek() {
            Some(TokenKind::If) => return self.if_stmt(),
            Some(TokenKind::While) => return self.while_stmt(),
            Some(TokenKind::For) => return self.for_stmt(),
            Some(TokenKind::Func) => return self.func_stmt(),
            Some(TokenKind::Return) => {
                self.advance();
                let e = self.expr()?;
                self.eat(&TokenKind::Semicolon)?;
                return Ok(Stmt { kind: StmtKind::Return(e), spatium: initium.ad(self.ultimum()) });
            }
            Some(TokenKind::Break) => {
                self.advance();
                self.eat(&TokenKind::Semicolon)?;
                return Ok(Stmt { kind: StmtKind::Break, spatium: initium.ad(self.ultimum()) });
            }
            Some(TokenKind::Continue) => {
                self.advance();
                self.eat(&TokenKind::Semicolon)?;
                return Ok(Stmt { kind: StmtKind::Continue, spatium: initium.ad(self.ultimum()) });
            }
            _ => {}
        }
        if let Some(ty) = self.type_keyword() {
            let (name, name_span) = self.ident()?;
            self.eat(&TokenKind::Assign)?;
            let value = self.expr()?;
            self.eat(&TokenKind::Semicolon)?;
            return Ok(Stmt { kind: StmtKind::Let { ty, name, name_span, value }, spatium: initium.ad(self.ultimum()) });
        }
        match self.peek() {
            Some(TokenKind::Print) => {
                self.advance();
                self.eat(&TokenKind::LParen)?;
                let e = self.expr()?;
                self.eat(&TokenKind::RParen)?;
                self.eat(&TokenKind::Semicolon)?;
                Ok(Stmt { kind: StmtKind::Print(e), spatium: initium.ad(self.ultimum()) })
            }
            // The lookahead is what distinguishes an assignment from a call.
            Some(TokenKind::Ident(_))
                if self.toks.get(self.pos + 1).map(|t| &t.kind) == Some(&TokenKind::Assign) =>
            {
                let mut s = self.assign_no_semi()?;
                self.eat(&TokenKind::Semicolon)?;
                // Re-spanned so the statement closes on the semicolon, which
                // `assign_no_semi` has not seen: as a `dum` step there is none.
                s.spatium = initium.ad(self.ultimum());
                Ok(s)
            }
            _ => {
                let e = self.expr()?;
                if matches!(e.kind, ExprKind::Index { .. }) && self.peek() == Some(&TokenKind::Assign) {
                    let (base, index) = match e.kind {
                        ExprKind::Index { base, index } => (base, index),
                        _ => unreachable!(),
                    };
                    let name = match base.kind {
                        ExprKind::Ident(n) => n,
                        _ => return Err(self.err("sinistra subscriptionis variabilis esse debet")),
                    };
                    self.advance();
                    let value = self.expr()?;
                    self.eat(&TokenKind::Semicolon)?;
                    return Ok(Stmt { kind: StmtKind::IndexAssign { name, index: *index, value }, spatium: initium.ad(self.ultimum()) });
                }
                if matches!(e.kind, ExprKind::Call { .. }) && self.peek() == Some(&TokenKind::LBrace) {
                    return Err(self.err("munus ante definitionem expectatum"));
                }
                self.eat(&TokenKind::Semicolon)?;
                Ok(Stmt { kind: StmtKind::ExprStmt(e), spatium: initium.ad(self.ultimum()) })
            }
        }
    }
}

/// `affer` lines first, then `munus` declarations, nothing else.
pub fn parse_file(tokens: &[Token]) -> LResult<FileAst> {
    let mut p = Parser { toks: tokens, pos: 0, depth: 0 };
    let mut imports: Vec<FileImport> = Vec::new();
    while p.peek() == Some(&TokenKind::Import) {
        let line = p.line();
        p.advance();
        let (package, _) = p.ident()?;
        p.eat(&TokenKind::Semicolon)?;
        if imports.iter().any(|i| i.package == package) {
            return Err(LinguaError {
                message: format!("fasciculus '{}' bis allatus", package),
                line, spatium: None,
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

    /// Parameter types and names, dropping the spans these tests are not about.
    fn nomina(params: &[(Type, String, Spatium)]) -> Vec<(Type, &str)> {
        params.iter().map(|(t, n, _)| (t.clone(), n.as_str())).collect()
    }
    use crate::lexer::tokenize;

    fn ri(n: i64) -> crate::roman::Roman { crate::roman::Roman::from_i64(n) }

    /// A one-line `munus initium` wrapper (D-19), so asserted line numbers do not move.
    fn wrap(src: &str) -> String {
        format!("munus initium reddit ni {{ {src} }}")
    }

    fn parse_stmts(src: &str) -> LResult<Vec<Stmt>> {
        let ast = parse_file(&tokenize(&wrap(src)).unwrap())?;
        match ast.funcs.first().map(|s| &s.kind) {
            Some(StmtKind::Func(f)) => Ok(f.body.clone()),
            other => panic!("the wrapper did not produce one function: {other:?}"),
        }
    }

    fn parse_first_expr(src: &str) -> Expr {
        let stmts = parse_stmts(src).unwrap();
        match &stmts[0].kind {
            StmtKind::ExprStmt(e) => e.clone(),
            other => panic!("expected ExprStmt, got {:?}", other),
        }
    }

    fn parse_all(src: &str) -> Vec<Stmt> {
        parse_stmts(src).unwrap()
    }

    fn parse_err(src: &str) -> String {
        parse_stmts(src).expect_err("expected a parse error").message
    }

    #[test]
    fn a_parameter_name_cannot_repeat() {
        // D-16
        assert_eq!(
            parse_err("munus f capit (ni a, voc a) reddit voc { redde a; }"),
            "parametrum 'a' bis nominatum"
        );
    }

    #[test]
    fn distinct_parameter_names_are_still_fine() {
        assert!(!parse_all("munus f capit (ni a, voc b) reddit voc { redde b; }").is_empty());
    }

    #[test]
    fn declaration_statement() {
        let s = parse_all("ni x = °V;");
        assert_eq!(s.len(), 1);
        assert!(matches!(
            &s[0].kind,
            StmtKind::Let { ty: Type::Int, name, value: Expr { kind: ExprKind::Int(v), .. }, .. }
                if name == "x" && *v == ri(5)
        ));
    }

    #[test]
    fn assignment_and_print() {
        let s = parse_all("x = °I; scribe(x);");
        assert_eq!(s[0].kind, StmtKind::Assign { name: "x".into(), value: ex(ExprKind::Int(ri(1))) });
        assert_eq!(s[1].kind, StmtKind::Print(ex(ExprKind::Ident("x".into()))));
    }

    #[test]
    fn arithmetic_precedence() {
        let e = parse_first_expr("°II + °III * °IV;");
        assert_eq!(e, ExprKind::Binary {
            op: BinOp::Add,
            left: bx(ExprKind::Int(ri(2))),
            right: bx(ExprKind::Binary {
                op: BinOp::Mul,
                left: bx(ExprKind::Int(ri(3))),
                right: bx(ExprKind::Int(ri(4))),
            }),
        });
    }

    #[test]
    fn comparison_and_call() {
        let e = parse_first_expr("summa maior_est adde(a, °I);");
        match e.kind {
            ExprKind::Binary { op: BinOp::Gt, right, .. } => match right.kind {
                ExprKind::Call { name, args, .. } => {
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
        let e = parse_first_expr("a et b vel c;");
        assert_eq!(e, ExprKind::Binary {
            op: BinOp::Or,
            left: bx(ExprKind::Binary {
                op: BinOp::And,
                left: bx(ExprKind::Ident("a".into())),
                right: bx(ExprKind::Ident("b".into())),
            }),
            right: bx(ExprKind::Ident("c".into())),
        });
    }

    #[test]
    fn not_binds_tighter_than_and() {
        let e = parse_first_expr("non a et b;");
        assert_eq!(e, ExprKind::Binary {
            op: BinOp::And,
            left: bx(ExprKind::Unary { op: UnOp::Not, expr: bx(ExprKind::Ident("a".into())) }),
            right: bx(ExprKind::Ident("b".into())),
        });
    }

    #[test]
    fn subtraction_is_left_associative() {
        let e = parse_first_expr("°X - °III - °I;");
        assert_eq!(e, ExprKind::Binary {
            op: BinOp::Sub,
            left: bx(ExprKind::Binary {
                op: BinOp::Sub,
                left: bx(ExprKind::Int(ri(10))),
                right: bx(ExprKind::Int(ri(3))),
            }),
            right: bx(ExprKind::Int(ri(1))),
        });
    }

    #[test]
    fn zero_arg_call() {
        let e = parse_first_expr("adde();");
        assert_eq!(e, ExprKind::Call { package: None, name: "adde".into(), args: vec![] });
    }

    #[test]
    fn if_else_if_else() {
        let s = parse_all("si (x) { scribe(°I); } aut si (y) { scribe(°II); } aut { scribe(°III); }");
        match &s[0].kind {
            StmtKind::If { then, else_, .. } => {
                assert_eq!(then.len(), 1);
                assert!(else_.is_some());
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
                assert_eq!(nomina(&f.params), vec![(Type::Int, "a"), (Type::Int, "b")]);
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
                assert_eq!(nomina(&f.params), vec![(Type::Text, "v"), (Type::Int, "n")]);
            }
            other => panic!("expected func, got {:?}", other),
        }
    }

    #[test]
    fn empty_capit_is_rejected() {
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

    // Load-bearing: without it a dual-syntax parser could quietly come back.
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
        assert_eq!(e, ExprKind::ListLit(vec![ex(ExprKind::Int(ri(1))), ex(ExprKind::Int(ri(2))), ex(ExprKind::Int(ri(3)))]));
        assert_eq!(parse_first_expr("[];"), ExprKind::ListLit(vec![]));
    }

    #[test]
    fn list_type_declaration_parses() {
        let s = parse_all("ni lista xs = [°I];");
        assert!(matches!(
            &s[0].kind,
            StmtKind::Let { ty, name, value: Expr { kind: ExprKind::ListLit(xs), .. }, .. }
                if *ty == Type::List(Box::new(Type::Int))
                    && name == "xs"
                    && *xs == vec![ExprKind::Int(ri(1))]
        ));
    }

    #[test]
    fn index_expression_parses() {
        let e = parse_first_expr("xs[°I];");
        assert_eq!(e, ExprKind::Index {
            base: bx(ExprKind::Ident("xs".into())),
            index: bx(ExprKind::Int(ri(1))),
        });
        let e2 = parse_first_expr("xs[°I][°II];");
        assert!(matches!(e2.kind, ExprKind::Index { base, .. } if matches!(base.kind, ExprKind::Index { .. })));
    }

    #[test]
    fn index_assignment_parses() {
        let s = parse_all("xs[°II] = °X;");
        assert_eq!(s[0].kind, StmtKind::IndexAssign {
            name: "xs".into(),
            index: ex(ExprKind::Int(ri(2))),
            value: ex(ExprKind::Int(ri(10))),
        });
    }

    #[test]
    fn bare_index_is_still_an_expr_statement() {
        let s = parse_all("xs[°I];");
        assert!(matches!(s[0].kind, StmtKind::ExprStmt(Expr { kind: ExprKind::Index { .. }, .. })));
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
        assert_eq!(e, ExprKind::Call {
            package: Some("mathematica".into()),
            name: "summa".into(),
            args: vec![ex(ExprKind::Int(ri(1))), ex(ExprKind::Int(ri(2)))],
        });
    }

    #[test]
    fn qualified_name_must_be_called() {
        let m = parse_err("ni x = mathematica.summa;");
        assert!(m.contains("vocatio"), "got: {m}");
    }

    #[test]
    fn dot_without_a_function_name_is_rejected() {
        assert!(parse_stmts("mathematica.(°I);").is_err());
    }
}
