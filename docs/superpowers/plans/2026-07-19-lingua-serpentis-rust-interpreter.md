# Lingua Serpentis — Rust Interpreter Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a statically-typed, tree-walking interpreter for Lingua Serpentis (Latin keywords, Roman-numeral math) as a fresh Rust crate.

**Architecture:** One pipeline, three walks over one AST — `lexer → parser → type checker → interpreter`. The lexer converts Roman-numeral lexemes to numbers up front; the parser emits typed Rust enums; the checker validates types before execution; the interpreter evaluates the validated tree and returns collected output. No transpilation, no `gcc`.

**Tech Stack:** Rust (stable, edition 2021), `cargo` for build/test. No third-party crates in v1 (standard library only).

## Global Constraints

- **Language:** Rust, edition 2021, standard library only (no external crates in v1).
- **Latin keywords are fixed** (exact spelling): `si` `aut` `indem` `dum` `munus` `redde` `verus` `falsus` `abrumpe` `continua` `scribe` `et` `vel` `non`; comparisons `par_est` `par_non_est` `maior_est` `minor_est`; type keywords `ni` `mnr` `nr` `voc` `lit` `veritas`; zero literal `nullus`.
- **Types:** explicit and static. Type enum is `Int, Float, Text, Char, Bool`. Per the spec's open-question resolution, **`mnr` and `nr` both map to `Type::Float`** in v1 (both kept as keywords for flavour, both backed by `f64`).
- **Roman-numeral encoding** (preserve exactly): `°` prefix; chunks split on `.` combine in multiplicative pairs with a trailing lone chunk added (`°XXX.M.V.C.VII` → `30*1000 + 5*100 + 7` = `30507`); optional fraction after `:` written `num/den` (`°V:I/II` → `5.5`); `nullus` → `0`. A purely-integer literal is `Int`; a literal with a fraction is `Float`.
- **Errors** carry a message + line number and print as `Error in linea <n>: <message>` with the message phrased in Latin.
- **Cross-platform:** must run on Windows/macOS/Linux (no shelling out to a compiler).
- **v1 scope:** typed variables, Roman literals, `+ - * / %`, Latin comparisons, `et`/`vel`/`non`, `si/aut`/`indem`/`dum` with `abrumpe`/`continua`, typed functions, `scribe`, and the static type checker. **Out of scope:** arrays, type inference, imports, try/catch, REPL, `>=`/`<=`.

## File Structure

```
lingua-serpentis/
  Cargo.toml
  src/
    main.rs        CLI: read .liss file, run pipeline (lex→parse→check→interp), print output/errors
    error.rs       LinguaError { message, line } + Display + LResult alias
    roman.rs       Roman numerals → RomanNumber { Int(i64) | Float(f64) }  (pure, unit-tested)
    token.rs       Token { kind: TokenKind, line } + TokenKind enum
    lexer.rs       tokenize(&str) -> LResult<Vec<Token>>
    ast.rs         Type, BinOp, UnOp, Expr, Stmt { kind: StmtKind, line }, StmtKind, FuncDecl
    parser.rs      parse(&[Token]) -> LResult<Vec<Stmt>>  (recursive descent)
    checker.rs     check(&[Stmt]) -> LResult<()>           (walk #1)
    value.rs       Value { Int|Float|Text|Char|Bool } + display()
    interp.rs      execute(&[Stmt]) -> LResult<String>     (walk #2, returns collected output)
  tests/
    programs.rs    integration tests: full .liss source → output, and expected type errors
  examples/
    salve.liss, calc.liss   sample programs
```

Module dependency direction: `error` ← everything; `roman` ← `lexer`; `token` ← `lexer`, `parser`; `ast` ← `parser`, `checker`, `interp`; `value` ← `interp`. No cycles.

---

### Task 1: Project scaffold + error type

**Files:**
- Create: `Cargo.toml`, `src/main.rs`, `src/error.rs`
- Test: inline `#[cfg(test)]` in `src/error.rs`

**Interfaces:**
- Produces: `error::LinguaError { message: String, line: usize }`, `error::LResult<T> = Result<T, LinguaError>`, `impl Display for LinguaError` printing `Error in linea {line}: {message}`.

- [ ] **Step 1: Create the crate skeleton**

`Cargo.toml`:
```toml
[package]
name = "lingua-serpentis"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "lingua-serpentis"
path = "src/main.rs"
```

`src/main.rs` (temporary stub, replaced in Task 13):
```rust
mod error;

fn main() {
    println!("lingua-serpentis: nondum paratum"); // "not ready yet"
}
```

- [ ] **Step 2: Write the failing test for the error type**

`src/error.rs`:
```rust
#[derive(Debug, Clone, PartialEq)]
pub struct LinguaError {
    pub message: String,
    pub line: usize,
}

pub type LResult<T> = Result<T, LinguaError>;

impl std::fmt::Display for LinguaError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Error in linea {}: {}", self.line, self.message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_with_line_and_message() {
        let e = LinguaError { message: "res mala".into(), line: 5 };
        assert_eq!(e.to_string(), "Error in linea 5: res mala");
    }
}
```

- [ ] **Step 3: Run test to verify it fails, then passes after the file compiles**

Run: `cargo test --lib error 2>&1 | tail -20`
Expected: compiles and `formats_with_line_and_message ... ok`.
(If `main.rs` fails to see `error`, ensure `mod error;` is present.)

- [ ] **Step 4: Verify the binary builds and runs**

Run: `cargo run` 
Expected: prints `lingua-serpentis: nondum paratum`.

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml src/main.rs src/error.rs
git commit -m "feat: scaffold crate + LinguaError type"
```

---

### Task 2: Roman numerals — single chunk + integer combination

**Files:**
- Create: `src/roman.rs`
- Modify: `src/main.rs` (add `mod roman;`)
- Test: inline `#[cfg(test)]` in `src/roman.rs`

**Interfaces:**
- Produces: `roman::convert_single(&str) -> Result<i64, String>` and `roman::convert_int_roman(&str) -> Result<i64, String>` (both `pub(crate)`). Error strings are Latin phrases without line numbers (the lexer attaches the line).

- [ ] **Step 1: Write failing tests**

`src/roman.rs`:
```rust
pub(crate) fn convert_single(s: &str) -> Result<i64, String> {
    unimplemented!()
}

pub(crate) fn convert_int_roman(s: &str) -> Result<i64, String> {
    unimplemented!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_basic_and_subtractive() {
        assert_eq!(convert_single("XXV").unwrap(), 25);
        assert_eq!(convert_single("VII").unwrap(), 7);
        assert_eq!(convert_single("IV").unwrap(), 4);
        assert_eq!(convert_single("IX").unwrap(), 9);
        assert_eq!(convert_single("M").unwrap(), 1000);
    }

    #[test]
    fn single_rejects_garbage() {
        assert!(convert_single("XQ").is_err());
    }

    #[test]
    fn int_multiplicative_pairs() {
        // XXX.M.V.C.VII = 30*1000 + 5*100 + 7 = 30507
        assert_eq!(convert_int_roman("XXX.M.V.C.VII").unwrap(), 30507);
        // VII.M = 7*1000 = 7000  (even number of chunks: pure product)
        assert_eq!(convert_int_roman("VII.M").unwrap(), 7000);
        // single chunk, no dots
        assert_eq!(convert_int_roman("XXV").unwrap(), 25);
    }
}
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test --lib roman 2>&1 | tail -20`
Expected: panics with `not implemented`.

- [ ] **Step 3: Implement**

Replace the two `unimplemented!()` bodies:
```rust
pub(crate) fn convert_single(s: &str) -> Result<i64, String> {
    if s.is_empty() {
        return Err("numerus Romanus vacuus".into());
    }
    fn val(c: char) -> Option<i64> {
        match c {
            'I' => Some(1), 'V' => Some(5), 'X' => Some(10), 'L' => Some(50),
            'C' => Some(100), 'D' => Some(500), 'M' => Some(1000), _ => None,
        }
    }
    let chars: Vec<char> = s.chars().collect();
    let mut total = 0i64;
    for i in 0..chars.len() {
        let cur = val(chars[i]).ok_or_else(|| format!("numerus Romanus invalidus: '{}'", s))?;
        let next = chars.get(i + 1).and_then(|c| val(*c)).unwrap_or(0);
        if cur < next { total -= cur; } else { total += cur; }
    }
    Ok(total)
}

pub(crate) fn convert_int_roman(s: &str) -> Result<i64, String> {
    let nums: Vec<i64> = s
        .split('.')
        .map(convert_single)
        .collect::<Result<_, _>>()?;
    let mut total = 0i64;
    let mut i = 0;
    while i + 1 < nums.len() {
        total += nums[i] * nums[i + 1];
        i += 2;
    }
    if nums.len() % 2 == 1 {
        total += nums[nums.len() - 1];
    }
    Ok(total)
}
```

Add `mod roman;` to `src/main.rs`.

- [ ] **Step 4: Run to verify pass**

Run: `cargo test --lib roman 2>&1 | tail -20`
Expected: all roman tests `ok`.

- [ ] **Step 5: Commit**

