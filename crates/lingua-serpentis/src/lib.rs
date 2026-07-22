pub mod error;
pub mod roman;
pub mod token;
pub mod lexer;
pub mod ast;
pub mod parser;
pub mod checker;
pub mod value;
pub mod interp;
pub mod latinity;

use error::LResult;
pub use error::Warning;

/// Lex → parse → type-check → lint → interpret.
/// Returns the collected `scribe` output together with any Latinity warnings.
/// Linting never fails and never affects whether the program runs.
pub fn run_with_warnings(src: &str) -> LResult<(String, Vec<Warning>)> {
    let tokens = lexer::tokenize(src)?;
    let mut program = parser::parse(&tokens)?;
    checker::check(&mut program)?;
    let warnings = latinity::lint(&program);
    let output = interp::execute(&program)?;
    Ok((output, warnings))
}

/// Lex → parse → type-check → interpret, discarding Latinity warnings.
pub fn run(src: &str) -> LResult<String> {
    run_with_warnings(src).map(|(output, _)| output)
}
