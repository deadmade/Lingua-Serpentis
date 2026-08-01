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

fn collect_liss(dir: &Path, prefix: &str, out: &mut Vec<SourceFile>) -> Result<(), ProgramError> {
    let entries = std::fs::read_dir(dir).map_err(|e| ProgramError::structural(
        format!("directorium '{}' legi non potest: {}", dir.display(), e)))?;
    for entry in entries {
        let entry = entry.map_err(|e| ProgramError::structural(
            format!("directorium legi non potest: {}", e)))?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        let rel = if prefix.is_empty() { name.clone() } else { format!("{}/{}", prefix, name) };
        if path.is_dir() {
            collect_liss(&path, &rel, out)?;
        } else if name.ends_with(".liss") {
            let src = std::fs::read_to_string(&path).map_err(|e| ProgramError::structural(
                format!("scida '{}' legi non potest: {}", rel, e)))?;
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
