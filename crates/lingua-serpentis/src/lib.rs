//! The Lingua Serpentis compiler.
//!
//! # There is no interpreter, and the library does not run programs
//!
//! Until Task 19 this crate exposed five `run*` entry points that lexed,
//! parsed, checked and then *executed* a program by walking its tree, and
//! `interp.rs` was that tree-walker. Spec §3.1 removes it: the corpus is the
//! oracle, a second implementation is a second thing to keep in step forever,
//! and differential testing is a way of detecting the drift that having two
//! implementations causes.
//!
//! So the pipeline stops at the typed IR. `lexer` → `parser::parse_file` →
//! `latinity` → `checker::check_program_typed` → `liss_ir::Programma`, and
//! [`cli::liss`] hands that to `liss_codegen` and runs the image it links.
//! **Running a program is a thing the `liss` binary does, not a thing this
//! library does.**
//!
//! Spec §9.6 said the opposite — that `run_program_with_warnings`,
//! `run_dir_with_warnings` and `run_file_with_warnings` would keep their exact
//! signatures with "the `String` … now the child's captured stdout" — and it
//! has been amended. Its argument was that the corpus tooling should keep
//! calling what it called; that premise died when the fixture harness moved to
//! `liss run`, which it had to, because Latinity warnings are written by the
//! parent before it spawns and a raw image has none. What §9.6 would have
//! preserved is a library function that forks a compiler: a small `liss run`
//! with a worse interface.
//!
//! What did survive the swap is [`render`], and that is what the corpus tooling
//! actually depends on — the shape of the two streams, not a signature.

pub mod error;
/// `Roman` and its arithmetic live in `liss-runtime` (spec §11): they are
/// compiled twice, once for the host inside this compiler and once for musl
/// inside `liss-abi`. This re-export keeps every existing
/// `lingua_serpentis::roman::...` call site working unchanged.
pub use liss_runtime::roman;
/// Fraction arithmetic, in `liss-runtime` alongside `roman` for the same
/// reason.
pub use liss_runtime::fraction;
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
    Ok(ParsedFile { path: f.path, ast })
}

/// Lex and parse every file of a loaded program, root and packages alike.
///
/// `cli::liss` is the only caller: it lints the result, checks it into the
/// typed IR and stops. It was split out of `run_loaded` — the private function
/// the five deleted `run*` entry points shared — so that the compiled and
/// interpreted paths could not drift into different notions of what loading
/// is. One path remains and the split is now merely where the code lives.
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
///
/// On the error path the failure's own warnings are written first and its
/// partial output still goes to stdout, so a run that printed something and
/// then failed reports all three (D-08, D-09). Warnings before the error
/// matches the success path's ordering and reads chronologically.
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

    // **Five tests stood here and are re-homed, not dropped.** Each drove
    // `run_program_with_warnings`, so each needed the interpreter to produce
    // the `Result` it then rendered. What they asserted is behaviour a user
    // sees, so all five say it end to end now — which is a stronger claim than
    // a rendering of a value built in process:
    //
    // - `output_printed_before_an_error_still_reaches_stdout` (D-08) →
    //   `conformance/errores/emissio-ante-erratum`, and now also
    //   `conformance/currunt/scribe-ante-acervum`, which is the harder half:
    //   the flush has to happen inside a SIGSEGV handler.
    // - `warnings_survive_an_error_and_are_rendered_before_it` (D-09) →
    //   `conformance/errores/monitum-cum-errato`.
    // - `warnings_survive_a_checker_error_too` →
    //   `tests/programs.rs::type_errors_halt_but_their_warnings_survive`,
    //   which asserts the same two lines in the same order.
    // - `an_out_of_range_exit_status_warns_before_it_is_truncated` (D-10) →
    //   `conformance/monita/status-exitus-redactus` and
    //   `tests/cli.rs::an_out_of_range_exit_status_is_truncated_and_says_so`.
    //   The note is the *runtime's* now (`liss-abi`'s `ingressus`), not the
    //   interpreter's, so only an end-to-end test can see it at all.
    // - `an_in_range_exit_status_says_nothing` →
    //   `tests/cli.rs::an_exit_status_that_already_fits_says_nothing`.
    //
    // The four above survive because `render` survives: they are about the
    // shape of the two streams, and they never needed a backend to make one.
}
