use crate::error::{LinguaError, LResult};
use crate::token::{Token, TokenKind};

pub fn tokenize(src: &str) -> LResult<Vec<Token>> {
    let chars: Vec<char> = src.chars().collect();
    let mut tokens = Vec::new();
    let mut i = 0;
    let mut line = 1;

    while i < chars.len() {
        let c = chars[i];
        match c {
            '\n' => { line += 1; i += 1; }
            ' ' | '\t' | '\r' => { i += 1; }
            '/' if chars.get(i + 1) == Some(&'/') => {
                while i < chars.len() && chars[i] != '\n' { i += 1; }
            }
            '(' => { tokens.push(Token { kind: TokenKind::LParen, line }); i += 1; }
            ')' => { tokens.push(Token { kind: TokenKind::RParen, line }); i += 1; }
            '{' => { tokens.push(Token { kind: TokenKind::LBrace, line }); i += 1; }
            '}' => { tokens.push(Token { kind: TokenKind::RBrace, line }); i += 1; }
            ';' => { tokens.push(Token { kind: TokenKind::Semicolon, line }); i += 1; }
            ',' => { tokens.push(Token { kind: TokenKind::Comma, line }); i += 1; }
            '+' => { tokens.push(Token { kind: TokenKind::Plus, line }); i += 1; }
            '-' => { tokens.push(Token { kind: TokenKind::Minus, line }); i += 1; }
            '*' => { tokens.push(Token { kind: TokenKind::Star, line }); i += 1; }
            '/' => { tokens.push(Token { kind: TokenKind::Slash, line }); i += 1; }
            '%' => { tokens.push(Token { kind: TokenKind::Percent, line }); i += 1; }
            '=' => { tokens.push(Token { kind: TokenKind::Assign, line }); i += 1; }
            _ => {
                // literals ('°', '"', '\'') and words handled in Task 5; for now, error.
                return Err(LinguaError {
                    message: format!("signum ignotum: '{}'", c),
                    line,
                });
            }
        }
    }
    Ok(tokens)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(src: &str) -> Vec<TokenKind> {
        tokenize(src).unwrap().into_iter().map(|t| t.kind).collect()
    }

    #[test]
    fn symbols_and_operators() {
        assert_eq!(
            kinds("(){};,+-*/%="),
            vec![
                TokenKind::LParen, TokenKind::RParen, TokenKind::LBrace, TokenKind::RBrace,
                TokenKind::Semicolon, TokenKind::Comma, TokenKind::Plus, TokenKind::Minus,
                TokenKind::Star, TokenKind::Slash, TokenKind::Percent, TokenKind::Assign,
            ]
        );
    }

    #[test]
    fn comments_and_lines_are_tracked() {
        let toks = tokenize("+ // ignored\n-").unwrap();
        assert_eq!(toks[0].kind, TokenKind::Plus);
        assert_eq!(toks[0].line, 1);
        assert_eq!(toks[1].kind, TokenKind::Minus);
        assert_eq!(toks[1].line, 2);
    }

    #[test]
    fn unknown_char_is_error() {
        // '<' is intentionally not a token (comparisons are Latin words)
        let err = tokenize("<").unwrap_err();
        assert_eq!(err.line, 1);
    }
}
