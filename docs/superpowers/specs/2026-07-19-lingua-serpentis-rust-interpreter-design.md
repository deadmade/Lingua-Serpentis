# Lingua Serpentis — Rust Interpreter Design

- **Date:** 2026-07-19
- **Status:** Approved (design) — pending implementation plan
- **Author:** Manuel (with Claude)
- **Amended:** 2026-07-21 — the two code samples in §5.6 and §5.9 were updated
  for the signature syntax introduced in
  `2026-07-21-haskell-style-signatures-design.md`. Prose unchanged.

> **Amended 2026-07-23:** the `ni` type is no longer `i64`-backed. Roman
> numerals are now the runtime representation — see
> `2026-07-23-roman-native-integers-design.md`. Passages below describing Roman
> literals collapsing to `i64` and integers printing in Arabic are superseded.

> **Amended 2026-08-01:** the script-style program model is superseded — a
> program is now a folder (or single file) that starts at `munus initium
> reddit ni`, and top level allows only declarations, so §5.5's top-level
> statements and §5.9's example no longer run as written. §9's "Imports /
> modules" v2 candidate is realized as Go-style packages (*fasciculi*). See
> `2026-08-01-programs-and-packages-design.md`.

> **Amended 2026-09-15:** §5.5's `dum (init; cond; step)` never constrained
> `init`. It is now a declaration (`ni i = °I;`) or an assignment (`i = °I;`)
> and nothing else — a terminator, a `si`/`indem`/`dum`/`munus`, a `scribe` or
> an expression statement there does not parse. Closes D-26. See
> `2026-09-15-dum-header-clause-design.md`.

## 1. Vision

**Lingua Serpentis** ("Language of the Serpent") is a small, statically-typed
programming language whose surface is a deliberate joke and whose insides are
genuinely real.

- **The joke:** keywords are Latin (`si`, `munus`, `scribe`, `redde`) and all
  numbers are written as **Roman numerals** (`°XXV`, `°VII.M.III:I/II`). Nobody
  knows Latin or Roman numerals, so it is gloriously impractical to write.
- **The real part:** it is **statically and explicitly typed**. A hand-written
  type checker rejects ill-typed programs *before* they run, and a tree-walking
  interpreter executes the well-typed ones. Type errors are reported — in Latin —
  with line numbers.

The language runs with no external toolchain (no `gcc`, no transpilation). The
interpreter is written in **Rust**.

## 2. Goals and Non-Goals

### Goals
- A language that is fun to read, impractical to write, and genuinely usable for
  small non-trivial programs (e.g. a Roman-numeral calculator, FizzBuzz).
- Enforced static types with clear, Latin-flavoured error messages.
- A clean, well-bounded Rust codebase that is easy to extend feature by feature.
- Cross-platform: runs anywhere Rust runs (fixes the current Windows `./out` bug).

### Non-Goals (v1)
- Arrays / lists.
- Type inference (`sit x = °V;`).
- Imports / modules.
- `try` / `catch`.
- A REPL.
- Transpilation to C / JS / Python (the previous approach; explicitly retired).

These are recorded as **v2 candidates** in §9, not rejected forever.

## 3. Key Decisions and Rationale

| Decision | Choice | Why |
|---|---|---|
| Execution model | **Tree-walking interpreter** (not transpiler) | No target language to fight; new features are "a few lines in `eval`" instead of "generate correct C/JS". Removes the `gcc` dependency and the Windows `./out` bug. |
| Typing | **Static + explicit** | The user wants real types and dislikes untyped languages. A dedicated checker pass runs before execution. |
| Type inference | **Deferred** | Start explicit-only; inference is an additive v2 feature. |
| Implementation language | **Rust** | Statically typed (pleasant for the author), fast, and its `enum` + `match` are ideal for ASTs and evaluators. Tree-walkers avoid Rust's hardest borrow-checker cases. |
| Existing Python code | **Reference-only** | Not edited. Good ideas (esp. Roman-numeral logic) are ported into a fresh Rust crate. |

## 4. Architecture

The entire pipeline is **three walks over one tree**:

```
source (.liss)
   │  lexer
   ▼
Vec<Token>
   │  parser (recursive descent)
   ▼
AST  ── Vec<Stmt>
   │  walk #1: type checker  ──►  Latin error + line number (halts before running)
   ▼
AST (validated)
   │  walk #2: interpreter   ──►  runs the program, produces Values / prints output
   ▼
program output
```

- **Lexer** turns source text into `Vec<Token>`.
- **Parser** turns tokens into an AST (`Vec<Stmt>`) via recursive descent.
- **Type checker** walks the AST, assigns a `Type` to every expression, and
  raises a Latin error (with line number) on any mismatch. No side effects.
- **Interpreter** walks the *validated* AST and executes it, producing runtime
  `Value`s and performing I/O (`scribe`).

