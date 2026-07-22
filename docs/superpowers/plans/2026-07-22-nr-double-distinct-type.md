# `nr` / double as a Distinct Type — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `nr` (double) a first-class type distinct from `mnr` (float) — `mnr` backed by `f32`, `nr` by `f64` — with fractional literals that adapt to context.

**Architecture:** Three tasks. Task 1 re-backs `mnr` as `f32` at the runtime layer only (still a `Float`/`Double`-collapsed world). Task 2 introduces `Type::Double`, the `nr` keyword, and an *elaborating checker*: the parser emits an abstract `Expr::FloatLit`, and the checker resolves each literal's precision by context and rewrites the node in place to `Expr::Float(f32)` / `Expr::Double(f64)`, so the interpreter stays a dumb second walk. Task 3 amends the superseded design doc.

**Tech Stack:** Rust (workspace, edition 2021). Two crates; only `crates/lingua-serpentis` changes. No third-party dependencies.

**Spec:** `docs/superpowers/specs/2026-07-22-nr-double-distinct-type-design.md`

## Global Constraints

- **Build/test command (this machine):** `cargo` is not on `PATH` and MSVC must be imported first. Unless a Native Tools prompt is already active, run tests through cmd. This command is referenced below as **`<TEST>`**:
  ```
  cmd /c '"C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat" >nul 2>&1 && "%USERPROFILE%\.cargo\bin\cargo.exe" test -p lingua-serpentis'
  ```
  Append a test-name filter to run one test, e.g. `<TEST> mnr_is_single_precision`. If `cargo` and MSVC are already on `PATH`, `cargo test -p lingua-serpentis` suffices.
- **Commit only files the task touches.** The working tree has pre-existing *uncommitted* edits to `Cargo.toml`, `crates/latinitas/Cargo.toml`, `crates/lingua-serpentis/Cargo.toml` (an `edition`/`resolver` drift). Never `git add` those; stage files by explicit path.
- **Zero new third-party dependencies.**
- **All existing tests stay green** (117 at baseline). Every task ends on a green `<TEST>`.
- Commit messages end with the repo's trailer: `Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>`.

---

## Task 1: Re-back `mnr` as `f32`

Make a value declared `mnr` genuinely single-precision. No new types yet — `nr` is still an alias of `mnr` after this task, and both are now `f32`. The deliverable is the observable f32 narrowing.

**Files:**
- Modify: `crates/lingua-serpentis/src/value.rs:2-8` (`Value::Float` payload)
- Modify: `crates/lingua-serpentis/src/interp.rs:43` (narrow the literal on eval)
- Test: `crates/lingua-serpentis/tests/programs.rs` (new integration test)

**Interfaces:**
- Produces: `Value::Float(f32)` (was `Value::Float(f64)`). No public function signatures change.

- [ ] **Step 1: Write the failing test**

Append to `crates/lingua-serpentis/tests/programs.rs`:

```rust
#[test]
fn mnr_is_single_precision() {
    // 1 + 1/3 has no exact binary form; f32 and f64 round it differently.
    // This pins that `mnr` really rounds at f32 precision.
    assert_eq!(run("mnr x = °I:I/III; scribe(x);").unwrap(), "1.3333334\n");
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `<TEST> mnr_is_single_precision`
Expected: FAIL — actual output is `1.3333333333333333\n` (the current f64 rendering).

- [ ] **Step 3: Change `Value::Float` to `f32`**

In `crates/lingua-serpentis/src/value.rs`, change the `Float` variant:

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Int(i64),
    Float(f32),
    Text(String),
    Char(char),
    Bool(bool),
}
```

`display`, `eval_binary`, and `cmp` need no textual change — their `Float` arms already flow the payload type through, and they now operate on `f32`.

- [ ] **Step 4: Narrow the literal when evaluating it**

In `crates/lingua-serpentis/src/interp.rs`, the `Expr::Float` arm of `eval` (line 43) currently reads:

```rust
            Expr::Float(f) => Ok(Value::Float(*f)),
```

`Expr::Float` still holds an `f64` at this stage, so narrow it:

```rust
            Expr::Float(f) => Ok(Value::Float(*f as f32)),
```

- [ ] **Step 5: Run the new test and the whole crate**

Run: `<TEST> mnr_is_single_precision`
Expected: PASS (`1.3333334\n`).

Run: `<TEST>`
Expected: PASS — all tests green (no existing test asserts on float output, so nothing else shifts).

- [ ] **Step 6: Commit**