```bash
git add src/roman.rs src/main.rs
git commit -m "feat: roman numeral single + integer conversion"
```

---

### Task 3: Roman numerals — public parser (fractions, nullus, `°`)

**Files:**
- Modify: `src/roman.rs`
- Test: inline `#[cfg(test)]` in `src/roman.rs`

**Interfaces:**
- Produces: `roman::RomanNumber { Int(i64), Float(f64) }` (pub, derives `Debug, Clone, PartialEq`) and `roman::parse_roman(&str) -> Result<RomanNumber, String>`. Accepts a full lexeme: either `nullus` or a `°`-prefixed body, optionally with a `:num/den` fraction.

- [ ] **Step 1: Write failing tests**

Add to `src/roman.rs`:
```rust
#[derive(Debug, Clone, PartialEq)]
pub enum RomanNumber {
    Int(i64),
    Float(f64),
}

pub fn parse_roman(literal: &str) -> Result<RomanNumber, String> {
    unimplemented!()
}
```
Add to the `tests` module:
```rust
    #[test]
    fn parse_integer_literal() {
        assert_eq!(parse_roman("°XXV").unwrap(), RomanNumber::Int(25));
        assert_eq!(parse_roman("°XXX.M.V.C.VII").unwrap(), RomanNumber::Int(30507));
    }

    #[test]
    fn parse_nullus_is_zero() {
        assert_eq!(parse_roman("nullus").unwrap(), RomanNumber::Int(0));
    }

    #[test]
    fn parse_fraction_literal() {
        assert_eq!(parse_roman("°V:I/II").unwrap(), RomanNumber::Float(5.5));
    }

    #[test]
    fn parse_rejects_missing_degree() {
        assert!(parse_roman("XXV").is_err());
    }
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test --lib roman::tests::parse 2>&1 | tail -20`
Expected: panics with `not implemented`.

- [ ] **Step 3: Implement**

Replace `parse_roman`:
```rust
pub fn parse_roman(literal: &str) -> Result<RomanNumber, String> {
    if literal == "nullus" {
        return Ok(RomanNumber::Int(0));
    }
    let body = literal
        .strip_prefix('°')
        .ok_or_else(|| format!("numerus sine '°': '{}'", literal))?;

    let mut parts = body.splitn(2, ':');
    let int_str = parts.next().unwrap();
    let int_val = convert_int_roman(int_str)?;

    match parts.next() {
        None => Ok(RomanNumber::Int(int_val)),
        Some(frac) => {
            let mut fp = frac.splitn(2, '/');
            let num_str = fp.next().ok_or("fractio sine numeratore")?;
            let den_str = fp.next().ok_or("fractio sine denominatore")?;
            let num = convert_int_roman(num_str)?;
            let den = convert_int_roman(den_str)?;
            if den == 0 {
                return Err("denominator est nullus".into());
            }
            Ok(RomanNumber::Float(int_val as f64 + num as f64 / den as f64))
        }
    }
}
```

- [ ] **Step 4: Run to verify pass**

Run: `cargo test --lib roman 2>&1 | tail -20`
Expected: all roman tests `ok`.

- [ ] **Step 5: Commit**

```bash
git add src/roman.rs
git commit -m "feat: roman parse_roman with fractions and nullus"
```

---

### Task 4: Lexer — symbols, operators, comments, whitespace, line tracking

**Files:**
- Create: `src/token.rs`, `src/lexer.rs`
- Modify: `src/main.rs` (add `mod token; mod lexer;`)
- Test: inline `#[cfg(test)]` in `src/lexer.rs`

**Interfaces:**
- Produces: `token::TokenKind` (full enum below), `token::Token { kind: TokenKind, line: usize }`, `lexer::tokenize(&str) -> LResult<Vec<Token>>`. This task implements only symbols/operators/whitespace/comments; literals and words are `unimplemented` paths added in Task 5.

- [ ] **Step 1: Define the full token set**

`src/token.rs`:
```rust
use crate::roman::RomanNumber;

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // literals (values already converted)
    Int(i64),
    Float(f64),
    Text(String),
    Char(char),
    True,
    False,
    // type keywords
    TyInt, TyFloat, TyText, TyChar, TyBool,   // ni | (mnr|nr) | voc | lit | veritas
    // keywords
    If, Else, While, For, Func, Return, Break, Continue, Print,
    // word operators
    And, Or, Not,                              // et | vel | non
    Eq, Ne, Gt, Lt,                            // par_est | par_non_est | maior_est | minor_est
    // symbol operators
    Assign, Plus, Minus, Star, Slash, Percent,
    // punctuation
    LParen, RParen, LBrace, RBrace, Semicolon, Comma,
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
            RomanNumber::Int(i) => TokenKind::Int(i),
            RomanNumber::Float(f) => TokenKind::Float(f),
        }
    }
}
```

- [ ] **Step 2: Write failing tests**

`src/lexer.rs`:
```rust
use crate::error::{LinguaError, LResult};
use crate::token::{Token, TokenKind};

pub fn tokenize(src: &str) -> LResult<Vec<Token>> {
    unimplemented!()
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
```

- [ ] **Step 3: Run to verify failure**

Run: `cargo test --lib lexer 2>&1 | tail -20`
Expected: panics with `not implemented`.

- [ ] **Step 4: Implement the symbol/whitespace/comment core**

Replace `tokenize`:
```rust
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
```

- [ ] **Step 5: Run to verify pass, then commit**

Run: `cargo test --lib lexer 2>&1 | tail -20`
Expected: the three lexer tests `ok`.
```bash
git add src/token.rs src/lexer.rs src/main.rs
git commit -m "feat: lexer symbols, comments, whitespace, line tracking"
```

---

### Task 5: Lexer — literals (Roman/string/char) + identifiers & keywords

**Files:**
- Modify: `src/lexer.rs`
- Test: inline `#[cfg(test)]` in `src/lexer.rs`

**Interfaces:**
- Consumes: `roman::parse_roman`, `TokenKind::from(RomanNumber)`.
- Produces: `tokenize` now handles `°…`/`nullus` numbers, `"…"` strings, `'x'` chars, and word tokens (keywords, type keywords, word operators, comparisons via a keyword table; everything else → `Ident`). Word matching is greedy over `[A-Za-z_][A-Za-z0-9_]*`, so keyword *prefixes* never split identifiers (`nimis` is one `Ident`, not `ni` + `mis`).

- [ ] **Step 1: Write failing tests**

Add to the `tests` module in `src/lexer.rs`:
```rust
    #[test]
    fn roman_numbers_become_values() {
        assert_eq!(kinds("°XXV"), vec![TokenKind::Int(25)]);
        assert_eq!(kinds("°V:I/II"), vec![TokenKind::Float(5.5)]);
        assert_eq!(kinds("nullus"), vec![TokenKind::Int(0)]);
    }

    #[test]
    fn strings_and_chars() {
        assert_eq!(kinds("\"salve\""), vec![TokenKind::Text("salve".into())]);
        assert_eq!(kinds("'x'"), vec![TokenKind::Char('x')]);
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
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test --lib lexer 2>&1 | tail -25`
Expected: new tests fail (the `_ =>` arm errors on `°`, `"`, letters).

- [ ] **Step 3: Implement literals + words**

In `tokenize`, replace the `_ =>` arm with number/string/char/word handling:
```rust
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
```
Add these helpers at the bottom of `src/lexer.rs` (outside `tokenize`):
```rust
fn is_roman_body(c: char) -> bool {
    matches!(c, 'I' | 'V' | 'X' | 'L' | 'C' | 'D' | 'M' | '.' | ':' | '/')
}

fn word_to_kind(word: String) -> TokenKind {
    match word.as_str() {
        "nullus" => TokenKind::Int(0),
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
        "ni" => TokenKind::TyInt,
        "mnr" | "nr" => TokenKind::TyFloat, // both float in v1
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
```

- [ ] **Step 4: Run to verify pass**

Run: `cargo test --lib lexer 2>&1 | tail -25`
Expected: all lexer tests `ok`.

- [ ] **Step 5: Commit**

```bash
git add src/lexer.rs
git commit -m "feat: lexer literals, identifiers, keyword table"
```

---

### Task 6: AST + expression parser

**Files:**
- Create: `src/ast.rs`, `src/parser.rs`
- Modify: `src/main.rs` (add `mod ast; mod parser;`)
- Test: inline `#[cfg(test)]` in `src/parser.rs`

**Interfaces:**
- Produces: the AST types in `ast.rs` and `parser::parse(&[Token]) -> LResult<Vec<Stmt>>`. This task implements expression parsing plus a temporary top-level "bare expression statement" so expressions are testable; Task 7 replaces the statement layer.
- Precedence (low→high): `vel` < `et` < comparisons < `+ -` < `* / %` < unary(`non`, `-`) < primary(literal, ident, call, `( … )`).

- [ ] **Step 1: Define the AST**

