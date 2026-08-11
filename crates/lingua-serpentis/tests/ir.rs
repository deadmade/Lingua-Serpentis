//! The typed IR the checker emits (spec §4): the contract codegen is written against.

use lingua_serpentis::ast::{ParsedFile, ParsedProgram};
use lingua_serpentis::checker::check_program_typed;
use lingua_serpentis::lexer::tokenize;
use lingua_serpentis::parser::parse_file;
use ir as ir;
use std::collections::BTreeMap;
use std::fmt::Write as _;

// ---------------------------------------------------------------- fixtures

fn pfile(path: &str, src: &str) -> ParsedFile {
    ParsedFile { path: path.into(), src: src.into(), ast: parse_file(&tokenize(src).unwrap()).unwrap() }
}

fn lower(root: &[(&str, &str)], pkgs: &[(&str, &[(&str, &str)])]) -> ir::Programma {
    let mut packages = BTreeMap::new();
    for (name, files) in pkgs {
        packages.insert(name.to_string(), files.iter().map(|(p, s)| pfile(p, s)).collect());
    }
    let prog =
        ParsedProgram { root: root.iter().map(|(p, s)| pfile(p, s)).collect(), packages };
    check_program_typed(&prog).expect("the fixture must type-check")
}

fn lower1(src: &str) -> ir::Programma {
    lower(&[("principale.liss", src)], &[])
}

// ------------------------------------------------------------- the printer

fn ty(t: &ir::Ty) -> String {
    match t {
        ir::Ty::Bool => "veritas".into(),
        ir::Ty::Char => "lit".into(),
        ir::Ty::Int => "ni".into(),
        ir::Ty::Frac(ir::Grain::Uncia) => "mnr".into(),
        ir::Ty::Frac(ir::Grain::Scripulum) => "nr".into(),
        ir::Ty::Text => "voc".into(),
        ir::Ty::List(inner) => format!("[{}]", ty(inner)),
    }
}

fn konst(c: &ir::Const) -> String {
    match c {
        ir::Const::Bool(b) => (if *b { "verus" } else { "falsus" }).to_string(),
        ir::Const::Char(ch) => format!("'{ch}'"),
        ir::Const::Int(r) => r.display(),
        ir::Const::Frac(_, count) => format!("{} grana", count.display()),
        ir::Const::Text(s) => format!("{s:?}"),
    }
}

fn expr(e: &ir::Expr) -> String {
    let body = match &e.kind {
        ir::ExprKind::Const(c) => format!("const {}", konst(c)),
        ir::ExprKind::Load(s) => format!("load s{}", s.0),
        ir::ExprKind::Unary { op, arg } => format!("{op:?} {}", expr(arg)),
        ir::ExprKind::Binary { op, operand_ty, lhs, rhs } => {
            format!("{op:?}<{}> {} {}", ty(operand_ty), expr(lhs), expr(rhs))
        }
        ir::ExprKind::Call { func, args } => {
            let a: Vec<_> = args.iter().map(expr).collect();
            format!("call #{} {}", func.0, a.join(" "))
        }
        ir::ExprKind::ListLit { items } => {
            let a: Vec<_> = items.iter().map(expr).collect();
            format!("list {}", a.join(" "))
        }
        ir::ExprKind::Index { base, index } => format!("index {} {}", expr(base), expr(index)),
        ir::ExprKind::Len(l) => format!("longitudo {}", expr(l)),
        ir::ExprKind::Append { list, item } => format!("adde {} {}", expr(list), expr(item)),
    };
    format!("({body}):{}", ty(&e.ty))
}

fn slot_list(slots: &[ir::SlotId]) -> String {
    let v: Vec<_> = slots.iter().map(|s| format!("s{}", s.0)).collect();
    format!("[{}]", v.join(" "))
}

fn block(out: &mut String, b: &ir::Block, ind: usize) {
    let pad = " ".repeat(ind);
    let _ = writeln!(out, "{pad}declares {}", slot_list(&b.declares));
    for s in &b.stmts {
        stmt(out, s, ind);
    }
}