```
git add crates/lingua-serpentis/src/value.rs crates/lingua-serpentis/src/interp.rs crates/lingua-serpentis/tests/programs.rs
git commit -m "feat: back mnr with f32 so it is genuinely single-precision

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

## Task 2: `nr` as a distinct, adaptive type

The vertical slice: add `Type::Double`, the `nr` keyword, the abstract `Expr::FloatLit`, and the elaborating checker that resolves literals by context and rewrites them in place. After this task, `nr` is `f64`, `mnr` is `f32`, they never mix, and a fractional literal adapts to whichever it is used as.

**Files:**
- Modify: `crates/lingua-serpentis/src/ast.rs:1-21` (`Type::Double`; split `Expr::Float`)
- Modify: `crates/lingua-serpentis/src/token.rs:13` (add `TyDouble`)
- Modify: `crates/lingua-serpentis/src/lexer.rs:101` (split `mnr`/`nr`; new lexer test)
- Modify: `crates/lingua-serpentis/src/parser.rs:111,144-155,235-239` (emit `FloatLit`; map/accept `TyDouble`)
- Modify: `crates/lingua-serpentis/src/value.rs:2-8` (add `Double`)
- Modify: `crates/lingua-serpentis/src/interp.rs:42-46,49-56,190-245` (`FloatLit`/`Double` arms)
- Modify: `crates/lingua-serpentis/src/checker.rs:1-238` (full non-test rewrite) + test helper + new tests
- Modify: `crates/lingua-serpentis/src/lib.rs:20-21` (`check(&mut program)`)
- Test: `crates/lingua-serpentis/tests/programs.rs` (precision + mixing integration tests)

**Interfaces:**
- Consumes: `Value::Float(f32)` from Task 1.
- Produces:
  - `ast::Type` = `{ Int, Float, Double, Text, Char, Bool }` (all `Copy`).
  - `ast::Expr` gains `FloatLit(f64)`, `Float(f32)`, `Double(f64)` (replacing the old `Float(f64)`).
  - `TokenKind::TyDouble`.
  - `Value::Double(f64)`.
  - `checker::check(program: &mut [Stmt]) -> LResult<()>` (signature changes from `&[Stmt]`).
  - Free fn `checker::resolve_floatlit(e: &mut Expr, ty: Type)` (module-private).

- [ ] **Step 1: Write the failing tests**

Append to `crates/lingua-serpentis/tests/programs.rs`:

```rust
#[test]
fn nr_is_double_precision() {
    assert_eq!(run("nr y = °I:I/III; scribe(y);").unwrap(), "1.3333333333333333\n");
}

#[test]
fn representable_fraction_prints_same_under_both() {
    // 5.5 is exact in both, so the two types agree here — guards against a test
    // that leans on divergence for a value that does not actually diverge.
    assert_eq!(run("mnr x = °V:I/II; scribe(x);").unwrap(), "5.5\n");
    assert_eq!(run("nr y = °V:I/II; scribe(y);").unwrap(), "5.5\n");
}

#[test]
fn mixing_float_types_is_a_type_error() {
    assert!(run("mnr a = °I:I/II; nr b = °I:I/IV; scribe(a + b);").is_err());
}

#[test]
fn nr_arithmetic_and_comparison_run() {
    // Exercises the interpreter's Double arms end-to-end: 1.25 + 1.25 = 2.5,
    // and 1.5 > 1.25. Without these the eval_binary/cmp Double arms are untested.
    assert_eq!(run("nr x = °I:I/IV + °I:I/IV; scribe(x);").unwrap(), "2.5\n");
    assert_eq!(run("scribe(°I:I/II maior_est °I:I/IV);").unwrap(), "verus\n");
}
```

Add these tests inside the `#[cfg(test)] mod tests` block in `crates/lingua-serpentis/src/checker.rs` (before its closing `}`):

