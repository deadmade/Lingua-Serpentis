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
//!
//! # The cost, and why there is a number on it
//!
//! Every case here is a **process spawn**, and spec §10.2 names this file as
//! the milestone's real cost risk: "its generator produces well-typed programs
//! that reach codegen every time … this is where a careless default turns
//! `cargo test` from seconds into minutes." Once `support::proc::BINARY` points
//! at `liss` — Task 19 flips it, together with the `judge` rule §8 needs — each
//! case is a Cranelift compile, a link and two `execve`s rather than one
//! interpreter run. Measured on this machine, debug: `liss run` on a program of
//! exactly this shape costs **384 ms** against the interpreter's **308 ms**, so
//! the count is the whole budget. [`CASUS`] and [`RETRACTATIONES`] are that
//! budget, written down.
//!
//! # D-26 is excluded, by a check rather than by luck
//!
//! **A terminator in a `dum` init clause is the one shape the two backends
//! disagree about**, by design and pending a language decision: the interpreter
//! discards the `Flow` and loops forever (`interp.rs`), compiled code returns at
//! once. See D-26 in `docs/defectus/2026-08-03-index.md`.
//!
//! This generator sidesteps it today only because it emits no statements at all
//! — no `dum`, and therefore no `dum` header. That is an accident of how narrow
//! it is, and the narrowness is exactly what a later reader is invited to widen.
//! So the exclusion is asserted rather than assumed: [`caput_dum_terminatum`]
//! rejects `dum (redde …; …)` and its `abrumpe` and `continua` spellings, and
//! both tests below check every program they emit against it. Widen the
//! generator and the guard is what tells you which shape you may not draw, and
//! why — instead of a hang under the interpreter or, once the binary is `liss`,
//! a differential failure that is neither backend's fault.
//!
//! It is deliberately **not** a `conformance/` fixture: a fixture would bless
//! one backend's answer as the language's. Codegen's side of it is pinned by
//! `caput_dum_terminatum_componitur` in `liss-codegen/tests/regimen.rs` — real
//! coverage, but not differential coverage.
mod support;

use proptest::prelude::*;
use proptest::strategy::ValueTree;
use proptest::test_runner::TestRunner;
use std::ffi::OsStr;
use std::time::Duration;
use support::diag::{judge, parse_diagnostic, Verdict, RECORDED_RULES};
use support::proc::{run_binary, temp_dir, Outcome};

const T: Duration = Duration::from_secs(10);

/// **How many programs the property draws — the cap spec §10.2 asks for.**
///
/// Sixty-four, down from two hundred. The number is a budget rather than a
/// confidence level, because confidence saturates long before it: the generator
/// draws `ni` arithmetic over three variables and five operators, and the whole
/// *behavioural* space it can reach is two outcomes — the program runs, or it
/// divides by zero. Two hundred draws of that buy nothing over sixty-four that
/// a hundred and thirty-six process spawns are worth.
///
/// At the measured 384 ms per case under `liss run`, sixty-four cases plus
/// [`SAMPLES`] is about **44 s**; the two hundred it replaces would have been
/// about 96 s, in the workspace's already-slowest target.
const CASUS: u32 = 64;

/// **The cap on shrinking, which is the larger uncapped cost.**
///
/// A shrink iteration is another spawn, so proptest's default budget of 2000
/// would turn one failure into roughly **thirteen minutes** of compiling and
/// linking before the minimal repro appeared. 256 bounds that at about a
/// hundred seconds, which is still far more shrinking than a four-deep
/// arithmetic expression over three variables can use.
const RETRACTATIONES: u32 = 256;

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

/// **D-26's shape, refused explicitly.** True when `src` holds a `dum` whose
/// **init clause** — everything from the header's `(` to its first `;` —
/// contains a terminator.
///
/// That is the one program shape the interpreter and the compiler answer
/// differently, and the disagreement is a pending language decision rather than
/// a bug in either (see this file's module documentation). Nothing this
/// generator emits today can trip it, which is precisely why it is a function
/// and not a sentence: a widening that adds loops will trip it on the first
/// draw and be sent to D-26, instead of hanging under the interpreter or, once
/// `BINARY` is `liss`, failing a differential that neither backend lost.
///
/// Textual, and deliberately so — it reads the source the generator *renders*,
/// which is the artefact that reaches the compiler, so no change to the `Expr`
/// tree can route around it.
fn caput_dum_terminatum(src: &str) -> bool {
    const TERMINI: [&str; 3] = ["redde", "abrumpe", "continua"];
    let mut reliquum = src;
    while let Some(i) = reliquum.find("dum") {
        reliquum = &reliquum[i + "dum".len()..];
        // The header opens at the next `(`; anything else after `dum` is not a
        // loop header at all, and the next iteration will find the next `dum`.
        let Some(j) = reliquum.find('(') else { return false };
        if !reliquum[..j].trim().is_empty() {
            continue;
        }
        let caput = &reliquum[j + 1..];
        let initium = caput.split(';').next().unwrap_or(caput);
        if TERMINI.iter().any(|t| initium.contains(t)) {
            return true;
        }
    }
    false
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
        prop_assert!(
            !caput_dum_terminatum(&src),
            "the generator drew D-26's shape — a terminator in a `dum` init \
             clause — on which the two backends disagree by design:\n{src}"
        );
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
        assert!(!caput_dum_terminatum(&src), "sample {i} drew D-26's shape:\n{src}");
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

/// **The D-26 guard, checked against the shape it names.**
///
/// Nothing this generator draws can exercise [`caput_dum_terminatum`] — that is
/// the point of it — so without this the guard could be a function that always
/// answers `false`, asserted two hundred times a run, protecting nothing. The
/// three spellings are D-26's own: `redde`, `abrumpe` and `continua` in a `dum`
/// **init** clause. A terminator in the *body* is an ordinary program both
/// backends agree about (`conformance/currunt/memoria-caput-dum-redditum`), and
/// is one of the negatives here.
#[test]
fn the_guard_recognises_the_shape_it_excludes_and_nothing_else() {
    for terminus in ["redde nullus", "abrumpe", "continua"] {
        let src = format!("munus initium reddit ni {{\n  dum ({terminus}; verus; ) {{ }}\n}}");
        assert!(caput_dum_terminatum(&src), "{src}");
    }
    for innocuum in [
        // A terminator in the body, not the header: `currunt`'s own fixture.
        "munus initium reddit ni {\n  dum (ni i = °I; verus; i = i + °I) { redde nullus; }\n}",
        // An ordinary loop, and one with no `dum` in it at all.
        "munus initium reddit ni {\n  dum (ni i = °I; verus; i = i + °I) { }\n}",
        "munus initium reddit ni {\n  redde nullus;\n}",
        // `dum` as the beginning of an identifier is not a loop header.
        "munus initium reddit ni {\n  ni dummodo = °I;\n  redde nullus;\n}",
    ] {
        assert!(!caput_dum_terminatum(innocuum), "{innocuum}");
    }
}