fn stmt(out: &mut String, s: &ir::Stmt, ind: usize) {
    let pad = " ".repeat(ind);
    let head = format!("{pad}L{:<3}", s.line);
    match &s.kind {
        ir::StmtKind::Let { slot, value } => {
            let _ = writeln!(out, "{head} let s{} = {}", slot.0, expr(value));
        }
        ir::StmtKind::Assign { slot, value } => {
            let _ = writeln!(out, "{head} set s{} = {}", slot.0, expr(value));
        }
        ir::StmtKind::IndexAssign { slot, index, value } => {
            let _ = writeln!(out, "{head} set s{}[{}] = {}", slot.0, expr(index), expr(value));
        }
        ir::StmtKind::Print(e) => {
            let _ = writeln!(out, "{head} scribe {}", expr(e));
        }
        ir::StmtKind::ExprStmt(e) => {
            let _ = writeln!(out, "{head} eval {}", expr(e));
        }
        ir::StmtKind::If { cond, then, else_ } => {
            let _ = writeln!(out, "{head} si {}", expr(cond));
            let _ = writeln!(out, "{pad}  then");
            block(out, then, ind + 4);
            if let Some(eb) = else_ {
                let _ = writeln!(out, "{pad}  aut");
                block(out, eb, ind + 4);
            }
        }
        ir::StmtKind::While { cond, body } => {
            let _ = writeln!(out, "{head} indem {}", expr(cond));
            block(out, body, ind + 4);
        }
        ir::StmtKind::For { header, cond, step, body } => {
            let _ = writeln!(out, "{head} dum");
            let _ = writeln!(out, "{pad}  caput");
            block(out, header, ind + 4);
            let _ = writeln!(out, "{pad}  cond {}", expr(cond));
            let _ = writeln!(out, "{pad}  step");
            stmt(out, step, ind + 4);
            let _ = writeln!(out, "{pad}  corpus");
            block(out, body, ind + 4);
        }
        ir::StmtKind::Return(e) => {
            let _ = writeln!(out, "{head} redde {}", expr(e));
        }
        ir::StmtKind::Break => {
            let _ = writeln!(out, "{head} abrumpe");
        }
        ir::StmtKind::Continue => {
            let _ = writeln!(out, "{head} continua");
        }
    }
}

fn dump(p: &ir::Programma) -> String {
    let mut out = String::new();
    for (i, f) in p.funcs.iter().enumerate() {
        let _ = writeln!(
            out,
            "#{i} {}  file={}  entry={}  params={}  reddit {}",
            f.symbol,
            f.file.as_deref().unwrap_or("-"),
            f.is_entry,
            f.params,
            ty(&f.ret)
        );
        let types: Vec<_> =
            f.slots.iter().enumerate().map(|(i, t)| format!("s{i}:{}", ty(t))).collect();
        let _ = writeln!(out, "  slots {}", types.join(" "));
        block(&mut out, &f.body, 2);
        out.push('\n');
    }
    out
}

// ------------------------------------------------------- walking utilities

