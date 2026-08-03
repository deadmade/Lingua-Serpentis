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

/// Everything a completed run puts in front of the user: the two streams and
/// the process status. Kept in the library so tests can assert exactly what
/// the CLI writes without spawning a process.
#[derive(Debug, Clone, PartialEq)]
pub struct Rendered {
    pub stdout: String,
    pub stderr: String,
    pub code: i32,
}

/// Render a pipeline result the way the CLI does: warnings to stderr — so they
/// never contaminate piped program output — program output to stdout, the
/// program's own exit status on success and 1 on any error.
pub fn render<E: std::fmt::Display>(
    result: Result<(String, u8, Vec<Warning>), E>,
) -> Rendered {
    match result {
        Ok((stdout, code, warnings)) => {
            let mut stderr = String::new();
            for w in &warnings {
                stderr.push_str(&w.to_string());
                stderr.push('\n');
            }
            Rendered { stdout, stderr, code: code as i32 }
        }
        Err(e) => Rendered { stdout: String::new(), stderr: format!("{e}\n"), code: 1 },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use error::ProgramError;

    #[test]
    fn warnings_go_to_stderr_and_output_to_stdout() {
        let r = render(Ok::<_, ProgramError>((
            "V\n".to_string(),
            0,
            vec![Warning { message: "res".into(), file: None, line: Some(2) }],
        )));
        assert_eq!(r.stdout, "V\n");
        assert_eq!(r.stderr, "Monitum in linea 2: res\n");
        assert_eq!(r.code, 0);
    }

    #[test]
    fn every_warning_gets_its_own_line_in_order() {
        let w = |m: &str| Warning { message: m.into(), file: None, line: None };
        let r = render(Ok::<_, ProgramError>((
            String::new(),
            0,
            vec![w("prima"), w("secunda")],
        )));
        assert_eq!(r.stderr, "Monitum: prima\nMonitum: secunda\n");
    }

    #[test]
    fn the_programs_exit_status_is_carried_through() {
        let r = render(Ok::<_, ProgramError>((String::new(), 5, vec![])));
        assert_eq!(r.code, 5);
    }

    #[test]
    fn an_error_renders_to_stderr_with_status_one_and_no_stdout() {
        let r = render(Err::<(String, u8, Vec<Warning>), _>(
            ProgramError::structural("munus 'initium' in programmate non inventum"),
        ));
        assert_eq!(r.stdout, "");
        assert_eq!(r.stderr, "Error: munus 'initium' in programmate non inventum\n");
        assert_eq!(r.code, 1);
    }
}