```rust
    #[test]
    fn fractional_literal_adapts_to_mnr_and_nr() {
        assert!(check_src("mnr x = °I:I/II;").is_ok());
        assert!(check_src("nr y = °I:I/II;").is_ok());
    }

    #[test]
    fn unconstrained_fraction_defaults_ok() {
        assert!(check_src("scribe(°I:I/II);").is_ok());
    }

    #[test]
    fn fraction_arithmetic_adapts_to_declared_double() {
        assert!(check_src("nr z = °I:I/II + °I:I/IV;").is_ok());
    }

    #[test]
    fn mixing_mnr_and_nr_is_rejected() {
        assert!(check_src("mnr a = °I:I/II; nr b = °I:I/IV; scribe(a + b);").is_err());
        assert!(check_src("mnr a = °I:I/II; nr b = a;").is_err());
        assert!(check_src("mnr a = °I:I/II; nr b = °I:I/IV; scribe(a maior_est b);").is_err());
    }

    #[test]
    fn fractional_literal_into_int_slot_is_rejected() {
        let err = check_src("ni n = °I:I/II;").unwrap_err();
        assert!(err.message.contains("fractus"), "got: {}", err.message);
    }

    #[test]
    fn function_returning_fraction_adapts_to_nr() {
        assert!(check_src("munus tau reddit nr { redde °I:I/III; }").is_ok());
    }

    #[test]
    fn function_accepts_fraction_argument_as_mnr() {
        // The literal argument adapts to the declared mnr parameter type.
        assert!(check_src("munus f capit (mnr a) reddit mnr { redde a; } scribe(f(°I:I/II));").is_ok());
    }
```

- [ ] **Step 2: Run to verify they fail**

Run: `<TEST>`
Expected: FAIL — this will be a **compile error** (e.g. `no variant Double`, `nr` typed as `mnr`), because the tests reference behavior not yet built. That is the red state.

- [ ] **Step 3: Extend the AST**

Replace the two enums at the top of `crates/lingua-serpentis/src/ast.rs`:

```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Type { Int, Float, Double, Text, Char, Bool }
```

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Int(i64),
    FloatLit(f64), // abstract fractional literal, pre-elaboration
    Float(f32),    // resolved mnr
    Double(f64),   // resolved nr
    Text(String),
    Char(char),
    Bool(bool),
    Ident(String),
    Unary { op: UnOp, expr: Box<Expr> },
    Binary { op: BinOp, left: Box<Expr>, right: Box<Expr> },
    Call { name: String, args: Vec<Expr> },
}
```

- [ ] **Step 4: Add the `TyDouble` token**

In `crates/lingua-serpentis/src/token.rs`, replace the type-keyword line (13):

```rust
    TyInt, TyFloat, TyDouble, TyText, TyChar, TyBool,   // ni | mnr | nr | voc | lit | veritas
```

- [ ] **Step 5: Split the `mnr` / `nr` keywords in the lexer**

In `crates/lingua-serpentis/src/lexer.rs`, replace the aliased line (101):

```rust
        "mnr" => TokenKind::TyFloat,
        "nr" => TokenKind::TyDouble,
```

Then add a lexer unit test inside the `#[cfg(test)] mod tests` block (near `keywords_types_and_operators`):

```rust
    #[test]
    fn mnr_and_nr_are_distinct_type_keywords() {
        assert_eq!(kinds("mnr"), vec![TokenKind::TyFloat]);
        assert_eq!(kinds("nr"), vec![TokenKind::TyDouble]);
    }
```

- [ ] **Step 6: Teach the parser `FloatLit` and `TyDouble`**

In `crates/lingua-serpentis/src/parser.rs`, three edits.

Primary-expression float literal (line 111) — emit the abstract node:

```rust
            Some(TokenKind::Float(f)) => { self.advance(); Ok(Expr::FloatLit(f)) }
```

`type_keyword` (add the `TyDouble` arm, after `TyFloat`):

```rust
    fn type_keyword(&mut self) -> Option<Type> {
        let ty = match self.peek()? {
            TokenKind::TyInt => Type::Int,
            TokenKind::TyFloat => Type::Float,
            TokenKind::TyDouble => Type::Double,
            TokenKind::TyText => Type::Text,
            TokenKind::TyChar => Type::Char,
            TokenKind::TyBool => Type::Bool,
            _ => return None,
        };
        self.advance();
        Some(ty)
    }
```

The old-inline-form guard in `func_stmt` (lines 235-239) — add `TyDouble` so `munus nr f(...)` is still caught:

```rust
        if matches!(
            self.peek(),
            Some(TokenKind::TyInt | TokenKind::TyFloat | TokenKind::TyDouble
                 | TokenKind::TyText | TokenKind::TyChar | TokenKind::TyBool)
        ) {
            return Err(self.err("typus reditus post 'reddit' ponendus, non ante nomen"));
        }
```

- [ ] **Step 7: Add `Value::Double`**

In `crates/lingua-serpentis/src/value.rs`, add the `Double` variant and its `display` arm:

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Int(i64),
    Float(f32),
    Double(f64),
    Text(String),
    Char(char),
    Bool(bool),
}