`src/ast.rs`:
```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Type { Int, Float, Text, Char, Bool }

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BinOp { Add, Sub, Mul, Div, Mod, Eq, Ne, Gt, Lt, And, Or }

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UnOp { Not, Neg }

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Int(i64),
    Float(f64),
    Text(String),
    Bool(bool),
    Ident(String),
    Unary { op: UnOp, expr: Box<Expr> },
    Binary { op: BinOp, left: Box<Expr>, right: Box<Expr> },
    Call { name: String, args: Vec<Expr> },
}

#[derive(Debug, Clone, PartialEq)]
pub struct FuncDecl {
    pub name: String,
    pub params: Vec<(Type, String)>,
    pub ret: Type,
    pub body: Vec<Stmt>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Stmt {
    pub kind: StmtKind,
    pub line: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum StmtKind {
    Let { ty: Type, name: String, value: Expr },
    Assign { name: String, value: Expr },
    Print(Expr),
    ExprStmt(Expr),
    If { cond: Expr, then: Vec<Stmt>, else_: Option<Vec<Stmt>> },
    While { cond: Expr, body: Vec<Stmt> },
    For { init: Box<Stmt>, cond: Expr, step: Box<Stmt>, body: Vec<Stmt> },
    Func(FuncDecl),
    Return(Expr),
    Break,
    Continue,
}
```

- [ ] **Step 2: Write failing expression tests**

`src/parser.rs`:
```rust
use crate::ast::*;
use crate::error::{LinguaError, LResult};
use crate::token::{Token, TokenKind};

pub fn parse(tokens: &[Token]) -> LResult<Vec<Stmt>> {
    unimplemented!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::tokenize;

    fn parse_first_expr(src: &str) -> Expr {
        let toks = tokenize(src).unwrap();
        let stmts = parse(&toks).unwrap();
        match &stmts[0].kind {
            StmtKind::ExprStmt(e) => e.clone(),
            other => panic!("expected ExprStmt, got {:?}", other),
        }
    }

    #[test]
    fn arithmetic_precedence() {
        // °II + °III * °IV  ==  Add(2, Mul(3, 4))
        let e = parse_first_expr("°II + °III * °IV;");
        assert_eq!(e, Expr::Binary {
            op: BinOp::Add,
            left: Box::new(Expr::Int(2)),
            right: Box::new(Expr::Binary {
                op: BinOp::Mul,
                left: Box::new(Expr::Int(3)),
                right: Box::new(Expr::Int(4)),
            }),
        });
    }

    #[test]
    fn comparison_and_call() {
        let e = parse_first_expr("summa maior_est adde(a, °I);");
        match e {
            Expr::Binary { op: BinOp::Gt, right, .. } => match *right {
                Expr::Call { name, args } => {
                    assert_eq!(name, "adde");
                    assert_eq!(args.len(), 2);
                }
                other => panic!("expected call, got {:?}", other),
            },
            other => panic!("expected Gt, got {:?}", other),
        }
    }
}
```

- [ ] **Step 3: Run to verify failure**

Run: `cargo test --lib parser 2>&1 | tail -20`
Expected: panics with `not implemented`.

- [ ] **Step 4: Implement a parser struct with precedence-climbing expressions**

Replace `parse` and add the `Parser` struct:
```rust
struct Parser<'a> {
    toks: &'a [Token],
    pos: usize,
}

impl<'a> Parser<'a> {
    fn peek(&self) -> Option<&TokenKind> {
        self.toks.get(self.pos).map(|t| &t.kind)
    }
    fn line(&self) -> usize {
        self.toks.get(self.pos).or_else(|| self.toks.last())
            .map(|t| t.line).unwrap_or(0)
    }
    fn advance(&mut self) -> &Token {
        let t = &self.toks[self.pos];
        self.pos += 1;
        t
    }
    fn eat(&mut self, kind: &TokenKind) -> LResult<()> {
        if self.peek() == Some(kind) {
            self.pos += 1;
            Ok(())
        } else {
            Err(self.err(&format!("expectatum: {:?}", kind)))
        }
    }
    fn err(&self, msg: &str) -> LinguaError {
        LinguaError { message: msg.to_string(), line: self.line() }
    }

    // --- expressions, lowest precedence first ---
    fn expr(&mut self) -> LResult<Expr> { self.or_expr() }

    fn or_expr(&mut self) -> LResult<Expr> {
        let mut left = self.and_expr()?;
        while self.peek() == Some(&TokenKind::Or) {
            self.advance();
            let right = self.and_expr()?;
            left = Expr::Binary { op: BinOp::Or, left: Box::new(left), right: Box::new(right) };
        }
        Ok(left)
    }
    fn and_expr(&mut self) -> LResult<Expr> {
        let mut left = self.cmp_expr()?;
        while self.peek() == Some(&TokenKind::And) {
            self.advance();
            let right = self.cmp_expr()?;
            left = Expr::Binary { op: BinOp::And, left: Box::new(left), right: Box::new(right) };
        }
        Ok(left)
    }
    fn cmp_expr(&mut self) -> LResult<Expr> {
        let mut left = self.add_expr()?;
        loop {
            let op = match self.peek() {
                Some(TokenKind::Eq) => BinOp::Eq,
                Some(TokenKind::Ne) => BinOp::Ne,
                Some(TokenKind::Gt) => BinOp::Gt,
                Some(TokenKind::Lt) => BinOp::Lt,
                _ => break,
            };
            self.advance();
            let right = self.add_expr()?;
            left = Expr::Binary { op, left: Box::new(left), right: Box::new(right) };
        }
        Ok(left)
    }
    fn add_expr(&mut self) -> LResult<Expr> {
        let mut left = self.mul_expr()?;
        loop {
            let op = match self.peek() {
                Some(TokenKind::Plus) => BinOp::Add,
                Some(TokenKind::Minus) => BinOp::Sub,
                _ => break,
            };
            self.advance();
            let right = self.mul_expr()?;
            left = Expr::Binary { op, left: Box::new(left), right: Box::new(right) };
        }
        Ok(left)
    }
    fn mul_expr(&mut self) -> LResult<Expr> {
        let mut left = self.unary()?;
        loop {
            let op = match self.peek() {
                Some(TokenKind::Star) => BinOp::Mul,
                Some(TokenKind::Slash) => BinOp::Div,
                Some(TokenKind::Percent) => BinOp::Mod,
                _ => break,
            };
            self.advance();
            let right = self.unary()?;
            left = Expr::Binary { op, left: Box::new(left), right: Box::new(right) };
        }
        Ok(left)
    }
    fn unary(&mut self) -> LResult<Expr> {
        match self.peek() {
            Some(TokenKind::Not) => { self.advance(); Ok(Expr::Unary { op: UnOp::Not, expr: Box::new(self.unary()?) }) }
            Some(TokenKind::Minus) => { self.advance(); Ok(Expr::Unary { op: UnOp::Neg, expr: Box::new(self.unary()?) }) }
            _ => self.primary(),
        }
    }
    fn primary(&mut self) -> LResult<Expr> {
        match self.peek().cloned() {
            Some(TokenKind::Int(i)) => { self.advance(); Ok(Expr::Int(i)) }
            Some(TokenKind::Float(f)) => { self.advance(); Ok(Expr::Float(f)) }
            Some(TokenKind::Text(s)) => { self.advance(); Ok(Expr::Text(s)) }
            Some(TokenKind::True) => { self.advance(); Ok(Expr::Bool(true)) }
            Some(TokenKind::False) => { self.advance(); Ok(Expr::Bool(false)) }
            Some(TokenKind::LParen) => {
                self.advance();
                let e = self.expr()?;
                self.eat(&TokenKind::RParen)?;
                Ok(e)
            }
            Some(TokenKind::Ident(name)) => {
                self.advance();
                if self.peek() == Some(&TokenKind::LParen) {
                    self.advance();
                    let mut args = Vec::new();
                    if self.peek() != Some(&TokenKind::RParen) {
                        args.push(self.expr()?);
                        while self.peek() == Some(&TokenKind::Comma) {
                            self.advance();
                            args.push(self.expr()?);
                        }
                    }
                    self.eat(&TokenKind::RParen)?;
                    Ok(Expr::Call { name, args })
                } else {
                    Ok(Expr::Ident(name))
                }
            }
            _ => Err(self.err("expressio expectata")),
        }
    }
}

pub fn parse(tokens: &[Token]) -> LResult<Vec<Stmt>> {
    let mut p = Parser { toks: tokens, pos: 0 };
    let mut stmts = Vec::new();
    while p.pos < p.toks.len() {
        // Temporary: treat "expr ;" as an ExprStmt. Replaced in Task 7.
        let line = p.line();
        let e = p.expr()?;
        p.eat(&TokenKind::Semicolon)?;
        stmts.push(Stmt { kind: StmtKind::ExprStmt(e), line });
    }
    Ok(stmts)
}
```

- [ ] **Step 5: Run to verify pass, then commit**

Run: `cargo test --lib parser 2>&1 | tail -20`
Expected: both parser tests `ok`.
```bash
git add src/ast.rs src/parser.rs src/main.rs
git commit -m "feat: AST types + expression parser with precedence"
```

---

### Task 7: Parser — declarations, assignment, print

**Files:**
- Modify: `src/parser.rs`
- Test: inline `#[cfg(test)]` in `src/parser.rs`

