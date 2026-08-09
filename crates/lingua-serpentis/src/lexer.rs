use crate::error::{LinguaError, LResult};
use crate::roman::Roman;
use crate::spatium::Spatium;
use crate::token::{Token, TokenKind};

pub fn tokenize(src: &str) -> LResult<Vec<Token>> {
    let chars: Vec<char> = src.chars().collect();
    // `offsets[k]` is the byte offset of `chars[k]`, and `offsets[chars.len()]` is
    // `src.len()`, so any `start..i` pair the loop already keeps converts to a byte
    // span by indexing twice. Built once rather than walking `char_indices` in step
    // with the cursor, which would touch every peek and slice below.
    let mut offsets: Vec<u32> = Vec::with_capacity(chars.len() + 1);
    let mut byte = 0u32;
    for c in &chars {
        offsets.push(byte);
        byte += c.len_utf8() as u32;
    }
    offsets.push(byte);
    let sp = |start: usize, end: usize, line: usize| Spatium::novum(offsets[start], offsets[end], line);

    let mut tokens = Vec::new();
    let mut i = 0;
    let mut line = 1;

    while i < chars.len() {
        let c = chars[i];
        // Every single-character arm below spans `i..i + 1`; the multi-character
        // ones close over the `start` they already keep.
        let uno = sp(i, i + 1, line);
        match c {
            '\n' => { line += 1; i += 1; }
            ' ' | '\t' | '\r' => { i += 1; }
            '/' if chars.get(i + 1) == Some(&'/') => {
                while i < chars.len() && chars[i] != '\n' { i += 1; }
            }
            '(' => { tokens.push(Token { kind: TokenKind::LParen, line, spatium: uno }); i += 1; }
            ')' => { tokens.push(Token { kind: TokenKind::RParen, line, spatium: uno }); i += 1; }
            '{' => { tokens.push(Token { kind: TokenKind::LBrace, line, spatium: uno }); i += 1; }
            '}' => { tokens.push(Token { kind: TokenKind::RBrace, line, spatium: uno }); i += 1; }
            '[' => { tokens.push(Token { kind: TokenKind::LBracket, line, spatium: uno }); i += 1; }
            ']' => { tokens.push(Token { kind: TokenKind::RBracket, line, spatium: uno }); i += 1; }
            ';' => { tokens.push(Token { kind: TokenKind::Semicolon, line, spatium: uno }); i += 1; }
            ',' => { tokens.push(Token { kind: TokenKind::Comma, line, spatium: uno }); i += 1; }
            '.' => { tokens.push(Token { kind: TokenKind::Dot, line, spatium: uno }); i += 1; }
            '+' => { tokens.push(Token { kind: TokenKind::Plus, line, spatium: uno }); i += 1; }
            '-' => { tokens.push(Token { kind: TokenKind::Minus, line, spatium: uno }); i += 1; }
            '*' => { tokens.push(Token { kind: TokenKind::Star, line, spatium: uno }); i += 1; }
            '/' => { tokens.push(Token { kind: TokenKind::Slash, line, spatium: uno }); i += 1; }
            '%' => { tokens.push(Token { kind: TokenKind::Percent, line, spatium: uno }); i += 1; }
            '=' => { tokens.push(Token { kind: TokenKind::Assign, line, spatium: uno }); i += 1; }
            '°' => {
                let start = i;
                i += 1;
                while i < chars.len() && is_roman_body(chars[i]) { i += 1; }
                // Give back a trailing separator, over separators and not per
                // character; `i > start + 1` keeps a bare '°' intact (D-14).
                while i > start + 1 && matches!(chars[i - 1], '.' | ':' | '/') {
                    i -= 1;
                }
                let lexeme: String = chars[start..i].iter().collect();
                let value = crate::roman::parse_roman(&lexeme)
                    .map_err(|m| LinguaError { message: m, line, spatium: None, })?;
                tokens.push(Token { kind: value.into(), line, spatium: sp(start, i, line) });
            }
            '"' => {
                // Attributed to the line it opens on, while `line` advances
                // past the newlines inside it (D-03).
                let opened_on = line;
                let quote = i;
                i += 1;
                let start = i;
                while i < chars.len() && chars[i] != '"' {
                    if chars[i] == '\n' { line += 1; }
                    i += 1;
                }
                if i >= chars.len() {
                    return Err(LinguaError { message: "chorda non clausa".into(), line: opened_on, spatium: None, });
                }
                let text: String = chars[start..i].iter().collect();
                i += 1;
                // The span covers both quotes; the `kind` covers neither.
                tokens.push(Token {
                    kind: TokenKind::Text(text),
                    line: opened_on,
                    spatium: sp(quote, i, opened_on),
                });
            }
            '\'' => {
                let quote = i;
                i += 1;
                if i + 1 >= chars.len() || chars[i + 1] != '\'' {
                    return Err(LinguaError { message: "littera invalida".into(), line, spatium: None, });
                }
                let ch = chars[i];
                i += 2;
                tokens.push(Token { kind: TokenKind::Char(ch), line, spatium: sp(quote, i, line) });
            }
            c if c.is_alphabetic() || c == '_' => {
                let start = i;
                while i < chars.len()
                    && (chars[i].is_alphanumeric() || chars[i] == '_')
                {
                    i += 1;
                }
                let word: String = chars[start..i].iter().collect();
                tokens.push(Token { kind: word_to_kind(word), line, spatium: sp(start, i, line) });
            }
            // Ahead of the catch-all; the digit is quoted (D-15).
            c if c.is_ascii_digit() => {
                return Err(LinguaError {
                    message: format!("numerus Arabicus '{}' non licet; scribe '°V' vel simile", c),
                    line, spatium: None,
                });
            }
            _ => {
                return Err(LinguaError {
                    message: format!("signum ignotum: '{}'", c),
                    line, spatium: None,
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

    /// The source text each token's span selects, which is what a caret underlines.
    fn lexemes(src: &str) -> Vec<&str> {
        tokenize(src)
            .unwrap()
            .into_iter()
            .map(|t| &src[t.spatium.principium as usize..t.spatium.finis as usize])
            .collect()
    }

    #[test]
    fn every_span_selects_the_text_the_token_was_lexed_from() {
        let src = "ni x = °XL + °II;";
        assert_eq!(lexemes(src), vec!["ni", "x", "=", "°XL", "+", "°II", ";"]);
    }

    #[test]
    fn a_numeral_span_covers_its_degree_sign() {
        // The caret must sit under `°XXV`, not under `XXV`: `°` is where the
        // numeral starts and is two bytes wide.
        let toks = tokenize("°XXV").unwrap();
        assert_eq!(toks[0].spatium, crate::spatium::Spatium::novum(0, 5, 1));
    }

    #[test]
    fn a_span_is_bytes_and_not_char_indices() {
        // `mathēmatica` is eleven chars and twelve bytes; a char-index span
        // would select one byte too few and split the `ē`.
        let src = "ni mathēmatica = °I;";
        assert_eq!(lexemes(src)[1], "mathēmatica");
    }

    #[test]
    fn a_multiline_string_span_runs_from_opening_quote_to_closing_quote() {
        // D-03: the line is the one it opens on, and the span is the whole literal.
        let src = "voc s = \"prima\nsecunda\";";
        let toks = tokenize(src).unwrap();
        let text = toks.iter().find(|t| matches!(t.kind, TokenKind::Text(_))).unwrap();
        assert_eq!(text.spatium.linea(), 1);
        assert_eq!(
            &src[text.spatium.principium as usize..text.spatium.finis as usize],
            "\"prima\nsecunda\""
        );
    }

    #[test]
    fn giving_back_a_separator_shrinks_the_span_with_it() {
        // D-14: the numeral gave back `.`, so its span must end before it.
        assert_eq!(lexemes("°V.summa"), vec!["°V", ".", "summa"]);
    }

    #[test]
    fn a_span_on_a_later_line_starts_after_the_newline() {
        let src = "ni x = °I;\nni y = °II;";
        let toks = tokenize(src).unwrap();
        let y = toks.iter().find(|t| t.kind == TokenKind::Ident("y".into())).unwrap();
        assert_eq!(y.spatium.linea(), 2);
        assert_eq!(&src[y.spatium.principium as usize..y.spatium.finis as usize], "y");
    }

    #[test]
    fn a_char_literal_span_covers_both_quotes() {
        assert_eq!(lexemes("lit c = 'a';"), vec!["lit", "c", "=", "'a'", ";"]);
    }

    #[test]
    fn a_multiline_string_advances_the_line_counter() {
        // D-03
        let src = "voc s = \"prima\nsecunda\";\nni x = °I;";
        assert_eq!(lines(src), vec![1, 1, 1, 1, 2, 3, 3, 3, 3, 3]);
    }

    #[test]
    fn a_string_literal_is_attributed_to_the_line_it_opens_on() {
        let src = "voc s = \"prima\nsecunda\";";
        let toks = tokenize(src).unwrap();
        let text = toks.iter().find(|t| matches!(t.kind, TokenKind::Text(_))).unwrap();
        assert_eq!(text.line, 1);
    }

    #[test]
    fn a_numeral_gives_back_a_trailing_separator() {
        // D-14
        assert_eq!(
            kinds("°X/°II"),
            vec![
                TokenKind::Int(Roman::from_u64(10)),
                TokenKind::Slash,
                TokenKind::Int(Roman::from_u64(2)),
            ]
        );
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
        assert_eq!(
            kinds("°V:I/II"),
            vec![TokenKind::Frac { numer: Roman::from_u64(11), den: Roman::from_u64(2) }]
        );
        assert_eq!(kinds("°XXX.M"), vec![TokenKind::Int(Roman::from_u64(30000))]);
    }

    #[test]
    fn a_numeral_that_is_only_a_degree_sign_still_reports_empty() {
        let e = tokenize("°").unwrap_err();
        assert_eq!(e.message, "numerus Romanus vacuus");
    }

    #[test]
    fn an_unterminated_fraction_still_reports_its_missing_denominator() {
        // A per-character retreat would parse `°I` and kill this diagnostic (D-14).
        let e = tokenize("nr x = °I:I;").unwrap_err();
        assert_eq!(e.message, "fractio sine denominatore");
    }

    #[test]
    fn an_arabic_digit_says_how_to_write_a_numeral() {
        // D-15
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
        assert_eq!(kinds("nimis"), vec![TokenKind::Ident("nimis".into())]);
    }

    #[test]
    fn header_keywords() {
        assert_eq!(kinds("capit reddit"), vec![TokenKind::Takes, TokenKind::Yields]);
        assert_eq!(kinds("capitulum"), vec![TokenKind::Ident("capitulum".into())]);
        assert_eq!(kinds("redditus"), vec![TokenKind::Ident("redditus".into())]);
    }

    #[test]
    fn literal_error_paths() {
        assert!(tokenize("\"unterminated").is_err());
        assert!(tokenize("''").is_err());
        assert!(tokenize("'ab'").is_err());
    }

    #[test]
    fn brackets_and_lista_keyword() {
        assert_eq!(
            kinds("[]"),
            vec![TokenKind::LBracket, TokenKind::RBracket]
        );
        assert_eq!(kinds("lista"), vec![TokenKind::Lista]);
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
        assert_eq!(kinds("afferre"), vec![TokenKind::Ident("afferre".into())]);
    }

    #[test]
    fn dot_inside_roman_literal_still_belongs_to_the_numeral() {
        assert_eq!(kinds("°XXX.M"), vec![TokenKind::Int(Roman::from_u64(30000))]);
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
        // `nullus` is excluded: it lexes to a value, `Int(zero)`, not a keyword kind.
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
