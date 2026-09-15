//! `proptest`-generated well-typed programs; the generator's limits are in
//! docs/internals/testing.md. Every case is a process spawn, which is what the counts below buy.
mod support;

use proptest::prelude::*;
use proptest::strategy::ValueTree;
use proptest::test_runner::TestRunner;
use std::ffi::OsStr;
use std::time::Duration;
use support::diag::{judge, parse_stderr, Genus, Verdict, RECORDED_RULES};
use support::proc::{run_binary, temp_dir, Outcome};

const T: Duration = Duration::from_secs(10);

/// Property draws, each a process spawn: about 25 s.
const CASUS: u32 = 64;

/// Draws for the statistical guard; no spawns at all.
const SPECULATIONES: usize = 2000;

/// Shrink cap; a shrink iteration is another spawn, and proptest's 2000 would be ~13 minutes.
const RETRACTATIONES: u32 = 256;

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

/// The weight of an explicit zero among the leaves: what keeps `divisio per nullum` reachable
/// at [`CASUS`] draws. Pinned by `the_generator_still_draws_a_zero_divisor`.
const PONDUS_NULLIUS: u32 = 3;

fn expr() -> impl Strategy<Value = Expr> {
    let leaf = prop_oneof![
        4 => (0u32..200).prop_map(Expr::Num),
        4 => proptest::sample::select(VARS.as_slice()).prop_map(Expr::Var),
        PONDUS_NULLIUS => Just(Expr::Num(0)),
    ];
    leaf.prop_recursive(4, 24, 2, |inner| {
        prop_oneof![
            (inner.clone(), proptest::sample::select(OPS.as_slice()), inner.clone())
                .prop_map(|(l, op, r)| Expr::Bin(Box::new(l), op, Box::new(r))),
            inner.prop_map(|e| Expr::Neg(Box::new(e))),
        ]
    })
}

fn program(e: &Expr) -> String {
    format!(
        "munus computa capit (ni a, ni b, ni c) reddit ni {{\n  redde {};\n}}\n\
         munus initium reddit ni {{\n  scribe(computa(°VII, °III, °II));\n  redde nullus;\n}}\n",
        e.render()
    )
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: CASUS,
        max_shrink_iters: RETRACTATIONES,
        ..ProptestConfig::default()
    })]

    #[test]
    fn generated_programs_never_escape_the_error_model(e in expr()) {
        let src = program(&e);
        let d = temp_dir("liss-gen");
        let p = d.write("fons.liss", &src);
        let outcome = run_binary(&[OsStr::new(p.to_str().unwrap())], T);
        let verdict = judge(&outcome, &[("fons.liss".to_string(), src.clone())]);
        match verdict {
            Verdict::Acceptable => {}
            Verdict::Violation { rule, .. } if RECORDED_RULES.contains(&rule) => {}
            Verdict::Violation { rule, detail } => {
                prop_assert!(false, "{rule}: {detail}\n--- source ---\n{src}");
            }
        }
    }
}

/// Draws for the reach test, each a spawn.
const SAMPLES: usize = 50;

/// The property accepts any recorded-rule violation, so a generator that only emitted syntax
/// errors would satisfy it in silence; this pins the reach instead.
#[test]
fn generation_reaches_the_runtime() {
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
                // Verbatim: `contains` would let a parse error that merely mentioned division count.
                // The last *block*, not the last line — under the block grammar the last
                // line of a failing run is a caret row.
                let msg = parse_stderr(stderr)
                    .0
                    .iter()
                    .rev()
                    .find(|d| d.genus == Genus::Erratum)
                    .map(|d| d.message.clone());
                matches!(msg.as_deref(), Some("divisio per nullum") | Some("modulus per nullum"))
            }
            _ => false,
        };
        assert!(
            ok,
            "sample {i} never reached the runtime\n{}\n--- source ---\n{src}",
            describe(&outcome)
        );
        reached += 1;
    }

    // Closes the vacuous-loop hole: a loop that ran zero times passes every assertion inside it.
    assert_eq!(reached, SAMPLES, "drew fewer programs than intended");
}

/// Counts only an *immediate* `/ nullus` or `% nullus`, a deliberate under-count, so the 4%
/// floor is a strict lower bound on the true error rate.
#[test]
fn the_generator_still_draws_a_zero_divisor() {
    let strategy = expr();
    let mut runner = TestRunner::default();
    let mut cum_nullo = 0usize;
    for _ in 0..SPECULATIONES {
        let e = strategy.new_tree(&mut runner).expect("draw an expression").current();
        let src = program(&e);
        if src.contains("/ nullus") || src.contains("% nullus") {
            cum_nullo += 1;
        }
    }
    let pars = cum_nullo as f64 / SPECULATIONES as f64;
    assert!(
        pars >= 0.04,
        "only {cum_nullo} of {SPECULATIONES} draws ({:.2}%) divide by a literal \
         zero; the error half of the property is close to unreachable at \
         CASUS = {CASUS}. Has PONDUS_NULLIUS been dropped from the leaf?",
        pars * 100.0,
    );
}