**Interfaces:**
- Produces: a real `statement()` method dispatching on the leading token; `parse` now loops over statements. Consumes the expression methods from Task 6.

- [ ] **Step 1: Write failing tests**

Add to the `tests` module:
```rust
    fn parse_all(src: &str) -> Vec<Stmt> {
        parse(&tokenize(src).unwrap()).unwrap()
    }

    #[test]
    fn declaration_statement() {
        let s = parse_all("ni x = °V;");
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].kind, StmtKind::Let {
            ty: Type::Int, name: "x".into(), value: Expr::Int(5),
        });
    }

    #[test]
    fn assignment_and_print() {
        let s = parse_all("x = °I; scribe(x);");
        assert_eq!(s[0].kind, StmtKind::Assign { name: "x".into(), value: Expr::Int(1) });
        assert_eq!(s[1].kind, StmtKind::Print(Expr::Ident("x".into())));
    }
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test --lib parser 2>&1 | tail -20`
Expected: `declaration_statement` fails (currently parsed as `ExprStmt`, or errors on `ni`).

- [ ] **Step 3: Implement the statement layer**

Add helper + methods to `impl Parser`:
```rust
    fn type_keyword(&mut self) -> Option<Type> {
        let ty = match self.peek()? {
            TokenKind::TyInt => Type::Int,
            TokenKind::TyFloat => Type::Float,
            TokenKind::TyText => Type::Text,
            TokenKind::TyChar => Type::Char,
            TokenKind::TyBool => Type::Bool,
            _ => return None,
        };
        self.advance();
        Some(ty)
    }

    fn ident(&mut self) -> LResult<String> {
        match self.peek().cloned() {
            Some(TokenKind::Ident(n)) => { self.advance(); Ok(n) }
            _ => Err(self.err("nomen expectatum")),
        }
    }

    fn statement(&mut self) -> LResult<Stmt> {
        let line = self.line();
        // declaration: <type> <ident> = <expr> ;
        if let Some(ty) = self.type_keyword() {
            let name = self.ident()?;
            self.eat(&TokenKind::Assign)?;
            let value = self.expr()?;
            self.eat(&TokenKind::Semicolon)?;
            return Ok(Stmt { kind: StmtKind::Let { ty, name, value }, line });
        }
        match self.peek() {
            Some(TokenKind::Print) => {
                self.advance();
                self.eat(&TokenKind::LParen)?;
                let e = self.expr()?;
                self.eat(&TokenKind::RParen)?;
                self.eat(&TokenKind::Semicolon)?;
                Ok(Stmt { kind: StmtKind::Print(e), line })
            }
            // assignment: <ident> = <expr> ;   (lookahead distinguishes from a call)
            Some(TokenKind::Ident(_))
                if self.toks.get(self.pos + 1).map(|t| &t.kind) == Some(&TokenKind::Assign) =>
            {
                let name = self.ident()?;
                self.eat(&TokenKind::Assign)?;
                let value = self.expr()?;
                self.eat(&TokenKind::Semicolon)?;
                Ok(Stmt { kind: StmtKind::Assign { name, value }, line })
            }
            _ => {
                let e = self.expr()?;
                self.eat(&TokenKind::Semicolon)?;
                Ok(Stmt { kind: StmtKind::ExprStmt(e), line })
            }
        }
    }
```
Replace the body of `parse`:
```rust
pub fn parse(tokens: &[Token]) -> LResult<Vec<Stmt>> {
    let mut p = Parser { toks: tokens, pos: 0 };
    let mut stmts = Vec::new();
    while p.pos < p.toks.len() {
        stmts.push(p.statement()?);
    }
    Ok(stmts)
}
```

- [ ] **Step 4: Run to verify pass**

Run: `cargo test --lib parser 2>&1 | tail -20`
Expected: all parser tests `ok` (Task 6 tests still pass — `°II + °III * °IV;` is now an `ExprStmt` via the fallthrough).

- [ ] **Step 5: Commit**

```bash
git add src/parser.rs
git commit -m "feat: parse declarations, assignment, print"
```

---

### Task 8: Parser — control flow + functions

**Files:**
- Modify: `src/parser.rs`
- Test: inline `#[cfg(test)]` in `src/parser.rs`

**Interfaces:**
- Produces: parsing for `si/aut`, `indem`, `dum`, `abrumpe`, `continua`, `munus`, `redde`. Adds a `block()` helper returning `Vec<Stmt>`.

- [ ] **Step 1: Write failing tests**

Add to the `tests` module:
```rust
    #[test]
    fn if_else_if_else() {
        let s = parse_all("si (x) { scribe(°I); } aut si (y) { scribe(°II); } aut { scribe(°III); }");
        match &s[0].kind {
            StmtKind::If { then, else_, .. } => {
                assert_eq!(then.len(), 1);
                assert!(else_.is_some());
                // else branch is a single nested If (the "aut si")
                match &else_.as_ref().unwrap()[0].kind {
                    StmtKind::If { else_: inner, .. } => assert!(inner.is_some()),
                    other => panic!("expected nested if, got {:?}", other),
                }
            }
            other => panic!("expected if, got {:?}", other),
        }
    }

    #[test]
    fn while_for_break_continue() {
        let s = parse_all("indem (x) { abrumpe; }");
        matches!(s[0].kind, StmtKind::While { .. });
        let f = parse_all("dum (ni i = °I; i minor_est °X; i = i + °I) { continua; }");
        matches!(f[0].kind, StmtKind::For { .. });
    }

    #[test]
    fn function_declaration_and_return() {
        let s = parse_all("munus ni adde(ni a, ni b) { redde a + b; }");
        match &s[0].kind {
            StmtKind::Func(f) => {
                assert_eq!(f.name, "adde");
                assert_eq!(f.ret, Type::Int);
                assert_eq!(f.params, vec![(Type::Int, "a".into()), (Type::Int, "b".into())]);
                assert!(matches!(f.body[0].kind, StmtKind::Return(_)));
            }
            other => panic!("expected func, got {:?}", other),
        }
    }
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test --lib parser 2>&1 | tail -25`
Expected: new tests fail (`si`, `indem`, `dum`, `munus` currently fall through to expression parsing and error).

- [ ] **Step 3: Implement control flow + functions**

Add a `block()` helper and dispatch arms. In `statement()`, add these arms **before** the declaration check:
```rust
        match self.peek() {
            Some(TokenKind::If) => return self.if_stmt(),
            Some(TokenKind::While) => return self.while_stmt(),
            Some(TokenKind::For) => return self.for_stmt(),
            Some(TokenKind::Func) => return self.func_stmt(),
            Some(TokenKind::Return) => {
                let line = self.line();
                self.advance();
                let e = self.expr()?;
                self.eat(&TokenKind::Semicolon)?;
                return Ok(Stmt { kind: StmtKind::Return(e), line });
            }
            Some(TokenKind::Break) => {
                let line = self.line();
                self.advance();
                self.eat(&TokenKind::Semicolon)?;
                return Ok(Stmt { kind: StmtKind::Break, line });
            }
            Some(TokenKind::Continue) => {
                let line = self.line();
                self.advance();
                self.eat(&TokenKind::Semicolon)?;
                return Ok(Stmt { kind: StmtKind::Continue, line });
            }
            _ => {}
        }
```
Add the methods to `impl Parser`:
```rust
    fn block(&mut self) -> LResult<Vec<Stmt>> {
        self.eat(&TokenKind::LBrace)?;
        let mut stmts = Vec::new();
        while self.peek() != Some(&TokenKind::RBrace) {
            if self.peek().is_none() {
                return Err(self.err("'}' expectatum"));
            }
            stmts.push(self.statement()?);
        }
        self.eat(&TokenKind::RBrace)?;
        Ok(stmts)
    }

    fn if_stmt(&mut self) -> LResult<Stmt> {
        let line = self.line();
        self.advance(); // si
        self.eat(&TokenKind::LParen)?;
        let cond = self.expr()?;
        self.eat(&TokenKind::RParen)?;
        let then = self.block()?;
        let else_ = if self.peek() == Some(&TokenKind::Else) {
            self.advance(); // aut
            if self.peek() == Some(&TokenKind::If) {
                Some(vec![self.if_stmt()?]) // "aut si" → nested if
            } else {
                Some(self.block()?)
            }
        } else {
            None
        };
        Ok(Stmt { kind: StmtKind::If { cond, then, else_ }, line })
    }

    fn while_stmt(&mut self) -> LResult<Stmt> {
        let line = self.line();
        self.advance(); // indem
        self.eat(&TokenKind::LParen)?;
        let cond = self.expr()?;
        self.eat(&TokenKind::RParen)?;
        let body = self.block()?;
        Ok(Stmt { kind: StmtKind::While { cond, body }, line })
    }

    fn for_stmt(&mut self) -> LResult<Stmt> {
        let line = self.line();
        self.advance(); // dum
        self.eat(&TokenKind::LParen)?;
        let init = Box::new(self.statement()?);      // consumes its own ';'
        let cond = self.expr()?;
        self.eat(&TokenKind::Semicolon)?;
        let step = Box::new(self.assign_no_semi()?); // step has no trailing ';'
        self.eat(&TokenKind::RParen)?;
        let body = self.block()?;
        Ok(Stmt { kind: StmtKind::For { init, cond, step, body }, line })
    }

    // for-loop step: <ident> = <expr>  (no trailing semicolon)
    fn assign_no_semi(&mut self) -> LResult<Stmt> {
        let line = self.line();
        let name = self.ident()?;
        self.eat(&TokenKind::Assign)?;
        let value = self.expr()?;
        Ok(Stmt { kind: StmtKind::Assign { name, value }, line })
    }

    fn func_stmt(&mut self) -> LResult<Stmt> {
        let line = self.line();
        self.advance(); // munus
        let ret = self.type_keyword().ok_or_else(|| self.err("typus reditus expectatus"))?;
        let name = self.ident()?;
        self.eat(&TokenKind::LParen)?;
        let mut params = Vec::new();
        if self.peek() != Some(&TokenKind::RParen) {
            loop {
                let pty = self.type_keyword().ok_or_else(|| self.err("typus parametri expectatus"))?;
                let pname = self.ident()?;
                params.push((pty, pname));
                if self.peek() == Some(&TokenKind::Comma) { self.advance(); } else { break; }
            }
        }
        self.eat(&TokenKind::RParen)?;
        let body = self.block()?;
        Ok(Stmt { kind: StmtKind::Func(FuncDecl { name, params, ret, body }), line })
    }
```

