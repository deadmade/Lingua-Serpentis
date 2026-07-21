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

/// Lex → parse → type-check → interpret. Returns collected `scribe` output.
pub fn run(src: &str) -> LResult<String> {
    let tokens = lexer::tokenize(src)?;
    let program = parser::parse(&tokens)?;
    checker::check(&program)?;
    interp::execute(&program)
}