## 5. Language Specification (v1)

### 5.1 Keywords (Latin)

Carried over from the existing Python `LanguageDefinition.py`:

| Concept | Keyword |
|---|---|
| if | `si` |
| else / else-if | `aut` |
| while | `indem` |
| for | `dum` |
| function | `munus` |
| return | `redde` |
| true | `verus` |
| false | `falsus` |
| break | `abrumpe` |
| continue | `continua` |
| print | `scribe` |

**New in v1:**

| Concept | Keyword |
|---|---|
| boolean AND | `et` |
| boolean OR | `vel` |
| boolean NOT | `non` |

### 5.2 Types

| Type | Keyword | Rust runtime | Notes |
|---|---|---|---|
| int | `ni` | `i64` | Roman-numeral integer literals |
| float | `mnr` | `f32` | |
| double | `nr` | `f64` | Distinct type since 2026-07-22 (float is `f32`) |
| string | `voc` | `String` | Literals in `"..."` |
| char | `lit` | `char` | |
| bool | `veritas` | `bool` | Holds `verus` / `falsus` |

Types are **explicit and required** on every declaration and every function
parameter/return. Assigning a value of the wrong type is a compile-time (type
checker) error.

### 5.3 Roman-numeral literals

Numbers are written with a leading `°` and Roman letters. The encoding (ported
from the existing `RomanNumeralConverter.py`, whose exact semantics must be
preserved) is:

- **Integer part**: chunks separated by `.` are read in **multiplicative pairs**,
  with a trailing lone chunk added. Formally, for chunks `[c0, c1, c2, …]`:
  `c0*c1 + c2*c3 + …` plus the final chunk if the count is odd.
  - `°XXV` → `25`
  - `°XXX.M.V.C.VII` → `30*1000 + 5*100 + 7` → `30507`
- **Fraction part** (optional): after a `:`, written `numerator/denominator`,
  each a Roman-numeral integer, added as a fraction.
  - `°V:I/II` → `5 + 1/2` → `5.5`
- `nullus` denotes zero.

A purely-integer literal has type `ni`; a literal with a fraction part has a
floating type (`mnr`/`nr`). Assigning a fractional literal to a `ni` is a type
error.

> **Porting notes / known limitations to verify against the Python reference:**
> subtractive notation is currently only handled for `IV`/`IX` (I before V/X);
> larger magnitudes are expressed via the `.` multiplication scheme rather than
> subtractive numerals. Preserve existing behaviour in v1; broaden subtractive
> support only if a test demands it (YAGNI).

### 5.4 Operators

- **Arithmetic:** `+`, `-`, `*`, `/`, `%`
- **Comparison (Latin words):** `par_est` (==), `par_non_est` (!=),
  `maior_est` (>), `minor_est` (<)
- **Boolean:** `et` (and), `vel` (or), `non` (not)
- **Assignment:** `=`

`>=` and `<=` are **not** in v1 (easy additive later: `maior_aequalis` /
`minor_aequalis`).

### 5.5 Statements and control flow

- **Declaration/assignment:** `ni x = °V;`  /  `x = x + °I;`
- **If / else-if / else:** `si (cond) { … } aut si (cond) { … } aut { … }`
- **While:** `indem (cond) { … }`
- **For:** `dum (init; cond; step) { … }`
- **Loop control:** `abrumpe;` (break), `continua;` (continue)
- **Blocks:** delimited by `{ … }`; statements end with `;`

### 5.6 Functions

```
munus summare capit (ni a, ni b) reddit ni {
    ni c = a + b;
    redde c;
}
```

Typed parameters and typed return. The type checker verifies the returned
expression matches the declared return type, and that call arguments match
parameter types.

### 5.7 Printing

`scribe(expr);` prints any typed value correctly (the interpreter dispatches on
the runtime `Value`, so no manual `printf` format specifiers).

### 5.8 Comments

`// line comment` (carried over).

### 5.9 Example program

```
munus summare capit (ni a, ni b) reddit ni {
    ni c = a + b;
    redde c;
}

ni pecunia_una = °XXV;
ni pecunia_dua = °XXX.M.V.C.VII;
ni summa = summare(pecunia_una, pecunia_dua);

si (summa minor_est °C) { summa = summa + °C; }

scribe(summa);
```

## 6. Rust Implementation

### 6.1 Crate layout

```
lingua-serpentis/
  Cargo.toml
  src/
    main.rs          CLI: read the .liss file, run the pipeline, print errors
    lexer.rs         source text  → Vec<Token>
    token.rs         the Token enum
    ast.rs           the Expr / Stmt / Type enums
    parser.rs        Vec<Token>   → Vec<Stmt>   (recursive descent)
    checker.rs       walk #1: static type checking
    interp.rs        walk #2: execution
    value.rs         the runtime Value enum + environment
    roman.rs         Roman numerals ↔ numbers (port of RomanNumeralConverter.py)
    error.rs         errors carrying a message + line number, phrased in Latin
  tests/
    programs.rs      integration tests: run .liss snippets, assert output/errors
  examples/
    *.liss           sample programs
```

