pub mod error;
/// `Roman` and its arithmetic moved to `liss-runtime` (spec §11) — it needs
/// to be compiled for the host, inside the compiler, and for musl, inside
/// `liss-abi`, so it cannot live inside the interpreter's own crate. This
/// re-export keeps every existing `crate::roman::...` / `lingua_serpentis::
/// roman::...` call site working unchanged.
pub use liss_runtime::roman;
/// Fraction arithmetic, moved to `liss-runtime` alongside `roman` for the
/// same reason.
pub use liss_runtime::fraction;
pub mod token;
pub mod lexer;
pub mod ast;
pub mod parser;
pub mod checker;
pub mod value;
pub mod interp;
pub mod latinity;
pub mod program;
pub mod cli;

use ast::{ParsedFile, ParsedProgram};
use error::{LResult, LinguaError};
use program::{Program, SourceFile};
pub use error::{ProgramError, Warning};

fn parse_one(f: SourceFile) -> Result<ParsedFile, ProgramError> {
    let tokens = lexer::tokenize(&f.src).map_err(|e| ProgramError::in_file(&f.path, e))?;
    let ast = parser::parse_file(&tokens).map_err(|e| ProgramError::in_file(&f.path, e))?;
    Ok(ParsedFile { path: f.path, ast })
}

/// Lex and parse every file of a loaded program, root and packages alike.
///
/// Split out of `run_loaded` for `cli::liss`, which needs the `ParsedProgram`
/// itself: the compiled path lints it, checks it into the typed IR and stops,
/// where the interpreted path goes on to execute. Sharing the parse is what
/// keeps the two from drifting into different notions of what loading is.
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

