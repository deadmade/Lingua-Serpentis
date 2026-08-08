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
/// Sixty-four, down from two hundred. The number is a budget: the generator
/// draws `ni` arithmetic over three variables and five operators, and the whole
/// *behavioural* space it can reach is two outcomes — the program runs, or it
/// divides by zero.
///
/// **The removed draws were not free, and it is worth being exact about what
/// they were buying.** Before [`PONDUS_NULLIUS`] existed the error path was
/// reached on 1.75% of draws, so cutting 200 to 64 took the chance of exercising
/// it at all from 97% to 67% — a thirty-point swing, not nothing. The answer is
/// not to buy it back with 136 more processes: weighting the leaf toward zero
/// recovers it for no spawns at all, and overshoots what the two hundred ever
/// managed. See [`PONDUS_NULLIUS`]. What sixty-four costs, after that, really is
/// nothing.
///
/// The budget itself, at the measured 384 ms per case under `liss run`: the two
/// tests in this file run on separate libtest threads, so wall time tracks the
/// **larger** of [`CASUS`] and [`SAMPLES`] rather than their sum — which is why
/// two hundred cases measured 62.3 s and not 250 × 310 ms. Sixty-four is
/// therefore about **25 s** under `liss`, where two hundred would have been
/// about 77 s, in the workspace's already-slowest target.
const CASUS: u32 = 64;

/// A generous sample for the statistical guards, which draw expressions without
/// running any of them. Two thousand trees cost microseconds and no process at
/// all, which is what makes the floor in
/// `the_generator_still_draws_a_zero_divisor` safe to state tightly.
const SPECULATIONES: usize = 2000;

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

/// **The weight of an explicit zero among the leaves**, against 4 apiece for an
/// arbitrary number and for a variable.
///
/// This is what keeps `divisio per nullum` reachable at [`CASUS`] draws, and it
/// is the fix for a real hole. A uniform `0..200` puts a zero in one numeric
/// leaf in two hundred, and once the chance of that zero landing to the right of
/// a `/` or `%` is folded in, **the error path came out at 1.75% per draw** —
/// measured over 400 samples. At two hundred cases that is a 97% chance of
/// exercising it; at sixty-four it is 67%, so roughly one `cargo test` in three
/// would have run `generated_programs_never_escape_the_error_model` without ever
/// producing a diagnostic — and its error half is precisely the half that only
/// exists when one is produced. A non-Latin `divisio per nullum`, a wrong line,
/// a `judge` rule mis-firing: all of it passes silently on such a run.
///
/// Weighting buys that back **for nothing**, which raising [`CASUS`] does not:
/// a weight costs no process, a case costs one. Measured the same way over 400
/// samples, weight 3 puts the error path at **13.75% per draw**, or 99.99% over
/// sixty-four — better than the two hundred cases ever gave — while 86% of
/// programs still run to completion, so neither half of the property goes hungry.
///
/// Pinned by `the_generator_still_draws_a_zero_divisor`, which is what fails if
/// this arm is ever dropped.
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
///
/// **The init clause is the whole of D-26, and that is provable rather than
/// hopeful.** A `dum` header has three clauses, and only two of them could in
/// principle carry a terminator. The *step* clause cannot: `parser.rs`'s
/// `assign_no_semi` admits only `<ident> = <expr>` there, so `dum (…; …; redde
/// x)` is a parse error under both backends and never reaches either one's
/// disagreement. The init clause is therefore the only route, and matching on it
/// alone is complete, not merely narrow.
fn caput_dum_terminatum(src: &str) -> bool {
    const TERMINI: [&str; 3] = ["redde", "abrumpe", "continua"];
    for (i, _) in src.match_indices("dum") {
        // `dummodo` is not a loop, and neither is `nondum`.
        if !verbum_est(src, i, "dum") {
            continue;
        }
        let post = &src[i + "dum".len()..];
        // The header opens at the next `(`; anything else between is some other
        // construct, and a later `dum` gets its own turn.
        let Some(j) = post.find('(') else { continue };
        if !post[..j].trim().is_empty() {
            continue;
        }
        let Some(initium) = clausula_initialis(&post[j + 1..]) else { continue };
        if TERMINI.iter().any(|t| initium.match_indices(t).any(|(k, _)| verbum_est(initium, k, t)))
        {
            return true;
        }
    }
    false
}

