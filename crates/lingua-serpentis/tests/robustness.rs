//! The robustness invariant: within a timeout, the process terminates with
//! either exit 0 and a stderr of warnings only, or exit 1 and a well-formed
//! Latin diagnostic. Anything else is a defect.
//!
//! Every input runs in its own process, because a stack overflow aborts and
//! cannot be caught in-process.
mod support;

use std::ffi::OsStr;
use std::time::Duration;
use support::diag::{judge, parse_diagnostic, Diagnostic, Verdict, RECORDED_RULES};
use support::proc::{run_binary, temp_dir, Outcome};

/// The **wide** deadline: the fuzzer's, and the one curated case whose cost is
/// genuinely codegen's. [`MORA_ARTA`] is the other, and the reason there are two.
///
/// **Ten seconds until Task 19, and it had to rise when `BINARY` became `liss`**
/// — not because anything got slower to *run*, but because each case now pays a
/// Cranelift pass and a link before it runs at all, and the curated pathologies
/// are deliberately enormous.
///
/// Measured on this machine, `liss run` against the sweep below:
///
/// | case | debug | release |
/// |---|---|---|
/// | `series longissima` (5,001-element literal) | **17.1 s** | 0.67 s |
/// | `nomen longissimum` (100,000-char identifier) | 0.39 s | — |
/// | `numerus longissimus` (5,000 `M`s) | 0.4 s | — |
/// | `parentheses nimium implicatae` | 0.05 s | — |
/// | `recursio infinita` | 0.6 s | — |
///
/// One case dominates, and the 25× gap is the debug build of Cranelift rather
/// than anything about the program: it answers `V̅I` — 5,001 — correctly in both
/// builds. Sixty seconds is roughly 3.5× the measured debug worst case, which is
/// headroom for a loaded machine without being so loose that the rule stops
/// meaning anything: D-20's `x = x * x` does not terminate *at all*, so no
/// finite deadline lets it through.
///
/// This is spec §10.2's "a careless default turns `cargo test` from seconds into
/// minutes" arriving in `robustness.rs`, where the plan only anticipated it in
/// `generative.rs`. The budget there is a case count; here it is this.
///
/// **Only `series longissima` and the fuzzer get this number.** Applying it to
/// the whole sweep is what retired D-23's guard; see [`MORA_ARTA`].
const T: Duration = Duration::from_secs(60);

#[test]
fn the_diagnostic_grammar_parses_all_three_shapes() {
    assert_eq!(
        parse_diagnostic("Error: munus 'initium' in programmate non inventum"),
        Some(Diagnostic::Structural {
            message: "munus 'initium' in programmate non inventum".into()
        })
    );
    assert_eq!(
        parse_diagnostic("Error in linea 2: chorda non clausa"),
        Some(Diagnostic::Line { line: 2, message: "chorda non clausa".into() })
    );
    assert_eq!(
        parse_diagnostic("Error in 'mathematica/summa.liss', linea 2: res mala"),
        Some(Diagnostic::FileLine {
            file: "mathematica/summa.liss".into(),
            line: 2,
            message: "res mala".into(),
        })
    );
    assert_eq!(parse_diagnostic("Monitum in linea 2: nomen"), None);
    assert_eq!(parse_diagnostic("thread 'main' panicked"), None);
}

