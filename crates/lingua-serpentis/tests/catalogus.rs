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
    // `liss-runtime` joined the scan when `roman.rs` (four diagnostics) and
    // `exitus.rs` (one) moved out of `lingua-serpentis` — without it those
    // five sites would silently vanish from the count rather than showing up
    // as a loss the ratchet must justify.
    //
    // `liss-abi` joins it on creation, for the same reason and ahead of need:
    // it is where the runtime's diagnostics live from now on, and a crate added
    // later would let a message slip in unrecorded in between. It contributes
    // nothing yet, because the marker that finds its sites — `liss_panic(` — is
    // deliberately **not** in `MARKERS` here.
    //
    // That deferral is the whole point. `liss-abi` re-raises `divisio per
    // nullum`, `modulus per nullum` and `index extra fines: {} (longitudo {})`,
    // which `interp.rs` still raises too. Adding the marker while both
    // implementations exist would make each of those patterns non-unique, and
    // `fixtured` decides on uniqueness — all six sites would read `ambiguous`
    // and the fixtured count would drop by three, recording a *loss* of
    // coverage where nothing was actually lost. The marker lands in Task 19,
    // in the same commit that deletes the interpreter and with it the
    // duplication; spec §10.3 predicts the result there — fewer sites and a
    // higher fixtured fraction.
    //
    // Task 19 will also need a guard for `liss_panic`'s own *definition*,
    // beside the existing one for `-> LinguaError {`: a function that declares
    // a diagnostic entry point is not itself a site that raises one, and
    // without the guard `defectus.rs`'s `pub unsafe extern "C" fn liss_panic(`
    // reads UNPARSED.
    let mut files: Vec<_> = ["lingua-serpentis", "latinitas", "liss-runtime", "liss-abi"]
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
                // `-> LinguaError {` is a return type followed by the function
                // body's brace, not a struct literal — a function that BUILDS a
                // diagnostic is not itself a site that RAISES one, and the real
                // site is the construction inside it, one line below. Without
                // this, every helper introduced to collapse duplicated wording
                // adds a phantom UNPARSED entry directly above the site it
                // exists to consolidate.
                if src[..abs].trim_end().ends_with("->") {
                    at = abs + marker.len();
                    continue;
                }
                let line = src[..abs].lines().count();
                // A marker that ends in the literal's own opening quote must
                // not consume it, or the scan starts inside the message.
                let skip = marker.len() - usize::from(marker.ends_with('"'));
                let pattern = literal_after(&src, abs + skip, 300)
                    .unwrap_or_else(|| "UNPARSED".to_string());
                // A pattern that is nothing but holes has no text of its own,
                // so it cannot identify anything: `eprintln!("{}", e)` prints
                // an error built elsewhere, and matching it against observed
                // messages would match every one of them. Same reasoning that
                // keeps `ProgramError::in_file` out of MARKERS — forwarding a
                // diagnostic is not raising one.
                if pattern.replace("{}", "").trim().is_empty() && pattern != "UNPARSED" {
                    at = abs + marker.len();
                    continue;
                }
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

/// Every message the corpus actually produces. `stderr` only, and that is
/// complete: every diagnostic — errors and Latinity warnings alike — is
/// rendered to `stderr` by `render()`. `stdout` carries only what a program
/// itself printed with `scribe`, which is program output, not a diagnostic.
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

/// How many sites write this exact format string. Uniqueness is decided on the
/// pattern *as written*, byte for byte: two patterns that differ only in where
/// a `{}` hole sits produce different sentences for a user and are therefore
/// different messages, so they must not be merged. Normalising the holes away
/// would collapse genuinely distinct diagnostics and understate coverage in the
/// opposite direction.
fn pattern_counts(sites: &[Site]) -> std::collections::HashMap<&str, usize> {
    let mut counts = std::collections::HashMap::new();
    for s in sites {
        *counts.entry(s.pattern.as_str()).or_insert(0) += 1;
    }
    counts
}

#[test]
fn catalogus() {
    let sites = extract_sites();
    let observed = observed_messages();
    let counts = pattern_counts(&sites);

    let mut covered = 0;
    let mut ambiguous = 0;
    let mut uncovered = 0;
    let mut unparsed = 0;
    let mut lines = Vec::new();
    for s in &sites {
        let seen = observed.iter().any(|o| matches(&s.pattern, o));
        // `fixtured` means *uniquely attributable*, and nothing weaker.
        //
        // Coverage is decided by matching a site's format string against the
        // messages the corpus produced. Nothing ties an observed message back
        // to the site that emitted it, so when several sites are worded
        // identically, provoking any one of them makes all of them look
        // covered. That is not a bug in the corpus, it is inherent to matching
        // on text: the interpreter's three `divisio per nullum` arms are one
        // string, and the message a user sees cannot tell you which fired.
        //
        // So a shared pattern that was observed marks *every* site sharing it
        // `ambiguous`: the corpus proves at least one of them was reached and
        // cannot say which. Only a pattern unique across the whole workspace
        // earns `fixtured`, and a later reviewer can trust that word without
        // checking anything else. A shared pattern nobody provoked is still
        // plainly UNCOVERED — no site sharing it was reached.
        //
        // Adding a fixture never moves a site from `ambiguous` to `fixtured`:
        // the ambiguity is in the wording, not in the coverage. Only rewording
        // one of the sites, or attributing messages to sites some other way,
        // could do that.
        let status = if s.pattern == "UNPARSED" {
            unparsed += 1;
            "UNPARSED"
        } else if !seen {
            uncovered += 1;
            "UNCOVERED"
        } else if counts[s.pattern.as_str()] > 1 {
            ambiguous += 1;
            "ambiguous"
        } else {
            covered += 1;
            "fixtured"
        };
        lines.push(format!("{}:{}  {}  → {status}", s.file, s.line, s.pattern));
    }

    let report = format!(
        "{} sites · {covered} fixtured · {ambiguous} ambiguous · {uncovered} uncovered \
         · {unparsed} unparsed\n\n{}",
        sites.len(),
        lines.join("\n")
    );
    insta::assert_snapshot!("catalogus", report);
}