impl Value {
    pub fn display(&self) -> String {
        match self {
            Value::Int(i) => i.to_string(),
            Value::Float(f) => f.to_string(),
            Value::Double(f) => f.to_string(),
            Value::Text(s) => s.clone(),
            Value::Char(c) => c.to_string(),
            Value::Bool(b) => (if *b { "verus" } else { "falsus" }).to_string(),
        }
    }
}
```

- [ ] **Step 8: Handle the new nodes in the interpreter**

In `crates/lingua-serpentis/src/interp.rs`:

Replace the literal arms of `eval` (lines 42-46). Note `Expr::Float` now holds an `f32`, so drop the `as f32` added in Task 1:

```rust
            Expr::Int(i) => Ok(Value::Int(*i)),
            Expr::FloatLit(f) => Ok(Value::Double(*f)), // defensive: an unelaborated literal defaults to nr
            Expr::Float(f) => Ok(Value::Float(*f)),
            Expr::Double(f) => Ok(Value::Double(*f)),
            Expr::Text(s) => Ok(Value::Text(s.clone())),
            Expr::Char(c) => Ok(Value::Char(*c)),
            Expr::Bool(b) => Ok(Value::Bool(*b)),
```

Add a `Double` arm to unary `Neg` (after the `Float` arm, line 54):

```rust
                    (UnOp::Neg, Value::Float(f)) => Ok(Value::Float(-f)),
                    (UnOp::Neg, Value::Double(f)) => Ok(Value::Double(-f)),
```

In `eval_binary`, add a `Double` arm beside each existing `Float` arm:

```rust
        BinOp::Add => match (l, r) {
            (Int(a), Int(b)) => a.checked_add(b).map(Int).ok_or_else(overflow),
            (Float(a), Float(b)) => Ok(Float(a + b)),
            (Double(a), Double(b)) => Ok(Double(a + b)),
            (Text(a), Text(b)) => Ok(Text(a + &b)),
            _ => Err(bad()),
        },
        BinOp::Sub => match (l, r) {
            (Int(a), Int(b)) => a.checked_sub(b).map(Int).ok_or_else(overflow),
            (Float(a), Float(b)) => Ok(Float(a - b)),
            (Double(a), Double(b)) => Ok(Double(a - b)),
            _ => Err(bad()),
        },
        BinOp::Mul => match (l, r) {
            (Int(a), Int(b)) => a.checked_mul(b).map(Int).ok_or_else(overflow),
            (Float(a), Float(b)) => Ok(Float(a * b)),
            (Double(a), Double(b)) => Ok(Double(a * b)),
            _ => Err(bad()),
        },
        BinOp::Div => match (l, r) {
            (Int(_), Int(0)) => Err(LinguaError { message: "divisio per nullum".into(), line }),
            (Int(a), Int(b)) => a.checked_div(b).map(Int).ok_or_else(overflow),
            (Float(a), Float(b)) => Ok(Float(a / b)),
            (Double(a), Double(b)) => Ok(Double(a / b)),
            _ => Err(bad()),
        },
```

Add a `Double` arm to `cmp` (after the `Float` arm, line 237):

```rust
        (Float(a), Float(b)) => a.partial_cmp(&b)
            .ok_or_else(|| LinguaError { message: "comparatio NaN".into(), line })?,
        (Double(a), Double(b)) => a.partial_cmp(&b)
            .ok_or_else(|| LinguaError { message: "comparatio NaN".into(), line })?,
```

- [ ] **Step 9: Rewrite the checker as an elaborating pass**

Replace everything in `crates/lingua-serpentis/src/checker.rs` **above** the `#[cfg(test)]` line (i.e. lines 1-238) with the following. This adds the `InferTy` tag, the `resolve_floatlit` rewrite, the `resolve_expr` demand helper, `Type::Double` in `type_name`, the `Double`/`FloatLit` rules in `check_binary`, and makes the whole walk take `&mut`:

```rust
use crate::ast::*;
use crate::error::{LinguaError, LResult};
use std::collections::HashMap;

/// Synthesized type of an expression during checking. `FloatLit` is the
/// not-yet-resolved type of a fractional literal — and of arithmetic that
/// combines only such literals; every other expression is `Known`.
#[derive(Clone, Copy, PartialEq)]
enum InferTy {
    Known(Type),
    FloatLit,
}

struct Checker {
    scopes: Vec<HashMap<String, Type>>,
    funcs: HashMap<String, (Vec<Type>, Type)>, // name -> (param types, return type)
    current_ret: Option<Type>, // Some(_) while checking a function body
}

impl Checker {
    fn new() -> Self {
        Checker { scopes: vec![HashMap::new()], funcs: HashMap::new(), current_ret: None }
    }

    fn lookup(&self, name: &str) -> Option<Type> {
        self.scopes.iter().rev().find_map(|s| s.get(name).copied())
    }
    fn declare(&mut self, name: String, ty: Type) {
        self.scopes.last_mut().unwrap().insert(name, ty);
    }

    fn type_name(t: Type) -> &'static str {
        match t {
            Type::Int => "ni", Type::Float => "mnr", Type::Double => "nr",
            Type::Text => "voc", Type::Char => "lit", Type::Bool => "veritas",
        }
    }

    /// Name used for an `InferTy` in error messages; a bare fractional literal
    /// reports as `nr`, its default type.
    fn infer_name(t: InferTy) -> &'static str {
        match t {
            InferTy::Known(ty) => Self::type_name(ty),
            InferTy::FloatLit => "nr",
        }
    }

    fn check_expr(&self, e: &mut Expr, line: usize) -> LResult<InferTy> {
        match e {
            Expr::Int(_) => Ok(InferTy::Known(Type::Int)),
            Expr::FloatLit(_) => Ok(InferTy::FloatLit),
            Expr::Float(_) => Ok(InferTy::Known(Type::Float)),
            Expr::Double(_) => Ok(InferTy::Known(Type::Double)),
            Expr::Text(_) => Ok(InferTy::Known(Type::Text)),
            Expr::Char(_) => Ok(InferTy::Known(Type::Char)),
            Expr::Bool(_) => Ok(InferTy::Known(Type::Bool)),
            Expr::Ident(name) => self.lookup(name)
                .map(InferTy::Known)
                .ok_or_else(|| LinguaError { message: format!("variabilis ignota: '{}'", name), line }),
            Expr::Unary { op, expr } => {
                let t = self.check_expr(expr, line)?;
                match op {
                    UnOp::Not if t == InferTy::Known(Type::Bool) => Ok(InferTy::Known(Type::Bool)),
                    UnOp::Neg => match t {
                        InferTy::FloatLit => Ok(InferTy::FloatLit),
                        InferTy::Known(Type::Int) => Ok(InferTy::Known(Type::Int)),
                        InferTy::Known(Type::Float) => Ok(InferTy::Known(Type::Float)),
                        InferTy::Known(Type::Double) => Ok(InferTy::Known(Type::Double)),
                        _ => Err(LinguaError { message: "operatio unaria typo non convenit".into(), line }),
                    },
                    _ => Err(LinguaError { message: "operatio unaria typo non convenit".into(), line }),
                }
            }
            Expr::Binary { op, left, right } => {
                let lt = self.check_expr(left, line)?;
                let rt = self.check_expr(right, line)?;
                self.check_binary(*op, lt, rt, left, right, line)
            }
            Expr::Call { name, args } => {
                let (params, ret) = self.funcs.get(name)
                    .cloned()
                    .ok_or_else(|| LinguaError { message: format!("functio ignota: '{}'", name), line })?;
                if params.len() != args.len() {
                    return Err(LinguaError { message: format!("functio '{}' numerum argumentorum falsum accepit", name), line });
                }
                for (want, arg) in params.iter().zip(args.iter_mut()) {
                    let got = self.resolve_expr(arg, Some(*want), line)?;
                    if got != *want {
                        return Err(LinguaError {
                            message: format!("argumentum typum '{}' expectat, sed '{}' datum est",
                                Self::type_name(*want), Self::type_name(got)),
                            line,
                        });
                    }
                }
                Ok(InferTy::Known(ret))
            }
        }
    }

    /// Synthesize `e`'s type, resolving a bare fractional literal to a concrete
    /// float type. `expected` is the type the context wants (`None` for an
    /// untyped sink such as `scribe`). A `FloatLit` becomes the expected float
    /// type; with no float context it defaults to `nr`; demanded as a non-float
    /// type it is an error.
    fn resolve_expr(&self, e: &mut Expr, expected: Option<Type>, line: usize) -> LResult<Type> {
        match self.check_expr(e, line)? {
            InferTy::Known(t) => Ok(t),
            InferTy::FloatLit => {
                let ty = match expected {
                    Some(Type::Float) => Type::Float,
                    Some(Type::Double) => Type::Double,
                    Some(other) => return Err(LinguaError {
                        message: format!("numerus fractus typum 'mnr' vel 'nr' requirit, non '{}'",
                            Self::type_name(other)),
                        line,
                    }),
                    None => Type::Double,
                };
                resolve_floatlit(e, ty);
                Ok(ty)
            }
        }
    }

    fn check_binary(&self, op: BinOp, lt: InferTy, rt: InferTy,
                    left: &mut Expr, right: &mut Expr, line: usize) -> LResult<InferTy> {
        use InferTy::*;
        let mismatch = || LinguaError {
            message: format!("operatio typis '{}' et '{}' non convenit",
                Self::infer_name(lt), Self::infer_name(rt)),
            line,
        };
        match op {
            BinOp::Add => match (lt, rt) {
                (FloatLit, FloatLit) => Ok(FloatLit),
                (FloatLit, Known(Type::Float)) => { resolve_floatlit(left, Type::Float); Ok(Known(Type::Float)) }
                (FloatLit, Known(Type::Double)) => { resolve_floatlit(left, Type::Double); Ok(Known(Type::Double)) }
                (Known(Type::Float), FloatLit) => { resolve_floatlit(right, Type::Float); Ok(Known(Type::Float)) }
                (Known(Type::Double), FloatLit) => { resolve_floatlit(right, Type::Double); Ok(Known(Type::Double)) }
                (Known(Type::Int), Known(Type::Int)) => Ok(Known(Type::Int)),
                (Known(Type::Float), Known(Type::Float)) => Ok(Known(Type::Float)),
                (Known(Type::Double), Known(Type::Double)) => Ok(Known(Type::Double)),
                (Known(Type::Text), Known(Type::Text)) => Ok(Known(Type::Text)),
                _ => Err(mismatch()),
            },
            BinOp::Sub | BinOp::Mul | BinOp::Div => match (lt, rt) {
                (FloatLit, FloatLit) => Ok(FloatLit),
                (FloatLit, Known(Type::Float)) => { resolve_floatlit(left, Type::Float); Ok(Known(Type::Float)) }
                (FloatLit, Known(Type::Double)) => { resolve_floatlit(left, Type::Double); Ok(Known(Type::Double)) }
                (Known(Type::Float), FloatLit) => { resolve_floatlit(right, Type::Float); Ok(Known(Type::Float)) }
                (Known(Type::Double), FloatLit) => { resolve_floatlit(right, Type::Double); Ok(Known(Type::Double)) }
                (Known(Type::Int), Known(Type::Int)) => Ok(Known(Type::Int)),
                (Known(Type::Float), Known(Type::Float)) => Ok(Known(Type::Float)),
                (Known(Type::Double), Known(Type::Double)) => Ok(Known(Type::Double)),
                _ => Err(mismatch()),
            },
            BinOp::Mod => match (lt, rt) {
                (Known(Type::Int), Known(Type::Int)) => Ok(Known(Type::Int)),
                _ => Err(mismatch()),
            },
            BinOp::Eq | BinOp::Ne | BinOp::Gt | BinOp::Lt => match (lt, rt) {
                (FloatLit, FloatLit) => {
                    resolve_floatlit(left, Type::Double);
                    resolve_floatlit(right, Type::Double);
                    Ok(Known(Type::Bool))
                }
                (FloatLit, Known(Type::Float)) => { resolve_floatlit(left, Type::Float); Ok(Known(Type::Bool)) }
                (FloatLit, Known(Type::Double)) => { resolve_floatlit(left, Type::Double); Ok(Known(Type::Bool)) }
                (Known(Type::Float), FloatLit) => { resolve_floatlit(right, Type::Float); Ok(Known(Type::Bool)) }
                (Known(Type::Double), FloatLit) => { resolve_floatlit(right, Type::Double); Ok(Known(Type::Bool)) }
                (Known(a), Known(b)) if a == b => Ok(Known(Type::Bool)),
                _ => Err(mismatch()),
            },
            BinOp::And | BinOp::Or => match (lt, rt) {
                (Known(Type::Bool), Known(Type::Bool)) => Ok(Known(Type::Bool)),
                _ => Err(mismatch()),
            },
        }
    }

    fn check_stmt(&mut self, s: &mut Stmt) -> LResult<()> {
        let line = s.line;
        match &mut s.kind {
            StmtKind::Let { ty, name, value } => {
                let vt = self.resolve_expr(value, Some(*ty), line)?;
                if vt != *ty {
                    return Err(LinguaError {
                        message: format!("'{}' typum '{}' expectat, sed '{}' datum est",
                            name, Self::type_name(*ty), Self::type_name(vt)),
                        line,
                    });
                }
                self.declare(name.clone(), *ty);
                Ok(())
            }
            StmtKind::Assign { name, value } => {
                let target = self.lookup(name)
                    .ok_or_else(|| LinguaError { message: format!("variabilis ignota: '{}'", name), line })?;
                let vt = self.resolve_expr(value, Some(target), line)?;
                if vt != target {
                    return Err(LinguaError {
                        message: format!("'{}' typum '{}' expectat, sed '{}' datum est",
                            name, Self::type_name(target), Self::type_name(vt)),
                        line,
                    });
                }
                Ok(())
            }
            StmtKind::Print(e) | StmtKind::ExprStmt(e) => {
                self.resolve_expr(e, None, line)?;
                Ok(())
            }
            StmtKind::If { cond, then, else_ } => {
                self.expect_bool(cond, line)?;
                self.check_block(then)?;
                if let Some(eb) = else_ { self.check_block(eb)?; }
                Ok(())
            }
            StmtKind::While { cond, body } => {
                self.expect_bool(cond, line)?;
                self.check_block(body)
            }
            StmtKind::For { init, cond, step, body } => {
                self.scopes.push(HashMap::new());
                let r = (|| {
                    self.check_stmt(init)?;
                    self.expect_bool(cond, line)?;
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
                let r = self.check_block_inner(&mut f.body);
                self.current_ret = saved;
                self.scopes.pop();
                r
            }
            StmtKind::Return(e) => {
                let want = self.current_ret
                    .ok_or_else(|| LinguaError { message: "'redde' extra functionem".into(), line })?;
                let got = self.resolve_expr(e, Some(want), line)?;
                if got != want {
                    return Err(LinguaError {
                        message: format!("reditus typum '{}' expectat, sed '{}' datum est",
                            Self::type_name(want), Self::type_name(got)),
                        line,
                    });
                }
                Ok(())
            }
            StmtKind::Break | StmtKind::Continue => Ok(()),
        }
    }

    fn check_block(&mut self, body: &mut [Stmt]) -> LResult<()> {
        self.scopes.push(HashMap::new());
        for s in body.iter_mut() {
            if let StmtKind::Func(f) = &s.kind {
                self.scopes.pop();
                return Err(LinguaError {
                    message: format!("functio '{}' intra corpus definiri non potest; functiones nidificatae non sustinentur", f.name),
                    line: s.line,
                });
            }
            if let Err(e) = self.check_stmt(s) {
                self.scopes.pop();
                return Err(e);
            }
        }
        self.scopes.pop();
        Ok(())
    }

    fn check_block_inner(&mut self, body: &mut [Stmt]) -> LResult<()> {
        for s in body.iter_mut() {
            if let StmtKind::Func(f) = &s.kind {
                return Err(LinguaError {
                    message: format!("functio '{}' intra corpus definiri non potest; functiones nidificatae non sustinentur", f.name),
                    line: s.line,
                });
            }
            self.check_stmt(s)?;
        }
        Ok(())
    }

    fn expect_bool(&self, cond: &mut Expr, line: usize) -> LResult<()> {
        if self.check_expr(cond, line)? == InferTy::Known(Type::Bool) {
            Ok(())
        } else {
            Err(LinguaError { message: "condicio 'veritas' esse debet".into(), line })
        }
    }
}

/// Rewrite every still-abstract fractional literal reachable through negation
/// and arithmetic into a concrete-precision node. Narrows to `f32` for `mnr`.
/// Only ever called on an expression whose synthesized type was `FloatLit`, so
/// there are no concrete float leaves to clobber.
fn resolve_floatlit(e: &mut Expr, ty: Type) {
    match e {
        Expr::FloatLit(f) => {
            *e = match ty {
                Type::Float => Expr::Float(*f as f32),
                Type::Double => Expr::Double(*f),
                _ => return,
            };
        }
        Expr::Unary { expr, .. } => resolve_floatlit(expr, ty),
        Expr::Binary { left, right, .. } => {
            resolve_floatlit(left, ty);
            resolve_floatlit(right, ty);
        }
        _ => {}
    }
}

pub fn check(program: &mut [Stmt]) -> LResult<()> {
    let mut c = Checker::new();
    for s in program.iter() {
        if let StmtKind::Func(f) = &s.kind {
            let ptypes = f.params.iter().map(|(t, _)| *t).collect();
            c.funcs.insert(f.name.clone(), (ptypes, f.ret));
        }
    }
    for s in program.iter_mut() {
        c.check_stmt(s)?;
    }
    Ok(())
}
```