- [ ] **Step 4: Run to verify pass**

Run: `cargo test --lib parser 2>&1 | tail -25`
Expected: all parser tests `ok`.

- [ ] **Step 5: Commit**

```bash
git add src/parser.rs
git commit -m "feat: parse control flow and function declarations"
```

---

### Task 9: Interpreter — values, expression evaluation, decl/assign/print

**Files:**
- Create: `src/value.rs`, `src/interp.rs`
- Modify: `src/main.rs` (add `mod value; mod interp;`)
- Test: inline `#[cfg(test)]` in `src/interp.rs`

**Interfaces:**
- Produces: `value::Value { Int(i64), Float(f64), Text(String), Char(char), Bool(bool) }` with `display() -> String` (bools print as `verus`/`falsus`); `interp::execute(&[Stmt]) -> LResult<String>` returning collected `scribe` output. This task handles literals, identifiers, unary/binary ops, `Let`, `Assign`, `Print`; other statements return an error until Tasks 10.

- [ ] **Step 1: Define Value**

`src/value.rs`:
```rust
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Int(i64),
    Float(f64),
    Text(String),
    Char(char),
    Bool(bool),
}

impl Value {
    pub fn display(&self) -> String {
        match self {
            Value::Int(i) => i.to_string(),
            Value::Float(f) => f.to_string(),
            Value::Text(s) => s.clone(),
            Value::Char(c) => c.to_string(),
            Value::Bool(b) => (if *b { "verus" } else { "falsus" }).to_string(),
        }
    }
}
```

- [ ] **Step 2: Write failing tests**

`src/interp.rs`:
```rust
use crate::ast::*;
use crate::error::{LinguaError, LResult};
use crate::value::Value;
use std::collections::HashMap;

pub fn execute(program: &[Stmt]) -> LResult<String> {
    unimplemented!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::tokenize;
    use crate::parser::parse;

    fn run(src: &str) -> String {
        let program = parse(&tokenize(src).unwrap()).unwrap();
        execute(&program).unwrap()
    }

    #[test]
    fn arithmetic_and_print() {
        assert_eq!(run("scribe(°V + °III * °II);"), "11\n");
    }

    #[test]
    fn variables() {
        assert_eq!(run("ni x = °X; x = x + °II; scribe(x);"), "12\n");
    }

    #[test]
    fn booleans_print_in_latin() {
        assert_eq!(run("scribe(°V maior_est °III);"), "verus\n");
    }

    #[test]
    fn string_concatenation() {
        assert_eq!(run("scribe(\"sal\" + \"ve\");"), "salve\n");
    }
}
```

- [ ] **Step 3: Run to verify failure**

Run: `cargo test --lib interp 2>&1 | tail -20`
Expected: panics with `not implemented`.

- [ ] **Step 4: Implement environment, eval, and the core statements**

Replace `execute` and add helpers:
```rust
struct Interp {
    scopes: Vec<HashMap<String, Value>>,
    funcs: HashMap<String, FuncDecl>,
    out: String,
}

impl Interp {
    fn new() -> Self {
        Interp { scopes: vec![HashMap::new()], funcs: HashMap::new(), out: String::new() }
    }

    fn get(&self, name: &str) -> Option<Value> {
        for scope in self.scopes.iter().rev() {
            if let Some(v) = scope.get(name) {
                return Some(v.clone());
            }
        }
        None
    }
    fn declare(&mut self, name: String, v: Value) {
        self.scopes.last_mut().unwrap().insert(name, v);
    }
    fn assign(&mut self, name: &str, v: Value, line: usize) -> LResult<()> {
        for scope in self.scopes.iter_mut().rev() {
            if scope.contains_key(name) {
                scope.insert(name.to_string(), v);
                return Ok(());
            }
        }
        Err(LinguaError { message: format!("variabilis ignota: '{}'", name), line })
    }

    fn eval(&mut self, e: &Expr, line: usize) -> LResult<Value> {
        match e {
            Expr::Int(i) => Ok(Value::Int(*i)),
            Expr::Float(f) => Ok(Value::Float(*f)),
            Expr::Text(s) => Ok(Value::Text(s.clone())),
            Expr::Bool(b) => Ok(Value::Bool(*b)),
            Expr::Ident(name) => self.get(name)
                .ok_or_else(|| LinguaError { message: format!("variabilis ignota: '{}'", name), line }),
            Expr::Unary { op, expr } => {
                let v = self.eval(expr, line)?;
                match (op, v) {
                    (UnOp::Not, Value::Bool(b)) => Ok(Value::Bool(!b)),
                    (UnOp::Neg, Value::Int(i)) => Ok(Value::Int(-i)),
                    (UnOp::Neg, Value::Float(f)) => Ok(Value::Float(-f)),
                    _ => Err(LinguaError { message: "operatio unaria invalida".into(), line }),
                }
            }
            Expr::Binary { op, left, right } => {
                let l = self.eval(left, line)?;
                let r = self.eval(right, line)?;
                eval_binary(*op, l, r, line)
            }
            Expr::Call { name, args } => {
                let mut vals = Vec::new();
                for a in args { vals.push(self.eval(a, line)?); }
                self.call(name, vals, line)
            }
        }
    }

    // Placeholder until Task 10 fills in real function calls.
    fn call(&mut self, name: &str, _args: Vec<Value>, line: usize) -> LResult<Value> {
        Err(LinguaError { message: format!("functio nondum vocari potest: '{}'", name), line })
    }

    fn exec(&mut self, s: &Stmt) -> LResult<()> {
        match &s.kind {
            StmtKind::Let { name, value, .. } => {
                let v = self.eval(value, s.line)?;
                self.declare(name.clone(), v);
                Ok(())
            }
            StmtKind::Assign { name, value } => {
                let v = self.eval(value, s.line)?;
                self.assign(name, v, s.line)
            }
            StmtKind::Print(e) => {
                let v = self.eval(e, s.line)?;
                self.out.push_str(&v.display());
                self.out.push('\n');
                Ok(())
            }
            StmtKind::ExprStmt(e) => { self.eval(e, s.line)?; Ok(()) }
            StmtKind::Func(f) => { self.funcs.insert(f.name.clone(), f.clone()); Ok(()) }
            _ => Err(LinguaError { message: "sententia nondum tractata".into(), line: s.line }),
        }
    }
}

fn eval_binary(op: BinOp, l: Value, r: Value, line: usize) -> LResult<Value> {
    use Value::*;
    let bad = || LinguaError { message: "operatio binaria invalida".into(), line };
    match op {
        BinOp::Add => match (l, r) {
            (Int(a), Int(b)) => Ok(Int(a + b)),
            (Float(a), Float(b)) => Ok(Float(a + b)),
            (Text(a), Text(b)) => Ok(Text(a + &b)),
            _ => Err(bad()),
        },
        BinOp::Sub => match (l, r) {
            (Int(a), Int(b)) => Ok(Int(a - b)),
            (Float(a), Float(b)) => Ok(Float(a - b)),
            _ => Err(bad()),
        },
        BinOp::Mul => match (l, r) {
            (Int(a), Int(b)) => Ok(Int(a * b)),
            (Float(a), Float(b)) => Ok(Float(a * b)),
            _ => Err(bad()),
        },
        BinOp::Div => match (l, r) {
            (Int(_), Int(0)) => Err(LinguaError { message: "divisio per nullum".into(), line }),
            (Int(a), Int(b)) => Ok(Int(a / b)),
            (Float(a), Float(b)) => Ok(Float(a / b)),
            _ => Err(bad()),
        },
        BinOp::Mod => match (l, r) {
            (Int(_), Int(0)) => Err(LinguaError { message: "modulus per nullum".into(), line }),
            (Int(a), Int(b)) => Ok(Int(a % b)),
            _ => Err(bad()),
        },
        BinOp::Eq => Ok(Bool(l == r)),
        BinOp::Ne => Ok(Bool(l != r)),
        BinOp::Gt => cmp(l, r, line, |o| o == std::cmp::Ordering::Greater),
        BinOp::Lt => cmp(l, r, line, |o| o == std::cmp::Ordering::Less),
        BinOp::And => match (l, r) {
            (Bool(a), Bool(b)) => Ok(Bool(a && b)),
            _ => Err(bad()),
        },
        BinOp::Or => match (l, r) {
            (Bool(a), Bool(b)) => Ok(Bool(a || b)),
            _ => Err(bad()),
        },
    }
}

fn cmp(l: Value, r: Value, line: usize, keep: impl Fn(std::cmp::Ordering) -> bool) -> LResult<Value> {
    use Value::*;
    let ord = match (l, r) {
        (Int(a), Int(b)) => a.cmp(&b),
        (Float(a), Float(b)) => a.partial_cmp(&b)
            .ok_or_else(|| LinguaError { message: "comparatio NaN".into(), line })?,
        _ => return Err(LinguaError { message: "comparatio invalida".into(), line }),
    };
    Ok(Value::Bool(keep(ord)))
}

pub fn execute(program: &[Stmt]) -> LResult<String> {
    let mut it = Interp::new();
    // Register functions first so order doesn't matter.
    for s in program {
        if let StmtKind::Func(f) = &s.kind {
            it.funcs.insert(f.name.clone(), f.clone());
        }
    }
    for s in program {
        if !matches!(s.kind, StmtKind::Func(_)) {
            it.exec(s)?;
        }
    }
    Ok(it.out)
}
```

