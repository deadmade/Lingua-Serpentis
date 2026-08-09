//! The robustness invariant `robustness.rs` and `generative.rs` check against.
#![allow(dead_code)]

use super::proc::Outcome;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Genus {
    Erratum,
    Monitum,
}

/// The snippet rows of one block, as read back off stderr.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pictura {
    /// The numeral in the gutter, exactly as written.
    pub numeralis: String,
    pub linea: usize,
    pub textus: String,
    pub caret: usize,
    pub latitudo: usize,
}

/// One diagnostic, read back off stderr.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostica {
    pub genus: Genus,
    pub file: Option<String>,
    pub line: Option<usize>,
    /// The line number exactly as rendered, so it can be checked against the way
    /// the language writes that number.
    pub numeralis: Option<String>,
    pub message: String,
    pub pictura: Option<Pictura>,
}

/// True for a gutter row: everything left of the first `|` is blank or Roman.
fn est_canalis(s: &str) -> bool {
    let Some(i) = s.find('|') else { return false };
    s[..i].chars().all(|c| c.is_whitespace() || "IVXLCDM\u{0305}".contains(c))
}

/// A Roman numeral back to a number. `parse_roman` is the language's own reader and
/// wants the `°` a diagnostic does not print, so it is restored here.
pub fn numerus(numeralis: &str) -> Option<usize> {
    match lingua_serpentis::roman::parse_roman(&format!("°{numeralis}")) {
        Ok(lingua_serpentis::roman::RomanNumber::Int(r)) => r.to_usize(),
        _ => None,
    }
}

fn romanus(n: usize) -> String {
    lingua_serpentis::roman::Roman::from_usize(n).display()
}

/// Every diagnostic on `stderr`, and every line that belonged to none of them.
///
/// Residue is a finding, not noise: under the block grammar the rows after a header
/// are framing the language wrote, so a line that is neither a header nor framing is
/// something else entirely — a panic, a leaked English sentence, a half-written
/// message.
pub fn parse_stderr(stderr: &str) -> (Vec<Diagnostica>, Vec<String>) {
    let mut diagnostica = Vec::new();
    let mut reliquum = Vec::new();
    let lineae: Vec<&str> = stderr.lines().collect();
    let mut i = 0;

    while i < lineae.len() {
        let linea = lineae[i];

        // The block grammar: a header at column 0, then its framing.
        let caput = linea
            .strip_prefix("erratum: ")
            .map(|m| (Genus::Erratum, m))
            .or_else(|| linea.strip_prefix("monitum: ").map(|m| (Genus::Monitum, m)));
        let Some((genus, message)) = caput else {
            reliquum.push(linea.to_string());
            i += 1;
            continue;
        };
        i += 1;

        let mut file = None;
        let mut numeralis = None;
        if let Some(locus) = lineae.get(i).and_then(|l| l.strip_prefix("  in ")) {
            i += 1;
            let (f, n) = block_locus(locus);
            file = f;
            numeralis = n;
        }

        let mut canales = Vec::new();
        while i < lineae.len() && est_canalis(lineae[i]) {
            canales.push(lineae[i]);
            i += 1;
        }

        diagnostica.push(Diagnostica {
            genus,
            file,
            line: numeralis.as_deref().and_then(numerus),
            numeralis,
            message: message.to_string(),
            pictura: pictura(&canales),
        });
    }

    (diagnostica, reliquum)
}

/// `'x.liss', linea III` and its three siblings, minus the leading `  in `.
fn block_locus(s: &str) -> (Option<String>, Option<String>) {
    if let Some(n) = s.strip_prefix("linea ") {
        return (None, Some(n.to_string()));
    }
    let Some(rest) = s.strip_prefix('\'') else { return (None, None) };
    if let Some((file, n)) = rest.split_once("', linea ") {
        return (Some(file.to_string()), Some(n.to_string()));
    }
    (rest.strip_suffix('\'').map(str::to_string), None)
}

