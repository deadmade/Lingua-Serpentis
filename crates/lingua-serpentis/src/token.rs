use crate::roman::{Roman, RomanNumber};

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    Int(Roman),
    Frac { numer: Roman, den: Roman },
    Text(String),
    Char(char),
    True,
    False,
    TyInt, TyFloat, TyDouble, TyText, TyChar, TyBool,   // ni | mnr | nr | voc | lit | veritas
    If, Else, While, For, Func, Return, Break, Continue, Print,
    Takes, Yields,                             // capit | reddit
    Lista,
    Import,                                    // affer
    And, Or, Not,                              // et | vel | non
    Eq, Ne, Gt, Lt,                            // par_est | par_non_est | maior_est | minor_est
    Assign, Plus, Minus, Star, Slash, Percent,
    LParen, RParen, LBrace, RBrace, LBracket, RBracket, Semicolon, Comma, Dot,
    Ident(String),
}

impl TokenKind {
    /// The name to show a user when this token is the one that was missing.
    /// Exhaustive with no catch-all arm, on purpose (D-02).
    pub fn name(&self) -> &'static str {
        match self {
            TokenKind::Semicolon => ";",
            TokenKind::Comma => ",",
            TokenKind::Dot => ".",
            TokenKind::LParen => "(",
            TokenKind::RParen => ")",
            TokenKind::LBrace => "{",
            TokenKind::RBrace => "}",
            TokenKind::LBracket => "[",
            TokenKind::RBracket => "]",
            TokenKind::Assign => "=",
            TokenKind::Plus => "+",
            TokenKind::Minus => "-",
            TokenKind::Star => "*",
            TokenKind::Slash => "/",
            TokenKind::Percent => "%",
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
        // Matches `conformance/errores/uncus-non-clausus`, which says `'}' expectatum`.
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
        // D-02
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
