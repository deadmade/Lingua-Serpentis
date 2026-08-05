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
            '.' => { tokens.push(Token { kind: TokenKind::Dot, line }); i += 1; }
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
                // Give back any trailing separator. '.', ':' and '/' join the
                // parts of a numeral (°XXX.M, °V:I/II) but are also the
                // language's Dot and Slash, and the scan above is greedy — so
                // `°X/°II` used to swallow the '/' and die with `numerus
                // Romanus invalidus: 'X/'`, while the identical `°X / °II`
                // evaluated fine. Whitespace was load-bearing (defectus D-14).
                //
                // Trimming unconditionally is exactly "try, fail, retreat over
                // separators", without the retry: `parse_roman` splits on ':',
                // then '/', then '.', and every leaf goes to `convert_single`,
                // which rejects the empty string. A lexeme ending in a
                // separator always yields an empty final leaf, so no such
                // lexeme can ever parse. Retreating one *character* at a time
                // instead would be wrong: `°I:I` would fall back to `°I` and
                // leave a stray `Ident("I")`, silently killing `fractio sine
                // denominatore`.
                //
                // `i > start + 1` keeps a bare '°' intact, so it still reaches
                // `parse_roman` and reports `numerus Romanus vacuus`.
                while i > start + 1 && matches!(chars[i - 1], '.' | ':' | '/') {
                    i -= 1;
                }
                let lexeme: String = chars[start..i].iter().collect();
                let value = crate::roman::parse_roman(&lexeme)
                    .map_err(|m| LinguaError { message: m, line })?;
                tokens.push(Token { kind: value.into(), line });
            }
            '"' => {
                // The literal is attributed to the line it opens on, so a
                // diagnostic about the string points at the string; `line`
                // itself advances past the newlines inside it, so every token
                // after a multi-line literal is numbered correctly. It used to
                // walk over '\n' without counting, understating every
                // subsequent line by one per embedded newline — a drift with
                // no bound (defectus D-03).
                let opened_on = line;
                i += 1; // consume opening quote
                let start = i;
                while i < chars.len() && chars[i] != '"' {
                    if chars[i] == '\n' { line += 1; }
                    i += 1;
                }
                if i >= chars.len() {
                    return Err(LinguaError { message: "chorda non clausa".into(), line: opened_on });
                }
                let text: String = chars[start..i].iter().collect();
                i += 1; // consume closing quote
                tokens.push(Token { kind: TokenKind::Text(text), line: opened_on });
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
            // Ahead of the catch-all: writing `5` is the likeliest first
            // mistake in a Roman-numeral language, and `signum ignotum: '5'`
            // is true but tells the user nothing about what to write instead
            // (defectus D-15). The digit is quoted, so the suite's
            // no-Arabic-digits rule — which strips quoted spans — is satisfied.
            c if c.is_ascii_digit() => {
                return Err(LinguaError {
                    message: format!("numerus Arabicus '{}' non licet; scribe '°V' vel simile", c),
                    line,
                });
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
        "affer" => TokenKind::Import,
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

    fn lines(src: &str) -> Vec<usize> {
        tokenize(src).unwrap().into_iter().map(|t| t.line).collect()
    }

    #[test]
    fn a_multiline_string_advances_the_line_counter() {
        // Defectus D-03. The string body was scanned with a loop that never
        // looked at '\n', so every token after a multi-line literal carried a
        // line number short by one per embedded newline — and the drift is
        // unbounded, not off-by-one.
        let src = "voc s = \"prima\nsecunda\";\nni x = °I;";
        // `voc s =` and the literal are on line 1 — a literal is attributed to
        // the line it opens on. The literal *ends* on source line 2, so the
        // `;` that follows it is genuinely on line 2, and `ni x = °I ;` is on
        // line 3. Before the fix the last six tokens all read one short: the
        // `;` said 1 and the rest said 2.
        assert_eq!(lines(src), vec![1, 1, 1, 1, 2, 3, 3, 3, 3, 3]);
    }

    #[test]
    fn a_string_literal_is_attributed_to_the_line_it_opens_on() {
        // The literal itself keeps the line where it starts, not where it
        // ends, so a diagnostic about the string points at the string.
        let src = "voc s = \"prima\nsecunda\";";
        let toks = tokenize(src).unwrap();
        let text = toks.iter().find(|t| matches!(t.kind, TokenKind::Text(_))).unwrap();
        assert_eq!(text.line, 1);
    }

    #[test]
    fn a_numeral_gives_back_a_trailing_separator() {
        // Defectus D-14. '.', ':' and '/' are numeral separators (°XXX.M,
        // °V:I/II) and also the language's Dot and Slash. The scan is greedy,
        // so `°X/°II` used to consume the '/' into the numeral and fail with
        // `numerus Romanus invalidus: 'X/'` — while `°X / °II`, the identical
        // expression, evaluated fine. Whitespace was load-bearing.
        assert_eq!(
            kinds("°X/°II"),
            vec![
                TokenKind::Int(Roman::from_u64(10)),
                TokenKind::Slash,
                TokenKind::Int(Roman::from_u64(2)),
            ]
        );
        // The same trap with '.', which used to give `numerus Romanus vacuus`.
        assert_eq!(
            kinds("°V.summa"),
            vec![
                TokenKind::Int(Roman::from_u64(5)),
                TokenKind::Dot,
                TokenKind::Ident("summa".into()),
            ]
        );
    }

    #[test]
    fn giving_back_separators_does_not_break_real_numerals() {
        // The trim must not touch a separator that is doing its job: a
        // fraction, or thousands-multiplication. Neither ends in a separator.
        assert_eq!(
            kinds("°V:I/II"),
            vec![TokenKind::Frac { numer: Roman::from_u64(11), den: Roman::from_u64(2) }]
        );
        assert_eq!(kinds("°XXX.M"), vec![TokenKind::Int(Roman::from_u64(30000))]);
    }

    #[test]
    fn a_numeral_that_is_only_a_degree_sign_still_reports_empty() {
        // The `i > start + 1` guard: '°' alone has no body to give back, so it
        // must still reach `parse_roman` and fail there rather than trimming
        // itself away.
        let e = tokenize("°").unwrap_err();
        assert_eq!(e.message, "numerus Romanus vacuus");
    }

    #[test]
    fn an_unterminated_fraction_still_reports_its_missing_denominator() {
        // `°I:I` ends in 'I', not a separator, so the trim leaves it alone and
        // it fails where it should. A per-character retreat — the fix D-14's
        // register entry implies — would instead have parsed `°I` and left a
        // stray `Ident("I")`, silently killing this diagnostic.
        let e = tokenize("nr x = °I:I;").unwrap_err();
        assert_eq!(e.message, "fractio sine denominatore");
    }

    #[test]
    fn an_arabic_digit_says_how_to_write_a_numeral() {
        // Defectus D-15. A digit used to fall through to the lexer's catch-all
        // as `signum ignotum: '5'`, which is true but useless — writing `5` is
        // the likeliest first mistake in a Roman-numeral language.
        let e = tokenize("ni x = 5;").unwrap_err();
        assert_eq!(e.message, "numerus Arabicus '5' non licet; scribe '°V' vel simile");
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
        assert_eq!(kinds("°V:I/II"), vec![TokenKind::Frac { numer: Roman::from_u64(11), den: Roman::from_u64(2) }]);
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

    #[test]
    fn affer_and_dot_tokenize() {
        assert_eq!(
            kinds("affer mathematica;"),
            vec![TokenKind::Import, TokenKind::Ident("mathematica".into()), TokenKind::Semicolon]
        );
        assert_eq!(kinds("."), vec![TokenKind::Dot]);
        // Word boundary: a word merely starting with `affer` stays one identifier.
        assert_eq!(kinds("afferre"), vec![TokenKind::Ident("afferre".into())]);
    }

    #[test]
    fn dot_inside_roman_literal_still_belongs_to_the_numeral() {
        // 30 * 1000 — the °-branch consumes the dot before the Dot rule sees it.
        assert_eq!(kinds("°XXX.M"), vec![TokenKind::Int(Roman::from_u64(30000))]);
        // A qualified call around a numeral: dot, numeral, dot stay separate.
        assert_eq!(
            kinds("mathematica.duplica(°V)"),
            vec![
                TokenKind::Ident("mathematica".into()), TokenKind::Dot,
                TokenKind::Ident("duplica".into()), TokenKind::LParen,
                TokenKind::Int(Roman::from_u64(5)), TokenKind::RParen,
            ]
        );
    }

    #[test]
    fn every_keyword_is_named_by_the_word_that_produces_it() {
        // The round trip that keeps D-02's fix honest: when the parser reports
        // an expected keyword, the word it prints must be the word the user
        // would have had to type. Adding a keyword to `word_to_kind` without
        // naming it in `TokenKind::name` breaks this.
        //
        // `nullus` is excluded deliberately: it lexes to a *value*,
        // `Int(zero)`, not to a keyword kind of its own.
        let keywords = [
            "verus", "falsus", "si", "aut", "indem", "dum", "munus", "redde",
            "abrumpe", "continua", "scribe", "affer", "capit", "reddit",
            "lista", "ni", "mnr", "nr", "voc", "lit", "veritas", "et", "vel",
            "non", "par_est", "par_non_est", "maior_est", "minor_est",
        ];
        for w in keywords {
            assert_eq!(
                word_to_kind(w.to_string()).name(),
                w,
                "the keyword {w:?} is not named after itself"
            );
        }
    }
}