fn walk_exprs(b: &ir::Block, f: &mut impl FnMut(&ir::Expr)) {
    fn ex(e: &ir::Expr, f: &mut impl FnMut(&ir::Expr)) {
        f(e);
        match &e.kind {
            ir::ExprKind::Const(_) | ir::ExprKind::Load(_) => {}
            ir::ExprKind::Unary { arg, .. } => ex(arg, f),
            ir::ExprKind::Binary { lhs, rhs, .. } => {
                ex(lhs, f);
                ex(rhs, f);
            }
            ir::ExprKind::Call { args, .. } => args.iter().for_each(|a| ex(a, f)),
            ir::ExprKind::ListLit { items } => items.iter().for_each(|a| ex(a, f)),
            ir::ExprKind::Index { base, index } => {
                ex(base, f);
                ex(index, f);
            }
            ir::ExprKind::Len(l) => ex(l, f),
            ir::ExprKind::Append { list, item } => {
                ex(list, f);
                ex(item, f);
            }
        }
    }
    fn st(s: &ir::Stmt, f: &mut impl FnMut(&ir::Expr)) {
        match &s.kind {
            ir::StmtKind::Let { value, .. } | ir::StmtKind::Assign { value, .. } => ex(value, f),
            ir::StmtKind::IndexAssign { index, value, .. } => {
                ex(index, f);
                ex(value, f);
            }
            ir::StmtKind::Print(e) | ir::StmtKind::ExprStmt(e) | ir::StmtKind::Return(e) => ex(e, f),
            ir::StmtKind::If { cond, then, else_ } => {
                ex(cond, f);
                blk(then, f);
                if let Some(b) = else_ {
                    blk(b, f);
                }
            }
            ir::StmtKind::While { cond, body } => {
                ex(cond, f);
                blk(body, f);
            }
            ir::StmtKind::For { header, cond, step, body } => {
                blk(header, f);
                ex(cond, f);
                st(step, f);
                blk(body, f);
            }
            ir::StmtKind::Break | ir::StmtKind::Continue => {}
        }
    }
    fn blk(b: &ir::Block, f: &mut impl FnMut(&ir::Expr)) {
        for s in &b.stmts {
            st(s, f);
        }
    }
    blk(b, f);
}

fn func<'a>(p: &'a ir::Programma, symbol_name: &str) -> &'a ir::Func {
    p.funcs
        .iter()
        .find(|f| f.symbol.ends_with(&format!("_{}_{}", symbol_name.len(), symbol_name)))
        .unwrap_or_else(|| panic!("no function named {symbol_name}"))
}

// -------------------------------------------------------------- the six

const BASICS: &str = "\
munus initium reddit ni {
  ni x = °V;
  x = x + °I;
  voc s = \"salve\";
  scribe(s);
  si (verus) {
    ni x = °X;
    scribe(x);
  }
  scribe(x);
  redde nullus;
}";

const CONDITIONALS: &str = "\
munus classifica capit (ni n) reddit voc {
  si (n minor_est nullus) { redde \"negativus\"; }
  aut si (n par_est nullus) { redde \"nihil\"; }
  aut { redde \"positivus\"; }
}
munus initium reddit ni {
  scribe(classifica(°I));
  classifica(°II);
  si (verus) { scribe('a'); }
  redde nullus;
}";

const LOOPS: &str = "\
munus initium reddit ni {
  ni i = nullus;
  indem (i minor_est °V) {
    i = i + °I;
    si (i par_est °III) { continua; }
    si (i par_est °IV) { abrumpe; }
  }
  dum (ni j = °I; j minor_est °V; j = j + °I) {
    ni k = j * °II;
    scribe(k);
    // A `redde` inside a `dum` leaves without ever reaching the exit block
    // where §7.3.5 emits the header teardown, so `j` is freed only because
    // `header.declares` is a block `redde` walks like any other.
    si (k par_est °VIII) { redde k; }
    si (k maior_est °VI) { abrumpe; }
    continua;
  }
  redde nullus;
}";

const PKG_ROOT: &str = "\
affer mathematica;
munus initium reddit ni {
  scribe(mathematica.summa(°I, °II));
  redde mathematica.initium(°III);
}";

const PKG_MATH: &str = "\
munus summa capit (ni a, ni b) reddit ni { redde a + b; }
munus initium capit (ni n) reddit ni { redde n; }";

const LISTS: &str = "\
munus initium reddit ni {
  ni lista xs = [°I, °II, °III];
  xs[°I] = °X;
  ni n = longitudo(xs);
  xs = adde(xs, n);
  scribe(xs[°II]);
  scribe(longitudo(adde([], °I)));
  redde nullus;
}";

const FRACTIONS: &str = "\
munus initium reddit ni {
  mnr a = °I:I/VII;
  nr b = °I:I/VII;
  mnr c = °I:I/VII + °I:I/VII;
  nr d = -°I:I/VII;
  scribe(a);
  scribe(b);
  scribe(c);
  scribe(d);
}";

// --------------------------------------------------------------- snapshots

#[test]
fn ir_basics() {
    insta::assert_snapshot!("basics", dump(&lower1(BASICS)));
}