- [ ] **Step 5: Run to verify pass, then commit**

Run: `cargo test --lib interp 2>&1 | tail -20`
Expected: the four interp tests `ok`.
```bash
git add src/value.rs src/interp.rs src/main.rs
git commit -m "feat: interpreter values, expressions, decl/assign/print"
```

---

### Task 10: Interpreter — control flow + functions

**Files:**
- Modify: `src/interp.rs`
- Test: inline `#[cfg(test)]` in `src/interp.rs`

**Interfaces:**
- Produces: a `Flow { Normal, Break, Continue, Return(Value) }` signal; `exec` returns `LResult<Flow>` (all arms updated); real `call()` with an isolated scope. Consumes the `funcs` map and `eval` from Task 9.

- [ ] **Step 1: Write failing tests**

Add to the `tests` module:
```rust
    #[test]
    fn while_loop_counts() {
        let src = "ni i = °nullus; ni s = °nullus; indem (i minor_est °V) { s = s + i; i = i + °I; } scribe(s);";
        assert_eq!(run(src), "10\n"); // 0+1+2+3+4
    }

    #[test]
    fn for_loop_with_break() {
        let src = "ni total = °nullus; dum (ni i = °nullus; i minor_est °X; i = i + °I) { si (i par_est °III) { abrumpe; } total = total + i; } scribe(total);";
        assert_eq!(run(src), "3\n"); // 0+1+2 then break at 3
    }

    #[test]
    fn function_call_returns() {
        let src = "munus ni adde(ni a, ni b) { redde a + b; } scribe(adde(°IV, °VI));";
        assert_eq!(run(src), "10\n");
    }
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test --lib interp 2>&1 | tail -25`
Expected: fail — `While`/`For` hit the "sententia nondum tractata" arm; `call` returns the placeholder error.

- [ ] **Step 3: Change `exec` to return `Flow` and implement the branches**

Add the enum near the top of `src/interp.rs`:
```rust
enum Flow { Normal, Break, Continue, Return(Value) }
```
Replace the whole `exec` method with a `Flow`-returning version, and add a block runner:
```rust
    fn exec(&mut self, s: &Stmt) -> LResult<Flow> {
        match &s.kind {
            StmtKind::Let { name, value, .. } => {
                let v = self.eval(value, s.line)?;
                self.declare(name.clone(), v);
                Ok(Flow::Normal)
            }
            StmtKind::Assign { name, value } => {
                let v = self.eval(value, s.line)?;
                self.assign(name, v, s.line)?;
                Ok(Flow::Normal)
            }
            StmtKind::Print(e) => {
                let v = self.eval(e, s.line)?;
                self.out.push_str(&v.display());
                self.out.push('\n');
                Ok(Flow::Normal)
            }
            StmtKind::ExprStmt(e) => { self.eval(e, s.line)?; Ok(Flow::Normal) }
            StmtKind::Func(f) => { self.funcs.insert(f.name.clone(), f.clone()); Ok(Flow::Normal) }
            StmtKind::Break => Ok(Flow::Break),
            StmtKind::Continue => Ok(Flow::Continue),
            StmtKind::Return(e) => {
                let v = self.eval(e, s.line)?;
                Ok(Flow::Return(v))
            }
            StmtKind::If { cond, then, else_ } => {
                if self.truthy(cond, s.line)? {
                    self.run_block(then)
                } else if let Some(eb) = else_ {
                    self.run_block(eb)
                } else {
                    Ok(Flow::Normal)
                }
            }
            StmtKind::While { cond, body } => {
                while self.truthy(cond, s.line)? {
                    match self.run_block(body)? {
                        Flow::Break => break,
                        Flow::Return(v) => return Ok(Flow::Return(v)),
                        _ => {}
                    }
                }
                Ok(Flow::Normal)
            }
            StmtKind::For { init, cond, step, body } => {
                self.scopes.push(HashMap::new()); // loop scope holds the init var
                self.exec(init)?;
                while self.truthy(cond, s.line)? {
                    match self.run_block(body)? {
                        Flow::Break => break,
                        Flow::Return(v) => { self.scopes.pop(); return Ok(Flow::Return(v)); }
                        _ => {}
                    }
                    self.exec(step)?;
                }
                self.scopes.pop();
                Ok(Flow::Normal)
            }
        }
    }

    fn run_block(&mut self, body: &[Stmt]) -> LResult<Flow> {
        self.scopes.push(HashMap::new());
        for s in body {
            match self.exec(s)? {
                Flow::Normal => {}
                other => { self.scopes.pop(); return Ok(other); }
            }
        }
        self.scopes.pop();
        Ok(Flow::Normal)
    }

    fn truthy(&mut self, cond: &Expr, line: usize) -> LResult<bool> {
        match self.eval(cond, line)? {
            Value::Bool(b) => Ok(b),
            _ => Err(LinguaError { message: "condicio non est veritas".into(), line }),
        }
    }
```
Replace the placeholder `call` with a real one (functions get an isolated scope stack — C-style, they see only their params, not caller locals):
```rust
    fn call(&mut self, name: &str, args: Vec<Value>, line: usize) -> LResult<Value> {
        let f = self.funcs.get(name)
            .cloned()
            .ok_or_else(|| LinguaError { message: format!("functio ignota: '{}'", name), line })?;
        if f.params.len() != args.len() {
            return Err(LinguaError {
                message: format!("functio '{}' numerum argumentorum falsum accepit", name),
                line,
            });
        }
        let mut frame = HashMap::new();
        for ((_, pname), v) in f.params.iter().zip(args) {
            frame.insert(pname.clone(), v);
        }
        // Swap in an isolated scope stack for the call, restore afterwards.
        let saved = std::mem::replace(&mut self.scopes, vec![frame]);
        let mut result = Value::Int(0);
        for s in &f.body {
            if let Flow::Return(v) = self.exec(s)? {
                result = v;
                break;
            }
        }
        self.scopes = saved;
        Ok(result)
    }
```
Finally, update `execute`'s statement loop to discard the returned `Flow`:
```rust
    for s in program {
        if !matches!(s.kind, StmtKind::Func(_)) {
            it.exec(s)?; // top-level Flow is ignored
        }
    }
```

- [ ] **Step 4: Run to verify pass**

Run: `cargo test --lib interp 2>&1 | tail -25`
Expected: all interp tests `ok`.

- [ ] **Step 5: Commit**

```bash
git add src/interp.rs
git commit -m "feat: interpreter control flow and function calls"
```

---

### Task 11: Type checker — expressions + declarations/assignment/print

**Files:**
- Create: `src/checker.rs`
- Modify: `src/main.rs` (add `mod checker;`)
- Test: inline `#[cfg(test)]` in `src/checker.rs`

