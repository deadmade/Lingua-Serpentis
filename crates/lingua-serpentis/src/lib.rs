pub mod error;
pub mod roman;
pub mod fraction;
pub mod token;
pub mod lexer;
pub mod ast;
pub mod parser;
pub mod checker;
pub mod value;
pub mod interp;
pub mod latinity;
pub mod program;

use ast::{ParsedFile, ParsedProgram};
use error::{LResult, LinguaError};
use program::{Program, SourceFile};
pub use error::{ProgramError, Warning};

fn parse_one(f: SourceFile) -> Result<ParsedFile, ProgramError> {
    let tokens = lexer::tokenize(&f.src).map_err(|e| ProgramError::in_file(&f.path, e))?;
    let ast = parser::parse_file(&tokens).map_err(|e| ProgramError::in_file(&f.path, e))?;
    Ok(ParsedFile { path: f.path, ast })
}

/// The full pipeline over a loaded program: parse every file, check the
/// whole program, lint, execute from `initium`. Returns the collected
/// `scribe` output, the exit status, and any Latinity warnings.
fn run_loaded(prog: Program) -> Result<(String, u8, Vec<Warning>), ProgramError> {
    let mut root = Vec::new();
    for f in prog.root {
        root.push(parse_one(f)?);
    }
    let mut packages = std::collections::BTreeMap::new();
    for (name, files) in prog.packages {
        let mut parsed = Vec::new();
        for f in files {
            parsed.push(parse_one(f)?);
        }
        packages.insert(name, parsed);
    }
    let mut parsed = ParsedProgram { root, packages };
    checker::check_program(&mut parsed)?;
    let warnings = latinity::lint_program(&parsed);
    let (output, code) = interp::execute_program(&parsed)?;
    Ok((output, code, warnings))
}

/// Run a program given as (relative path, source) pairs — the disk-free
/// entry integration tests use. Paths are '/'-separated and follow the
/// folder rules: `"a.liss"` is a root file, `"mathematica/summa.liss"` is in
/// the `mathematica` package, anything deeper is an error.
pub fn run_program_with_warnings(
    files: &[(&str, &str)],
) -> Result<(String, u8, Vec<Warning>), ProgramError> {
    let files = files.iter()
        .map(|(p, s)| SourceFile { path: (*p).to_string(), src: (*s).to_string() })
        .collect();
    run_loaded(program::assemble(files)?)
}

/// Run the program folder at `dir`.
pub fn run_dir_with_warnings(
    dir: &std::path::Path,
) -> Result<(String, u8, Vec<Warning>), ProgramError> {
    run_loaded(program::load_dir(dir)?)
}

/// Run a single-file program — the degenerate case of §4.1: same rules
/// (`initium` required, declarations only), but errors and warnings keep the
/// file-less phrasing the single-source API has always had. Structural
/// errors, which have no line of their own, surface at line 1.
pub fn run_file_with_warnings(src: &str) -> Result<(String, u8, Vec<Warning>), LinguaError> {
    let file = SourceFile { path: "fons.liss".into(), src: src.to_string() };
    let prog = Program { root: vec![file], packages: std::collections::BTreeMap::new() };
    match run_loaded(prog) {
        Ok((out, code, mut warnings)) => {
            for w in &mut warnings {
                w.file = None;
            }
            Ok((out, code, warnings))
        }
        Err(e) => Err(LinguaError { message: e.message, line: e.line.unwrap_or(1) }),
    }
}

/// Single-file pipeline, discarding the exit status (library convenience).
pub fn run_with_warnings(src: &str) -> LResult<(String, Vec<Warning>)> {
    run_file_with_warnings(src).map(|(output, _, warnings)| (output, warnings))
}

/// Single-file pipeline, discarding warnings too.
pub fn run(src: &str) -> LResult<String> {
    run_with_warnings(src).map(|(output, _)| output)
}
