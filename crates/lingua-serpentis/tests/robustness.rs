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
use support::proc::{run_binary, temp_dir};

const T: Duration = Duration::from_secs(10);

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
            // Verified 2026-08-03, not as the brief predicted: this times out
            // under the 10s deadline rather than running clean. Root cause is
            // `latinitas::analyze::analyze` (crates/latinitas/src/analyze.rs:31-40),
            // whose `for i in 0..=w.len()` loop hashes a `&w[..i]` slice on every
            // iteration — O(n) per iteration, O(n^2) total in the identifier's
            // length. Measured directly against the binary: 50_000 chars ~6.2s,
            // 100_000 chars ~24s (still terminates, just past this timeout).
            // Recorded as D-23, rule `terminates` (already allowlisted via D-20).
            "nomen longissimum",
            format!(
                "munus initium reddit ni {{ ni {} = °I; redde nullus; }}",
                "a".repeat(100_000)
            ),
            Some("terminates"), // D-23
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
            Some("no abort"), // D-05
        ),
        (
            "recursio infinita",
            "munus recurre capit (ni n) reddit ni { redde recurre(n + °I); }\n\
             munus initium reddit ni { scribe(recurre(°I)); redde nullus; }"
                .to_string(),
            Some("no abort"), // D-04
        ),
        (
            "index extra fines",
            "munus initium reddit ni {\n  ni lista xs = [°I];\n  scribe(xs[°X]);\n  redde nullus;\n}"
                .to_string(),
            Some("no Arabic digits"), // D-12 — `longitudo 1` is Arabic
        ),
        (
            // Verified 2026-08-03, not as the brief predicted. The brief's
            // intent (per its comment, preserved below) was to show D-03's wrong
            // line number staying inside the file, undetectable by `line in
            // range`. But this literal source is missing the semicolon after
            // `ni x = °I`, which is itself a parse error before any D-03-style
            // runtime lookup happens: `Error in linea 4: expectatum: Semicolon`.
            // That trips D-02 (`no Rust type names`), not a clean run. It still
            // incidentally corroborates D-03: the missing semicolon is on source
            // line 5, immediately before `redde` on line 6, but the reported
            // line is 4 — a 2-line drift matching the string's 2 embedded
            // newlines. D-03's own repro (conformance/defectus/D-03-linea-post-
            // chordam) remains the fixture for the line-drift-alone case.
            //
            // D-03 is NOT detectable by this invariant: the reported line is
            // wrong but still inside the file, so `line in range` passes. It is
            // caught by conformance/defectus/D-03-linea-post-chordam instead.
            // Kept here to prove the input at least stays inside the error model.
            "chorda multilinea",
            "munus initium reddit ni {\n  voc s = \"prima\nsecunda\ntertia\";\n  ni x = °I\n  redde nullus;\n}"
                .to_string(),
            Some("no Rust type names"), // D-02, plus D-03's line drift visible in "linea 4"
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