#[test]
fn ir_conditionals() {
    insta::assert_snapshot!("conditionals", dump(&lower1(CONDITIONALS)));
}

#[test]
fn ir_loops() {
    insta::assert_snapshot!("loops", dump(&lower1(LOOPS)));
}

#[test]
fn ir_packages() {
    let p = lower(
        &[("principale.liss", PKG_ROOT)],
        &[("mathematica", &[("mathematica/summa.liss", PKG_MATH)] as &[_])],
    );
    insta::assert_snapshot!("packages", dump(&p));
}

#[test]
fn ir_lists() {
    insta::assert_snapshot!("lists", dump(&lower1(LISTS)));
}

#[test]
fn ir_fractions_and_fallthrough() {
    insta::assert_snapshot!("fractions", dump(&lower1(FRACTIONS)));
}

// ------------------------------------------------------ targeted assertions

#[test]
fn every_expression_node_carries_a_type() {
    for src in [BASICS, CONDITIONALS, LOOPS, LISTS, FRACTIONS] {
        let p = lower1(src);
        for f in &p.funcs {
            walk_exprs(&f.body, &mut |e| {
                match &e.kind {
                    ir::ExprKind::Load(s) => {
                        assert_eq!(&e.ty, &f.slots[s.0 as usize], "load s{} in {}", s.0, f.symbol);
                    }
                    ir::ExprKind::Const(ir::Const::Int(_)) => assert_eq!(e.ty, ir::Ty::Int),
                    ir::ExprKind::Const(ir::Const::Text(_)) => assert_eq!(e.ty, ir::Ty::Text),
                    ir::ExprKind::Const(ir::Const::Bool(_)) => assert_eq!(e.ty, ir::Ty::Bool),
                    ir::ExprKind::Const(ir::Const::Char(_)) => assert_eq!(e.ty, ir::Ty::Char),
                    ir::ExprKind::Const(ir::Const::Frac(g, _)) => {
                        assert_eq!(e.ty, ir::Ty::Frac(*g))
                    }
                    ir::ExprKind::Binary { op, operand_ty, lhs, rhs } => {
                        assert_eq!(&lhs.ty, operand_ty, "lhs of {op:?}");
                        assert_eq!(&rhs.ty, operand_ty, "rhs of {op:?}");
                        use runtime::BinOp;
                        let comparison =
                            matches!(op, BinOp::Eq | BinOp::Ne | BinOp::Gt | BinOp::Lt);
                        if comparison {
                            assert_eq!(e.ty, ir::Ty::Bool, "a comparison yields veritas");
                        } else {
                            assert_eq!(&e.ty, operand_ty, "{op:?} preserves its operand type");
                        }
                    }
                    ir::ExprKind::Len(_) => assert_eq!(e.ty, ir::Ty::Int),
                    ir::ExprKind::Index { base, .. } => {
                        assert_eq!(ir::Ty::List(Box::new(e.ty.clone())), base.ty);
                    }
                    ir::ExprKind::Append { list, item } => {
                        assert_eq!(&e.ty, &list.ty);
                        assert_eq!(ir::Ty::List(Box::new(item.ty.clone())), list.ty);
                    }
                    _ => {}
                };
            });
        }
    }
}

#[test]
fn a_block_declares_exactly_its_own_slots() {
    let p = lower1(BASICS);
    let f = func(&p, "initium");
    // The function body is the parameter frame; the shadowing `x` inside the `si` is not here.
    assert_eq!(f.body.declares, vec![ir::SlotId(0), ir::SlotId(1)]);
    let ir::StmtKind::If { then, .. } = &f.body.stmts[4].kind else {
        panic!("statement 5 is the `si`, got {:?}", f.body.stmts[4].kind);
    };
    assert_eq!(then.declares, vec![ir::SlotId(2)]);
}

