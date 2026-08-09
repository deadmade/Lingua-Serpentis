//! The robustness invariant `robustness.rs` and `generative.rs` check against.
#![allow(dead_code)]

use super::proc::Outcome;

#[derive(Debug, Clone, PartialEq)]
pub enum Diagnostic {
    /// `Error: <message>`
    Structural { message: String },
    /// `Error in linea N: <message>`
    Line { line: usize, message: String },
    /// `Error in '<file>', linea N: <message>`
    FileLine { file: String, line: usize, message: String },
}

impl Diagnostic {
    pub fn message(&self) -> &str {
        match self {
            Diagnostic::Structural { message }
            | Diagnostic::Line { message, .. }
            | Diagnostic::FileLine { message, .. } => message,
        }
    }
    pub fn line(&self) -> Option<usize> {
        match self {
            Diagnostic::Structural { .. } => None,
            Diagnostic::Line { line, .. } | Diagnostic::FileLine { line, .. } => Some(*line),
        }
    }
}

/// `None` when the line does not match the grammar at all — itself a finding.
pub fn parse_diagnostic(s: &str) -> Option<Diagnostic> {
    let rest = s.strip_prefix("Error")?;
    if let Some(m) = rest.strip_prefix(": ") {
        return Some(Diagnostic::Structural { message: m.to_string() });
    }
    let rest = rest.strip_prefix(" in ")?;
    if let Some(m) = rest.strip_prefix("linea ") {
        let (n, msg) = m.split_once(": ")?;
        return Some(Diagnostic::Line { line: n.parse().ok()?, message: msg.to_string() });
    }
    let rest = rest.strip_prefix('\'')?;
    let (file, rest) = rest.split_once("', linea ")?;
    let (n, msg) = rest.split_once(": ")?;
    Some(Diagnostic::FileLine {
        file: file.to_string(),
        line: n.parse().ok()?,
        message: msg.to_string(),
    })
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

    if code == 0 {
        for line in stderr.lines() {
            if !line.starts_with("Monitum") {
                return violation("clean success", format!("exit 0 but stderr has {line:?}"));
            }
        }
        return Verdict::Acceptable;
    }

    if code == 101 {
        return violation("no panic", format!("exit 101; stderr {stderr:?}"));
    }
    if code != 1 {
        // Any other status came from `initium`, which is legitimate.
        return Verdict::Acceptable;
    }

    let Some(last) = stderr.lines().last() else {
        return violation("diagnostic present", "exit 1 with empty stderr".into());
    };
    let Some(d) = parse_diagnostic(last) else {
        return violation("diagnostic grammar", format!("unparseable: {last:?}"));
    };

    // These three are well-formed Latin, and violations for what they report.
    if d.message() == ABRUPTE_CESSAVIT {
        return violation("no abort", format!("the child was killed by a signal: {last:?}"));
    }
    if d.message().starts_with(DEFECTUS_INTERNUS) {
        return violation("no internal defect", format!("the compiler reported a bug: {last:?}"));
    }
    if d.message() == MEMORIA_NON_LIBERATA {
        return violation("no leak", format!("the live-allocation count was nonzero: {last:?}"));
    }

    if let Diagnostic::FileLine { file, .. } = &d {
        if !sources.iter().any(|(p, _)| p == file) {
            return violation("file exists", format!("names {file:?}, not in the program"));
        }
    }
    if let Some(line) = d.line() {
        let src = match &d {
            Diagnostic::FileLine { file, .. } => {
                sources.iter().find(|(p, _)| p == file).map(|(_, s)| s.as_str())
            }
            _ => sources.first().map(|(_, s)| s.as_str()),
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

    let msg = d.message();
    if msg.is_empty() {
        return violation("message non-empty", format!("{last:?}"));
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