Each module has one clear responsibility and a narrow interface, so units can be
understood and tested independently.

### 6.2 Core data types

```rust
// ast.rs
enum Type { Int, Float, Double, Text, Char, Bool }

enum Expr {
    Int(i64),                                       // °XXV     → 25   (no fraction part)
    Float(f64),                                     // °V:I/II  → 5.5  (has fraction part)
    Text(String),
    Bool(bool),
    Ident(String),
    Binary { op: BinOp, left: Box<Expr>, right: Box<Expr> },
    Unary  { op: UnOp,  expr: Box<Expr> },          // non
    Call   { name: String, args: Vec<Expr> },
}

enum Stmt {
    Let    { ty: Type, name: String, value: Expr },
    Assign { name: String, value: Expr },
    If     { cond: Expr, then: Vec<Stmt>, else_: Option<Vec<Stmt>> },
    While  { cond: Expr, body: Vec<Stmt> },
    For    { init: Box<Stmt>, cond: Expr, step: Box<Stmt>, body: Vec<Stmt> },
    Func   { ret: Type, name: String, params: Vec<(Type, String)>, body: Vec<Stmt> },
    Return(Expr), Print(Expr), Break, Continue,
}

// value.rs
enum Value { Int(i64), Float(f64), Text(String), Char(char), Bool(bool) }
```

The compiler's exhaustiveness checking on `match` replaces the Python version's
fragile `isinstance` / tuple-unwrapping.

### 6.3 Environment / scopes

Variables live in an environment: a stack of `HashMap<String, Value>` scopes
(one pushed per block/function call). This is the one place Rust ownership needs
care; the plain stack-of-maps pattern is sufficient for v1 (no closures yet).

### 6.4 Errors (in Latin)

A single `Error { message: String, line: usize }` type flows out of the lexer,
parser, checker, and interpreter. Messages are phrased in Latin, e.g.:

> `Error in linea V: 'x' typum 'ni' expectat, sed 'voc' datum est.`
> (line 5: 'x' expects type 'ni' but was given 'voc'.)

The lexer must tokenise on whole-word boundaries so keyword prefixes like `ni`
do not accidentally match inside identifiers such as `nimis` — a latent bug in
the current Python regex lexer that the Rust rewrite fixes by construction.

## 7. Testing Strategy

- **Unit tests** per module (`roman.rs` especially — pure functions, table-driven).
- **Integration tests** (`tests/programs.rs`): feed `.liss` source through the
  full pipeline and assert on printed output *and* on expected type-checker
  errors (e.g. "this program must fail with a Latin type error on line N").
- Run via `cargo test`.

## 8. Relationship to the existing Python code

The Python project is **reference-only** and is not modified. Concepts ported:

- `LanguageDefinition.py` → keyword/token set (§5.1).
- `RomanNumeralConverter.py` → `roman.rs` (§5.3), semantics preserved.
- `MainParser.py` recursive-descent structure → `parser.rs`, but emitting typed
  Rust enums instead of dicts.
- `GenerateC.py` / `ExecuteC.py` → **not ported** (the transpiler is retired).

## 9. Roadmap

### v1 milestones (build order)
1. Project skeleton + CLI reads a `.liss` file.
2. `roman.rs` with unit tests (self-contained, high value, easy first win).
3. Lexer → `Vec<Token>` (word-boundary correct).
4. Parser → AST for declarations, arithmetic, `scribe`.
5. Interpreter for the above (variables, arithmetic, print).
6. Type checker for the above (Latin errors, line numbers).
7. Control flow: `si/aut`, `indem`, `dum`, `abrumpe`/`continua`.
8. Functions: declaration, typed params/return, calls.
9. Booleans + `et`/`vel`/`non` + comparisons across all types.
10. Example programs + integration test suite.

### v2 candidates (deferred, not rejected)
- Arrays / lists.
- Type inference (`sit`).
- `>=` / `<=`.
- Imports / modules.
- `try` / `catch`.
- A REPL.
- Optional "party-trick" backend that also emits C/JS.

## 10. Open Questions
- Final Latin wording for type-error messages (cosmetic; settled during impl).
- ~~Whether `mnr` (float) and `nr` (double) stay distinct types or collapse to
  one in v1.~~ **Resolved 2026-07-22:** distinct types — `mnr` is `f32`, `nr` is
  `f64`. See `docs/superpowers/specs/2026-07-22-nr-double-distinct-type-design.md`.