#[test]
fn a_shadowing_declaration_gets_a_fresh_slot_and_the_outer_name_comes_back() {
    let p = lower1(BASICS);
    let f = func(&p, "initium");
    assert_eq!(f.slots, vec![ir::Ty::Int, ir::Ty::Text, ir::Ty::Int]);
    let ir::StmtKind::If { then, .. } = &f.body.stmts[4].kind else { unreachable!() };
    let ir::StmtKind::Print(inner) = &then.stmts[1].kind else { panic!("inner scribe") };
    assert_eq!(inner.kind, ir::ExprKind::Load(ir::SlotId(2)));
    let ir::StmtKind::Print(outer) = &f.body.stmts[5].kind else { panic!("outer scribe") };
    assert_eq!(outer.kind, ir::ExprKind::Load(ir::SlotId(0)));
}

#[test]
fn parameters_are_the_first_slots() {
    let p = lower(
        &[("principale.liss", PKG_ROOT)],
        &[("mathematica", &[("mathematica/summa.liss", PKG_MATH)] as &[_])],
    );
    let f = func(&p, "summa");
    assert_eq!(f.params, 2);
    assert_eq!(f.slots, vec![ir::Ty::Int, ir::Ty::Int]);
    assert_eq!(f.body.declares, vec![ir::SlotId(0), ir::SlotId(1)]);
}

#[test]
fn aut_si_arrives_already_nested() {
    // The parser flattens `aut si` into a one-statement else block, so lowering has nothing to do.
    let p = lower1(CONDITIONALS);
    let f = func(&p, "classifica");
    let ir::StmtKind::If { else_: Some(outer_else), .. } = &f.body.stmts[0].kind else {
        panic!("the first statement is the `si`");
    };
    assert_eq!(outer_else.stmts.len(), 1, "the `aut si` is one nested statement");
    assert!(matches!(outer_else.stmts[0].kind, ir::StmtKind::If { else_: Some(_), .. }));
}

#[test]
fn dum_is_not_desugared_into_indem() {
    // `continua` must still run the step, so codegen builds the CFG (§4.4).
    let p = lower1(LOOPS);
    let f = func(&p, "initium");
    assert!(matches!(f.body.stmts[1].kind, ir::StmtKind::While { .. }), "the `indem`");
    let ir::StmtKind::For { header, step, body, .. } = &f.body.stmts[2].kind else {
        panic!("the `dum` stays a For, got {:?}", f.body.stmts[2].kind);
    };
    assert!(matches!(header.stmts[0].kind, ir::StmtKind::Let { .. }));
    assert!(matches!(step.kind, ir::StmtKind::Assign { .. }));
    assert!(body.stmts.iter().any(|s| matches!(s.kind, ir::StmtKind::Continue)));
    assert!(!f.body.stmts.iter().any(|s| matches!(s.kind, ir::StmtKind::While { .. })
        && matches!(s.kind, ir::StmtKind::For { .. })));
}

#[test]
fn a_dum_header_declares_its_own_slot() {
    let p = lower1(LOOPS);
    let f = func(&p, "initium");
    let ir::StmtKind::For { header, body, .. } = &f.body.stmts[2].kind else {
        panic!("the `dum`")
    };
    let ir::StmtKind::Let { slot, .. } = header.stmts[0].kind else { panic!("the init `let`") };
    assert_eq!(header.declares, vec![slot], "the header declares exactly `j`");
    assert!(!body.declares.contains(&slot), "and the body does not");
    assert!(!f.body.declares.contains(&slot), "nor does the function body");
    // Every owned slot is in exactly one `declares`: the invariant codegen relies on.
    let mut seen: Vec<ir::SlotId> = Vec::new();
    collect_declares(&f.body, &mut seen);
    seen.sort_by_key(|s| s.0);
    assert_eq!(seen, (0..f.slots.len() as u32).map(ir::SlotId).collect::<Vec<_>>());
}

