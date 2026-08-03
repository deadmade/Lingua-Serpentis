//! Generated well-formed programs. Mutation rarely gets past the parser, so
//! this is what exercises the checker and interpreter in depth. proptest
//! shrinks a failure to a minimal repro, which a hand-rolled generator cannot.
mod support;

use proptest::prelude::*;
use std::ffi::OsStr;
use std::time::Duration;
use support::diag::{judge, Verdict, RECORDED_RULES};
use support::proc::{run_binary, temp_dir};

const T: Duration = Duration::from_secs(10);

/// A generated expression, rendered to source. Only `ni`-typed shapes, so the
/// programs mostly type-check and reach the interpreter.
#[derive(Debug, Clone)]
enum Expr {
    Num(u32),
    Var(&'static str),
    Bin(Box<Expr>, &'static str, Box<Expr>),
    Neg(Box<Expr>),
}

const VARS: [&str; 3] = ["a", "b", "c"];
const OPS: [&str; 5] = ["+", "-", "*", "/", "%"];

fn roman(mut n: u32) -> String {
    if n == 0 {
        return "nullus".to_string();
    }
    const TABULA: [(u32, &str); 13] = [
        (1000, "M"), (900, "CM"), (500, "D"), (400, "CD"), (100, "C"), (90, "XC"),
        (50, "L"), (40, "XL"), (10, "X"), (9, "IX"), (5, "V"), (4, "IV"), (1, "I"),
    ];
    let mut s = String::from("°");
    for (v, sym) in TABULA {
        while n >= v {
            s.push_str(sym);
            n -= v;
        }
    }
    s
}

impl Expr {
    fn render(&self) -> String {
        match self {
            Expr::Num(n) => roman(*n),
            Expr::Var(v) => v.to_string(),
            Expr::Bin(l, op, r) => format!("({} {} {})", l.render(), op, r.render()),
            Expr::Neg(e) => format!("(-{})", e.render()),
        }
    }
}

fn expr() -> impl Strategy<Value = Expr> {
    let leaf = prop_oneof![
        (0u32..200).prop_map(Expr::Num),
        proptest::sample::select(VARS.as_slice()).prop_map(Expr::Var),
    ];
    leaf.prop_recursive(4, 24, 2, |inner| {
        prop_oneof![
            (inner.clone(), proptest::sample::select(OPS.as_slice()), inner.clone())
                .prop_map(|(l, op, r)| Expr::Bin(Box::new(l), op, Box::new(r))),
            inner.prop_map(|e| Expr::Neg(Box::new(e))),
        ]
    })
}

/// Wrap an expression in a complete, declaration-only program.
fn program(e: &Expr) -> String {
    format!(
        "munus computa capit (ni a, ni b, ni c) reddit ni {{\n  redde {};\n}}\n\
         munus initium reddit ni {{\n  scribe(computa(°VII, °III, °II));\n  redde nullus;\n}}\n",
        e.render()
    )
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 200, max_shrink_iters: 2000, ..ProptestConfig::default() })]

    #[test]
    fn generated_programs_never_escape_the_error_model(e in expr()) {
        let src = program(&e);
        let d = temp_dir("liss-gen");
        let p = d.write("fons.liss", &src);
        let outcome = run_binary(&[OsStr::new(p.to_str().unwrap())], T);
        let verdict = judge(&outcome, &[("fons.liss".to_string(), src.clone())]);
        match verdict {
            // Divide/modulo by zero is a legitimate, well-formed error, so
            // `judge` already calls it Acceptable.
            Verdict::Acceptable => {}
            Verdict::Violation { rule, .. } if RECORDED_RULES.contains(&rule) => {}
            Verdict::Violation { rule, detail } => {
                prop_assert!(false, "{rule}: {detail}\n--- source ---\n{src}");
            }
        }
    }
}
