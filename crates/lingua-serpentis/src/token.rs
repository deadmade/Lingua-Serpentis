use crate::roman::{Roman, RomanNumber};

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // literals (values already converted)
    Int(Roman),
    Frac { numer: Roman, den: Roman },
    Text(String),
    Char(char),
    True,
    False,
    // type keywords
    TyInt, TyFloat, TyDouble, TyText, TyChar, TyBool,   // ni | mnr | nr | voc | lit | veritas
    // keywords
    If, Else, While, For, Func, Return, Break, Continue, Print,
    Takes, Yields,                             // capit | reddit
    Lista,                                     // lista  (list type constructor)
    Import,                                    // affer
    // word operators
    And, Or, Not,                              // et | vel | non
    Eq, Ne, Gt, Lt,                            // par_est | par_non_est | maior_est | minor_est
    // symbol operators
    Assign, Plus, Minus, Star, Slash, Percent,
    // punctuation
    LParen, RParen, LBrace, RBrace, LBracket, RBracket, Semicolon, Comma, Dot,
    // identifiers
    Ident(String),
}

impl TokenKind {
    /// The name to show a user when this token is the one that was missing.
    ///
    /// Punctuation renders as its glyph, keywords as the Latin word that
    /// produces them, and the value-carrying kinds as a Latin category noun.
    ///
    /// Exhaustive on purpose, with no catch-all arm. The defect this replaces
    /// (D-02) was `format!("expectatum: {:?}", kind)`, which coupled
    /// user-facing text to a private Rust enum so that renaming a variant
    /// silently rewrote a diagnostic. A catch-all would quietly reintroduce
    /// exactly that for every variant added later; as written, adding one is a
    /// compile error until it is named here.
    pub fn name(&self) -> &'static str {
        match self {
            // punctuation
            TokenKind::Semicolon => ";",
            TokenKind::Comma => ",",
            TokenKind::Dot => ".",
            TokenKind::LParen => "(",
            TokenKind::RParen => ")",
            TokenKind::LBrace => "{",
            TokenKind::RBrace => "}",
            TokenKind::LBracket => "[",
            TokenKind::RBracket => "]",
            // symbol operators
            TokenKind::Assign => "=",
            TokenKind::Plus => "+",
            TokenKind::Minus => "-",
            TokenKind::Star => "*",
            TokenKind::Slash => "/",
            TokenKind::Percent => "%",
            // keywords, each named by the word `word_to_kind` maps to it
            TokenKind::If => "si",
            TokenKind::Else => "aut",
            TokenKind::While => "indem",
            TokenKind::For => "dum",
            TokenKind::Func => "munus",
            TokenKind::Return => "redde",
            TokenKind::Break => "abrumpe",
            TokenKind::Continue => "continua",
            TokenKind::Print => "scribe",
            TokenKind::Takes => "capit",
            TokenKind::Yields => "reddit",
            TokenKind::Lista => "lista",
            TokenKind::Import => "affer",
            TokenKind::TyInt => "ni",
            TokenKind::TyFloat => "mnr",
            TokenKind::TyDouble => "nr",
            TokenKind::TyText => "voc",
            TokenKind::TyChar => "lit",
            TokenKind::TyBool => "veritas",
            TokenKind::And => "et",
            TokenKind::Or => "vel",
            TokenKind::Not => "non",
            TokenKind::Eq => "par_est",
            TokenKind::Ne => "par_non_est",
            TokenKind::Gt => "maior_est",
            TokenKind::Lt => "minor_est",
            TokenKind::True => "verus",
            TokenKind::False => "falsus",
            // Value-carrying kinds. `eat` is never called with one of these —
            // it matches on a kind the parser already knows — but naming them
            // is what keeps the match exhaustive and the mapping honest.
            TokenKind::Int(_) => "numerus",
            TokenKind::Frac { .. } => "numerus fractus",
            TokenKind::Text(_) => "chorda",
            TokenKind::Char(_) => "littera",
            TokenKind::Ident(_) => "nomen",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub line: usize,
}

// Helper so a RomanNumber lands in the right token kind.
impl From<RomanNumber> for TokenKind {
    fn from(n: RomanNumber) -> Self {
        match n {
            RomanNumber::Int(r) => TokenKind::Int(r),
            RomanNumber::Frac { numer, den } => TokenKind::Frac { numer, den },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn punctuation_is_named_by_the_glyph_the_user_would_type() {
        // The seven kinds `Parser::eat` is actually called with. A missing
        // token is a missing *character*, so the actionable thing to show is
        // the character — `expectatum: ';'`, not a word for the semicolon.
        // This also matches the wording the parser already used by hand for
        // the same idea: `errores/uncus-non-clausus` says `'}' expectatum`.
        assert_eq!(TokenKind::Semicolon.name(), ";");
        assert_eq!(TokenKind::Assign.name(), "=");
        assert_eq!(TokenKind::LParen.name(), "(");
        assert_eq!(TokenKind::RParen.name(), ")");
        assert_eq!(TokenKind::LBrace.name(), "{");
        assert_eq!(TokenKind::RBrace.name(), "}");
        assert_eq!(TokenKind::RBracket.name(), "]");
    }

    #[test]
    fn no_kind_is_named_by_its_rust_variant() {
        // Defectus D-02: `eat` formatted the kind with `{:?}`, so a missing
        // semicolon read `expectatum: Semicolon` — user-facing text coupled to
        // a private Rust enum, where renaming a variant silently rewrites what
        // the user sees. No name may be its own Debug rendering.
        let kinds = [
            TokenKind::Semicolon, TokenKind::Comma, TokenKind::Dot,
            TokenKind::LParen, TokenKind::RParen, TokenKind::LBrace,
            TokenKind::RBrace, TokenKind::LBracket, TokenKind::RBracket,
            TokenKind::Assign, TokenKind::Plus, TokenKind::Minus,
            TokenKind::Star, TokenKind::Slash, TokenKind::Percent,
            TokenKind::If, TokenKind::Else, TokenKind::While, TokenKind::For,
            TokenKind::Func, TokenKind::Return, TokenKind::Break,
            TokenKind::Continue, TokenKind::Print, TokenKind::Takes,
            TokenKind::Yields, TokenKind::Lista, TokenKind::Import,
            TokenKind::TyInt, TokenKind::TyFloat, TokenKind::TyDouble,
            TokenKind::TyText, TokenKind::TyChar, TokenKind::TyBool,
            TokenKind::And, TokenKind::Or, TokenKind::Not,
            TokenKind::Eq, TokenKind::Ne, TokenKind::Gt, TokenKind::Lt,
            TokenKind::True, TokenKind::False,
            TokenKind::Text(String::new()), TokenKind::Char('a'),
            TokenKind::Ident(String::new()),
        ];
        for k in kinds {
            assert_ne!(
                k.name(),
                format!("{k:?}"),
                "{k:?} is named by its Rust variant"
            );
        }
    }
}