/// The **init clause** of a `dum` header, given everything after its opening
/// `(`: the text up to the first `;` at the header's own parenthesis depth.
///
/// `None` where the header closes without one. `dum (verus)` is not a
/// three-clause header, and reading past its `)` for a `;` would find the one
/// ending the first statement of the **body** — so `dum (verus) { redde nullus;
/// }`, an ordinary program, would be reported as D-26's shape. That was a real
/// bug in the first draft of this guard, caught by the negative below.
///
/// Depth-tracked rather than a plain `split_once`, so a parenthesised
/// initialiser such as `dum (ni i = (°I + °II); …)` is not cut short at its own
/// inner `)`. Quotes are tracked for the same reason, so a `;` inside a string
/// literal does not end the clause early. Escape sequences are not, which can
/// only make the guard *miss* a shape, never invent one.
fn clausula_initialis(post: &str) -> Option<&str> {
    let mut altitudo = 1usize;
    let mut in_chorda = false;
    for (i, c) in post.char_indices() {
        match c {
            '"' => in_chorda = !in_chorda,
            _ if in_chorda => {}
            '(' => altitudo += 1,
            ')' => {
                altitudo -= 1;
                if altitudo == 0 {
                    return None;
                }
            }
            ';' if altitudo == 1 => return Some(&post[..i]),
            _ => {}
        }
    }
    None
}

/// Whether the `verbum` sitting at byte `i` of `haec` is a whole word rather
/// than part of a longer name.
///
/// Substring matching over-excludes, which is the safe direction — but silently:
/// an init clause declaring `ni reddendum = °I`, or one holding a string with
/// `continua` in it, would be refused as D-26's shape and the draw lost with no
/// sign. A widened generator would simply produce fewer programs than it
/// thought.
fn verbum_est(haec: &str, i: usize, verbum: &str) -> bool {
    let pars_nominis = |c: char| c.is_alphanumeric() || c == '_';
    let ante = haec[..i].chars().next_back().is_none_or(|c| !pars_nominis(c));
    let post = haec[i + verbum.len()..].chars().next().is_none_or(|c| !pars_nominis(c));
    ante && post
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

/// **The zero divisor is still drawn**, and the count is the thing being
/// asserted rather than a comment about it.
///
/// [`PONDUS_NULLIUS`] is one line, and a later reader tidying the leaf strategy
/// back to two arms would lose the error half of
/// `generated_programs_never_escape_the_error_model` with the whole suite green
/// — the property would still pass, on programs that never produce a
/// diagnostic. That is the regression this test exists to make loud.
///
/// **It spawns nothing.** Two thousand trees are drawn and rendered, and the
/// count is of sources with an *immediate* literal zero divisor — `/ nullus` or
/// `% nullus`. That is a deliberate under-count: `(x / (nullus + nullus))`,
/// `(x / (-nullus))` and `(x / (a - a))` all divide by zero too and none is
/// matched here. Measured against the interpreter, the syntactic rate is 8.0%
/// and the true error rate 13.75%, so this is a strict lower bound on the
/// property that matters, which is the right direction for a floor.
///
/// The floor is 4% — half the measured rate, and fifty times the **0.15%** the
/// unweighted leaf produces. That last number is not a projection: deleting the
/// weighted arm drops this count to 3 of 2000 and the test says so by name.
/// With 2000 draws the gap is far wider than any sampling noise, so this is a
/// ratchet and not a coin toss.
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
        // `dum` inside a longer name is not a loop header, before or after.
        "munus initium reddit ni {\n  ni dummodo = °I;\n  redde nullus;\n}",
        "munus initium reddit ni {\n  ni nondum = °I;\n  dum (ni i = °I; verus; i = i + °I) { }\n}",
        // And a *terminator* inside a longer name is not a terminator. These
        // are the draws a substring match would have silently thrown away.
        "munus initium reddit ni {\n  dum (ni reddendum = °I; verus; i = i + °I) { }\n}",
        "munus initium reddit ni {\n  dum (voc s = \"continuatio\"; verus; i = i + °I) { }\n}",
        // A header with no init clause at all; its body's `redde` is not one,
        // and must not be read as one. This negative caught a real bug.
        "munus initium reddit ni {\n  dum (verus) { redde nullus; }\n}",
        // A parenthesised initialiser must not be cut short at its own `)`.
        "munus initium reddit ni {\n  dum (ni i = (°I + °II); verus; i = i + °I) \
         { redde nullus; }\n}",
    ] {
        assert!(!caput_dum_terminatum(innocuum), "{innocuum}");
    }
}