/// The full pipeline over a loaded program: parse every file, lint, check the
/// whole program, execute from `initium`. Returns the collected `scribe`
/// output, the exit status, and any Latinity warnings.
///
/// Linting runs *before* the checker, and its result is attached to a failure
/// rather than dropped with it (D-09). A program with a non-Latin name and a
/// type error has two problems and should report both; a lint that only
/// appears once the program compiles is one nobody reads. This is safe to
/// reorder: `latinity` matches on declaration names only, and the sole thing
/// `check_program` mutates is `Expr::FloatLit` nodes.
///
/// The one place warnings genuinely cannot survive is a lex, parse or
/// structural failure — there is no `ParsedProgram` to lint yet, and nothing
/// has run, so the error carries an empty `stdout` and no warnings.
fn run_loaded(prog: Program) -> Result<(String, u8, Vec<Warning>), ProgramError> {
    let mut parsed = parse_loaded(prog)?;
    let warnings = latinity::lint_program(&parsed);
    if let Err(mut e) = checker::check_program(&mut parsed) {
        e.warnings = warnings;
        return Err(e);
    }
    match interp::execute_program(&parsed) {
        // The interpreter's own warnings — currently only the truncated-exit
        // status note (D-10) — come after the Latinity ones, which are about
        // the source rather than the run.
        Ok((output, code, run_warnings)) => {
            let mut all = warnings;
            all.extend(run_warnings);
            Ok((output, code, all))
        }
        // `e.stdout` is already whatever `scribe` printed before the failure.
        Err(mut e) => {
            e.warnings = warnings;
            Err(e)
        }
    }
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
pub fn run_file_with_warnings(src: &str) -> Result<(String, u8, Vec<Warning>), ProgramError> {
    let file = SourceFile { path: "fons.liss".into(), src: src.to_string() };
    let prog = Program { root: vec![file], packages: std::collections::BTreeMap::new() };
    let strip = |ws: &mut Vec<Warning>| {
        for w in ws {
            w.file = None;
        }
    };
    match run_loaded(prog) {
        Ok((out, code, mut warnings)) => {
            strip(&mut warnings);
            Ok((out, code, warnings))
        }
        // The error type used to be `LinguaError`, which had nowhere to carry
        // the partial output and warnings. `ProgramError`'s "line but no file"
        // Display arm is byte-identical to `LinguaError`'s, so the message a
        // user sees is unchanged; structural errors, which have no line of
        // their own, still surface at line 1.
        Err(mut e) => {
            e.file = None;
            e.line = Some(e.line.unwrap_or(1));
            strip(&mut e.warnings);
            Err(e)
        }
    }
}

/// Single-file pipeline, discarding the exit status (library convenience).
///
/// Keeps the `LinguaError` result the single-source API has always had, so
/// callers are unaffected by `run_file_with_warnings` widening its error.
pub fn run_with_warnings(src: &str) -> LResult<(String, Vec<Warning>)> {
    run_file_with_warnings(src)
        .map(|(output, _, warnings)| (output, warnings))
        .map_err(|e| LinguaError { message: e.message, line: e.line.unwrap_or(1) })
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

    #[test]
    fn output_printed_before_an_error_still_reaches_stdout() {
        // Defectus D-08. `scribe` output is accumulated in `Interp::out` and
        // was returned only on the Ok arm, so any runtime error threw away
        // everything the program had provably already printed.
        let src = "munus initium reddit ni {\n  scribe(\"ante\");\n  ni z = °I - °I;\n  scribe(°V / z);\n  redde nullus;\n}";
        let e = run_program_with_warnings(&[("principale.liss", src)])
            .expect_err("dividing by zero is an error");
        assert_eq!(e.stdout, "ante\n");
        // And it reaches the real stdout, not just the error value.
        let r = render(run_program_with_warnings(&[("principale.liss", src)]));
        assert_eq!(r.stdout, "ante\n");
        assert_eq!(r.stderr, "Error in 'principale.liss', linea 4: divisio per nullum\n");
        assert_eq!(r.code, 1);
    }

    #[test]
    fn warnings_survive_an_error_and_are_rendered_before_it() {
        // Defectus D-09. A program with a non-Latin name AND a division by
        // zero reported only the error: the warnings were computed and then
        // discarded by `run_loaded`'s `?`. A program with two problems only
        // ever told you about one.
        let r = render(run_program_with_warnings(&[(
            "principale.liss",
            "munus initium reddit ni {\n  ni counter = °I - °I;\n  scribe(°V / counter);\n  redde nullus;\n}",
        )]));
        assert_eq!(
            r.stderr,
            "Monitum in 'principale.liss', linea 2: nomen 'counter' fortasse non Latinum est\n\
             Error in 'principale.liss', linea 3: divisio per nullum\n"
        );
        assert_eq!(r.code, 1);
    }

    #[test]
    fn warnings_survive_a_checker_error_too() {
        // Not only runtime errors. Linting runs before the checker now, so a
        // program that fails to type-check still reports its style problems —
        // a lint that only appears once the program compiles is useless.
        let r = render(run_program_with_warnings(&[(
            "principale.liss",
            "munus initium reddit ni {\n  ni counter = \"non numerus\";\n  redde nullus;\n}",
        )]));
        assert!(
            r.stderr.starts_with(
                "Monitum in 'principale.liss', linea 2: nomen 'counter' fortasse non Latinum est\n"
            ),
            "warnings must come first, got: {:?}",
            r.stderr
        );
        assert!(r.stderr.contains("Error in 'principale.liss', linea 2:"));
    }

    #[test]
    fn an_out_of_range_exit_status_warns_before_it_is_truncated() {
        // Defectus D-10. The packages design §4.3 specifies the truncation, so
        // the behaviour stays; what was missing was any signal that it
        // happened. `redde °CCC` still exits 44, but says so.
        let (_, code, warnings) = run_program_with_warnings(&[(
            "principale.liss",
            "munus initium reddit ni {\n  redde °CCC;\n}",
        )])
        .unwrap();
        assert_eq!(code, 44);
        assert_eq!(
            warnings.iter().map(|w| w.to_string()).collect::<Vec<_>>(),
            vec!["Monitum: reditus 'initium' extra fines status exitus est; ad XLIV redactus"]
        );
    }

    #[test]
    fn an_in_range_exit_status_says_nothing() {
        let (_, code, warnings) = run_program_with_warnings(&[(
            "principale.liss",
            "munus initium reddit ni {\n  redde °III;\n}",
        )])
        .unwrap();
        assert_eq!(code, 3);
        assert!(warnings.is_empty(), "got {warnings:?}");
    }
}
