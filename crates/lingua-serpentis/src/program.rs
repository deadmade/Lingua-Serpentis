use crate::error::ProgramError;
use crate::token::TokenKind;
use std::collections::BTreeMap;
use std::path::Path;

/// One source file, with its path relative to the program root (always
/// '/'-separated) kept for error attribution.
#[derive(Debug, Clone, PartialEq)]
pub struct SourceFile {
    pub path: String,
    pub src: String,
}

/// A loaded (not yet parsed) program: the root folder's files plus one entry
/// per package (§4.1–4.2 of the design). Files sorted by path, packages by
/// name, so everything downstream is deterministic.
#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub root: Vec<SourceFile>,
    pub packages: BTreeMap<String, Vec<SourceFile>>,
}

/// A package name must be writable in code: it has to lex as exactly one
/// identifier. This rules out spaces, hyphens, leading digits — and keywords.
fn is_valid_package_name(name: &str) -> bool {
    match crate::lexer::tokenize(name) {
        Ok(toks) => toks.len() == 1 && matches!(toks[0].kind, TokenKind::Ident(_)),
        Err(_) => false,
    }
}

/// Group relative `.liss` paths into the program shape, enforcing the flat
/// rule: root files, one level of packages, nothing deeper.
pub fn assemble(files: Vec<SourceFile>) -> Result<Program, ProgramError> {
    let mut root = Vec::new();
    let mut packages: BTreeMap<String, Vec<SourceFile>> = BTreeMap::new();
    for f in files {
        let parts: Vec<&str> = f.path.split('/').collect();
        match parts.as_slice() {
            [_file] => root.push(f),
            [pkg, _file] => {
                if !is_valid_package_name(pkg) {
                    return Err(ProgramError::structural(
                        format!("nomen fasciculi '{}' non validum est", pkg)));
                }
                packages.entry(pkg.to_string()).or_default().push(f);
            }
            [pkg, sub, ..] => {
                return Err(ProgramError::structural(
                    format!("fasciculus in fasciculo non licet: '{}/{}'", pkg, sub)));
            }
            [] => unreachable!("split yields at least one part"),
        }
    }
    root.sort_by(|a, b| a.path.cmp(&b.path));
    for files in packages.values_mut() {
        files.sort_by(|a, b| a.path.cmp(&b.path));
    }
    Ok(Program { root, packages })
}

/// Walk a program folder, collect every `.liss` file with its relative path,
/// and assemble. Non-`.liss` files are ignored; folders with no `.liss`
/// beneath them contribute nothing and are thereby ignored; a `.liss` file
/// deeper than one folder level is rejected by `assemble`.
pub fn load_dir(dir: &Path) -> Result<Program, ProgramError> {
    let mut files = Vec::new();
    collect_liss(dir, "", &mut files)?;
    assemble(files)
}

/// Why a `.liss` file could not be read, in Latin.
///
/// Defectus D-22. These messages used to be built as a Latin frame with
/// `std::io::Error`'s English `Display` interpolated onto the end — `scida
/// 'principale.liss' legi non potest: Permission denied (os error 13)`. The
/// frame was right and the tail was a foreign string the library did not
/// author, so every embedder inherited the one rule the language has being
/// broken. The fix is to classify the failure rather than forward it.
///
/// One whole literal per cause, rather than a shared frame plus an
/// interpolated reason: the diagnostic catalogue matches sites by their format
/// string, and a single `scida '{}' {}` would collapse all four into one
/// unattributable pattern.
///
/// "not valid UTF-8" cannot be named as such — `UTF-8` carries a digit, and no
/// message in this language may. `signis incognitis scripta est`, "written in
/// unknown signs", says the same thing and chimes with the lexer's own
/// `signum ignotum`.
/// Each arm constructs its own error rather than sharing one constructor over
/// a computed string. The catalogue finds sites by their constructor and reads
/// the first literal after it, so wrapping a single constructor around the
/// match would show up as one site owning the first arm's wording, hiding the
/// other three entirely.
pub fn scida_legi_non_potest(rel: &str, e: &std::io::Error) -> ProgramError {
    use std::io::ErrorKind::*;
    match e.kind() {
        NotFound => ProgramError::structural(format!("scida '{}' non inventa est", rel)),
        PermissionDenied => ProgramError::structural(format!("scida '{}' legi non potest", rel)),
        InvalidData => {
            ProgramError::structural(format!("scida '{}' signis incognitis scripta est", rel))
        }
        _ => ProgramError::structural(format!("scida '{}' aperiri non potest", rel)),
    }
}

