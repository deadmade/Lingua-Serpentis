//! The Latin-diagnostic ratchet: every message the source can emit, and whether a fixture
//! produces it. The rules it imposes on source are in docs/internals/testing.md.
mod support;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Site {
    file: String,
    line: usize,
    /// The format string as written, e.g. `variabilis ignota: '{}'`.
    pattern: String,
}

/// Compares the literal segments between `{...}` holes, in order, anchored at both ends unless
/// the pattern begins or ends with a hole; without the anchors a prefix would match.
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

/// Every message the corpus produces: stderr only, which is complete.
fn observed_messages() -> Vec<String> {
    let mut out = Vec::new();
    for f in support::discover() {
        let r = support::run_fixture(&f);
        for line in r.stderr.lines() {
            // Only a line that *opens* a diagnostic is a sentence the language
            // wrote. The rest of a block is a location row, a gutter, and echoed
            // source — and echoed source is the dangerous one: a program line
            // that happened to read like a diagnostic would mark its pattern
            // `fixtured` when no fixture provokes it, which is a false green in
            // the one test that guards every user-facing sentence.
            let Some(msg) = line
                .strip_prefix("erratum: ")
                .or_else(|| line.strip_prefix("monitum: "))
            else {
                continue;
            };
            out.push(msg.to_string());
        }
    }
    out.sort();
    out.dedup();
    out
}

/// Uniqueness is decided on the pattern as written, byte for byte.
fn pattern_counts(sites: &[Site]) -> std::collections::HashMap<&str, usize> {
    let mut counts = std::collections::HashMap::new();
    for s in sites {
        *counts.entry(s.pattern.as_str()).or_insert(0) += 1;
    }
    counts
}

