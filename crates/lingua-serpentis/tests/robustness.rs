//! The robustness invariant (docs/internals/testing.md); every input runs in its own process.
mod support;

use std::ffi::OsStr;
use std::time::Duration;
use support::diag::{
    judge, parse_stderr, Genus, Verdict, RECORDED_RULES,
};
use support::proc::{run_binary, temp_dir, Outcome};

/// The wide deadline: the fuzzer's, and the one curated case whose cost is genuinely codegen's.
const T: Duration = Duration::from_secs(60);

#[test]
fn a_warning_is_not_a_failure_however_it_is_framed() {
    // The grammar recognises errors only. Were a warning to count as one, `judge`
    // would accept a run that warned and never failed as a well-formed failure.
    let (d, reliquum) = parse_stderr("monitum: nomen 'x' fortasse non Latinum est\n");
    assert!(reliquum.is_empty());
    assert_eq!(d.len(), 1);
    assert_eq!(d[0].genus, Genus::Monitum);
    assert!(d.iter().all(|x| x.genus != Genus::Erratum));
}

/// The block grammar, before anything emits it. Nothing else would catch a parser
/// that quietly returned no diagnostics and no residue — which reads as "clean" to
/// every rule `judge` has.
#[test]
fn the_block_grammar_parses_a_header_a_location_and_a_snippet() {
    let stderr = "\
erratum: variabilis ignota: 'y'
  in 'principale.liss', linea III
     |
 III | ni x = y + °I;
     |        ^
";
    let (d, reliquum) = parse_stderr(stderr);
    assert!(reliquum.is_empty(), "framing must not be residue: {reliquum:?}");
    assert_eq!(d.len(), 1);
    assert_eq!(d[0].genus, Genus::Erratum);
    assert_eq!(d[0].file.as_deref(), Some("principale.liss"));
    assert_eq!(d[0].line, Some(3));
    assert_eq!(d[0].message, "variabilis ignota: 'y'");
    let p = d[0].pictura.as_ref().expect("a snippet");
    assert_eq!(p.textus, "ni x = y + °I;");
    assert_eq!(p.caret, 7);
    assert_eq!(p.latitudo, 1);
    assert_eq!(p.textus.chars().nth(p.caret), Some('y'));
}

#[test]
fn the_block_grammar_parses_the_shapes_that_carry_less() {
    let (d, reliquum) = parse_stderr(
        "erratum: munus 'initium' in programmate non inventum\nmonitum: nomen fasciculi 'foobar' fortasse non Latinum est\n",
    );
    assert!(reliquum.is_empty());
    assert_eq!(d.len(), 2);
    assert_eq!(d[0].genus, Genus::Erratum);
    assert_eq!((d[0].file.as_deref(), d[0].line), (None, None));
    assert_eq!(d[1].genus, Genus::Monitum);
    assert!(d[1].pictura.is_none());

    let (d, _) = parse_stderr("erratum: chorda non clausa\n  in linea IV\n");
    assert_eq!((d[0].file.as_deref(), d[0].line), (None, Some(4)));
}

#[test]
fn three_blocks_in_a_row_are_three_diagnostics() {
    // `monita/ordo-trans-scidas`, in the new grammar: nothing separates the blocks
    // but the column-0 severity word, which is what the parser keys on.
    let (d, reliquum) = parse_stderr(
        "monitum: nomen fasciculi 'foobar' fortasse non Latinum est
monitum: nomen 'result' fortasse non Latinum est
  in 'principale.liss', linea III
     |
 III | ni result = °I;
     |    ^^^^^^
monitum: nomen 'answer' fortasse non Latinum est
  in 'foobar/x.liss', linea II
",
    );
    assert!(reliquum.is_empty(), "{reliquum:?}");
    assert_eq!(d.len(), 3);
    assert_eq!(d[1].pictura.as_ref().unwrap().latitudo, 6);
    assert_eq!(d[2].line, Some(2));
}

#[test]
fn a_line_belonging_to_no_block_is_residue_and_not_silence() {
    let (d, reliquum) = parse_stderr("thread 'main' panicked at src/lib.rs:1\n");
    assert!(d.is_empty());
    assert_eq!(reliquum, vec!["thread 'main' panicked at src/lib.rs:1"]);
}

