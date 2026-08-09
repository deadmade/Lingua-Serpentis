//! The blessing net: what every fixture's stderr *says*, independent of how it is framed.
//!
//! `LISS_BLESS=1` rewrites all 86 `nuntii` files from actual output, and blessing a
//! regression is the one way the conformance corpus fails silently. This snapshot
//! records the `(genus, scida, linea, sententia)` of every diagnostic in the corpus with
//! the framing normalised away, so a change of *shape* leaves it untouched and a change
//! of *sentence* does not.
//!
//! `docs/superpowers/specs/2026-08-09-caret-diagnostics-design.md` §10.

mod support;

/// Error or warning. The two are `Error`/`Monitum` in the old grammar and
/// `erratum`/`monitum` in the new; this collapses them so one snapshot spans both.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Genus {
    Erratum,
    Monitum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Sententia {
    genus: Genus,
    scida: Option<String>,
    linea: Option<usize>,
    nuntius: String,
}

/// `Error in 'x.liss', linea 3: …` and its three siblings, for each of the two words.
fn vetus(linea: &str) -> Option<Sententia> {
    let (genus, reliquum) = match linea.strip_prefix("Error") {
        Some(r) => (Genus::Erratum, r),
        None => (Genus::Monitum, linea.strip_prefix("Monitum")?),
    };
    if let Some(m) = reliquum.strip_prefix(": ") {
        return Some(Sententia { genus, scida: None, linea: None, nuntius: m.to_string() });
    }
    let reliquum = reliquum.strip_prefix(" in ")?;
    if let Some(m) = reliquum.strip_prefix("linea ") {
        let (n, nuntius) = m.split_once(": ")?;
        return Some(Sententia {
            genus,
            scida: None,
            linea: Some(n.parse().ok()?),
            nuntius: nuntius.to_string(),
        });
    }
    let reliquum = reliquum.strip_prefix('\'')?;
    if let Some((scida, m)) = reliquum.split_once("', linea ") {
        let (n, nuntius) = m.split_once(": ")?;
        return Some(Sententia {
            genus,
            scida: Some(scida.to_string()),
            linea: Some(n.parse().ok()?),
            nuntius: nuntius.to_string(),
        });
    }
    let (scida, nuntius) = reliquum.split_once("': ")?;
    Some(Sententia {
        genus,
        scida: Some(scida.to_string()),
        linea: None,
        nuntius: nuntius.to_string(),
    })
}

/// Every diagnostic in one fixture's stderr, framing discarded.
///
/// Only lines that open a diagnostic are read. Under the block grammar the rest of a
/// diagnostic is a location row, a gutter row and echoed source, none of which is a
/// sentence the language wrote — and echoed source is the one place a line of a *program*
/// could otherwise be mistaken for one.
fn sententiae(stderr: &str) -> Vec<Sententia> {
    stderr.lines().filter_map(vetus).collect()
}

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
        for s in sententiae(&stderr) {
            total += 1;
            let genus = match s.genus {
                Genus::Erratum => "erratum",
                Genus::Monitum => "monitum",
            };
            let locus = match (&s.scida, s.linea) {
                (Some(p), Some(l)) => format!("{p}:{l}"),
                (Some(p), None) => p.clone(),
                (None, Some(l)) => format!("linea {l}"),
                (None, None) => "-".to_string(),
            };
            report.push_str(&format!("  {genus}  {locus}  {}\n", s.nuntius));
        }
    }

    let report = format!("{fixtures} fixtures · {total} diagnostics\n\n{report}");
    insta::assert_snapshot!("nuntii", report);
}

/// Every non-empty `nuntii` line must parse. A line that does not is either a diagnostic
/// in a shape nothing here knows, or framing that `sententiae` should have skipped — and
/// silently dropping either is how this net would stop catching anything.
#[test]
fn every_line_of_every_nuntii_is_accounted_for() {
    let mut orphans = Vec::new();
    for f in support::discover() {
        for linea in f.expected().stderr.lines() {
            if linea.is_empty() {
                orphans.push(format!("{}: an empty line", f.name));
            } else if vetus(linea).is_none() {
                orphans.push(format!("{}: {linea:?}", f.name));
            }
        }
    }
    assert!(orphans.is_empty(), "unparsed nuntii lines:\n{}", orphans.join("\n"));
}
