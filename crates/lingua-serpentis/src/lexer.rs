use crate::error::{LinguaError, LResult};
use crate::roman::Roman;
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
            '[' => { tokens.push(Token { kind: TokenKind::LBracket, line }); i += 1; }
            ']' => { tokens.push(Token { kind: TokenKind::RBracket, line }); i += 1; }
            ';' => { tokens.push(Token { kind: TokenKind::Semicolon, line }); i += 1; }
            ',' => { tokens.push(Token { kind: TokenKind::Comma, line }); i += 1; }
            '+' => { tokens.push(Token { kind: TokenKind::Plus, line }); i += 1; }
            '-' => { tokens.push(Token { kind: TokenKind::Minus, line }); i += 1; }
            '*' => { tokens.push(Token { kind: TokenKind::Star, line }); i += 1; }
            '/' => { tokens.push(Token { kind: TokenKind::Slash, line }); i += 1; }
            '%' => { tokens.push(Token { kind: TokenKind::Percent, line }); i += 1; }
            '=' => { tokens.push(Token { kind: TokenKind::Assign, line }); i += 1; }
            '°' => {
                let start = i;
                i += 1; // consume °
                while i < chars.len() && is_roman_body(chars[i]) { i += 1; }
                let lexeme: String = chars[start..i].iter().collect();
                let value = crate::roman::parse_roman(&lexeme)
                    .map_err(|m| LinguaError { message: m, line })?;
                tokens.push(Token { kind: value.into(), line });
            }
            '"' => {
                i += 1; // consume opening quote
                let start = i;
                while i < chars.len() && chars[i] != '"' { i += 1; }
                if i >= chars.len() {
                    return Err(LinguaError { message: "chorda non clausa".into(), line });
                }
                let text: String = chars[start..i].iter().collect();
                i += 1; // consume closing quote
                tokens.push(Token { kind: TokenKind::Text(text), line });
            }
            '\'' => {
                i += 1; // consume opening quote
                if i + 1 >= chars.len() || chars[i + 1] != '\'' {
                    return Err(LinguaError { message: "littera invalida".into(), line });
                }
                let ch = chars[i];
                i += 2; // char + closing quote
                tokens.push(Token { kind: TokenKind::Char(ch), line });
            }
            c if c.is_alphabetic() || c == '_' => {
                let start = i;
                while i < chars.len()
                    && (chars[i].is_alphanumeric() || chars[i] == '_')
                {
                    i += 1;
                }
                let word: String = chars[start..i].iter().collect();
                tokens.push(Token { kind: word_to_kind(word), line });
            }
            _ => {
                return Err(LinguaError {
                    message: format!("signum ignotum: '{}'", c),
                    line,
                });
            }
        }
    }
    Ok(tokens)
}

fn is_roman_body(c: char) -> bool {
    matches!(c, 'I' | 'V' | 'X' | 'L' | 'C' | 'D' | 'M' | '.' | ':' | '/')
}

fn word_to_kind(word: String) -> TokenKind {
    match word.as_str() {
        "nullus" => TokenKind::Int(Roman::zero()),
        "verus" => TokenKind::True,
        "falsus" => TokenKind::False,
        "si" => TokenKind::If,
        "aut" => TokenKind::Else,
        "indem" => TokenKind::While,
        "dum" => TokenKind::For,
        "munus" => TokenKind::Func,
        "redde" => TokenKind::Return,
        "abrumpe" => TokenKind::Break,
        "continua" => TokenKind::Continue,
        "scribe" => TokenKind::Print,
        "capit" => TokenKind::Takes,
        "reddit" => TokenKind::Yields,
        "lista" => TokenKind::Lista,
        "ni" => TokenKind::TyInt,
        "mnr" => TokenKind::TyFloat,
        "nr" => TokenKind::TyDouble,
        "voc" => TokenKind::TyText,
        "lit" => TokenKind::TyChar,
        "veritas" => TokenKind::TyBool,
        "et" => TokenKind::And,
        "vel" => TokenKind::Or,
        "non" => TokenKind::Not,
        "par_est" => TokenKind::Eq,
        "par_non_est" => TokenKind::Ne,
        "maior_est" => TokenKind::Gt,
        "minor_est" => TokenKind::Lt,
        _ => TokenKind::Ident(word),
    }
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
    fn arrow_does_not_swallow_plain_minus() {
        assert_eq!(kinds("- -"), vec![TokenKind::Minus, TokenKind::Minus]);
        assert_eq!(kinds("a-b"), vec![
            TokenKind::Ident("a".into()), TokenKind::Minus, TokenKind::Ident("b".into()),
        ]);
    }

    #[test]
    fn lone_colon_is_still_unknown() {
        assert!(tokenize(":").is_err());
    }

    #[test]
    fn retired_signature_symbols_no_longer_tokenize() {
        // `::` and `->` were the old function-signature notation; both are gone,
        // so each now fails as an unknown symbol.
        assert!(tokenize("::").is_err());
        assert!(tokenize("->").is_err());
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

    #[test]
    fn roman_numbers_become_values() {
        assert_eq!(kinds("°XXV"), vec![TokenKind::Int(Roman::from_u64(25))]);
        assert_eq!(kinds("°V:I/II"), vec![TokenKind::Float(5.5)]);
        assert_eq!(kinds("nullus"), vec![TokenKind::Int(Roman::zero())]);
    }

    #[test]
    fn strings_and_chars() {
        assert_eq!(kinds("\"salve\""), vec![TokenKind::Text("salve".into())]);
        assert_eq!(kinds("'x'"), vec![TokenKind::Char('x')]);
    }

    #[test]
    fn mnr_and_nr_are_distinct_type_keywords() {
        assert_eq!(kinds("mnr"), vec![TokenKind::TyFloat]);
        assert_eq!(kinds("nr"), vec![TokenKind::TyDouble]);
    }

    #[test]
    fn keywords_types_and_operators() {
        assert_eq!(
            kinds("ni si redde par_non_est veritas et"),
            vec![
                TokenKind::TyInt, TokenKind::If, TokenKind::Return,
                TokenKind::Ne, TokenKind::TyBool, TokenKind::And,
            ]
        );
    }

    #[test]
    fn keyword_prefix_does_not_split_identifier() {
        // "nimis" starts with keyword "ni" but must stay a single identifier
        assert_eq!(kinds("nimis"), vec![TokenKind::Ident("nimis".into())]);
    }

    #[test]
    fn header_keywords() {
        assert_eq!(kinds("capit reddit"), vec![TokenKind::Takes, TokenKind::Yields]);
        // A longer word that merely starts with a header keyword stays one ident.
        assert_eq!(kinds("capitulum"), vec![TokenKind::Ident("capitulum".into())]);
        assert_eq!(kinds("redditus"), vec![TokenKind::Ident("redditus".into())]);
    }

    #[test]
    fn literal_error_paths() {
        assert!(tokenize("\"unterminated").is_err()); // chorda non clausa
        assert!(tokenize("''").is_err());             // empty char literal
        assert!(tokenize("'ab'").is_err());           // multi-char char literal
    }

    #[test]
    fn brackets_and_lista_keyword() {
        assert_eq!(
            kinds("[]"),
            vec![TokenKind::LBracket, TokenKind::RBracket]
        );
        assert_eq!(kinds("lista"), vec![TokenKind::Lista]);
        // A longer word that merely starts with `lista` stays one identifier.
        assert_eq!(kinds("listae"), vec![TokenKind::Ident("listae".into())]);
        assert_eq!(
            kinds("ni lista"),
            vec![TokenKind::TyInt, TokenKind::Lista]
        );
    }
}