**Interfaces:**
- Produces: `checker::check(&[Stmt]) -> LResult<()>`. Walks the tree assigning a `Type` to every expression and erroring (with the statement's line) on mismatch. Handles literals, identifiers, unary/binary/comparison/boolean expressions, `Let`, `Assign`, `Print`, `ExprStmt`. Control flow + functions come in Task 12 (they error as "nondum tractata" until then — but Task 12 lands before integration tests, so no program relies on the gap).

- [ ] **Step 1: Write failing tests**

`src/checker.rs`:
```rust
use crate::ast::*;
use crate::error::{LinguaError, LResult};
use std::collections::HashMap;

pub fn check(program: &[Stmt]) -> LResult<()> {
    unimplemented!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::tokenize;
    use crate::parser::parse;

    fn check_src(src: &str) -> LResult<()> {
        check(&parse(&tokenize(src).unwrap()).unwrap())
    }

    #[test]
    fn well_typed_program_passes() {
        assert!(check_src("ni x = °V; x = x + °I; scribe(x);").is_ok());
    }

    #[test]
    fn assigning_text_to_int_fails() {
        let err = check_src("ni x = \"salve\";").unwrap_err();
        assert!(err.message.contains("typ")); // "typum ... expectat"
        assert_eq!(err.line, 1);
    }

    #[test]
    fn comparison_yields_bool() {
        assert!(check_src("veritas b = °V maior_est °III;").is_ok());
    }

    #[test]
    fn adding_int_and_text_fails() {
        assert!(check_src("ni x = °V + \"a\";").is_err());
    }

    #[test]
    fn using_undeclared_variable_fails() {
        assert!(check_src("scribe(y);").is_err());
    }
```

Close the `mod tests` block with `}` after Task 12 adds its tests; for now add a trailing `}`.

- [ ] **Step 2: Run to verify failure**

Run: `cargo test --lib checker 2>&1 | tail -20`
Expected: panics with `not implemented`.

- [ ] **Step 3: Implement the checker core**

Replace `check` and add helpers:
```rust
struct Checker {
    scopes: Vec<HashMap<String, Type>>,
    funcs: HashMap<String, (Vec<Type>, Type)>, // name -> (param types, return type)
}

impl Checker {
    fn new() -> Self {
        Checker { scopes: vec![HashMap::new()], funcs: HashMap::new() }
    }

    fn lookup(&self, name: &str) -> Option<Type> {
        self.scopes.iter().rev().find_map(|s| s.get(name).copied())
    }
    fn declare(&mut self, name: String, ty: Type) {
        self.scopes.last_mut().unwrap().insert(name, ty);
    }

    fn type_name(t: Type) -> &'static str {
        match t {
            Type::Int => "ni", Type::Float => "mnr",
            Type::Text => "voc", Type::Char => "lit", Type::Bool => "veritas",
        }
    }

    fn check_expr(&self, e: &Expr, line: usize) -> LResult<Type> {
        match e {
            Expr::Int(_) => Ok(Type::Int),
            Expr::Float(_) => Ok(Type::Float),
            Expr::Text(_) => Ok(Type::Text),
            Expr::Bool(_) => Ok(Type::Bool),
            Expr::Ident(name) => self.lookup(name)
                .ok_or_else(|| LinguaError { message: format!("variabilis ignota: '{}'", name), line }),
            Expr::Unary { op, expr } => {
                let t = self.check_expr(expr, line)?;
                match op {
                    UnOp::Not if t == Type::Bool => Ok(Type::Bool),
                    UnOp::Neg if t == Type::Int || t == Type::Float => Ok(t),
                    _ => Err(LinguaError { message: "operatio unaria typo non convenit".into(), line }),
                }
            }
            Expr::Binary { op, left, right } => {
                let lt = self.check_expr(left, line)?;
                let rt = self.check_expr(right, line)?;
                self.check_binary(*op, lt, rt, line)
            }
            Expr::Call { name, args } => {
                let (params, ret) = self.funcs.get(name)
                    .cloned()
                    .ok_or_else(|| LinguaError { message: format!("functio ignota: '{}'", name), line })?;
                if params.len() != args.len() {
                    return Err(LinguaError { message: format!("functio '{}' numerum argumentorum falsum accepit", name), line });
                }
                for (want, arg) in params.iter().zip(args) {
                    let got = self.check_expr(arg, line)?;
                    if got != *want {
                        return Err(LinguaError {
                            message: format!("argumentum typum '{}' expectat, sed '{}' datum est",
                                Self::type_name(*want), Self::type_name(got)),
                            line,
                        });
                    }
                }
                Ok(ret)
            }
        }
    }

    fn check_binary(&self, op: BinOp, lt: Type, rt: Type, line: usize) -> LResult<Type> {
        let mismatch = || LinguaError {
            message: format!("operatio typis '{}' et '{}' non convenit",
                Self::type_name(lt), Self::type_name(rt)),
            line,
        };
        match op {
            BinOp::Add => match (lt, rt) {
                (Type::Int, Type::Int) => Ok(Type::Int),
                (Type::Float, Type::Float) => Ok(Type::Float),
                (Type::Text, Type::Text) => Ok(Type::Text),
                _ => Err(mismatch()),
            },
            BinOp::Sub | BinOp::Mul | BinOp::Div => match (lt, rt) {
                (Type::Int, Type::Int) => Ok(Type::Int),
                (Type::Float, Type::Float) => Ok(Type::Float),
                _ => Err(mismatch()),
            },
            BinOp::Mod => match (lt, rt) {
                (Type::Int, Type::Int) => Ok(Type::Int),
                _ => Err(mismatch()),
            },
            BinOp::Eq | BinOp::Ne | BinOp::Gt | BinOp::Lt => {
                if lt == rt { Ok(Type::Bool) } else { Err(mismatch()) }
            }
            BinOp::And | BinOp::Or => match (lt, rt) {
                (Type::Bool, Type::Bool) => Ok(Type::Bool),
                _ => Err(mismatch()),
            },
        }
    }

    fn check_stmt(&mut self, s: &Stmt) -> LResult<()> {
        match &s.kind {
            StmtKind::Let { ty, name, value } => {
                let vt = self.check_expr(value, s.line)?;
                if vt != *ty {
                    return Err(LinguaError {
                        message: format!("'{}' typum '{}' expectat, sed '{}' datum est",
                            name, Self::type_name(*ty), Self::type_name(vt)),
                        line: s.line,
                    });
                }
                self.declare(name.clone(), *ty);
                Ok(())
            }
            StmtKind::Assign { name, value } => {
                let target = self.lookup(name)
                    .ok_or_else(|| LinguaError { message: format!("variabilis ignota: '{}'", name), line: s.line })?;
                let vt = self.check_expr(value, s.line)?;
                if vt != target {
                    return Err(LinguaError {
                        message: format!("'{}' typum '{}' expectat, sed '{}' datum est",
                            name, Self::type_name(target), Self::type_name(vt)),
                        line: s.line,
                    });
                }
                Ok(())
            }
            StmtKind::Print(e) | StmtKind::ExprStmt(e) => {
                self.check_expr(e, s.line)?;
                Ok(())
            }
            _ => Err(LinguaError { message: "sententia nondum tractata".into(), line: s.line }),
        }
    }
}

pub fn check(program: &[Stmt]) -> LResult<()> {
    let mut c = Checker::new();
    for s in program {
        c.check_stmt(s)?;
    }
    Ok(())
}
```

- [ ] **Step 4: Run to verify pass**

Run: `cargo test --lib checker 2>&1 | tail -20`
Expected: the five checker tests `ok`.

- [ ] **Step 5: Commit**

```bash
git add src/checker.rs src/main.rs
git commit -m "feat: type checker for expressions and basic statements"
```

---

### Task 12: Type checker — control flow + functions

**Files:**
- Modify: `src/checker.rs`
- Test: inline `#[cfg(test)]` in `src/checker.rs`

**Interfaces:**
- Produces: `check_stmt` handling `If`/`While`/`For` (condition must be `Bool`; bodies checked in a pushed scope), `Func` (params in scope, body checked, `Return` matches declared type), `Break`/`Continue`. Functions are pre-registered so calls can appear before declarations.

- [ ] **Step 1: Write failing tests**

Add to the `tests` module (before its closing `}`):
```rust
    #[test]
    fn non_bool_condition_fails() {
        let err = check_src("si (°V) { scribe(°I); }").unwrap_err();
        assert!(err.message.contains("veritas") || err.message.contains("condicio"));
    }

    #[test]
    fn function_return_type_checked() {
        assert!(check_src("munus ni adde(ni a, ni b) { redde a + b; } scribe(adde(°I, °II));").is_ok());
    }

    #[test]
    fn returning_wrong_type_fails() {
        assert!(check_src("munus ni f() { redde \"salve\"; }").is_err());
    }

    #[test]
    fn calling_with_wrong_arg_type_fails() {
        assert!(check_src("munus ni f(ni a) { redde a; } scribe(f(\"x\"));").is_err());
    }
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test --lib checker 2>&1 | tail -25`
Expected: the new tests fail (control-flow/func hit "nondum tractata"; calls to undeclared functions error).

- [ ] **Step 3: Implement control-flow + function checking**

Add a return-type context field to `Checker`:
```rust
struct Checker {
    scopes: Vec<HashMap<String, Type>>,
    funcs: HashMap<String, (Vec<Type>, Type)>,
    current_ret: Option<Type>, // Some(_) while checking a function body
}
```
Update `Checker::new()` to set `current_ret: None`. Pre-register functions at the top of `check`:
```rust
pub fn check(program: &[Stmt]) -> LResult<()> {
    let mut c = Checker::new();
    for s in program {
        if let StmtKind::Func(f) = &s.kind {
            let ptypes = f.params.iter().map(|(t, _)| *t).collect();
            c.funcs.insert(f.name.clone(), (ptypes, f.ret));
        }
    }
    for s in program {
        c.check_stmt(s)?;
    }
    Ok(())
}
```
Add a block helper and replace the `_ =>` arm of `check_stmt` with real arms:
```rust
    fn check_block(&mut self, body: &[Stmt]) -> LResult<()> {
        self.scopes.push(HashMap::new());
        for s in body {
            if let Err(e) = self.check_stmt(s) {
                self.scopes.pop();
                return Err(e);
            }
        }
        self.scopes.pop();
        Ok(())
    }

    fn expect_bool(&self, cond: &Expr, line: usize) -> LResult<()> {
        if self.check_expr(cond, line)? == Type::Bool {
            Ok(())
        } else {
            Err(LinguaError { message: "condicio 'veritas' esse debet".into(), line })
        }
    }
```
Replace the final `_ =>` arm in `check_stmt` with:
```rust
            StmtKind::If { cond, then, else_ } => {
                self.expect_bool(cond, s.line)?;
                self.check_block(then)?;
                if let Some(eb) = else_ { self.check_block(eb)?; }
                Ok(())
            }
            StmtKind::While { cond, body } => {
                self.expect_bool(cond, s.line)?;
                self.check_block(body)
            }
            StmtKind::For { init, cond, step, body } => {
                self.scopes.push(HashMap::new());
                let r = (|| {
                    self.check_stmt(init)?;
                    self.expect_bool(cond, s.line)?;
                    self.check_stmt(step)?;
                    self.check_block(body)
                })();
                self.scopes.pop();
                r
            }
            StmtKind::Func(f) => {
                self.scopes.push(HashMap::new());
                for (t, n) in &f.params { self.declare(n.clone(), *t); }
                let saved = self.current_ret;
                self.current_ret = Some(f.ret);
                let r = self.check_block_inner(&f.body);
                self.current_ret = saved;
                self.scopes.pop();
                r
            }
            StmtKind::Return(e) => {
                let want = self.current_ret
                    .ok_or_else(|| LinguaError { message: "'redde' extra functionem".into(), line: s.line })?;
                let got = self.check_expr(e, s.line)?;
                if got != want {
                    return Err(LinguaError {
                        message: format!("reditus typum '{}' expectat, sed '{}' datum est",
                            Self::type_name(want), Self::type_name(got)),
                        line: s.line,
                    });
                }
                Ok(())
            }
            StmtKind::Break | StmtKind::Continue => Ok(()),
```
Add a body-checker that does **not** push another scope (params already pushed):
```rust
    fn check_block_inner(&mut self, body: &[Stmt]) -> LResult<()> {
        for s in body {
            self.check_stmt(s)?;
        }
        Ok(())
    }
```

- [ ] **Step 4: Run to verify pass**

Run: `cargo test --lib checker 2>&1 | tail -25`
Expected: all checker tests `ok`.

- [ ] **Step 5: Commit**

```bash
git add src/checker.rs
git commit -m "feat: type checker for control flow and functions"
```

---

### Task 13: Pipeline wiring + example programs + integration tests

**Files:**
- Modify: `src/main.rs`
- Create: `examples/salve.liss`, `examples/calc.liss`, `tests/programs.rs`

**Interfaces:**
- Consumes: `lexer::tokenize`, `parser::parse`, `checker::check`, `interp::execute`.
- Produces: a `run(&str) -> LResult<String>` pipeline used by both `main` and integration tests; a working CLI (`cargo run -- <file.liss>`).

- [ ] **Step 1: Write the failing integration tests**

`tests/programs.rs`:
```rust
// Integration tests drive the whole pipeline via the library API.
use lingua_serpentis::run;

#[test]
fn full_program_with_function_and_if() {
    let src = r#"
        munus ni summare(ni a, ni b) {
            ni c = a + b;
            redde c;
        }
        ni summa = summare(°XXV, °XXX.M.V.C.VII);
        si (summa minor_est °C) { summa = summa + °C; }
        scribe(summa);
    "#;
    assert_eq!(run(src).unwrap(), "30532\n"); // 25 + 30507 = 30532 (not < 100, so unchanged)
}

#[test]
fn type_error_reports_latin_and_line() {
    let src = "ni x = \"salve\";";
    let err = run(src).unwrap_err();
    assert_eq!(err.line, 1);
    assert!(err.to_string().starts_with("Error in linea 1:"));
}
```

This requires a library target exposing `run`. 

- [ ] **Step 2: Run to verify failure**

Run: `cargo test --test programs 2>&1 | tail -20`
Expected: fails to compile — `lingua_serpentis::run` and the lib target don't exist yet.

- [ ] **Step 3: Add a library target and the `run` pipeline**

Create `src/lib.rs`:
```rust
pub mod error;
pub mod roman;
pub mod token;
pub mod lexer;
pub mod ast;
pub mod parser;
pub mod checker;
pub mod value;
pub mod interp;

use error::LResult;

/// Lex → parse → type-check → interpret. Returns collected `scribe` output.
pub fn run(src: &str) -> LResult<String> {
    let tokens = lexer::tokenize(src)?;
    let program = parser::parse(&tokens)?;
    checker::check(&program)?;
    interp::execute(&program)
}
```
Update `Cargo.toml` to declare both targets:
```toml
[lib]
name = "lingua_serpentis"
path = "src/lib.rs"

[[bin]]
name = "lingua-serpentis"
path = "src/main.rs"
```
Replace `src/main.rs` entirely (modules now live in `lib.rs`):
```rust
use lingua_serpentis::run;
use std::process::exit;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("Usus: lingua-serpentis <path.liss>");
        exit(1);
    }
    let path = &args[1];
    if !path.ends_with(".liss") {
        eprintln!("Error: file must have .liss extension");
        exit(1);
    }
    let src = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => { eprintln!("Error reading file: {e}"); exit(1); }
    };
    match run(&src) {
        Ok(output) => print!("{output}"),
        Err(e) => { eprintln!("{e}"); exit(1); }
    }
}
```

> Note: with modules declared in `lib.rs`, the old `mod …;` lines in `main.rs` are gone. Cross-module references inside the library still use `crate::` and keep working.

- [ ] **Step 4: Run to verify pass, and check the whole suite**

Run: `cargo test 2>&1 | tail -30`
Expected: unit tests + both integration tests `ok`.

- [ ] **Step 5: Add example programs and verify the CLI**

`examples/salve.liss`:
```
scribe("Salve, munde!");
```
`examples/calc.liss`:
```
munus ni summare(ni a, ni b) {
    ni c = a + b;
    redde c;
}
ni summa = summare(°XXV, °XXX.M.V.C.VII);
si (summa minor_est °C) { summa = summa + °C; }
scribe(summa);
```
Run: `cargo run -- examples/calc.liss`
Expected output: `30532`
Run: `cargo run -- examples/salve.liss`
Expected output: `Salve, munde!`

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml src/lib.rs src/main.rs tests/programs.rs examples/
git commit -m "feat: wire pipeline, add lib target, examples, integration tests"
```

---

## Self-Review

**Spec coverage** — every v1 item maps to a task:
- Typed variables + `veritas` bool → Tasks 6/7 (parse), 9 (eval), 11 (check).
- Roman literals + fractions + `nullus` → Tasks 2–3, wired in lexer Task 5.
- Operators `+ - * / %`, Latin comparisons, `et`/`vel`/`non` → Tasks 6 (parse), 9 (eval), 11 (check).
- `si`/`aut`/`indem`/`dum` + `abrumpe`/`continua` → Tasks 8 (parse), 10 (interp), 12 (check).
- Typed functions `munus`/`redde` → Tasks 8, 10, 12.
- `scribe` any type → Tasks 7 (parse), 9 (eval), 11 (check).
- Static type checker + Latin errors + line numbers → Tasks 11–12; error type Task 1.
- Cross-platform / no gcc → inherent (pure Rust); CLI Task 13.
- Word-boundary lexing fix → Task 5 (explicit test).

**Deferred items correctly absent:** arrays, type inference, imports, try/catch, REPL, `>=`/`<=` — none appear in any task. ✔

**Placeholder scan:** the only `unimplemented!()` calls are deliberate red-to-green TDD starting points, each replaced within the same task. No "TBD"/"handle edge cases"/"similar to Task N". ✔

**Type consistency:** `TokenKind` variants used by the parser (Task 6/7/8) match Task 4/5 definitions; `Type`/`Expr`/`Stmt`/`StmtKind`/`FuncDecl` (Task 6) are used unchanged by interp (9/10) and checker (11/12); `Value` (Task 9) variants match `eval_binary`/`display`; `interp::execute`, `checker::check`, `lexer::tokenize`, `parser::parse` signatures match their call sites in `lib.rs` (Task 13). `mnr`/`nr` → `Type::Float` is applied consistently in lexer (Task 5) and checker `type_name` (Task 11). ✔

**Known intentional simplifications (documented, not gaps):**
- Functions use an isolated scope (no closures / no access to caller locals) — matches the C-like model in the spec.
- The type checker does not verify that every control path returns; a function that falls off the end yields `Int(0)` at runtime. Acceptable for v1; a "missing return" check is a good early v2 addition.
- `mnr`/`nr` collapse to one float type (per spec open-question resolution).
