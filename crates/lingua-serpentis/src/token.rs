use crate::roman::{Roman, RomanNumber};

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // literals (values already converted)
    Int(Roman),
    Float(f64),
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
    // word operators
    And, Or, Not,                              // et | vel | non
    Eq, Ne, Gt, Lt,                            // par_est | par_non_est | maior_est | minor_est
    // symbol operators
    Assign, Plus, Minus, Star, Slash, Percent,
    // punctuation
    LParen, RParen, LBrace, RBrace, LBracket, RBracket, Semicolon, Comma,
    // identifiers
    Ident(String),
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
            RomanNumber::Float(f) => TokenKind::Float(f),
        }
    }
}