- [ ] **Step 10: Fix the checker test helper**

In the `#[cfg(test)] mod tests` block of `crates/lingua-serpentis/src/checker.rs`, `check_src` now needs a mutable program (its callers, the existing tests, are unchanged):

```rust
    fn check_src(src: &str) -> LResult<()> {
        let mut program = parse(&tokenize(src).unwrap()).unwrap();
        check(&mut program)
    }
```

- [ ] **Step 11: Make the pipeline pass a mutable program**

In `crates/lingua-serpentis/src/lib.rs`, lines 20-21:

```rust
    let mut program = parser::parse(&tokens)?;
    checker::check(&mut program)?;
```

`latinity::lint(&program)` and `interp::execute(&program)` on the following lines are unchanged — they reborrow the now-elaborated program immutably.

- [ ] **Step 12: Run the full crate**

Run: `<TEST>`
Expected: PASS — all tests green, including every new test from Step 1 and every pre-existing test.

If `nr_is_double_precision` or `mnr_is_single_precision` show an unexpected decimal rendering, do **not** hand-tweak the expected string — a mismatch there means a literal is reaching the interpreter unelaborated (a `resolve_expr`/`resolve_floatlit` gap), so revisit Step 9.

- [ ] **Step 13: Commit**

```
git add crates/lingua-serpentis/src/ast.rs crates/lingua-serpentis/src/token.rs crates/lingua-serpentis/src/lexer.rs crates/lingua-serpentis/src/parser.rs crates/lingua-serpentis/src/value.rs crates/lingua-serpentis/src/interp.rs crates/lingua-serpentis/src/checker.rs crates/lingua-serpentis/src/lib.rs crates/lingua-serpentis/tests/programs.rs
git commit -m "feat: make nr a distinct f64 type with adaptive fractional literals

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

## Task 3: Amend the superseded design note

The original interpreter design (2026-07-19) records `mnr`/`nr` collapsing to one `f64` type and lists it as an open question. Both are now false. Amend in place.

**Files:**
- Modify: `docs/superpowers/specs/2026-07-19-lingua-serpentis-rust-interpreter-design.md:117,331-334`

- [ ] **Step 1: Correct the §5.2 type-table note**

In `docs/superpowers/specs/2026-07-19-lingua-serpentis-rust-interpreter-design.md`, the `double` row (line 117) currently reads:

```
| double | `nr` | `f64` | Distinct keyword, same backing as float in v1 |
```

Change the note, and the `float` row's backing, to reflect the split:

```
| float | `mnr` | `f32` | |
| double | `nr` | `f64` | Distinct type since 2026-07-22 (float is `f32`) |
```

(The `float` row is line 116; update its `f64`→`f32` too.)

- [ ] **Step 2: Mark the open question resolved**

The Open Questions section (lines 331-334) currently reads:

```
## 10. Open Questions
- Final Latin wording for type-error messages (cosmetic; settled during impl).
- Whether `mnr` (float) and `nr` (double) stay distinct types or collapse to one
  in v1 (leaning: keep the keywords, back both with `f64`).