/// The source row and the caret row, read back out of the gutter.
fn pictura(canales: &[&str]) -> Option<Pictura> {
    let fons = canales.iter().find(|l| l.find('|').is_some_and(|b| !l[..b].trim().is_empty()))?;
    let barra = fons.find('|')?;
    let numeralis = fons[..barra].trim().to_string();
    let linea = numerus(&numeralis)?;
    // `| ` — one space of padding — then the source, verbatim.
    let textus = fons.get(barra + 2..).unwrap_or("").to_string();

    let cursus = canales.iter().find(|l| {
        l.find('|').is_some_and(|b| l[b + 1..].contains('^'))
    })?;
    let barra = cursus.find('|')?;
    let post = &cursus[barra + 2..];
    let caret = post.chars().take_while(|c| *c == ' ').count();
    let latitudo = post.chars().filter(|c| *c == '^').count();

    Some(Pictura { numeralis, linea, textus, caret, latitudo })
}

pub const RUST_TYPE_NAMES: [&str; 24] = [
    "Semicolon", "RParen", "LParen", "RBrace", "LBrace", "RBracket", "LBracket",
    "Assign", "Comma", "Dot", "Ident", "TyInt", "TyFloat", "TyDouble", "TyText",
    "TyChar", "TyBool", "Takes", "Yields", "Lista", "Import", "Percent", "Star", "Slash",
];

pub const ENGLISH_WORDS: [&str; 8] = [
    "file", "must", "have", "extension", "reading", "directory", "invalid", "expected",
];

/// Violations the suite tolerates; never add one without a defect-register entry.
pub const RECORDED_RULES: [&str; 1] = [
    "terminates", // D-20
];

const ABRUPTE_CESSAVIT: &str = "programma abrupte cessavit";

const DEFECTUS_INTERNUS: &str = "defectus internus: ";

const MEMORIA_NON_LIBERATA: &str = "memoria non liberata est";