fn collect_liss(dir: &Path, prefix: &str, out: &mut Vec<SourceFile>) -> Result<(), ProgramError> {
    // Never `dir.display()`: that is an absolute filesystem path, where every
    // other diagnostic in the language names a path relative to the program
    // root. `prefix` already carries the relative name (D-22).
    let entries = std::fs::read_dir(dir).map_err(|_| ProgramError::structural(
        if prefix.is_empty() {
            "directorium programmatis legi non potest".to_string()
        } else {
            format!("directorium '{}' legi non potest", prefix)
        }))?;
    for entry in entries {
        // A distinct sentence from the one above: two sites, two conditions.
        let entry = entry.map_err(|_| ProgramError::structural(
            "directorium percurri non potest".to_string()))?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        let rel = if prefix.is_empty() { name.clone() } else { format!("{}/{}", prefix, name) };
        if path.is_dir() {
            // Defectus D-17. Every directory entry was recursed into, so a
            // program folder that happened to be a git checkout died on
            // `.git` failing to lex as a package name — a failure with
            // nothing to do with the program. The packages design §4.1
            // already ignores folders holding no `.liss`; this extends that
            // to the case where one does, which is what a `.git` looks like.
            // Skipping hidden directories is what every tool that walks a
            // source tree does.
            if name.starts_with('.') {
                continue;
            }
            collect_liss(&path, &rel, out)?;
        } else if name.ends_with(".liss") {
            let src = std::fs::read_to_string(&path)
                .map_err(|e| scida_legi_non_potest(&rel, &e))?;
            out.push(SourceFile { path: rel, src });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sf(path: &str) -> SourceFile {
        SourceFile { path: path.into(), src: String::new() }
    }

    #[test]
    fn groups_root_and_packages_sorted() {
        let p = assemble(vec![
            sf("b.liss"), sf("a.liss"),
            sf("mathematica/summa.liss"), sf("mathematica/fractio.liss"),
        ]).unwrap();
        let root: Vec<&str> = p.root.iter().map(|f| f.path.as_str()).collect();
        assert_eq!(root, vec!["a.liss", "b.liss"]);
        let pkgs: Vec<&String> = p.packages.keys().collect();
        assert_eq!(pkgs, vec!["mathematica"]);
        let files: Vec<&str> = p.packages["mathematica"].iter().map(|f| f.path.as_str()).collect();
        assert_eq!(files, vec!["mathematica/fractio.liss", "mathematica/summa.liss"]);
    }

    #[test]
    fn nested_package_is_rejected() {
        let e = assemble(vec![sf("mathematica/fractio/imum.liss")]).unwrap_err();
        assert!(e.message.contains("fasciculo"), "got: {}", e.message);
        assert!(e.message.contains("mathematica/fractio"), "got: {}", e.message);
        assert_eq!(e.file, None);
        assert_eq!(e.line, None);
    }

    #[test]
    fn invalid_package_names_are_rejected() {
        // a hyphen cannot lex as one identifier; a keyword lexes as a keyword
        for bad in ["my-utils/x.liss", "si/x.liss", "duo verba/x.liss"] {
            let e = assemble(vec![sf(bad)]).unwrap_err();
            assert!(e.message.contains("non validum"), "path {bad:?} got: {}", e.message);
        }
    }

    #[test]
    fn load_dir_walks_ignores_and_assembles() {
        let base = std::env::temp_dir().join(format!("liss-load-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(base.join("mathematica")).unwrap();
        std::fs::create_dir_all(base.join("vacua/intus")).unwrap(); // no .liss anywhere → ignored
        std::fs::write(base.join("principale.liss"), "").unwrap();
        std::fs::write(base.join("notae.txt"), "non liss").unwrap(); // ignored
        std::fs::write(base.join("mathematica/summa.liss"), "").unwrap();
        let p = load_dir(&base).unwrap();
        std::fs::remove_dir_all(&base).unwrap();
        assert_eq!(p.root.len(), 1);
        assert_eq!(p.root[0].path, "principale.liss");
        assert_eq!(p.packages.len(), 1);
        assert_eq!(p.packages["mathematica"][0].path, "mathematica/summa.liss");
    }

    #[test]
    fn load_dir_rejects_a_liss_file_below_level_one() {
        let base = std::env::temp_dir().join(format!("liss-deep-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(base.join("mathematica/fractio")).unwrap();
        std::fs::write(base.join("mathematica/fractio/imum.liss"), "").unwrap();
        let e = load_dir(&base).unwrap_err();
        std::fs::remove_dir_all(&base).unwrap();
        assert!(e.message.contains("fasciculo"), "got: {}", e.message);
    }
}