#[test]
fn a_redde_out_of_a_dum_still_reaches_the_header_slot() {
    // The regression in the shape it was found: §7.3.5's post-exit teardown never fires here.
    let p = lower1(
        "munus initium reddit ni {\n  dum (voc s = \"salve\"; verus; s = s) {\n    redde nullus;\n  }\n}",
    );
    let f = func(&p, "initium");
    let ir::StmtKind::For { header, body, .. } = &f.body.stmts[0].kind else { panic!("the `dum`") };
    let ir::StmtKind::Let { slot, .. } = header.stmts[0].kind else { panic!("the init `let`") };
    assert_eq!(f.slots[slot.0 as usize], ir::Ty::Text, "a heap value");
    assert_eq!(header.declares, vec![slot]);
    assert!(matches!(body.stmts[0].kind, ir::StmtKind::Return(_)));
    let mut seen = Vec::new();
    collect_declares(&f.body, &mut seen);
    assert_eq!(seen, vec![slot], "the header is where the only slot lives");
}

/// Includes loop headers.
fn collect_declares(b: &ir::Block, out: &mut Vec<ir::SlotId>) {
    out.extend(b.declares.iter().copied());
    for s in &b.stmts {
        match &s.kind {
            ir::StmtKind::If { then, else_, .. } => {
                collect_declares(then, out);
                if let Some(eb) = else_ {
                    collect_declares(eb, out);
                }
            }
            ir::StmtKind::While { body, .. } => collect_declares(body, out),
            ir::StmtKind::For { header, body, .. } => {
                collect_declares(header, out);
                collect_declares(body, out);
            }
            _ => {}
        }
    }
}

#[test]
fn an_expression_statement_keeps_the_value_nothing_consumed() {
    // §7.3.1: the statement root is freed if nothing consumed it.
    let p = lower1(CONDITIONALS);
    let f = func(&p, "initium");
    let ir::StmtKind::ExprStmt(e) = &f.body.stmts[1].kind else {
        panic!("got {:?}", f.body.stmts[1].kind)
    };
    assert_eq!(e.ty, ir::Ty::Text);
    assert!(matches!(e.kind, ir::ExprKind::Call { .. }));
}

#[test]
fn a_root_initium_falling_off_the_end_gains_a_return_of_zero() {
    let p = lower1(FRACTIONS);
    let f = func(&p, "initium");
    assert!(f.is_entry);
    let last = f.body.stmts.last().expect("a body");
    let ir::StmtKind::Return(e) = &last.kind else { panic!("got {:?}", last.kind) };
    assert_eq!(e.ty, ir::Ty::Int);
    let ir::ExprKind::Const(ir::Const::Int(z)) = &e.kind else { panic!("got {:?}", e.kind) };
    assert!(z.is_zero(), "the appended return is zero, got {}", z.display());
}

#[test]
fn a_body_that_already_returns_gains_nothing() {
    let p = lower1(BASICS);
    let f = func(&p, "initium");
    assert_eq!(f.body.stmts.len(), 7, "no eighth, synthesized, return");
}

#[test]
fn only_the_root_initium_is_the_entry_point() {
    let p = lower(
        &[("principale.liss", PKG_ROOT)],
        &[("mathematica", &[("mathematica/summa.liss", PKG_MATH)] as &[_])],
    );
    let entries: Vec<_> = p.funcs.iter().filter(|f| f.is_entry).map(|f| &f.symbol).collect();
    assert_eq!(entries, vec![&ir::symbol(None, "initium")]);
    let pkg = p
        .funcs
        .iter()
        .find(|f| f.symbol == ir::symbol(Some("mathematica"), "initium"))
        .expect("the package initium");
    assert!(!pkg.is_entry);
    assert_eq!(pkg.params, 1);
}

#[test]
fn a_qualified_call_resolves_to_the_package_functions_id() {
    let p = lower(
        &[("principale.liss", PKG_ROOT)],
        &[("mathematica", &[("mathematica/summa.liss", PKG_MATH)] as &[_])],
    );
    let summa = p
        .funcs
        .iter()
        .position(|f| f.symbol == ir::symbol(Some("mathematica"), "summa"))
        .unwrap();
    let root = func(&p, "initium");
    let ir::StmtKind::Print(call) = &root.body.stmts[0].kind else { panic!("scribe") };
    let ir::ExprKind::Call { func, args } = &call.kind else { panic!("a call") };
    assert_eq!(func.0 as usize, summa);
    assert_eq!(args.len(), 2);
}

