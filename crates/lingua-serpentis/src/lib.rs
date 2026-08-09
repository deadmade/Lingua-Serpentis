//! The Lingua Serpentis compiler. The pipeline stops at the typed IR; the `liss`
//! binary is what compiles and runs.

pub mod error;
pub use liss_runtime::roman;
pub use liss_runtime::fraction;
pub mod fragmentum;
pub mod pictura;
pub mod spatium;
pub mod token;
pub mod lexer;
pub mod ast;
pub mod parser;
pub mod checker;
pub mod latinity;
pub mod program;
pub mod cli;

use ast::{ParsedFile, ParsedProgram};
use program::{Program, SourceFile};
pub use error::{ProgramError, Warning};

fn parse_one(f: SourceFile) -> Result<ParsedFile, ProgramError> {
    let tokens = lexer::tokenize(&f.src).map_err(|e| ProgramError::in_file(&f.path, e))?;
    let ast = parser::parse_file(&tokens).map_err(|e| ProgramError::in_file(&f.path, e))?;
    Ok(ParsedFile { path: f.path, src: f.src, ast })
}

/// Lex and parse every file of a loaded program, root and packages alike.
pub(crate) fn parse_loaded(prog: Program) -> Result<ParsedProgram, ProgramError> {
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
    Ok(ParsedProgram { root, packages })
}

#[derive(Debug, Clone, PartialEq)]
pub struct Rendered {
    pub stdout: String,
    pub stderr: String,
    pub code: i32,
}

/// Warnings to stderr, program output to stdout, the program's own status on success
/// and 1 on any error; on the error path the warnings come first (D-08, D-09).
pub fn render(result: Result<(String, u8, Vec<Warning>), ProgramError>) -> Rendered {
    fn warnings_to(stderr: &mut String, warnings: &[Warning]) {
        for w in warnings {
            stderr.push_str(&w.to_string());
            stderr.push('\n');
        }
    }
    match result {
        Ok((stdout, code, warnings)) => {
            let mut stderr = String::new();
            warnings_to(&mut stderr, &warnings);
            Rendered { stdout, stderr, code: code as i32 }
        }
        Err(e) => {
            let mut stderr = String::new();
            warnings_to(&mut stderr, &e.warnings);
            stderr.push_str(&format!("{e}\n"));
            Rendered { stdout: e.stdout, stderr, code: 1 }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use error::ProgramError;

    fn monitum(message: &str, line: Option<usize>) -> Warning {
        Warning { message: message.into(), file: None, line, spatium: None, fragmentum: None }
    }

    #[test]
    fn warnings_go_to_stderr_and_output_to_stdout() {
        let r = render(Ok::<_, ProgramError>((
            "V\n".to_string(),
            0,
            vec![monitum("res", Some(2))],
        )));
        assert_eq!(r.stdout, "V\n");
        assert_eq!(r.stderr, "monitum: res\n  in linea II\n");
        assert_eq!(r.code, 0);
    }

    #[test]
    fn every_warning_gets_its_own_block_in_order() {
        // Nothing separates two blocks but the column-0 severity word.
        let r = render(Ok::<_, ProgramError>((
            String::new(),
            0,
            vec![monitum("prima", None), monitum("secunda", None)],
        )));
        assert_eq!(r.stderr, "monitum: prima\nmonitum: secunda\n");
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
        assert_eq!(r.stderr, "erratum: munus 'initium' in programmate non inventum\n");
        assert_eq!(r.code, 1);
    }

    // Five interpreter-era tests were re-homed to conformance fixtures and
    // tests/{cli,programs}.rs; docs/internals/rationale.md indexes which.
}
