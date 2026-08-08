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

/// The per-input deadline the `terminates` rule is judged against.
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

/// Run one single-file source through the binary and judge it.
fn judge_source(name: &str, src: &str) -> Verdict {
    let d = temp_dir("liss-rob");
    let p = d.write("fons.liss", src);
    let outcome = run_binary(&[OsStr::new(p.to_str().unwrap())], T);
    let verdict = judge(&outcome, &[("fons.liss".to_string(), src.to_string())]);
    if let Verdict::Violation { rule, detail } = &verdict {
        eprintln!("[{name}] {rule}: {detail}");
    }
    verdict
}

/// Every curated pathology with the rule it is known to break — `None` means
/// it must satisfy the invariant. The rule, not a defect ID: both stack
/// overflows break `no abort`, and the register is what maps rules to IDs.
fn pathologies() -> Vec<(&'static str, String, Option<&'static str>)> {
    vec![
        ("vacuum", String::new(), None),
        ("spatia sola", "   \n\t\n".to_string(), None),
        ("BOM", "\u{feff}munus initium reddit ni { redde nullus; }".to_string(), None),
        (
            "CRLF",
            "munus initium reddit ni {\r\n  scribe(°I);\r\n  redde nullus;\r\n}".to_string(),
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
            // stays in the sweep as the regression guard for that bound.
            "nomen longissimum",
            format!(
                "munus initium reddit ni {{ ni {} = °I; redde nullus; }}",
                "a".repeat(100_000)
            ),
            None,
        ),
        (
            "numerus longissimus",
            format!(
                "munus initium reddit ni {{ scribe(°{}); redde nullus; }}",
                "M".repeat(5_000)
            ),
            None,
        ),
        (
            "series longissima",
            format!(
                "munus initium reddit ni {{ ni lista xs = [{}°I]; scribe(longitudo(xs)); redde nullus; }}",
                "°I, ".repeat(5_000)
            ),
            None,
        ),
        (
            "parentheses nimium implicatae",
            format!(
                "munus initium reddit ni {{ scribe({}°I{}); redde nullus; }}",
                "(".repeat(20_000),
                ")".repeat(20_000)
            ),
            None,
        ),
        (
            "recursio infinita",
            "munus recurre capit (ni n) reddit ni { redde recurre(n + °I); }\n\
             munus initium reddit ni { scribe(recurre(°I)); redde nullus; }"
                .to_string(),
            None,
        ),
        (
            "index extra fines",
            "munus initium reddit ni {\n  ni lista xs = [°I];\n  scribe(xs[°X]);\n  redde nullus;\n}"
                .to_string(),
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
            None,
        ),
    ]
}

#[test]
fn every_curated_pathology_matches_its_recorded_verdict() {
    let mut surprises = Vec::new();
    for (name, src, expected_rule) in pathologies() {
        let verdict = judge_source(name, &src);
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
