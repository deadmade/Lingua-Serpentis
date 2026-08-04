//! Generated well-formed programs. Mutation rarely gets past the parser, so
//! this is what exercises the checker and interpreter in depth. proptest
//! shrinks a failure to a minimal repro, which a hand-rolled generator cannot.
//!
//! The generator is narrow, and a later reader should not mistake it for
//! coverage of the language. It produces `ni`-typed arithmetic and nothing
//! else: the five binary operators `+ - * / %`, unary negation, three
//! variables, expressions at most four levels deep. No `si`, no `dum`, no
//! `lista`, no type other than `ni`, no recursion, no multi-file programs.
//! Every program it emits is meant to type-check, so generation exercises the
//! checker's *accept* path only — every reject path is still reached solely by
//! the hand-written `conformance/errores/` fixtures.
mod support;

use proptest::prelude::*;
use proptest::strategy::ValueTree;
use proptest::test_runner::TestRunner;
use std::ffi::OsStr;
use std::time::Duration;
use support::diag::{judge, parse_diagnostic, Verdict, RECORDED_RULES};
use support::proc::{run_binary, temp_dir, Outcome};

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

/// How many programs the reach test draws. Every sample is a process spawn and
/// this is already the slowest target in the workspace, so the count is kept to
/// the smallest number that still catches a wholesale regression — which, given
/// the measurement recorded below, is a very small number indeed.
const SAMPLES: usize = 50;

/// Pin the thing the property above cannot pin.
///
/// `generated_programs_never_escape_the_error_model` accepts any violation
/// whose rule is already in `RECORDED_RULES`, and `no Rust type names` — D-02 —
/// is the rule a *parse error* trips. So a generator that regressed until every
/// program it emitted was a syntax error would still satisfy that property, in
/// silence. That is precisely the regression this file exists to prevent:
/// Task 11 showed that mutation almost never gets past the parser, and
/// generation was written to get past it. A property that cannot notice the
/// generator falling back into that blind spot does not pin the reach.
///
/// So assert the reach directly. An all-`ni` arithmetic program has exactly two
/// legitimate failure modes, both raised by the interpreter and therefore only
/// reachable once the lexer, parser and checker have all accepted the program.
/// Anything else — a parse error, a checker error, a signal, a timeout — means
/// generation stopped producing programs that run, and that is the finding.
///
/// The reach was measured independently over 400 samples: 388 exited 0, 12
/// exited 1, and all twelve of those were `divisio per nullum` or
/// `modulus per nullum`. Zero parse errors, zero checker errors, zero signals,
/// zero timeouts. This test is that measurement, made standing.
#[test]
fn generation_reaches_the_interpreter() {
    /// Everything an observer needs to diagnose a program that did not reach
    /// the interpreter. A bare assertion failure here would say only that the
    /// generator broke, not how.
    fn describe(outcome: &Outcome) -> String {
        match outcome {
            Outcome::Exited { code, stdout, stderr } => {
                format!("exit {code}\n--- stdout ---\n{stdout}--- stderr ---\n{stderr}")
            }
            Outcome::Signalled { signal, stdout, stderr } => {
                format!("killed by signal {signal}\n--- stdout ---\n{stdout}--- stderr ---\n{stderr}")
            }
            Outcome::TimedOut => format!("still running after {T:?}"),
        }
    }

    let strategy = expr();
    let mut runner = TestRunner::default();
    let mut reached = 0usize;

    for i in 0..SAMPLES {
        let e = strategy.new_tree(&mut runner).expect("draw an expression").current();
        let src = program(&e);
        let d = temp_dir("liss-attingit");
        let p = d.write("fons.liss", &src);
        let outcome = run_binary(&[OsStr::new(p.to_str().unwrap())], T);

        let ok = match &outcome {
            Outcome::Exited { code: 0, .. } => true,
            Outcome::Exited { code: 1, stderr, .. } => {
                // Verbatim, per the Global Constraints. `contains` would let a
                // parse error that merely mentioned division count as a run.
                let msg = stderr
                    .lines()
                    .last()
                    .and_then(parse_diagnostic)
                    .map(|d| d.message().to_string());
                matches!(msg.as_deref(), Some("divisio per nullum") | Some("modulus per nullum"))
            }
            _ => false,
        };
        assert!(
            ok,
            "sample {i} never reached the interpreter\n{}\n--- source ---\n{src}",
            describe(&outcome)
        );
        reached += 1;
    }

    // Not redundant with the assertion in the loop: a loop that ran zero times
    // passes every assertion inside it. Vacuous success is the same defect one
    // level up from the one this test was written to close.
    assert_eq!(reached, SAMPLES, "drew fewer programs than intended");
}
