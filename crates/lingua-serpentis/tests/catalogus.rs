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
/// The last three are the back end's, added in Task 19 with the crates that
/// own them (spec §10.3).
///
/// `liss_panic(` finds the runtime's raises; `Defectus::novus(` finds the
/// compiler's own failures; `NUNTIUS_` finds the signal handler's messages,
/// which are not call sites at all but `static` byte constants with no marker
/// near them — the same situation that already forced `Err("` and `.ok_or("`
/// into this list.
///
/// **The runtime writes its sentences inline at the call site**, never as a
/// reference to a shared table, and §10.3 makes that a rule rather than a
/// habit: a site written `liss_panic(nuntii::DIVISIO_PER_NULLUM.as_ptr(), …)`
/// carries no literal of its own, reports `UNPARSED`, and hides the message
/// from the ratchet entirely.
const MARKERS: [&str; 11] = [
    "LinguaError {",
    ".err(",
    "ProgramError::structural(",
    "Warning {",
    "eprintln!(",
    "Err(\"",
    ".ok_or(\"",
    "ok_or_else(|| format!(",
    "liss_panic(",
    "Defectus::novus(",
    "NUNTIUS_",
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
    // **Every crate that can say a Latin sentence, and the list is now
    // complete.** `liss-runtime` joined when `roman.rs` and `exitus.rs` moved
    // out of `lingua-serpentis`; `liss-abi` joined on creation, ahead of need,
    // so that no message could slip in unrecorded between its birth and its
    // marker; `liss-ir` and `liss-codegen` join here with `Defectus::novus(`.
    //
    // The three back-end markers were deferred to this task on purpose, and the
    // reason has now expired. `liss-abi` re-raises `divisio per nullum`,
    // `modulus per nullum` and `index extra fines: {} (longitudo {})`, and
    // `interp.rs` raised all three too. Adding `liss_panic(` while both
    // implementations stood would have made each pattern non-unique — and
    // `fixtured` decides on uniqueness — so six sites would have read
    // `ambiguous` and the count would have recorded a *loss* of coverage where
    // nothing was lost. The interpreter is gone in this same commit, and with
    // it the duplication.
    //
    // Two duplicate patterns survive, and both are honest. `defectus internus:
    // memoria violata est` is written twice — `defectus.rs` raises it through
    // `liss_panic`, and `custos.rs`'s handler must write it without calling
    // `liss_panic` at all — and `imago non coniuncta est` is written at four
    // link-failure sites in `nexus.rs`. Neither can be provoked by a fixture, so
    // both read UNCOVERED rather than `ambiguous`: the ladder tests `!seen`
    // first, and a shared pattern nobody reached is plainly uncovered.
    let mut files: Vec<_> =
        ["lingua-serpentis", "latinitas", "liss-runtime", "liss-ir", "liss-codegen", "liss-abi"]
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
                //
                // `fn liss_panic(` is the same distinction one crate down, and
                // needs its own test because the marker ends in `(` rather than
                // `{`: a function that DECLARES a diagnostic entry point is not
                // a site that raises one. Without this,
                // `defectus.rs`'s `pub unsafe extern "C" fn liss_panic(` reads
                // UNPARSED — the definition has no literal near it — and the
                // ratchet grows a row for the door rather than for anyone who
                // walked through it.
                let ante = src[..abs].trim_end();
                if ante.ends_with("->") || ante.ends_with("fn") {
                    at = abs + marker.len();
                    continue;
                }
                // **Prose about a diagnostic is not a diagnostic.** The scanner
                // reads whole files, comments included, and that was harmless
                // while every marker was punctuation-heavy enough not to appear
                // in English. `NUNTIUS_` and `Defectus::novus(` are not: the
                // notes explaining them mention them by name, and each mention
                // became a site — one of them reading `Error: ` as its message,
                // quoted out of a sentence explaining why `Error: ` is not a
                // message. Eight phantom UNPARSED rows, and a fixtured fraction
                // that fell because the file documented itself.
                //
                // Only `//` needs handling: nothing in this workspace writes a
                // block comment. A `//` *after* the marker on the same line is
                // an ordinary trailing note and must not count, which is why
                // this looks left from the marker rather than at the whole line.
                let linea = src[..abs].rfind('\n').map_or(0, |i| i + 1);
                if src[linea..abs].contains("//") {
                    at = abs + marker.len();
                    continue;
                }
                // `NUNTIUS_` names a **constant**, and only its declaration is
                // the site. Every *use* — `erratum(NUNTIUS_ACERVUS)` and the
                // two arms that choose between them — carries no literal and
                // would report UNPARSED, turning one message into four rows.
                //
                // This is what makes the marker match spec §10.3's rule rather
                // than merely its spelling: the rule is that every
                // async-signal-safe *message constant* is named `NUNTIUS_*`,
                // and a constant is where it is declared.
                if marker == "NUNTIUS_"
                    && !(ante.ends_with("const") || ante.ends_with("static"))
                {
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

/// **The runtime's literals are not `"…"`, and the scanner must read them
/// anyway.**
///
/// `liss-abi` is `no_std`-shaped and async-signal-safe, so its messages are
/// byte strings and C strings — `b"Error: "`, `c"acervus vocationum exhaustus
/// est"` — never plain `&str`. [`opening_quote`] finds the first `"` that opens
/// a literal, and for both forms that is the quote *after* the prefix letter,
/// so both already work.
///
/// "Already work" is exactly why this test exists. Nothing about the extractor
/// mentions `b` or `c`; it works by accident of where the quote falls, and a
/// future `opening_quote` that anchored on a preceding delimiter, or skipped a
/// quote whose previous byte is a letter, would silently stop reading every
/// message the runtime can say — and the ratchet's own header would not move,
/// because the sites would go UNPARSED rather than disappear.
///
/// The last case is the one that motivates the guard rather than the prefix:
/// a `b"…"` immediately after a marker must still be found.
#[test]
fn byte_and_c_string_literals_are_extracted_like_any_other() {
    let cases = [
        ("NUNTIUS_", r#"const NUNTIUS_X: &[u8] = b"acervus exhaustus";"#, "acervus exhaustus"),
        ("NUNTIUS_", r#"const NUNTIUS_Y: &CStr = c"memoria violata est";"#, "memoria violata est"),
        ("NUNTIUS_", r#"const NUNTIUS_Z: &str = "planus";"#, "planus"),
        // The shape every `liss_panic` site actually ships: a `c"…"` literal
        // taken `.as_ptr()`, with the sentence inline rather than behind a
        // shared constant (§10.3).
        ("liss_panic(", r#"liss_panic(c"divisio per nullum".as_ptr(), locus);"#, "divisio per nullum"),
        // An escaped quote inside the message does not end it.
        ("liss_panic(", r#"liss_panic(c"nomen \"x\" ignotum".as_ptr());"#, "nomen \"x\" ignotum"),
    ];
    for (marker, src, want) in cases {
        // Start where `extract_sites` starts: just past the marker. That
        // matters for `NUNTIUS_`, which is a prefix of the constant's own name,
        // so scanning from zero would stop at the marker before reaching the
        // literal it introduces.
        let from = src.find(marker).expect("the fragment contains its marker") + marker.len();
        assert_eq!(
            literal_after(src, from, 300).as_deref(),
            Some(want),
            "failed to extract from {src:?}",
        );
    }

    // And a char literal standing between the marker and the message is still
    // skipped, which is what `opening_quote` was written for — the byte- and
    // C-string prefixes must not have disturbed it.
    let src = r#"self.err(if c == '"' { "chorda non clausa" } else { "aliud" })"#;
    let from = src.find(".err(").expect("marker") + ".err(".len();
    assert_eq!(literal_after(src, from, 300).as_deref(), Some("chorda non clausa"));
}

/// A function that *declares* a diagnostic entry point is not a site that
/// raises one — the `fn` half of `extract_sites`'s guard.
///
/// `defectus.rs`'s `pub unsafe extern "C" fn liss_panic(` matches the
/// `liss_panic(` marker exactly, and without the guard it reads UNPARSED: the
/// definition has no literal near it. The row would be for the door rather than
/// for anyone who walked through it, and it would depress the fixtured fraction
/// permanently.
#[test]
fn a_diagnostic_entry_points_own_definition_is_not_a_site() {
    let sites = extract_sites();
    let definitio = sites
        .iter()
        .find(|s| s.file.ends_with("defectus.rs") && s.pattern == "UNPARSED");
    assert!(
        definitio.is_none(),
        "`fn liss_panic(`'s definition was counted as a site: {definitio:?}",
    );
    // The guard must not have swallowed the raises the marker exists to find.
    assert!(
        sites.iter().filter(|s| s.file.ends_with("defectus.rs")).count() >= 5,
        "the `liss_panic(` marker found no call sites at all",
    );
}

/// **The comment guard decides whether a diagnostic is visible at all**, so it
/// gets a test of its own rather than being trusted to the snapshot.
///
/// It was added because `NUNTIUS_` and `Defectus::novus(` are the first markers
/// whose own explanatory prose names them, and each mention became a site —
/// including one that extracted `Error: ` as a diagnostic out of a sentence
/// saying `Error: ` is not one. The heuristic is deliberately narrow (`//`
/// to the left of the marker on its own line; no block comments exist in this
/// workspace), and narrow heuristics fail in both directions:
///
/// - too eager, and a **real** site whose line happens to contain `//` earlier
///   — inside a string — vanishes silently, taking its message off the ratchet
///   with the header still reading the same total;
/// - too shy, and the phantom rows come back.
///
/// Both directions are asserted here.
#[test]
fn a_marker_in_a_comment_is_prose_and_a_marker_in_code_is_not() {
    let sites = extract_sites();

    // Too shy: the notes in `catalogus.rs`'s own scanned crates that mention
    // `NUNTIUS_` and `Defectus::novus(` by name must not be sites. The
    // giveaway row was `Error: `, quoted in `custos.rs`'s note about the
    // renamed prefix constant.
    assert!(
        !sites.iter().any(|s| s.pattern == "Error: "),
        "a comment quoting `Error: ` was read as a diagnostic",
    );

    // Too eager: `custos.rs` really does declare two `NUNTIUS_*` messages, and
    // both must still be found.
    for want in ["acervus vocationum exhaustus est", "defectus internus: memoria violata est"] {
        assert!(
            sites.iter().any(|s| s.file.ends_with("custos.rs") && s.pattern == want),
            "the `NUNTIUS_` declaration for {want:?} was skipped",
        );
    }

    // And the guard is keyed on `//` *before* the marker, not anywhere on the
    // line: a trailing note after a real site must not erase it.
    let src = "    eprintln!(\"Error: machina ignota est\"); // and exit\n";
    assert_eq!(
        literal_after(src, src.find("eprintln!(").expect("marker") + "eprintln!(".len(), 300)
            .as_deref(),
        Some("Error: machina ignota est"),
    );
}

/// A `NUNTIUS_*` constant is a site where it is **declared**, and nowhere else.
///
/// `custos.rs` uses each of its two message constants two or three times —
/// `erratum(NUNTIUS_ACERVUS)` and the arms that choose between them — and each
/// use carries no literal, so without the `const`/`static` guard one message
/// becomes four rows, three of them UNPARSED.
#[test]
fn only_a_nuntius_declaration_is_a_site_not_its_uses() {
    let custos: Vec<_> = extract_sites()
        .into_iter()
        .filter(|s| s.file.ends_with("custos.rs"))
        .collect();
    assert_eq!(
        custos.len(),
        2,
        "expected exactly the two `NUNTIUS_*` declarations, got {custos:#?}",
    );
    assert!(
        !custos.iter().any(|s| s.pattern == "UNPARSED"),
        "a bare use of a message constant was counted: {custos:#?}",
    );
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