#[derive(Debug, Clone, PartialEq)]
pub enum Verdict {
    Acceptable,
    Violation { rule: &'static str, detail: String },
}

/// Quoted spans are user input echoed back; the lexical rules are about text the language wrote.
fn without_quoted(msg: &str) -> String {
    let mut out = String::new();
    let mut quote: Option<char> = None;
    for c in msg.chars() {
        match quote {
            Some(q) if c == q => quote = None,
            Some(_) => {}
            None if c == '\'' || c == '"' => quote = Some(c),
            None => out.push(c),
        }
    }
    out
}

/// `sources` maps program-relative path to source text, so line numbers can be range-checked.
pub fn judge(outcome: &Outcome, sources: &[(String, String)]) -> Verdict {
    let violation = |rule: &'static str, detail: String| Verdict::Violation { rule, detail };

    let (code, stderr) = match outcome {
        Outcome::TimedOut => return violation("terminates", "still running at the deadline".into()),
        Outcome::Signalled { signal, .. } => {
            return violation("no abort", format!("killed by signal {signal}"))
        }
        Outcome::Exited { code, stderr, .. } => (*code, stderr.as_str()),
    };

    let (diagnostica, reliquum) = parse_stderr(stderr);

    if code == 0 {
        // Not "every line starts with Monitum": a warning's location, gutter and
        // caret rows do not, and are not lines the reader was meant to parse.
        if let Some(l) = reliquum.first() {
            return violation("clean success", format!("exit 0 but stderr has {l:?}"));
        }
        if let Some(d) = diagnostica.iter().find(|d| d.genus == Genus::Erratum) {
            return violation("clean success", format!("exit 0 but stderr has {:?}", d.message));
        }
        return picturae_congruunt(&diagnostica).unwrap_or(Verdict::Acceptable);
    }

    if code == 101 {
        return violation("no panic", format!("exit 101; stderr {stderr:?}"));
    }
    if code != 1 {
        // Any other status came from `initium`, which is legitimate.
        return Verdict::Acceptable;
    }

    if let Some(l) = reliquum.first() {
        return violation("diagnostic grammar", format!("unparseable: {l:?}"));
    }
    let Some(d) = diagnostica.iter().rev().find(|d| d.genus == Genus::Erratum) else {
        return violation("diagnostic present", "exit 1 with no diagnostic".into());
    };

    // These three are well-formed Latin, and violations for what they report.
    if d.message == ABRUPTE_CESSAVIT {
        return violation("no abort", format!("the child was killed by a signal: {stderr:?}"));
    }
    if d.message.starts_with(DEFECTUS_INTERNUS) {
        return violation("no internal defect", format!("the compiler reported a bug: {stderr:?}"));
    }
    if d.message == MEMORIA_NON_LIBERATA {
        return violation("no leak", format!("the live-allocation count was nonzero: {stderr:?}"));
    }

    if let Some(file) = &d.file {
        if !sources.iter().any(|(p, _)| p == file) {
            return violation("file exists", format!("names {file:?}, not in the program"));
        }
    }
    if let Some(line) = d.line {
        let src = match &d.file {
            Some(file) => sources.iter().find(|(p, _)| p == file).map(|(_, s)| s.as_str()),
            None => sources.first().map(|(_, s)| s.as_str()),
        };
        if let Some(src) = src {
            let n = src.lines().count().max(1);
            if line == 0 || line > n {
                return violation(
                    "line in range",
                    format!("linea {line} but the file has {n} lines"),
                );
            }
        }
    }

    // The location's numeral must be the one the language would have written, which
    // is the check that keeps an Arabic line number out of the location row.
    for d in &diagnostica {
        let Some(numeralis) = &d.numeralis else { continue };
        // An unreadable numeral must fail here rather than leave `line` at `None`
        // and be skipped: `linea 3` parses as no numeral at all, and skipping is
        // exactly how an Arabic line number would survive the rule meant to ban it.
        let Some(line) = d.line else {
            return violation("numeral round-trips", format!("{numeralis:?} is not a numeral"));
        };
        if *numeralis != romanus(line) {
            return violation(
                "numeral round-trips",
                format!("{numeralis:?} is not how {line} is written"),
            );
        }
    }
    if let Some(v) = picturae_congruunt(&diagnostica) {
        return v;
    }

    let msg = &d.message;
    if msg.is_empty() {
        return violation("message non-empty", format!("{stderr:?}"));
    }
    let bare = without_quoted(msg);
    let lower = bare.to_lowercase();
    if let Some(w) = ENGLISH_WORDS
        .iter()
        .find(|w| lower.split(|c: char| !c.is_alphabetic()).any(|t| t == **w))
    {
        return violation("Latin only", format!("{msg:?} contains {w:?}"));
    }
    if bare.chars().any(|c| c.is_ascii_digit()) {
        return violation("no Arabic digits", format!("{msg:?}"));
    }
    if let Some(w) = RUST_TYPE_NAMES.iter().find(|w| bare.contains(**w)) {
        return violation("no Rust type names", format!("{msg:?} contains {w:?}"));
    }
    Verdict::Acceptable
}

/// The only automated check that a caret points where it claims to: the gutter's
/// numeral must be the location's, and the caret must lie within the line it is
/// under — at its end is the insertion point a missing `;` is reported at, past it
/// is a renderer miscounting columns.
fn picturae_congruunt(diagnostica: &[Diagnostica]) -> Option<Verdict> {
    let violation = |rule: &'static str, detail: String| Verdict::Violation { rule, detail };
    for d in diagnostica {
        let Some(p) = &d.pictura else { continue };
        if d.numeralis.as_deref() != Some(p.numeralis.as_str()) {
            return Some(violation(
                "caret aligns",
                format!("gutter says {:?}, location says {:?}", p.numeralis, d.numeralis),
            ));
        }
        let longitudo = p.textus.chars().count();
        if p.caret > longitudo || p.latitudo == 0 || p.caret + p.latitudo > longitudo + 1 {
            return Some(violation(
                "caret aligns",
                format!("caret {} + {} over {:?}", p.caret, p.latitudo, p.textus),
            ));
        }
    }
    None
}