/// **`Error: programma abrupte cessavit` is an abort, not a diagnostic.**
///
/// `BINARY` is `liss`, so the process this harness spawns is the compiling
/// *parent*, which does not die by signal: the driver waits for the child,
/// reports the signal as this sentence and exits 1 (spec §9.5). Every other
/// route to `Outcome::Signalled` therefore describes the compiler being killed,
/// never a compiled program — and §8's promise is about compiled programs.
///
/// This sentence is deliberately well-formed: Latin, no digits, no Rust type
/// name, a valid structural diagnostic. It passes every lexical rule `judge`
/// has, which is exactly why it needed a rule of its own. Asserted here rather
/// than left to the fuzzer, because the input that provokes it is a compiler
/// defect nobody can write on purpose.
#[test]
fn a_child_killed_by_a_signal_is_an_abort_however_politely_it_is_reported() {
    let outcome = Outcome::Exited {
        code: 1,
        stdout: String::new(),
        stderr: "Error: programma abrupte cessavit\n".to_string(),
    };
    match judge(&outcome, &[("fons.liss".to_string(), "munus initium reddit ni { redde nullus; }".to_string())]) {
        Verdict::Violation { rule: "no abort", .. } => {}
        other => panic!("§8 has no fuzzed enforcement: {other:?}"),
    }
    // And the neighbouring sentence is still an ordinary diagnostic, so the
    // rule is keyed on this message rather than on "exit 1 with a structural
    // error", which would condemn most of the corpus.
    let ordinarium = Outcome::Exited {
        code: 1,
        stdout: String::new(),
        stderr: "Error: acervus vocationum exhaustus est\n".to_string(),
    };
    assert_eq!(
        judge(&ordinarium, &[("fons.liss".to_string(), "munus initium reddit ni { redde nullus; }".to_string())]),
        Verdict::Acceptable,
    );
}

/// **The compiler admitting to a bug, and the runtime admitting to a leak, are
/// not diagnostics either.**
///
/// The same shape as the sentence above and the same reason. `BINARY` is `liss`,
/// so both fuzzers drive the *compiler*; `defectus internus: …` is what spec
/// §10.3's `Defectus::novus(` sites say when it breaks, and `memoria non
/// liberata est` is what §7.5 says when a compiled program leaks. Both are
/// well-formed structural Latin with no file, no line, no English, no Arabic
/// digit and no Rust type name, so before this every lexical rule passed and
/// the verdict was `Acceptable`.
///
/// **Task 12's Critical is the proof that this is reachable**: `redde nullus;
/// scribe("numquam");` — a legal program — made release codegen emit
/// `munus componi non potuit`. A generator that drew it would have been told
/// nothing was wrong.
///
/// Asserted here rather than left to the fuzzer for the same reason as the
/// signal case: the input that provokes either one is a compiler defect nobody
/// can write on purpose.
#[test]
fn the_compilers_own_failures_are_violations_and_not_diagnostics() {
    let fons =
        || vec![("fons.liss".to_string(), "munus initium reddit ni { redde nullus; }".to_string())];
    for (nuntius, regula) in [
        // Task 12's Critical, verbatim.
        ("defectus internus: munus componi non potuit", "no internal defect"),
        // The prefix is the rule, not the list: a sentence §10.3 has not been
        // written yet must be caught the day it is.
        ("defectus internus: machina ignota est", "no internal defect"),
        ("memoria non liberata est", "no leak"),
    ] {
        let outcome = Outcome::Exited {
            code: 1,
            stdout: String::new(),
            stderr: format!("Error: {nuntius}\n"),
        };
        match judge(&outcome, &fons()) {
            Verdict::Violation { rule, .. } if rule == regula => {}
            other => panic!("{nuntius:?} must break {regula:?}, got {other:?}"),
        }
    }

    // And the rules are keyed on these sentences, not on "exit 1 with a
    // structural error" — a neighbouring language-level failure with the same
    // shape must stay an ordinary diagnostic.
    let ordinarium = Outcome::Exited {
        code: 1,
        stdout: String::new(),
        stderr: "Error: memoria exhausta est\n".to_string(),
    };
    assert_eq!(judge(&ordinarium, &fons()), Verdict::Acceptable);
}

/// Run one single-file source through the binary and judge it against the
/// **general** deadline. The curated sweep passes its own per-case one.
fn judge_source(name: &str, src: &str) -> Verdict {
    judge_source_intra(name, src, T)
}

/// [`judge_source`] with the deadline named.
fn judge_source_intra(name: &str, src: &str, mora: Duration) -> Verdict {
    let d = temp_dir("liss-rob");
    let p = d.write("fons.liss", src);
    let outcome = run_binary(&[OsStr::new(p.to_str().unwrap())], mora);
    let verdict = judge(&outcome, &[("fons.liss".to_string(), src.to_string())]);
    if let Verdict::Violation { rule, detail } = &verdict {
        eprintln!("[{name}] {rule}: {detail}");
    }
    verdict
}