#[test]
fn a_caret_past_the_end_of_its_line_is_a_violation() {
    // The rule exists because nothing else notices a renderer counting columns in
    // bytes: `°` is two bytes and one column, so a byte-counted caret drifts right.
    let outcome = Outcome::Exited {
        code: 1,
        stdout: String::new(),
        stderr: "\
erratum: res mala
  in 'fons.liss', linea I
   |
 I | °I;
   |      ^
"
        .to_string(),
    };
    match judge(&outcome, &[("fons.liss".to_string(), "°I;".to_string())]) {
        Verdict::Violation { rule: "caret aligns", .. } => {}
        other => panic!("a caret past the line must be a violation, got {other:?}"),
    }
}

#[test]
fn an_arabic_line_number_in_the_location_row_is_a_violation() {
    let outcome = Outcome::Exited {
        code: 1,
        stdout: String::new(),
        stderr: "erratum: res mala\n  in 'fons.liss', linea 3\n".to_string(),
    };
    match judge(&outcome, &[("fons.liss".to_string(), "a\nb\nc\n".to_string())]) {
        // `3` is not a numeral, so it never becomes a line, and the row is framing
        // no rule recognises — which is the residue finding, not a silent pass.
        Verdict::Violation { rule, .. } => {
            assert!(
                rule == "numeral round-trips" || rule == "diagnostic grammar",
                "got {rule:?}"
            );
        }
        other => panic!("an Arabic linea must be a violation, got {other:?}"),
    }
}

#[test]
fn a_child_killed_by_a_signal_is_an_abort_however_politely_it_is_reported() {
    let outcome = Outcome::Exited {
        code: 1,
        stdout: String::new(),
        stderr: "erratum: programma abrupte cessavit\n".to_string(),
    };
    match judge(&outcome, &[("fons.liss".to_string(), "munus initium reddit ni { redde nullus; }".to_string())]) {
        Verdict::Violation { rule: "no abort", .. } => {}
        other => panic!("§8 has no fuzzed enforcement: {other:?}"),
    }
    let ordinarium = Outcome::Exited {
        code: 1,
        stdout: String::new(),
        stderr: "erratum: acervus vocationum exhaustus est\n".to_string(),
    };
    assert_eq!(
        judge(&ordinarium, &[("fons.liss".to_string(), "munus initium reddit ni { redde nullus; }".to_string())]),
        Verdict::Acceptable,
    );
}

#[test]
fn the_compilers_own_failures_are_violations_and_not_diagnostics() {
    let fons =
        || vec![("fons.liss".to_string(), "munus initium reddit ni { redde nullus; }".to_string())];
    for (nuntius, regula) in [
        ("defectus internus: munus componi non potuit", "no internal defect"),
        ("defectus internus: machina ignota est", "no internal defect"),
        ("memoria non liberata est", "no leak"),
    ] {
        let outcome = Outcome::Exited {
            code: 1,
            stdout: String::new(),
            stderr: format!("erratum: {nuntius}\n"),
        };
        match judge(&outcome, &fons()) {
            Verdict::Violation { rule, .. } if rule == regula => {}
            other => panic!("{nuntius:?} must break {regula:?}, got {other:?}"),
        }
    }

    let ordinarium = Outcome::Exited {
        code: 1,
        stdout: String::new(),
        stderr: "erratum: memoria exhausta est\n".to_string(),
    };
    assert_eq!(judge(&ordinarium, &fons()), Verdict::Acceptable);
}

fn judge_source(name: &str, src: &str) -> Verdict {
    judge_source_intra(name, src, T)
}

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

/// The tight deadline: it is data, and `T` applied to `nomen longissimum` would retire D-23's guard.
const MORA_ARTA: Duration = Duration::from_secs(5);

/// `(name, source, deadline, expected rule)`; `None` means it must satisfy the invariant.
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
            // D-23: the workspace's only bound on `latinitas::analyze`, hence MORA_ARTA and not T.
            "nomen longissimum",
            format!(
                "munus initium reddit ni {{ ni {} = °I; redde nullus; }}",
                "a".repeat(100_000)
            ),
            MORA_ARTA,
            None,
        ),
        // D-13
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
            "catena numeri longissima",
            format!(
                "munus initium reddit ni {{ scribe(°{}); redde nullus; }}",
                vec!["MMMCMXCIX"; 500].join(".")
            ),
            MORA_ARTA,
            None,
        ),
        // D-20
        (
            "quadratum sine fine",
            "munus initium reddit ni { ni x = °II; indem (verus) { x = x * x; } redde nullus; }"
                .to_string(),
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
            // D-02, and D-03 (fixed; conformance/errores/linea-post-chordam), which this
            // invariant cannot detect: a wrong line inside the file passes `line in range`.
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
                    break;
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