/// `liss-abi`'s messages are `b"…"` and `c"…"`, and nothing in the extractor mentions either.
#[test]
fn byte_and_c_string_literals_are_extracted_like_any_other() {
    let cases = [
        (r#"const NUNTIUS_X: &[u8] = b"acervus exhaustus";"#, "acervus exhaustus"),
        (r#"const NUNTIUS_Y: &CStr = c"memoria violata est";"#, "memoria violata est"),
        (r#"const NUNTIUS_Z: &str = "planus";"#, "planus"),
        (r#"liss_panic(c"divisio per nullum".as_ptr(), locus);"#, "divisio per nullum"),
        (r#"liss_panic(c"nomen \"x\" ignotum".as_ptr());"#, "nomen \"x\" ignotum"),
        // A char literal before the message is not a message; `Lit::Char` is not one of the
        // three arms `primum_literale` accepts.
        (
            r#"self.err(if c == '"' { "chorda non clausa" } else { "aliud" });"#,
            "chorda non clausa",
        ),
    ];
    for (src, want) in cases {
        let sites = sites_of(src);
        assert_eq!(
            sites.iter().map(|s| s.pattern.as_str()).collect::<Vec<_>>(),
            [want],
            "failed to extract from {src:?}",
        );
    }
}

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
    assert!(
        sites.iter().filter(|s| s.file.ends_with("defectus.rs")).count() >= 5,
        "the `liss_panic(` marker found no call sites at all",
    );
}

#[test]
fn a_marker_in_a_comment_is_prose_and_a_marker_in_code_is_not() {
    let sites = extract_sites();

    // Too shy: a note naming a marker must not be a site.
    assert!(
        !sites.iter().any(|s| s.pattern == "Error: "),
        "a comment quoting `Error: ` was read as a diagnostic",
    );

    // Too eager: `custos.rs`'s two real `NUNTIUS_*` declarations must still be found.
    for want in
        ["acervus vocationum exhaustus est", "defectus internus: memoria violata est: signum"]
    {
        assert!(
            sites.iter().any(|s| s.file.ends_with("custos.rs") && s.pattern == want),
            "the `NUNTIUS_` declaration for {want:?} was skipped",
        );
    }

    // Comments are not tokens, so neither direction can go wrong any more. A trailing note
    // after a real site does not erase it, and a block comment quoting a marker in prose —
    // which the scanner could not survive at all — invents nothing.
    let src = "eprintln!(\"Error: machina ignota est\"); // and exit\n\
               /* prose: NUNTIUS_ and Defectus::novus( and Error: */\n";
    assert_eq!(
        sites_of(src).iter().map(|s| s.pattern.as_str()).collect::<Vec<_>>(),
        ["Error: machina ignota est"],
    );
}

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
        // `fixtured` means uniquely attributable; adding a fixture never moves a site out of
        // `ambiguous`, because the ambiguity is in the wording.
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

// ---------------------------------------------------------------------------
// The AST extractor. Replaces the byte scanner above; see
// docs/superpowers/specs/2026-08-09-test-harness-libraries-design.md §4-§5.
// ---------------------------------------------------------------------------

use proc_macro2::{TokenStream, TokenTree};
use quote::ToTokens;
use syn::spanned::Spanned;

/// The first string, byte-string or C-string literal in `tokens`, descending into groups.
/// A site with none of its own is `UNPARSED`: spec §10.3 requires the runtime to write its
/// sentences inline at the call site.
fn primum_literale(tokens: TokenStream) -> Option<String> {
    for tt in tokens {
        match tt {
            TokenTree::Literal(l) => match syn::Lit::new(l) {
                syn::Lit::Str(s) => return Some(s.value()),
                syn::Lit::ByteStr(s) => {
                    return Some(String::from_utf8_lossy(&s.value()).into_owned());
                }
                syn::Lit::CStr(s) => return Some(s.value().to_string_lossy().into_owned()),
                _ => {}
            },
            TokenTree::Group(g) => {
                if let Some(s) = primum_literale(g.stream()) {
                    return Some(s);
                }
            }
            _ => {}
        }
    }
    None
}

/// Does `path` end in `nomen`, which may itself be `A::b`?
fn semita_desinit(path: &syn::Path, nomen: &str) -> bool {
    let wanted: Vec<&str> = nomen.split("::").collect();
    let got: Vec<String> = path.segments.iter().map(|s| s.ident.to_string()).collect();
    got.len() >= wanted.len()
        && got[got.len() - wanted.len()..].iter().zip(&wanted).all(|(a, b)| a == b)
}

type Args = syn::punctuated::Punctuated<syn::Expr, syn::Token![,]>;

/// Do `args` *open* with a string literal? This models the markers `Err("` and `.ok_or("`
/// exactly: the quote follows the paren directly. `Err("numerus Romanus vacuus".into())`
/// qualifies — the literal is the first token, whatever is done to it after — and
/// `Err(format!(…))` does not, because `format` is.
fn incipit_literale(args: &Args) -> bool {
    match args.to_token_stream().into_iter().next() {
        Some(TokenTree::Literal(l)) => matches!(syn::Lit::new(l), syn::Lit::Str(_)),
        _ => false,
    }
}

struct Lector {
    file: String,
    sites: Vec<Site>,
}

impl Lector {
    /// Record a site at `span`'s line, carrying the first literal in `tokens`.
    fn nota(&mut self, span: proc_macro2::Span, tokens: TokenStream) {
        let pattern = primum_literale(tokens).unwrap_or_else(|| "UNPARSED".to_string());
        // The one guard that survives: a pattern that is nothing but holes identifies nothing.
        if pattern != "UNPARSED" && pattern.replace("{}", "").trim().is_empty() {
            return;
        }
        self.sites.push(Site { file: self.file.clone(), line: span.start().line, pattern });
    }
}

/// A Latin sentence written inside a unit test is not a diagnostic.
fn est_test(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|a| {
        a.path().is_ident("cfg") && a.to_token_stream().to_string().contains("test")
    })
}

impl<'ast> syn::visit::Visit<'ast> for Lector {
    fn visit_item_mod(&mut self, i: &'ast syn::ItemMod) {
        if est_test(&i.attrs) {
            return;
        }
        syn::visit::visit_item_mod(self, i);
    }

    fn visit_item_fn(&mut self, i: &'ast syn::ItemFn) {
        if est_test(&i.attrs) {
            return;
        }
        syn::visit::visit_item_fn(self, i);
    }

    fn visit_item_const(&mut self, i: &'ast syn::ItemConst) {
        if i.ident.to_string().starts_with("NUNTIUS_") {
            self.nota(i.ident.span(), i.expr.to_token_stream());
        }
        syn::visit::visit_item_const(self, i);
    }

    fn visit_item_static(&mut self, i: &'ast syn::ItemStatic) {
        if i.ident.to_string().starts_with("NUNTIUS_") {
            self.nota(i.ident.span(), i.expr.to_token_stream());
        }
        syn::visit::visit_item_static(self, i);
    }

    fn visit_expr(&mut self, e: &'ast syn::Expr) {
        match e {
            syn::Expr::Struct(s)
                if semita_desinit(&s.path, "LinguaError") || semita_desinit(&s.path, "Warning") =>
            {
                self.nota(s.path.span(), s.fields.to_token_stream());
            }
            syn::Expr::Call(c) => {
                if let syn::Expr::Path(p) = &*c.func {
                    let s = &p.path;
                    let hit = semita_desinit(s, "ProgramError::structural")
                        || semita_desinit(s, "liss_panic")
                        || semita_desinit(s, "Defectus::novus")
                        || (semita_desinit(s, "Err") && incipit_literale(&c.args));
                    if hit {
                        self.nota(c.func.span(), c.args.to_token_stream());
                    }
                }
            }
            syn::Expr::MethodCall(m) => {
                let nomen = m.method.to_string();
                let hit = nomen == "err"
                    || (nomen == "ok_or" && incipit_literale(&m.args))
                    || (nomen == "ok_or_else"
                        && matches!(m.args.first(), Some(syn::Expr::Closure(c))
                            if matches!(&*c.body, syn::Expr::Macro(mac)
                                if mac.mac.path.is_ident("format"))));
                if hit {
                    self.nota(m.method.span(), m.args.to_token_stream());
                }
            }
            syn::Expr::Macro(m) if m.mac.path.is_ident("eprintln") => {
                self.nota(m.mac.path.span(), m.mac.tokens.clone());
            }
            _ => {}
        }
        syn::visit::visit_expr(self, e);
    }

    /// `eprintln!(…);` in statement position is a `Stmt::Macro`, not an `Expr::Macro`, and
    /// every one of `cli.rs`'s twelve is written that way.
    fn visit_stmt_macro(&mut self, s: &'ast syn::StmtMacro) {
        if s.mac.path.is_ident("eprintln") {
            self.nota(s.mac.path.span(), s.mac.tokens.clone());
        }
        syn::visit::visit_stmt_macro(self, s);
    }
}

/// The same six crates, the same non-recursive `read_dir`, the same sorted `.rs` files —
/// parsed rather than scanned.
fn extract_sites() -> Vec<Site> {
    let root = support::repo_root();
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
        let src = std::fs::read_to_string(&path).unwrap();
        let rel = path.strip_prefix(&root).unwrap().display().to_string();
        let ast = syn::parse_file(&src).unwrap_or_else(|e| panic!("{rel} does not parse: {e}"));
        let mut lector = Lector { file: rel, sites: Vec::new() };
        syn::visit::Visit::visit_file(&mut lector, &ast);
        sites.extend(lector.sites);
    }
    sites.sort();
    sites.dedup();
    sites
}

/// The sites in one source fragment, for the tests that assert on a fragment rather than on
/// the tree. **Wrapped in a function first**, which is where these constructs really live: at
/// file level `eprintln!(…);` is an `Item::Macro` and no visitor here looks at those, so a
/// bare `parse_file` would silently find nothing. The fallback is for a fragment that is only
/// valid at file level.
fn sites_of(src: &str) -> Vec<Site> {
    let ast = syn::parse_file(&format!("fn probatio() {{ {src} }}"))
        .or_else(|_| syn::parse_file(src))
        .unwrap_or_else(|e| panic!("fragment does not parse: {e}\n{src}"));
    let mut lector = Lector { file: "probatio.rs".to_string(), sites: Vec::new() };
    syn::visit::Visit::visit_file(&mut lector, &ast);
    lector.sites
}