/// **The tight deadline, for the cases that are a bound rather than a shape.**
///
/// `T` had to rise to 60 s when `BINARY` became `liss` — every case now pays a
/// Cranelift pass and a link, and one of them takes 17.1 s in a debug-built
/// compiler. Raising it *globally* was the cheap fix, and it silently retired a
/// different guard.
///
/// `nomen longissimum` is that guard. It timed out under the old 10 s deadline
/// until D-23 was fixed, and its note below calls itself the regression guard
/// for `latinitas::analyze`'s bound: ~24 s for 100,000 characters before the
/// fix, 0.32 s after. **Nothing else in the workspace times `analyze` at all**,
/// so under a 60 s deadline a full D-23 regression passes unnoticed. Five
/// seconds is ample for the 0.39 s the case actually costs — most of which is
/// the process's own dictionary build — and far under the ~24 s a regression
/// measures.
///
/// So the deadline is **data, like the expected rule beside it**. This is the
/// milestone's own "rules may only shrink" principle applied to a number: a
/// bound raised for one case must not be raised for the others by accident.
const MORA_ARTA: Duration = Duration::from_secs(5);

/// Every curated pathology with the deadline it is judged against and the rule
/// it is known to break — `None` means it must satisfy the invariant. The rule,
/// not a defect ID: both stack overflows break `no abort`, and the register is
/// what maps rules to IDs.
///
/// Only the cases whose cost is genuinely in *codegen* get `T`. Everything else
/// gets [`MORA_ARTA`], so that a case which starts taking twenty seconds is a
/// failure rather than a shrug.
fn pathologies() -> Vec<(&'static str, String, Duration, Option<&'static str>)> {
    vec![
        ("vacuum", String::new(), MORA_ARTA, None),
        ("spatia sola", "   \n\t\n".to_string(), MORA_ARTA, None),
        ("BOM", "\u{feff}munus initium reddit ni { redde nullus; }".to_string(), MORA_ARTA, None),
        (
            "CRLF",
            "munus initium reddit ni {\r\n  scribe(°I);\r\n  redde nullus;\r\n}".to_string(),
            MORA_ARTA,
            None,
        ),
        (
            // This timed out under the 10s deadline until D-23 was fixed. The
            // cause was `latinitas::analyze::analyze`, whose `for i in
            // 0..=w.len()` loop hashed a `&w[..i]` slice on every iteration —
            // O(n) per iteration, O(n^2) in the identifier's length. Measured
            // against the binary on 2026-08-03: 20_000 chars 1.26s, 50_000
            // ~6.2s, 100_000 ~24s.
            //
            // `analyze` now skips the splits whose stem or ending is longer
            // than any in the dictionary, which is constant work however long
            // the name is. Re-measured 2026-08-05, same machine: 20_000 chars
            // 318ms, 50_000 317ms, 100_000 322ms — flat, and the remainder is
            // the process's own dictionary-build cost, not the name. This case
            // stays in the sweep as the regression guard for that bound —
            // which is why it is judged against MORA_ARTA and not T. Under a
            // 60s deadline the pre-fix ~24s passes, and this case silently
            // stops being a guard at all.
            "nomen longissimum",
            format!(
                "munus initium reddit ni {{ ni {} = °I; redde nullus; }}",
                "a".repeat(100_000)
            ),
            MORA_ARTA,
            None,
        ),
        (
            "numerus longissimus",
            format!(
                "munus initium reddit ni {{ scribe(°{}); redde nullus; }}",
                "M".repeat(5_000)
            ),
            MORA_ARTA,
            None,
        ),
        (
            "series longissima",
            format!(
                "munus initium reddit ni {{ ni lista xs = [{}°I]; scribe(longitudo(xs)); redde nullus; }}",
                "°I, ".repeat(5_000)
            ),
            T,
            None,
        ),
        (
            "parentheses nimium implicatae",
            format!(
                "munus initium reddit ni {{ scribe({}°I{}); redde nullus; }}",
                "(".repeat(20_000),
                ")".repeat(20_000)
            ),
            MORA_ARTA,
            None,
        ),
        (
            "recursio infinita",
            "munus recurre capit (ni n) reddit ni { redde recurre(n + °I); }\n\
             munus initium reddit ni { scribe(recurre(°I)); redde nullus; }"
                .to_string(),
            MORA_ARTA,
            None,
        ),
        (
            "index extra fines",
            "munus initium reddit ni {\n  ni lista xs = [°I];\n  scribe(xs[°X]);\n  redde nullus;\n}"
                .to_string(),
            MORA_ARTA,
            None,
        ),
        (
            // This source is missing the semicolon after `ni x = °I`, so it is
            // a parse error before any runtime lookup happens. Until D-02 was
            // fixed that error read `expectatum: Semicolon` and tripped the
            // `no Rust type names` rule; it now reads `expectatum: ';'` and is
            // an ordinary well-formed diagnostic, so this case satisfies the
            // invariant.
            //
            // It still incidentally corroborates D-03: the missing semicolon
            // is on source line 5, immediately before `redde` on line 6, but
            // the reported line is 4 — a two-line drift matching the string's
            // two embedded newlines. D-03 is NOT detectable by this invariant,
            // because a wrong line that is still inside the file passes `line
            // in range`; conformance/defectus/D-03-linea-post-chordam is its
            // fixture. Kept here to prove the input stays inside the error
            // model, and as the place the drift is visible while it lasts.
            "chorda multilinea",
            "munus initium reddit ni {\n  voc s = \"prima\nsecunda\ntertia\";\n  ni x = °I\n  redde nullus;\n}"
                .to_string(),
            MORA_ARTA,
            None,
        ),
    ]
}