```

Replace the second bullet:

```
## 10. Open Questions
- Final Latin wording for type-error messages (cosmetic; settled during impl).
- ~~Whether `mnr` (float) and `nr` (double) stay distinct types or collapse to
  one in v1.~~ **Resolved 2026-07-22:** distinct types — `mnr` is `f32`, `nr` is
  `f64`. See `docs/superpowers/specs/2026-07-22-nr-double-distinct-type-design.md`.
```

- [ ] **Step 3: Commit**

```
git add docs/superpowers/specs/2026-07-19-lingua-serpentis-rust-interpreter-design.md
git commit -m "docs: mark mnr/nr collapse resolved; nr is now a distinct f64 type

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

## Notes for the implementer

- **README needs no change.** `README.md` is a German brainstorming scratch file; it lists no type keywords and makes no claim about `nr`, so there is nothing to update there (spec §6).
- **`latinity.rs` needs no change.** It matches only on `StmtKind` (unchanged) and never on `Type`/`Expr`/`Value`; it compiles as-is. If a build error ever points there, it means an enum it does touch changed — it does not in this plan.
- **Why the interpreter keeps an `Expr::FloatLit` arm.** In production `check` always runs before `execute`, so no `FloatLit` survives. But `interp.rs`'s own unit tests call `execute` without the checker; the arm (defaulting to `nr`) keeps the exhaustive `match` total and panic-free. Do not route float-precision assertions through those unit tests — use the full-pipeline integration tests in `tests/programs.rs`, which run the checker.