#[test]
fn longitudo_and_adde_are_not_calls() {
    // They are not functions and have no symbol (§4.4).
    let p = lower1(LISTS);
    let f = func(&p, "initium");
    let mut lens = 0;
    let mut appends = 0;
    walk_exprs(&f.body, &mut |e| match &e.kind {
        ir::ExprKind::Len(_) => lens += 1,
        ir::ExprKind::Append { .. } => appends += 1,
        ir::ExprKind::Call { .. } => panic!("longitudo/adde must not lower to a Call"),
        _ => {}
    });
    assert_eq!(lens, 2);
    assert_eq!(appends, 2);
}

#[test]
fn a_fraction_literal_narrows_onto_the_grain_the_context_demands() {
    // `°I:I/VII` is 8/7: thirteen unciae as `mnr`, three hundred twenty-nine scripula as `nr`.
    use lingua_serpentis::roman::Roman;
    let p = lower1(FRACTIONS);
    let f = func(&p, "initium");
    let want = [
        (ir::Grain::Uncia, 13i64),     // mnr a
        (ir::Grain::Scripulum, 329),   // nr b
        (ir::Grain::Uncia, 13),        // mnr c, left operand
        (ir::Grain::Uncia, 13),        // mnr c, right operand
        (ir::Grain::Scripulum, 329),   // nr d, under the negation
    ];
    let mut got = Vec::new();
    walk_exprs(&f.body, &mut |e| {
        if let ir::ExprKind::Const(ir::Const::Frac(g, c)) = &e.kind {
            got.push((*g, c.clone()));
        }
    });
    let want: Vec<_> = want.iter().map(|(g, n)| (*g, Roman::from_i64(*n))).collect();
    assert_eq!(got, want);
    assert_eq!(f.slots, vec![
        ir::Ty::Frac(ir::Grain::Uncia),
        ir::Ty::Frac(ir::Grain::Scripulum),
        ir::Ty::Frac(ir::Grain::Uncia),
        ir::Ty::Frac(ir::Grain::Scripulum),
    ]);
}

#[test]
fn the_ir_mirror_narrows_exactly_as_the_ast_does() {
    use lingua_serpentis::fraction;
    use lingua_serpentis::roman::Roman;
    // `°A:B/C` is A + B/C, so the exact rational is (A·C + B)/C.
    let cases = [("°I:I/VII", 8, 7), ("°I:I/II", 3, 2), ("°II:III/V", 13, 5),
                 ("°I:XI/XII", 23, 12), ("°III:I/CCLXXXVIII", 865, 288)];
    for (literal, numer, den) in cases {
        for (keyword, grain, want_grain) in [
            ("mnr", fraction::uncia_grain(), ir::Grain::Uncia),
            ("nr", fraction::scripulum_grain(), ir::Grain::Scripulum),
        ] {
            let src = format!(
                "munus initium reddit ni {{ {keyword} x = {literal}; scribe(x); redde nullus; }}"
            );
            let p = lower1(&src);
            let f = func(&p, "initium");
            let ir::StmtKind::Let { value, .. } = &f.body.stmts[0].kind else { panic!("a let") };
            let want = fraction::from_rational(
                &Roman::from_i64(numer), &Roman::from_i64(den), &grain);
            assert_eq!(
                value.kind,
                ir::ExprKind::Const(ir::Const::Frac(want_grain, want)),
                "{keyword} {literal}"
            );
            assert_eq!(value.ty, ir::Ty::Frac(want_grain));
        }
    }
}

#[test]
fn checking_and_lowering_are_one_walk() {
    let bad = "munus initium reddit ni { ni x = \"salve\"; redde nullus; }";
    let a = ParsedProgram {
        root: vec![pfile("principale.liss", bad)],
        packages: BTreeMap::new(),
    };
    let b = a.clone();
    let e1 = lingua_serpentis::checker::check_program(&a).unwrap_err();
    let e2 = check_program_typed(&b).unwrap_err();
    assert_eq!(e1.message, e2.message);
    assert_eq!(e1.line, e2.line);
    assert_eq!(e1.file, e2.file);
}
