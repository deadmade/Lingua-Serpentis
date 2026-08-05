//! The catalogue of every diagnostic the source can emit, and whether a
//! fixture produces it. A new message added without a fixture changes the
//! snapshot and fails review — which is what keeps this suite from rotting.
mod support;

use std::path::Path;

/// Markers that introduce a user-facing message. `ProgramError::in_file` is
/// deliberately absent: it re-wraps an existing error rather than producing one.
///
/// The last three exist because `roman.rs` is the one module whose errors are
/// bare `String`s, wrapped into a `LinguaError` only later at `lexer.rs:40`.
/// Its six Latin diagnostics carry none of the first five markers, so without
/// these they would be invisible to the ratchet — a new Roman-numeral message
/// could be added with no fixture and nothing would change. They are exact:
/// between them they match those six sites and nothing else in the workspace.
const MARKERS: [&str; 8] = [
    "LinguaError {",
    ".err(",
    "ProgramError::structural(",
    "Warning {",
    "eprintln!(",
    "Err(\"",
    ".ok_or(\"",
    "ok_or_else(|| format!(",
];

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Site {
    file: String,
    line: usize,
    /// The format string as written, e.g. `variabilis ignota: '{}'`.
    pattern: String,
}

/// Source with its `#[cfg(test)]` module removed. Every file in this workspace
/// has at most one, and it always runs to EOF, so truncating is exact.
fn production_source(path: &Path) -> String {
    let text = std::fs::read_to_string(path).unwrap();
    match text.find("\n#[cfg(test)]\n") {
        Some(i) => text[..i + 1].to_string(),
        None => text,
    }
}

/// Byte offset of the first `"` in `hay` that opens a Rust string literal.
/// A `"` written as the char literal `'"'` is skipped: it is Rust source about
/// quotes, not a message. Without this the `°`-parsing site at `lexer.rs:40`,
/// which forwards a message built elsewhere and so has no literal of its own,
/// lifts `chars[i] != '"'` out of the *next* match arm and reports four lines
/// of unrelated code as its "pattern".
fn opening_quote(hay: &str) -> Option<usize> {
    let b = hay.as_bytes();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'"' {
            let char_literal = i >= 1 && b[i - 1] == b'\'' && b.get(i + 1) == Some(&b'\'');
            if !char_literal {
                return Some(i);
            }
            i += 2;
            continue;
        }
        i += 1;
    }
    None
}

/// The first string literal after `from`, within `window` bytes and before the
/// next marker. Returns None when the site has no literal of its own.
fn literal_after(src: &str, from: usize, window: usize) -> Option<String> {
    // The window is a byte count, so it can land inside a multi-byte character
    // — `°` and the Latin text are not all ASCII. Grow to the next boundary
    // rather than panicking on the slice.
    let mut end = src.len().min(from + window);
    while !src.is_char_boundary(end) {
        end += 1;
    }
    let hay = &src[from..end];
    let stop = MARKERS.iter().filter_map(|m| hay.find(m)).min().unwrap_or(hay.len());
    let hay = &hay[..stop];
    let start = opening_quote(hay)? + 1;
    let rest = &hay[start..];
    let mut out = String::new();
    let mut chars = rest.chars();
    while let Some(c) = chars.next() {
        match c {
            '"' => return Some(out),
            '\\' => {
                if let Some(n) = chars.next() {
                    out.push(match n {
                        'n' => '\n',
                        't' => '\t',
                        other => other,
                    });
                }
            }
            _ => out.push(c),
        }
    }
    None
}

