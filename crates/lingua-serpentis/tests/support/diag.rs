//! The shape every diagnostic must have, and the rules it must satisfy. This
//! is the specification the fuzzer checks against.
#![allow(dead_code)]

use super::proc::Outcome;

#[derive(Debug, Clone, PartialEq)]
pub enum Diagnostic {
    /// `Error: <message>` — structural, about the program's shape.
    Structural { message: String },
    /// `Error in linea N: <message>` — single-source mode.
    Line { line: usize, message: String },
    /// `Error in '<file>', linea N: <message>` — folder mode.
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

/// Parse one rendered diagnostic line. Returns `None` when the line does not
/// match the grammar at all — which is itself a finding.
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
    // `'<file>', linea N: <message>`
    let rest = rest.strip_prefix('\'')?;
    let (file, rest) = rest.split_once("', linea ")?;
    let (n, msg) = rest.split_once(": ")?;
    Some(Diagnostic::FileLine {
        file: file.to_string(),
        line: n.parse().ok()?,
        message: msg.to_string(),
    })
}

/// Rust `TokenKind` variant names. None of these may ever reach a user.
pub const RUST_TYPE_NAMES: [&str; 24] = [
    "Semicolon", "RParen", "LParen", "RBrace", "LBrace", "RBracket", "LBracket",
    "Assign", "Comma", "Dot", "Ident", "TyInt", "TyFloat", "TyDouble", "TyText",
    "TyChar", "TyBool", "Takes", "Yields", "Lista", "Import", "Percent", "Star", "Slash",
];

/// Words that betray an untranslated English string.
pub const ENGLISH_WORDS: [&str; 8] = [
    "file", "must", "have", "extension", "reading", "directory", "invalid", "expected",
];

/// Rules whose violations are already in the defect register. A violation of
/// any OTHER rule is a new finding and fails the test that saw it. Shrinking
/// this array is how progress gets measured — never add to it without adding
/// a register entry too.
pub const RECORDED_RULES: [&str; 3] = [
    "terminates", // D-20
    "no panic",   // D-06
    "Latin only", // D-11, D-22
];

// Three entries have been retired, and none by weakening its rule:
//
// `no abort` (D-04, D-05) — the parser and the interpreter each carry a depth
// limit now, so neither unbounded recursion nor deep nesting can overflow the
// stack. Both defects had to go together: one rule covered them both.
//
// `no Rust type names` (D-02) — `parser.rs`'s `eat` was the only site in the
// workspace that formatted a `TokenKind` with `{:?}`, and it now names the
// token through `TokenKind::name`.
//
// `no Arabic digits` (D-12) — `index extra fines` interpolated the list length
// as a bare `usize`; it renders through `Roman::display` now, so no message in
// the language carries an Arabic digit outside a quoted span.
//
// Both underlying checks stay in `judge`. They are what keep these fixed.

/// A run either satisfies the invariant or names the rule it broke. The
/// mapping from rule to recorded defect lives in
/// `docs/defectus/2026-08-03-index.md`, not here, so this file stays a
/// specification rather than a bug list — and so one rule can cover several
/// defects (both stack overflows break `no abort`).
#[derive(Debug, Clone, PartialEq)]
pub enum Verdict {
    /// Ran to completion, or failed with a well-formed Latin diagnostic.
    Acceptable,
    Violation { rule: &'static str, detail: String },
}

/// Strip quoted spans. They are user input echoed back — an identifier, a
/// numeral, a character — and the lexical rules below are about text the
/// *language* wrote. Without this, `signum ignotum: '5'` would trip the
/// no-Arabic-digits rule for quoting the user's own `5` back at them, and
/// `variabilis ignota: 'Semicolon'` would trip the Rust-type-name rule for a
/// user who named a variable `Semicolon`.
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

/// Judge one run against the invariant. `sources` maps program-relative path
/// to source text, so line numbers can be range-checked.
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
    // English before digits: `Error: file must have .liss extension` should be
    // reported as an untranslated string, not as a stray numeral.
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
