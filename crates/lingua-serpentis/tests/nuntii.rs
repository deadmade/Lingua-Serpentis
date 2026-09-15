//! The blessing net: what every fixture's stderr *says*, independent of how it is framed.
//!
//! `LISS_BLESS=1` rewrites all 92 `nuntii` files from actual output, and blessing a
//! regression is the one way the conformance corpus fails silently. This snapshot
//! records the `(genus, scida, linea, sententia)` of every diagnostic in the corpus with
//! the framing normalised away, so a change of *shape* leaves it untouched and a change
//! of *sentence* does not.
//!
//! The reader is `support::diag::parse_stderr`, the same one `judge` uses, which makes
//! this a second check on it: a parser that quietly returned nothing would empty the
//! snapshot rather than pass.
//!
//! `docs/superpowers/specs/2026-08-09-caret-diagnostics-design.md` §10.

mod support;

use support::diag::{parse_stderr, Genus};

/// Read from the committed `nuntii` files rather than by running the corpus: this test
/// is about what blessing wrote down, and must not re-derive it from the compiler it is
/// guarding.
#[test]
fn corpus_diagnostics() {
    let mut report = String::new();
    let mut fixtures = 0;
    let mut total = 0;

    for f in support::discover() {
        let stderr = f.expected().stderr;
        if stderr.is_empty() {
            continue;
        }
        fixtures += 1;
        report.push_str(&format!("{}\n", f.name));
        for d in parse_stderr(&stderr).0 {
            total += 1;
            let genus = match d.genus {
                Genus::Erratum => "erratum",
                Genus::Monitum => "monitum",
            };
            let locus = match (&d.file, d.line) {
                (Some(p), Some(l)) => format!("{p}:{l}"),
                (Some(p), None) => p.clone(),
                (None, Some(l)) => format!("linea {l}"),
                (None, None) => "-".to_string(),
            };
            report.push_str(&format!("  {genus}  {locus}  {}\n", d.message));
        }
    }

    let report = format!("{fixtures} fixtures · {total} diagnostics\n\n{report}");
    insta::assert_snapshot!("nuntii", report);
}

/// Every non-empty `nuntii` line must be accounted for — a diagnostic, or framing
/// belonging to one. A line that is neither is either a shape nothing here knows or
/// framing the reader should have consumed, and silently dropping either is how this
/// net would stop catching anything.
#[test]
fn every_line_of_every_nuntii_is_accounted_for() {
    let mut orphans = Vec::new();
    for f in support::discover() {
        let stderr = f.expected().stderr;
        for linea in stderr.lines() {
            if linea.is_empty() {
                orphans.push(format!("{}: an empty line", f.name));
            }
        }
        for linea in parse_stderr(&stderr).1 {
            orphans.push(format!("{}: {linea:?}", f.name));
        }
    }
    assert!(orphans.is_empty(), "unaccounted nuntii lines:\n{}", orphans.join("\n"));
}

/// Every diagnostic in the corpus names its line as a Roman numeral, written the way
/// the language writes it. The corpus is the widest sample of rendered numerals there
/// is, so this is where an off-by-one or a bad glyph would show up first.
#[test]
fn every_line_number_in_the_corpus_is_a_roman_numeral() {
    let mut mala = Vec::new();
    for f in support::discover() {
        for d in parse_stderr(&f.expected().stderr).0 {
            let (Some(numeralis), Some(line)) = (&d.numeralis, d.line) else { continue };
            let expected = lingua_serpentis::roman::Roman::from_usize(line).display();
            if *numeralis != expected {
                mala.push(format!("{}: {numeralis:?} is not {expected:?}", f.name));
            }
        }
    }
    assert!(mala.is_empty(), "{}", mala.join("\n"));
}