fn extract_sites() -> Vec<Site> {
    let root = support::repo_root();
    let mut files: Vec<_> = ["lingua-serpentis", "latinitas"]
        .iter()
        .flat_map(|c| {
            std::fs::read_dir(root.join("crates").join(c).join("src"))
                .unwrap()
                .map(|e| e.unwrap().path())
        })
        .filter(|p| p.extension().is_some_and(|x| x == "rs"))
        .collect();
    files.sort();

    let mut sites = Vec::new();
    for path in files {
        let src = production_source(&path);
        let rel = path.strip_prefix(&root).unwrap().display().to_string();
        for marker in MARKERS {
            let mut at = 0;
            while let Some(i) = src[at..].find(marker) {
                let abs = at + i;
                let line = src[..abs].lines().count();
                // A marker that ends in the literal's own opening quote must
                // not consume it, or the scan starts inside the message.
                let skip = marker.len() - usize::from(marker.ends_with('"'));
                let pattern = literal_after(&src, abs + skip, 300)
                    .unwrap_or_else(|| "UNPARSED".to_string());
                sites.push(Site { file: rel.clone(), line, pattern });
                at = abs + marker.len();
            }
        }
    }
    sites.sort();
    sites.dedup();
    sites
}

/// Does `observed` match the format string `pattern`? Compares the literal
/// segments between `{...}` holes, in order, anchored at both ends unless the
/// pattern begins or ends with a hole.
///
/// The anchors matter. Searching for each segment with an unanchored `find`
/// alone makes any message that is a *prefix* of a longer one match it: the
/// interpreter's `'longitudo' seriem expectat` would be reported as fixtured
/// by the checker's `'longitudo' seriem expectat, non 'ni'`, and its
/// `'{}' non est series` by the checker's `'x' non est series (typus 'ni')`.
/// Both are distinct diagnostics from distinct phases, and neither is
/// reachable, so counting them as covered would be padding.
fn matches(pattern: &str, observed: &str) -> bool {
    let mut segments = Vec::new();
    let mut rest = pattern;
    loop {
        match rest.find('{') {
            None => {
                segments.push(rest);
                break;
            }
            Some(i) => {
                segments.push(&rest[..i]);
                match rest[i..].find('}') {
                    None => break,
                    Some(j) => rest = &rest[i + j + 1..],
                }
            }
        }
    }
    // A leading or trailing hole leaves an empty segment, and both anchors are
    // vacuously true for it — which is exactly the wanted behaviour.
    if !observed.starts_with(segments.first().copied().unwrap_or("")) {
        return false;
    }
    if !observed.ends_with(segments.last().copied().unwrap_or("")) {
        return false;
    }
    let mut cursor = 0;
    for seg in segments {
        if seg.is_empty() {
            continue;
        }
        match observed[cursor..].find(seg) {
            Some(k) => cursor += k + seg.len(),
            None => return false,
        }
    }
    true
}

/// Every message the corpus actually produces, from both streams.
fn observed_messages() -> Vec<String> {
    let mut out = Vec::new();
    for f in support::discover() {
        let r = support::run_fixture(&f);
        for line in r.stderr.lines() {
            let msg = line
                .split_once(": ")
                .map(|(_, m)| m.to_string())
                .unwrap_or_else(|| line.to_string());
            out.push(msg);
        }
    }
    out.sort();
    out.dedup();
    out
}

#[test]
fn catalogus() {
    let sites = extract_sites();
    let observed = observed_messages();

    let mut covered = 0;
    let mut uncovered = 0;
    let mut unparsed = 0;
    let mut lines = Vec::new();
    for s in &sites {
        let status = if s.pattern == "UNPARSED" {
            unparsed += 1;
            "UNPARSED".to_string()
        } else if observed.iter().any(|o| matches(&s.pattern, o)) {
            covered += 1;
            "fixtured".to_string()
        } else {
            uncovered += 1;
            "UNCOVERED".to_string()
        };
        lines.push(format!("{}:{}  {}  → {status}", s.file, s.line, s.pattern));
    }

    let report = format!(
        "{} sites · {covered} fixtured · {uncovered} uncovered · {unparsed} unparsed\n\n{}",
        sites.len(),
        lines.join("\n")
    );
    insta::assert_snapshot!("catalogus", report);
}