#[test]
fn every_curated_pathology_matches_its_recorded_verdict() {
    let mut surprises = Vec::new();
    for (name, src, mora, expected_rule) in pathologies() {
        let verdict = judge_source_intra(name, &src, mora);
        match (&verdict, expected_rule) {
            (Verdict::Acceptable, None) => {}
            (Verdict::Violation { rule, .. }, Some(want)) if *rule == want => {}
            (Verdict::Acceptable, Some(want)) => surprises.push(format!(
                "{name}: expected to break the {want:?} rule, but it now passes — \
                 if a defect was fixed, update pathologies() and the register"
            )),
            (v, w) => surprises.push(format!("{name}: expected {w:?}, got {v:?}")),
        }
    }
    assert!(surprises.is_empty(), "{}", surprises.join("\n"));
}

fn env_usize(key: &str, default: usize) -> usize {
    std::env::var(key).ok().and_then(|v| v.parse().ok()).unwrap_or(default)
}

#[test]
fn no_mutation_of_any_known_program_escapes_the_error_model() {
    use support::mutate::{mutate, seed_corpus, Rng};

    let seeds = seed_corpus(&support::repo_root());
    assert!(!seeds.is_empty(), "no .liss files found to mutate");

    let cases = env_usize("LISS_FUZZ_CASES", 300);
    let mut rng = Rng::new(env_usize("LISS_FUZZ_SEED", 0x5EED) as u64);

    let mut violations = Vec::new();
    for n in 0..cases {
        let (label, base) = &seeds[rng.below(seeds.len())];
        let src = mutate(base, &mut rng);
        match judge_source(&format!("mutatio {n} ex {label}"), &src) {
            Verdict::Acceptable => {}
            Verdict::Violation { rule, .. } if RECORDED_RULES.contains(&rule) => {}
            Verdict::Violation { rule, detail } => {
                violations.push(format!("--- {rule}: {detail}\n--- source ---\n{src}\n"));
                if violations.len() >= 5 {
                    break; // five repros is plenty; stop burning time
                }
            }
        }
    }
    assert!(
        violations.is_empty(),
        "{} new robustness violation(s):\n{}",
        violations.len(),
        violations.join("\n")
    );
}
