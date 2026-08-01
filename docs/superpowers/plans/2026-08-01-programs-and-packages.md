# Programs and Packages (*fasciculi*) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Give Lingua Serpentis a concrete program model — program = folder, package = subfolder named by the folder alone, execution starts at `munus initium reddit ni`, top level is declarations only, `affer` imports with qualified calls — per `docs/superpowers/specs/2026-08-01-programs-and-packages-design.md`.

**Architecture:** The existing per-source pipeline (lexer → parser → checker → interpreter) is kept and *wrapped*, not rewritten. A new `program.rs` loader groups `.liss` files into root + packages; the parser gains a file-level entry (`parse_file`: imports then functions only) alongside the old statement parser (which survives as the unit-test workhorse); the checker and interpreter gain whole-program entries (`check_program`, `execute_program`) that drive the existing per-statement machinery with per-namespace function tables. Errors gain optional file attribution via a new `ProgramError`; the single-source API (`run`, `run_with_warnings`) keeps its exact signature and file-less phrasing.

**Tech Stack:** Rust (edition 2024), no third-party dependencies. Spec: `docs/superpowers/specs/2026-08-01-programs-and-packages-design.md`.

## Global Constraints

- **Build/test command (this machine):** `cargo` is not on `PATH`; run everything through the nix dev shell, from the repo root: `nix develop -c cargo test`. Append `-p lingua-serpentis <filter>` for a focused run. Baseline before this plan: 170 tests green.
- **No third-party crates.** The interpreter and `latinitas` depend on nothing but `std`. Do not add dependencies.
- **Latin error messages.** All new user-facing strings are Latin (exact wordings given per task — they were pre-checked; if you coin a different word, validate each content word with `latinitas::is_latin` first).
- **Branch:** work stays on `rust-rewrite`. The index may contain unrelated staged changes (`.envrc`, `flake.*`, `.idea` deletions, `.gitignore`) — **never `git add -A`**; stage only the files each task names, and commit with `git commit -m "…" -- <paths>` so the unrelated staged entries stay out of your commits.
- **Commit trailer:** end every commit message with `Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>`.
- **Spec rules that bind every task:** entry function is `munus initium reddit ni` (no parameters ⇒ no `capit` clause), exactly once in the root; exit status = returned value mod 256, non-negative; top level allows only `affer` lines then `munus` declarations; packages are flat (one level), named by folder name alone, everything public; imports are per-file, before declarations, duplicates are an error, unused imports are NOT checked; cycles are allowed (no check at all).

---

## File Structure

- `crates/lingua-serpentis/src/error.rs` — add `ProgramError { file: Option<String>, line: Option<usize>, message }`; `Warning` gains `file: Option<String>` and its `line` becomes `Option<usize>` (Task 1).
- `crates/lingua-serpentis/src/token.rs` — add `TokenKind::Import` (`affer`) and `TokenKind::Dot` (Task 2).
- `crates/lingua-serpentis/src/lexer.rs` — lex `.` and the `affer` keyword (Task 2).
- `crates/lingua-serpentis/src/ast.rs` — `Expr::Call` gains `package: Option<String>`; new `FileImport`, `FileAst`, `ParsedFile`, `ParsedProgram` (Task 3).
- `crates/lingua-serpentis/src/parser.rs` — qualified-call parsing, extracted `call_args`, new `parse_file` (Task 3). Old `parse` stays (unit tests across modules use it).
- `crates/lingua-serpentis/src/program.rs` — **new module**: `SourceFile`, `Program`, `assemble` (pure, disk-free), `load_dir` (Task 4).
- `crates/lingua-serpentis/src/checker.rs` — `Checker` gains `packages` + `imports`; extracted `check_args`; new `check_program` with namespace signature collection and `initium` validation (Task 5). Old `check` stays.
- `crates/lingua-serpentis/src/interp.rs` — `funcs` values become `(FuncDecl, String)` (file path); innermost-frame error attribution; new `execute_program` + `exit_status` (Task 6). Old `execute` stays.
- `crates/lingua-serpentis/src/latinity.rs` — new `lint_program` (package names + per-file attribution) (Task 7). Old `lint` stays.
- `crates/lingua-serpentis/src/lib.rs` — declare `pub mod program;`; new `run_program_with_warnings`, `run_dir_with_warnings`, `run_file_with_warnings`; `run`/`run_with_warnings` rewired onto the package model (Task 8).
- `crates/lingua-serpentis/tests/programs.rs` — every source wrapped in `initium`; new package-model integration tests (Task 8).
- `examples/salve.liss`, `examples/calc.liss`, `examples/praecisio.liss`, `examples/series.liss` — rewritten to the `initium` form (Task 8).
- `crates/lingua-serpentis/src/main.rs` — file-vs-folder dispatch, exit status (Task 9).
- `examples/computatrum/` — **new folder example** with a `mathematica` package (Task 9).
- `docs/superpowers/specs/2026-07-19-lingua-serpentis-rust-interpreter-design.md`, `docs/superpowers/specs/2026-08-01-programs-and-packages-design.md` — amendment notes (Task 10).

Dependency order: Task 1 → 2 → 3 → 4 → 5 → 6 → 7 → 8 → 9 → 10. Every task leaves `nix develop -c cargo test` green.

---

## Task 1: `ProgramError` and file-attributed `Warning`

**Files:**
- Modify: `crates/lingua-serpentis/src/error.rs`
- Modify: `crates/lingua-serpentis/src/latinity.rs` (Warning construction + line asserts)
- Modify: `crates/lingua-serpentis/tests/programs.rs` (one `warnings[0].line` assert)

**Interfaces:**
- Consumes: existing `LinguaError { message: String, line: usize }`.
- Produces: `pub struct ProgramError { pub file: Option<String>, pub line: Option<usize>, pub message: String }` with `ProgramError::structural(msg)`, `ProgramError::in_file(file: &str, e: LinguaError)`, `impl From<LinguaError>`, and `Display`; `pub struct Warning { pub message: String, pub file: Option<String>, pub line: Option<usize> }`. Every later task builds errors through these.

- [ ] **Step 1: Write the failing tests**

Append to the `tests` module in `crates/lingua-serpentis/src/error.rs`:

```rust
    #[test]
    fn program_error_formats_all_three_shapes() {
        let both = ProgramError {
            file: Some("mathematica/summa.liss".into()), line: Some(5), message: "res mala".into(),
        };
        assert_eq!(both.to_string(), "Error in 'mathematica/summa.liss', linea 5: res mala");
        let line_only: ProgramError = LinguaError { message: "res mala".into(), line: 5 }.into();
        assert_eq!(line_only.to_string(), "Error in linea 5: res mala");
        let structural = ProgramError::structural("munus 'initium' in programmate non inventum");
        assert_eq!(structural.to_string(), "Error: munus 'initium' in programmate non inventum");
    }

    #[test]
    fn in_file_attaches_the_path() {
        let e = ProgramError::in_file("summa.liss", LinguaError { message: "x".into(), line: 3 });
        assert_eq!(e.file.as_deref(), Some("summa.liss"));
        assert_eq!(e.line, Some(3));
    }

    #[test]
    fn warning_formats_with_and_without_file() {
        let plain = Warning { message: "nomen 'result' fortasse non Latinum est".into(), file: None, line: Some(3) };
        assert_eq!(plain.to_string(), "Monitum in linea 3: nomen 'result' fortasse non Latinum est");
        let in_file = Warning { message: "m".into(), file: Some("a.liss".into()), line: Some(2) };
        assert_eq!(in_file.to_string(), "Monitum in 'a.liss', linea 2: m");
        let bare = Warning { message: "nomen fasciculi 'foobar' fortasse non Latinum est".into(), file: None, line: None };
        assert_eq!(bare.to_string(), "Monitum: nomen fasciculi 'foobar' fortasse non Latinum est");
    }
```

Delete the old `warning_formats_with_line_and_message` test (replaced by `warning_formats_with_and_without_file`).

- [ ] **Step 2: Run tests to verify they fail**

Run: `nix develop -c cargo test -p lingua-serpentis error::`
Expected: COMPILE ERROR (`ProgramError` not found; `Warning` has no field `file`).

- [ ] **Step 3: Implement**

In `crates/lingua-serpentis/src/error.rs`, replace the `Warning` struct and its `Display` with:

```rust
/// A non-fatal style note. Unlike errors, a `Warning` never stops a program
/// from running. `file` is the path relative to the program root (`None` in
/// single-source mode); `line` is `None` for warnings about things that have
/// no line, such as package folder names.
#[derive(Debug, Clone, PartialEq)]
pub struct Warning {
    pub message: String,
    pub file: Option<String>,
    pub line: Option<usize>,
}

impl std::fmt::Display for Warning {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match (&self.file, self.line) {
            (Some(p), Some(l)) => write!(f, "Monitum in '{}', linea {}: {}", p, l, self.message),
            (Some(p), None) => write!(f, "Monitum in '{}': {}", p, self.message),
            (None, Some(l)) => write!(f, "Monitum in linea {}: {}", l, self.message),
            (None, None) => write!(f, "Monitum: {}", self.message),
        }
    }
}
```

Add below `LinguaError`:

```rust
/// An error at program (multi-file) level: optionally attributed to a source
/// file (path relative to the program root) and a line. Structural errors —
/// about the program's shape rather than any source text — carry neither.
#[derive(Debug, Clone, PartialEq)]
pub struct ProgramError {
    pub file: Option<String>,
    pub line: Option<usize>,
    pub message: String,
}

impl ProgramError {
    pub fn structural(message: impl Into<String>) -> Self {
        ProgramError { file: None, line: None, message: message.into() }
    }
    pub fn in_file(file: &str, e: LinguaError) -> Self {
        ProgramError { file: Some(file.to_string()), line: Some(e.line), message: e.message }
    }
}

impl From<LinguaError> for ProgramError {
    fn from(e: LinguaError) -> Self {
        ProgramError { file: None, line: Some(e.line), message: e.message }
    }
}

impl std::fmt::Display for ProgramError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match (&self.file, self.line) {
            (Some(p), Some(l)) => write!(f, "Error in '{}', linea {}: {}", p, l, self.message),
            (Some(p), None) => write!(f, "Error in '{}': {}", p, self.message),
            (None, Some(l)) => write!(f, "Error in linea {}: {}", l, self.message),
            (None, None) => write!(f, "Error: {}", self.message),
        }
    }
}
```

In `crates/lingua-serpentis/src/latinity.rs`:
- `check_name` builds `Warning { message: format!("nomen '{name}' fortasse non Latinum est"), file: None, line: Some(line) }`.
- The `lint` sort stays `out.sort_by_key(|w| w.line);` (`Option<usize>` is `Ord`).
- Update the line asserts in its tests: `w[0].line` → `Some(1)`; in `parameters_are_reported_on_the_definition_line`: `param.line == Some(2)`, `name.line == Some(1)`, `local.line == Some(3)`; in `warnings_are_sorted_by_line`: `assert_eq!(lines, vec![Some(1), Some(2), Some(3)]);`.

In `crates/lingua-serpentis/tests/programs.rs`, in `non_latin_name_warns_but_program_still_runs`: `assert_eq!(warnings[0].line, Some(1));`.

- [ ] **Step 4: Run the full suite**

Run: `nix develop -c cargo test`
Expected: all green (170 tests, 3 net-new).

- [ ] **Step 5: Commit**

```bash
git add crates/lingua-serpentis/src/error.rs crates/lingua-serpentis/src/latinity.rs crates/lingua-serpentis/tests/programs.rs
git commit -m "feat(error): program-level errors and file-attributed warnings

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>" -- crates/lingua-serpentis/src/error.rs crates/lingua-serpentis/src/latinity.rs crates/lingua-serpentis/tests/programs.rs
```

---

## Task 2: Lex `affer` and `.`

**Files:**
- Modify: `crates/lingua-serpentis/src/token.rs`
- Modify: `crates/lingua-serpentis/src/lexer.rs`

**Interfaces:**
- Produces: `TokenKind::Import` (the word `affer`) and `TokenKind::Dot` (a bare `.`). Dots *inside* `°…` Roman literals keep belonging to the numeral token — the `°` branch consumes them first.

- [ ] **Step 1: Write the failing tests**

Append to the `tests` module in `crates/lingua-serpentis/src/lexer.rs`:

```rust
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
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `nix develop -c cargo test -p lingua-serpentis lexer::`
Expected: COMPILE ERROR (`Import`/`Dot` not found).

- [ ] **Step 3: Implement**

In `crates/lingua-serpentis/src/token.rs`, extend `TokenKind`:
- to the keywords line, add `Import,` with the comment updated: `If, Else, While, For, Func, Return, Break, Continue, Print,` stays; add a line `Import,                                    // affer`
- to the punctuation line add `Dot`: `LParen, RParen, LBrace, RBrace, LBracket, RBracket, Semicolon, Comma, Dot,`

In `crates/lingua-serpentis/src/lexer.rs`:
- add a symbol arm next to `','`: `'.' => { tokens.push(Token { kind: TokenKind::Dot, line }); i += 1; }`
- add to `word_to_kind`: `"affer" => TokenKind::Import,` (next to the other keywords).

- [ ] **Step 4: Run the full suite**

Run: `nix develop -c cargo test`
Expected: all green.

- [ ] **Step 5: Commit**

```bash
git add crates/lingua-serpentis/src/token.rs crates/lingua-serpentis/src/lexer.rs
git commit -m "feat(lexer): 'affer' keyword and '.' token

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>" -- crates/lingua-serpentis/src/token.rs crates/lingua-serpentis/src/lexer.rs
```

---

## Task 3: Qualified calls and the declaration-only file grammar

**Files:**
- Modify: `crates/lingua-serpentis/src/ast.rs`
- Modify: `crates/lingua-serpentis/src/parser.rs`
- Modify: `crates/lingua-serpentis/src/checker.rs` (Call arm compiles + rejects qualified calls in the no-import context — this is real behavior, refined in Task 5)
- Modify: `crates/lingua-serpentis/src/interp.rs` (Call arm computes the qualified lookup key — final behavior)

**Interfaces:**
- Consumes: `TokenKind::Import`, `TokenKind::Dot` (Task 2).
- Produces:
  - `Expr::Call { package: Option<String>, name: String, args: Vec<Expr> }`
  - `pub struct FileImport { pub package: String, pub line: usize }`
  - `pub struct FileAst { pub imports: Vec<FileImport>, pub funcs: Vec<Stmt> }` (every `funcs` element is a `StmtKind::Func`)
  - `pub struct ParsedFile { pub path: String, pub ast: FileAst }`
  - `pub struct ParsedProgram { pub root: Vec<ParsedFile>, pub packages: BTreeMap<String, Vec<ParsedFile>> }`
  - `pub fn parse_file(tokens: &[Token]) -> LResult<FileAst>` in `parser.rs`.
  - The old `pub fn parse` is unchanged and stays — module unit tests parse statements with it.

- [ ] **Step 1: Write the failing tests**

Append to the `tests` module in `crates/lingua-serpentis/src/parser.rs`:

```rust
    fn parse_file_ok(src: &str) -> FileAst {
        parse_file(&tokenize(src).unwrap()).expect("expected a valid file")
    }

    fn parse_file_err(src: &str) -> String {
        parse_file(&tokenize(src).unwrap())
            .expect_err("expected a file-level parse error")
            .message
    }

    #[test]
    fn file_with_imports_then_functions() {
        let f = parse_file_ok(
            "affer mathematica;\naffer fractio;\nmunus initium reddit ni { redde nullus; }",
        );
        assert_eq!(f.imports.len(), 2);
        assert_eq!(f.imports[0], FileImport { package: "mathematica".into(), line: 1 });
        assert_eq!(f.imports[1], FileImport { package: "fractio".into(), line: 2 });
        assert_eq!(f.funcs.len(), 1);
        assert!(matches!(f.funcs[0].kind, StmtKind::Func(_)));
    }

    #[test]
    fn empty_file_parses_to_nothing() {
        let f = parse_file_ok("// solum commentarium\n");
        assert!(f.imports.is_empty() && f.funcs.is_empty());
    }

    #[test]
    fn duplicate_import_is_rejected() {
        let m = parse_file_err("affer mathematica;\naffer mathematica;");
        assert!(m.contains("bis allatus"), "got: {m}");
    }

    #[test]
    fn import_after_a_function_is_rejected() {
        let m = parse_file_err("munus f reddit ni { redde nullus; }\naffer mathematica;");
        assert!(m.contains("ante omnia munera"), "got: {m}");
    }

    #[test]
    fn top_level_statement_is_rejected() {
        for src in ["scribe(°I);", "ni x = °V;", "si (verus) { scribe(°I); }"] {
            let m = parse_file_err(src);
            assert!(m.contains("nihil nisi declarationes"), "src {src:?} got: {m}");
        }
    }

    #[test]
    fn qualified_call_parses() {
        let e = parse_first_expr("mathematica.summa(°I, °II);");
        assert_eq!(e, Expr::Call {
            package: Some("mathematica".into()),
            name: "summa".into(),
            args: vec![Expr::Int(ri(1)), Expr::Int(ri(2))],
        });
    }

    #[test]
    fn qualified_name_must_be_called() {
        let toks = tokenize("ni x = mathematica.summa;").unwrap();
        let m = parse(&toks).expect_err("expected error").message;
        assert!(m.contains("vocatio"), "got: {m}");
    }

    #[test]
    fn dot_without_a_function_name_is_rejected() {
        let toks = tokenize("mathematica.(°I);").unwrap();
        assert!(parse(&toks).is_err());
    }
```

Also update the two existing tests that build `Expr::Call` literally — in `zero_arg_call` and `comparison_and_call` the pattern/literal gains `package: None`:

```rust
    #[test]
    fn zero_arg_call() {
        let e = parse_first_expr("adde();");
        assert_eq!(e, Expr::Call { package: None, name: "adde".into(), args: vec![] });
    }
```

and in `comparison_and_call` the match arm becomes `Expr::Call { name, args, .. } => {`.

- [ ] **Step 2: Run tests to verify they fail**

Run: `nix develop -c cargo test -p lingua-serpentis parser::`
Expected: COMPILE ERROR (`Call` has no field `package`; `parse_file`/`FileImport`/`FileAst` not found).

- [ ] **Step 3: Implement the AST**

In `crates/lingua-serpentis/src/ast.rs`:

```rust
    Call { package: Option<String>, name: String, args: Vec<Expr> },
```

replaces the current `Call` variant. Append at the end of the file:

```rust
/// One `affer` line at the top of a file.
#[derive(Debug, Clone, PartialEq)]
pub struct FileImport {
    pub package: String,
    pub line: usize,
}

/// A parsed `.liss` file under the package model: imports first, then
/// functions — nothing else may appear at top level. Every element of
/// `funcs` is a `StmtKind::Func`.
#[derive(Debug, Clone, PartialEq)]
pub struct FileAst {
    pub imports: Vec<FileImport>,
    pub funcs: Vec<Stmt>,
}

/// A parsed file plus its path relative to the program root, kept for error
/// attribution.
#[derive(Debug, Clone, PartialEq)]
pub struct ParsedFile {
    pub path: String,
    pub ast: FileAst,
}

/// The whole parsed program: the root folder's files plus one entry per
/// package. `BTreeMap` keeps package order deterministic.
#[derive(Debug, Clone, PartialEq)]
pub struct ParsedProgram {
    pub root: Vec<ParsedFile>,
    pub packages: std::collections::BTreeMap<String, Vec<ParsedFile>>,
}
```

- [ ] **Step 4: Implement the parser**

In `crates/lingua-serpentis/src/parser.rs`, extract the argument list (the body currently inlined in `primary`'s `Ident` arm) into a method on `Parser`:

```rust
    // argument list: consumes '(' … ')'
    fn call_args(&mut self) -> LResult<Vec<Expr>> {
        self.eat(&TokenKind::LParen)?;
        let mut args = Vec::new();
        if self.peek() != Some(&TokenKind::RParen) {
            args.push(self.expr()?);
            while self.peek() == Some(&TokenKind::Comma) {
                self.advance();
                args.push(self.expr()?);
            }
        }
        self.eat(&TokenKind::RParen)?;
        Ok(args)
    }
```

and rewrite `primary`'s `Ident` arm:

```rust
            Some(TokenKind::Ident(name)) => {
                self.advance();
                if self.peek() == Some(&TokenKind::Dot) {
                    // qualified call: the only thing a package name can do
                    self.advance(); // .
                    let fname = self.ident()?;
                    if self.peek() != Some(&TokenKind::LParen) {
                        return Err(self.err("vocatio functionis post nomen fasciculi expectata"));
                    }
                    let args = self.call_args()?;
                    return Ok(Expr::Call { package: Some(name), name: fname, args });
                }
                if self.peek() == Some(&TokenKind::LParen) {
                    let args = self.call_args()?;
                    Ok(Expr::Call { package: None, name, args })
                } else {
                    Ok(Expr::Ident(name))
                }
            }
```

Update `func_stmt`'s error-help arm near the end of `statement` — the existing `matches!(e, Expr::Call { .. })` patterns still compile unchanged. Add the file-level entry after `pub fn parse`:

```rust
/// Parse one file under the package model (§4.4–4.5 of the design):
/// `affer` lines first, then `munus` declarations, nothing else.
pub fn parse_file(tokens: &[Token]) -> LResult<FileAst> {
    let mut p = Parser { toks: tokens, pos: 0 };
    let mut imports: Vec<FileImport> = Vec::new();
    while p.peek() == Some(&TokenKind::Import) {
        let line = p.line();
        p.advance(); // affer
        let package = p.ident()?;
        p.eat(&TokenKind::Semicolon)?;
        if imports.iter().any(|i| i.package == package) {
            return Err(LinguaError {
                message: format!("fasciculus '{}' bis allatus", package),
                line,
            });
        }
        imports.push(FileImport { package, line });
    }
    let mut funcs = Vec::new();
    while p.pos < p.toks.len() {
        match p.peek() {
            Some(TokenKind::Func) => funcs.push(p.func_stmt()?),
            Some(TokenKind::Import) => {
                return Err(p.err("'affer' ante omnia munera ponendum"));
            }
            _ => return Err(p.err("extra munera nihil nisi declarationes licet")),
        }
    }
    Ok(FileAst { imports, funcs })
}
```

- [ ] **Step 5: Fix the two consumers so the tree compiles**

`crates/lingua-serpentis/src/checker.rs`, `check_expr`'s `Call` arm — destructure the new field and reject qualified calls (correct for the import-less single-namespace `check`; Task 5 replaces this early-return with the import lookup):

```rust
            Expr::Call { package, name, args } => {
                if let Some(p) = package {
                    return Err(LinguaError {
                        message: format!("fasciculus '{}' non allatus est", p),
                        line,
                    });
                }
```

(the rest of the arm continues exactly as today).

`crates/lingua-serpentis/src/interp.rs`, `eval`'s `Call` arm — builtins stay unqualified-only, and a qualified call looks up a dotted key (final behavior; `execute_program` in Task 6 registers those keys):

```rust
            Expr::Call { package, name, args } => {
                let mut vals = Vec::new();
                for a in args { vals.push(self.eval(a, line)?); }
                if package.is_none() && name.as_str() == "longitudo" {
                    // … existing longitudo body unchanged …
                }
                if package.is_none() && name.as_str() == "adde" {
                    // … existing adde body unchanged …
                }
                match package {
                    Some(p) => self.call(&format!("{}.{}", p, name), vals, line),
                    None => self.call(name, vals, line),
                }
            }
```

- [ ] **Step 6: Run the full suite**

Run: `nix develop -c cargo test`
Expected: all green.

- [ ] **Step 7: Commit**

```bash
git add crates/lingua-serpentis/src/ast.rs crates/lingua-serpentis/src/parser.rs crates/lingua-serpentis/src/checker.rs crates/lingua-serpentis/src/interp.rs
git commit -m "feat(parser): qualified calls and the declaration-only file grammar

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>" -- crates/lingua-serpentis/src/ast.rs crates/lingua-serpentis/src/parser.rs crates/lingua-serpentis/src/checker.rs crates/lingua-serpentis/src/interp.rs
```

---

## Task 4: The program loader (`program.rs`)

**Files:**
- Create: `crates/lingua-serpentis/src/program.rs`
- Modify: `crates/lingua-serpentis/src/lib.rs` (add `pub mod program;` after `pub mod latinity;`)

**Interfaces:**
- Consumes: `lexer::tokenize`, `TokenKind::Ident`, `ProgramError` (Task 1).
- Produces:
  - `pub struct SourceFile { pub path: String, pub src: String }` (path relative to program root, `/`-separated)
  - `pub struct Program { pub root: Vec<SourceFile>, pub packages: BTreeMap<String, Vec<SourceFile>> }`
  - `pub fn assemble(files: Vec<SourceFile>) -> Result<Program, ProgramError>` — pure, disk-free
  - `pub fn load_dir(dir: &Path) -> Result<Program, ProgramError>`

- [ ] **Step 1: Write the file with failing tests first**

Create `crates/lingua-serpentis/src/program.rs` containing only the test module (implementation follows in Step 3), and add `pub mod program;` to `lib.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn sf(path: &str) -> SourceFile {
        SourceFile { path: path.into(), src: String::new() }
    }

    #[test]
    fn groups_root_and_packages_sorted() {
        let p = assemble(vec![
            sf("b.liss"), sf("a.liss"),
            sf("mathematica/summa.liss"), sf("mathematica/fractio.liss"),
        ]).unwrap();
        let root: Vec<&str> = p.root.iter().map(|f| f.path.as_str()).collect();
        assert_eq!(root, vec!["a.liss", "b.liss"]);
        let pkgs: Vec<&String> = p.packages.keys().collect();
        assert_eq!(pkgs, vec!["mathematica"]);
        let files: Vec<&str> = p.packages["mathematica"].iter().map(|f| f.path.as_str()).collect();
        assert_eq!(files, vec!["mathematica/fractio.liss", "mathematica/summa.liss"]);
    }

    #[test]
    fn nested_package_is_rejected() {
        let e = assemble(vec![sf("mathematica/fractio/imum.liss")]).unwrap_err();
        assert!(e.message.contains("fasciculo"), "got: {}", e.message);
        assert!(e.message.contains("mathematica/fractio"), "got: {}", e.message);
        assert_eq!(e.file, None);
        assert_eq!(e.line, None);
    }

    #[test]
    fn invalid_package_names_are_rejected() {
        // a hyphen cannot lex as one identifier; a keyword lexes as a keyword
        for bad in ["my-utils/x.liss", "si/x.liss", "duo verba/x.liss"] {
            let e = assemble(vec![sf(bad)]).unwrap_err();
            assert!(e.message.contains("non validum"), "path {bad:?} got: {}", e.message);
        }
    }

    #[test]
    fn load_dir_walks_ignores_and_assembles() {
        let base = std::env::temp_dir().join(format!("liss-load-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(base.join("mathematica")).unwrap();
        std::fs::create_dir_all(base.join("vacua/intus")).unwrap(); // no .liss anywhere → ignored
        std::fs::write(base.join("principale.liss"), "").unwrap();
        std::fs::write(base.join("notae.txt"), "non liss").unwrap(); // ignored
        std::fs::write(base.join("mathematica/summa.liss"), "").unwrap();
        let p = load_dir(&base).unwrap();
        std::fs::remove_dir_all(&base).unwrap();
        assert_eq!(p.root.len(), 1);
        assert_eq!(p.root[0].path, "principale.liss");
        assert_eq!(p.packages.len(), 1);
        assert_eq!(p.packages["mathematica"][0].path, "mathematica/summa.liss");
    }

    #[test]
    fn load_dir_rejects_a_liss_file_below_level_one() {
        let base = std::env::temp_dir().join(format!("liss-deep-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(base.join("mathematica/fractio")).unwrap();
        std::fs::write(base.join("mathematica/fractio/imum.liss"), "").unwrap();
        let e = load_dir(&base).unwrap_err();
        std::fs::remove_dir_all(&base).unwrap();
        assert!(e.message.contains("fasciculo"), "got: {}", e.message);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `nix develop -c cargo test -p lingua-serpentis program::`
Expected: COMPILE ERROR (`SourceFile`/`assemble`/`load_dir` not defined).

- [ ] **Step 3: Implement**

Above the test module in `program.rs`:

```rust
use crate::error::ProgramError;
use crate::token::TokenKind;
use std::collections::BTreeMap;
use std::path::Path;

/// One source file, with its path relative to the program root (always
/// '/'-separated) kept for error attribution.
#[derive(Debug, Clone, PartialEq)]
pub struct SourceFile {
    pub path: String,
    pub src: String,
}

/// A loaded (not yet parsed) program: the root folder's files plus one entry
/// per package (§4.1–4.2 of the design). Files sorted by path, packages by
/// name, so everything downstream is deterministic.
#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub root: Vec<SourceFile>,
    pub packages: BTreeMap<String, Vec<SourceFile>>,
}

/// A package name must be writable in code: it has to lex as exactly one
/// identifier. This rules out spaces, hyphens, leading digits — and keywords.
fn is_valid_package_name(name: &str) -> bool {
    match crate::lexer::tokenize(name) {
        Ok(toks) => toks.len() == 1 && matches!(toks[0].kind, TokenKind::Ident(_)),
        Err(_) => false,
    }
}

/// Group relative `.liss` paths into the program shape, enforcing the flat
/// rule: root files, one level of packages, nothing deeper.
pub fn assemble(files: Vec<SourceFile>) -> Result<Program, ProgramError> {
    let mut root = Vec::new();
    let mut packages: BTreeMap<String, Vec<SourceFile>> = BTreeMap::new();
    for f in files {
        let parts: Vec<&str> = f.path.split('/').collect();
        match parts.as_slice() {
            [_file] => root.push(f),
            [pkg, _file] => {
                if !is_valid_package_name(pkg) {
                    return Err(ProgramError::structural(
                        format!("nomen fasciculi '{}' non validum est", pkg)));
                }
                packages.entry(pkg.to_string()).or_default().push(f);
            }
            [pkg, sub, ..] => {
                return Err(ProgramError::structural(
                    format!("fasciculus in fasciculo non licet: '{}/{}'", pkg, sub)));
            }
            [] => unreachable!("split yields at least one part"),
        }
    }
    root.sort_by(|a, b| a.path.cmp(&b.path));
    for files in packages.values_mut() {
        files.sort_by(|a, b| a.path.cmp(&b.path));
    }
    Ok(Program { root, packages })
}

/// Walk a program folder, collect every `.liss` file with its relative path,
/// and assemble. Non-`.liss` files are ignored; folders with no `.liss`
/// beneath them contribute nothing and are thereby ignored; a `.liss` file
/// deeper than one folder level is rejected by `assemble`.
pub fn load_dir(dir: &Path) -> Result<Program, ProgramError> {
    let mut files = Vec::new();
    collect_liss(dir, "", &mut files)?;
    assemble(files)
}

fn collect_liss(dir: &Path, prefix: &str, out: &mut Vec<SourceFile>) -> Result<(), ProgramError> {
    let entries = std::fs::read_dir(dir).map_err(|e| ProgramError::structural(
        format!("directorium '{}' legi non potest: {}", dir.display(), e)))?;
    for entry in entries {
        let entry = entry.map_err(|e| ProgramError::structural(
            format!("directorium legi non potest: {}", e)))?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        let rel = if prefix.is_empty() { name.clone() } else { format!("{}/{}", prefix, name) };
        if path.is_dir() {
            collect_liss(&path, &rel, out)?;
        } else if name.ends_with(".liss") {
            let src = std::fs::read_to_string(&path).map_err(|e| ProgramError::structural(
                format!("scida '{}' legi non potest: {}", rel, e)))?;
            out.push(SourceFile { path: rel, src });
        }
    }
    Ok(())
}
```

- [ ] **Step 4: Run the full suite**

Run: `nix develop -c cargo test`
Expected: all green.

- [ ] **Step 5: Commit**

```bash
git add crates/lingua-serpentis/src/program.rs crates/lingua-serpentis/src/lib.rs
git commit -m "feat(program): folder loader and flat package assembly

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>" -- crates/lingua-serpentis/src/program.rs crates/lingua-serpentis/src/lib.rs
```

---

## Task 5: Whole-program checking (`check_program`)

**Files:**
- Modify: `crates/lingua-serpentis/src/checker.rs`

**Interfaces:**
- Consumes: `ParsedProgram`, `ParsedFile`, `FileAst`, `FileImport` (Task 3); `ProgramError` (Task 1); `parse_file` (tests).
- Produces: `pub fn check_program(prog: &mut ParsedProgram) -> Result<(), ProgramError>`. The old `pub fn check(&mut [Stmt])` keeps working unchanged (single-namespace, no imports).
- New Latin errors (exact strings): `functio '<n>' bis definita`, `munus 'initium' in programmate non inventum` (structural), `munus 'initium' nihil capere et 'ni' reddere debet`, `fasciculus '<p>' non inventus`, `fasciculus '<p>' non allatus est`, `functio '<f>' in fasciculo '<p>' non inventa`.

- [ ] **Step 1: Write the failing tests**

Append to the `tests` module in `crates/lingua-serpentis/src/checker.rs`:

```rust
    use crate::ast::{ParsedFile, ParsedProgram};
    use crate::parser::parse_file;
    use std::collections::BTreeMap;

    fn pfile(path: &str, src: &str) -> ParsedFile {
        ParsedFile { path: path.into(), ast: parse_file(&tokenize(src).unwrap()).unwrap() }
    }

    fn pprog(root: &[(&str, &str)], pkgs: &[(&str, &[(&str, &str)])]) -> ParsedProgram {
        let mut packages = BTreeMap::new();
        for (name, files) in pkgs {
            packages.insert(
                name.to_string(),
                files.iter().map(|(p, s)| pfile(p, s)).collect(),
            );
        }
        ParsedProgram { root: root.iter().map(|(p, s)| pfile(p, s)).collect(), packages }
    }

    #[test]
    fn program_with_import_and_qualified_call_checks() {
        let mut p = pprog(
            &[("principale.liss",
               "affer mathematica;\nmunus initium reddit ni { scribe(mathematica.summa(°I, °II)); redde nullus; }")],
            &[("mathematica",
               &[("mathematica/summa.liss",
                  "munus summa capit (ni a, ni b) reddit ni { redde a + b; }")])],
        );
        assert!(check_program(&mut p).is_ok());
    }

    #[test]
    fn missing_initium_is_a_structural_error() {
        let mut p = pprog(&[("a.liss", "munus f reddit ni { redde nullus; }")], &[]);
        let e = check_program(&mut p).unwrap_err();
        assert!(e.message.contains("initium"), "got: {}", e.message);
        assert_eq!(e.file, None);
        assert_eq!(e.line, None);
    }

    #[test]
    fn initium_with_wrong_signature_is_rejected() {
        let mut with_params = pprog(
            &[("a.liss", "munus initium capit (ni n) reddit ni { redde n; }")], &[]);
        let e = check_program(&mut with_params).unwrap_err();
        assert!(e.message.contains("nihil capere"), "got: {}", e.message);
        assert_eq!(e.file.as_deref(), Some("a.liss"));

        let mut wrong_ret = pprog(
            &[("a.liss", "munus initium reddit voc { redde \"salve\"; }")], &[]);
        let e = check_program(&mut wrong_ret).unwrap_err();
        assert!(e.message.contains("reddere"), "got: {}", e.message);
    }

    #[test]
    fn duplicate_function_across_files_of_one_folder_is_rejected() {
        let mut p = pprog(
            &[("a.liss", "munus initium reddit ni { redde nullus; }")],
            &[("mathematica", &[
                ("mathematica/a.liss", "munus summa reddit ni { redde °I; }"),
                ("mathematica/b.liss", "munus summa reddit ni { redde °II; }"),
            ])],
        );
        let e = check_program(&mut p).unwrap_err();
        assert!(e.message.contains("bis definita"), "got: {}", e.message);
        assert_eq!(e.file.as_deref(), Some("mathematica/b.liss"));
    }

    #[test]
    fn importing_an_unknown_package_is_rejected() {
        let mut p = pprog(
            &[("a.liss", "affer ignota;\nmunus initium reddit ni { redde nullus; }")], &[]);
        let e = check_program(&mut p).unwrap_err();
        assert!(e.message.contains("non inventus"), "got: {}", e.message);
        assert_eq!(e.file.as_deref(), Some("a.liss"));
        assert_eq!(e.line, Some(1));
    }

    #[test]
    fn qualified_call_without_import_is_rejected() {
        let mut p = pprog(
            &[("a.liss", "munus initium reddit ni { redde mathematica.summa(°I, °II); }")],
            &[("mathematica", &[("mathematica/s.liss",
                "munus summa capit (ni a, ni b) reddit ni { redde a + b; }")])],
        );
        let e = check_program(&mut p).unwrap_err();
        assert!(e.message.contains("non allatus"), "got: {}", e.message);
    }

    #[test]
    fn unknown_function_in_a_real_package_is_rejected() {
        let mut p = pprog(
            &[("a.liss", "affer mathematica;\nmunus initium reddit ni { redde mathematica.ignota(); }")],
            &[("mathematica", &[("mathematica/s.liss",
                "munus summa capit (ni a, ni b) reddit ni { redde a + b; }")])],
        );
        let e = check_program(&mut p).unwrap_err();
        assert!(e.message.contains("in fasciculo"), "got: {}", e.message);
    }

    #[test]
    fn import_cycles_are_allowed() {
        let mut p = pprog(
            &[("a.liss", "affer par;\nmunus initium reddit ni { redde par.aequum(°IV); }")],
            &[
                ("par", &[("par/p.liss",
                    "affer impar;\nmunus aequum capit (ni n) reddit ni { si (n par_est nullus) { redde °I; } redde impar.imparum(n - °I); }")]),
                ("impar", &[("impar/i.liss",
                    "affer par;\nmunus imparum capit (ni n) reddit ni { si (n par_est nullus) { redde nullus; } redde par.aequum(n - °I); }")]),
            ],
        );
        assert!(check_program(&mut p).is_ok());
    }

    #[test]
    fn local_and_imported_functions_may_share_a_name() {
        // root's own `summa` returns voc; the package's returns ni — the call
        // sites type-check against the right one, so resolution is provably
        // by qualifier, not by accident.
        let mut p = pprog(
            &[("a.liss",
               "affer mathematica;\nmunus summa reddit voc { redde \"summa\"; }\nmunus initium reddit ni { scribe(summa()); redde mathematica.summa(°I, °II); }")],
            &[("mathematica", &[("mathematica/s.liss",
                "munus summa capit (ni a, ni b) reddit ni { redde a + b; }")])],
        );
        assert!(check_program(&mut p).is_ok());
    }

    #[test]
    fn initium_in_a_package_is_an_ordinary_function() {
        let mut p = pprog(
            &[("a.liss", "affer alia;\nmunus initium reddit ni { redde alia.initium(°V); }")],
            &[("alia", &[("alia/x.liss",
                "munus initium capit (ni n) reddit ni { redde n; }")])],
        );
        assert!(check_program(&mut p).is_ok());
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `nix develop -c cargo test -p lingua-serpentis checker::`
Expected: COMPILE ERROR (`check_program` not found).

- [ ] **Step 3: Implement**

In `crates/lingua-serpentis/src/checker.rs`:

Imports at the top become:

```rust
use crate::ast::*;
use crate::error::{LinguaError, LResult, ProgramError};
use std::collections::{HashMap, HashSet};
```

Add a signature alias and extend `Checker`:

```rust
type Sig = (Vec<Type>, Type); // (param types, return type)

struct Checker {
    scopes: Vec<HashMap<String, Type>>,
    funcs: HashMap<String, Sig>,                       // current namespace (folder)
    packages: HashMap<String, HashMap<String, Sig>>,   // program-wide package signatures
    imports: HashSet<String>,                          // current file's affer set
    current_ret: Option<Type>,
}
```

`Checker::new` initializes the two new fields empty:

```rust
    fn new() -> Self {
        Checker {
            scopes: vec![HashMap::new()],
            funcs: HashMap::new(),
            packages: HashMap::new(),
            imports: HashSet::new(),
            current_ret: None,
        }
    }
```

Extract the shared argument-checking loop (used by both call forms) as a method:

```rust
    fn check_args(&self, fname: &str, params: &[Type], args: &mut [Expr], line: usize) -> LResult<()> {
        if params.len() != args.len() {
            return Err(LinguaError {
                message: format!("functio '{}' numerum argumentorum falsum accepit", fname),
                line,
            });
        }
        for (want, arg) in params.iter().zip(args.iter_mut()) {
            let got = self.resolve_expr(arg, Some(want.clone()), line)?;
            if got != *want {
                return Err(LinguaError {
                    message: format!("argumentum typum '{}' expectat, sed '{}' datum est",
                        Self::type_name(want), Self::type_name(&got)),
                    line,
                });
            }
        }
        Ok(())
    }
```

Replace Task 3's early-return in the `Call` arm with the real qualified path, and route both forms through `check_args`:

```rust
            Expr::Call { package: Some(p), name, args } => {
                if !self.imports.contains(p) {
                    return Err(LinguaError {
                        message: format!("fasciculus '{}' non allatus est", p),
                        line,
                    });
                }
                let sigs = self.packages.get(p).ok_or_else(|| LinguaError {
                    message: format!("fasciculus '{}' non inventus", p),
                    line,
                })?;
                let (params, ret) = sigs.get(name).cloned().ok_or_else(|| LinguaError {
                    message: format!("functio '{}' in fasciculo '{}' non inventa", name, p),
                    line,
                })?;
                self.check_args(name, &params, args, line)?;
                Ok(InferTy::Known(ret))
            }
            Expr::Call { package: None, name, args } => {
                // … existing `longitudo` and `adde` builtin blocks unchanged …
                let (params, ret) = self.funcs.get(name)
                    .cloned()
                    .ok_or_else(|| LinguaError { message: format!("functio ignota: '{}'", name), line })?;
                self.check_args(name, &params, args, line)?;
                Ok(InferTy::Known(ret))
            }
```

Add the program-level entry after `pub fn check` (which stays as-is):

```rust
/// Collect one folder's function signatures across all its files, rejecting
/// reserved names and duplicates (§4.2 — two `initium` in the root land here
/// as the ordinary duplicate error).
fn collect_sigs(files: &[ParsedFile]) -> Result<HashMap<String, Sig>, ProgramError> {
    let mut sigs: HashMap<String, Sig> = HashMap::new();
    for file in files {
        for s in &file.ast.funcs {
            let StmtKind::Func(f) = &s.kind else { continue };
            if f.name == "longitudo" || f.name == "adde" {
                return Err(ProgramError::in_file(&file.path, LinguaError {
                    message: format!("nomen '{}' reservatum est", f.name),
                    line: s.line,
                }));
            }
            if sigs.contains_key(&f.name) {
                return Err(ProgramError::in_file(&file.path, LinguaError {
                    message: format!("functio '{}' bis definita", f.name),
                    line: s.line,
                }));
            }
            let ptypes = f.params.iter().map(|(t, _)| t.clone()).collect();
            sigs.insert(f.name.clone(), (ptypes, f.ret.clone()));
        }
    }
    Ok(sigs)
}

/// Check one file's bodies against its folder's namespace and its own imports.
fn check_file(
    file: &mut ParsedFile,
    ns_sigs: &HashMap<String, Sig>,
    pkg_sigs: &HashMap<String, HashMap<String, Sig>>,
) -> Result<(), ProgramError> {
    for imp in &file.ast.imports {
        if !pkg_sigs.contains_key(&imp.package) {
            return Err(ProgramError::in_file(&file.path, LinguaError {
                message: format!("fasciculus '{}' non inventus", imp.package),
                line: imp.line,
            }));
        }
    }
    let mut c = Checker::new();
    c.funcs = ns_sigs.clone();
    c.packages = pkg_sigs.clone();
    c.imports = file.ast.imports.iter().map(|i| i.package.clone()).collect();
    for s in file.ast.funcs.iter_mut() {
        c.check_stmt(s).map_err(|e| ProgramError::in_file(&file.path, e))?;
    }
    Ok(())
}

/// Check a whole parsed program (§4 of the design): per-namespace signatures,
/// the `initium` entry point, import validity, then every file's bodies.
pub fn check_program(prog: &mut ParsedProgram) -> Result<(), ProgramError> {
    let root_sigs = collect_sigs(&prog.root)?;
    let mut pkg_sigs: HashMap<String, HashMap<String, Sig>> = HashMap::new();
    for (name, files) in &prog.packages {
        pkg_sigs.insert(name.clone(), collect_sigs(files)?);
    }

    match root_sigs.get("initium") {
        None => return Err(ProgramError::structural(
            "munus 'initium' in programmate non inventum")),
        Some((params, ret)) => {
            if !params.is_empty() || *ret != Type::Int {
                // find the declaration to attribute the error
                for file in &prog.root {
                    for s in &file.ast.funcs {
                        if let StmtKind::Func(f) = &s.kind {
                            if f.name == "initium" {
                                return Err(ProgramError::in_file(&file.path, LinguaError {
                                    message: "munus 'initium' nihil capere et 'ni' reddere debet".into(),
                                    line: s.line,
                                }));
                            }
                        }
                    }
                }
                unreachable!("initium was in root_sigs, so its declaration exists");
            }
        }
    }

    for file in prog.root.iter_mut() {
        check_file(file, &root_sigs, &pkg_sigs)?;
    }
    for (name, files) in prog.packages.iter_mut() {
        let ns = &pkg_sigs[name];
        for file in files.iter_mut() {
            check_file(file, ns, &pkg_sigs)?;
        }
    }
    Ok(())
}
```

- [ ] **Step 4: Run the full suite**

Run: `nix develop -c cargo test`
Expected: all green (the existing single-namespace checker tests must be untouched by the refactor).

- [ ] **Step 5: Commit**

```bash
git add crates/lingua-serpentis/src/checker.rs
git commit -m "feat(checker): whole-program checking with imports and initium

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>" -- crates/lingua-serpentis/src/checker.rs
```

---

## Task 6: Program execution (`execute_program`, exit status, file attribution)

**Files:**
- Modify: `crates/lingua-serpentis/src/interp.rs`

**Interfaces:**
- Consumes: `ParsedProgram` (Task 3), `ProgramError` (Task 1), `Roman::{from_u64, div_rem, to_i64}`.
- Produces: `pub fn execute_program(prog: &ParsedProgram) -> Result<(String, u8), ProgramError>` — output plus exit status. Old `pub fn execute(&[Stmt])` stays for unit tests.
- Attribution rule: a runtime error is attributed to the file of the *innermost* function frame that raised it (§4.6; every runtime error is inside some function because top level has no statements).

- [ ] **Step 1: Write the failing tests**

Append to the `tests` module in `crates/lingua-serpentis/src/interp.rs`:

```rust
    use crate::ast::{ParsedFile, ParsedProgram};
    use crate::parser::parse_file;
    use std::collections::BTreeMap;

    fn pfile(path: &str, src: &str) -> ParsedFile {
        ParsedFile { path: path.into(), ast: parse_file(&tokenize(src).unwrap()).unwrap() }
    }

    fn run_prog(root: &[(&str, &str)], pkgs: &[(&str, &[(&str, &str)])])
        -> Result<(String, u8), crate::error::ProgramError>
    {
        let mut packages = BTreeMap::new();
        for (name, files) in pkgs {
            packages.insert(name.to_string(), files.iter().map(|(p, s)| pfile(p, s)).collect());
        }
        let prog = ParsedProgram {
            root: root.iter().map(|(p, s)| pfile(p, s)).collect(),
            packages,
        };
        execute_program(&prog)
    }

    #[test]
    fn initium_runs_and_exits_zero() {
        let r = run_prog(&[("a.liss", "munus initium reddit ni { scribe(°V); redde nullus; }")], &[]);
        assert_eq!(r.unwrap(), ("V\n".to_string(), 0));
    }

    #[test]
    fn exit_status_is_the_return_mod_256() {
        let r = run_prog(&[("a.liss", "munus initium reddit ni { redde °CCLVII; }")], &[]);
        assert_eq!(r.unwrap().1, 1); // 257 mod 256
        let r = run_prog(&[("a.liss", "munus initium reddit ni { redde °CCLVI; }")], &[]);
        assert_eq!(r.unwrap().1, 0); // 256 mod 256
        let r = run_prog(&[("a.liss", "munus initium reddit ni { redde °V; }")], &[]);
        assert_eq!(r.unwrap().1, 5);
    }

    #[test]
    fn negative_return_wraps_like_unix() {
        let r = run_prog(&[("a.liss", "munus initium reddit ni { redde -°I; }")], &[]);
        assert_eq!(r.unwrap().1, 255);
    }

    #[test]
    fn falling_off_initium_exits_zero() {
        // The existing fall-through semantics return Int zero.
        let r = run_prog(&[("a.liss", "munus initium reddit ni { scribe(°I); }")], &[]);
        assert_eq!(r.unwrap(), ("I\n".to_string(), 0));
    }

    #[test]
    fn qualified_call_reaches_the_package() {
        let r = run_prog(
            &[("principale.liss",
               "affer mathematica;\nmunus initium reddit ni { scribe(mathematica.duplum(°XXI)); redde nullus; }")],
            &[("mathematica", &[("mathematica/m.liss",
               "munus duplum capit (ni n) reddit ni { redde n * °II; }")])],
        );
        assert_eq!(r.unwrap().0, "XLII\n");
    }

    #[test]
    fn same_folder_files_share_a_namespace() {
        let r = run_prog(
            &[
                ("auxilia.liss", "munus quinque reddit ni { redde °V; }"),
                ("principale.liss", "munus initium reddit ni { scribe(quinque()); redde nullus; }"),
            ],
            &[],
        );
        assert_eq!(r.unwrap().0, "V\n");
    }

    #[test]
    fn runtime_error_names_the_file_of_the_failing_function() {
        let r = run_prog(
            &[("principale.liss",
               "affer mathematica;\nmunus initium reddit ni { redde mathematica.divide(°I, nullus); }")],
            &[("mathematica", &[("mathematica/m.liss",
               "munus divide capit (ni a, ni b) reddit ni { redde a / b; }")])],
        );
        let e = r.unwrap_err();
        assert_eq!(e.file.as_deref(), Some("mathematica/m.liss"));
        assert_eq!(e.line, Some(1));
        assert!(e.message.contains("divisio"), "got: {}", e.message);
    }

    #[test]
    fn runtime_error_in_initium_names_the_root_file() {
        let r = run_prog(
            &[("principale.liss", "munus initium reddit ni { redde °I / nullus; }")], &[]);
        let e = r.unwrap_err();
        assert_eq!(e.file.as_deref(), Some("principale.liss"));
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `nix develop -c cargo test -p lingua-serpentis interp::`
Expected: COMPILE ERROR (`execute_program` not found).

- [ ] **Step 3: Implement**

In `crates/lingua-serpentis/src/interp.rs`:

Change the `Interp` struct — function table values carry the declaring file's path, plus the attribution slot:

```rust
struct Interp {
    scopes: Vec<HashMap<String, Value>>,
    funcs: HashMap<String, (FuncDecl, String)>, // qualified name -> (decl, source path)
    out: String,
    err_file: Option<String>, // set by the innermost frame that raised an error
}
```

`Interp::new` gains `err_file: None`. The two existing registration sites wrap the value: in `exec`'s `Func` arm — `self.funcs.insert(f.name.clone(), (f.clone(), String::new()));` — and in `execute`'s pre-registration loop likewise `(f.clone(), String::new())`.

`call` destructures the tuple and, on error, claims attribution if no inner frame has:

```rust
    fn call(&mut self, name: &str, args: Vec<Value>, line: usize) -> LResult<Value> {
        let (f, file) = self.funcs.get(name)
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
        // Swap in an isolated scope stack for the call, restore afterwards
        // (unconditionally, even if the body raises an error via `?`).
        let saved = std::mem::replace(&mut self.scopes, vec![frame]);
        let result = (|| -> LResult<Value> {
            for s in &f.body {
                if let Flow::Return(v) = self.exec(s)? { return Ok(v); }
            }
            Ok(Value::Int(Roman::zero()))
        })();
        self.scopes = saved;
        // Innermost frame first: the failing function's own file wins.
        if result.is_err() && self.err_file.is_none() && !file.is_empty() {
            self.err_file = Some(file);
        }
        result
    }
```

Add at the bottom (next to `execute`, which stays):

```rust
/// Unix keeps an exit status's low 8 bits: the value mod CCLVI, shifted
/// non-negative (a negative remainder gains one modulus).
fn exit_status(r: &Roman) -> u8 {
    let rem = r.div_rem(&Roman::from_u64(256)).1;
    let x = rem.to_i64().expect("|rem| < 256 fits i64");
    ((x + 256) % 256) as u8
}

/// Run a checked program (§4.3 of the design): register every function under
/// its qualified name, call the root's `initium`, and turn its returned ni
/// into the process exit status.
pub fn execute_program(prog: &ParsedProgram) -> Result<(String, u8), ProgramError> {
    let mut it = Interp::new();
    for file in &prog.root {
        for s in &file.ast.funcs {
            if let StmtKind::Func(f) = &s.kind {
                it.funcs.insert(f.name.clone(), (f.clone(), file.path.clone()));
            }
        }
    }
    for (pkg, files) in &prog.packages {
        for file in files {
            for s in &file.ast.funcs {
                if let StmtKind::Func(f) = &s.kind {
                    it.funcs.insert(format!("{}.{}", pkg, f.name), (f.clone(), file.path.clone()));
                }
            }
        }
    }
    match it.call("initium", Vec::new(), 0) {
        Ok(v) => {
            let code = match v {
                Value::Int(r) => exit_status(&r),
                _ => 0, // checker guarantees Int; defensive only
            };
            Ok((it.out, code))
        }
        Err(e) => Err(ProgramError {
            file: it.err_file.take(),
            line: Some(e.line),
            message: e.message,
        }),
    }
}
```

and extend the imports at the top of the file:

```rust
use crate::error::{LinguaError, LResult, ProgramError};
```

- [ ] **Step 4: Run the full suite**

Run: `nix develop -c cargo test`
Expected: all green.

- [ ] **Step 5: Commit**

```bash
git add crates/lingua-serpentis/src/interp.rs
git commit -m "feat(interp): program execution from initium with Roman exit codes

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>" -- crates/lingua-serpentis/src/interp.rs
```

---

## Task 7: Latinity for programs (`lint_program`)

**Files:**
- Modify: `crates/lingua-serpentis/src/latinity.rs`

**Interfaces:**
- Consumes: `ParsedProgram` (Task 3), the existing `lint(&[Stmt])`, `latinitas::is_latin`.
- Produces: `pub fn lint_program(prog: &ParsedProgram) -> Vec<Warning>` — package-name warnings first (no file, no line), then each file's declaration warnings tagged with the file's path. Old `lint` stays.

- [ ] **Step 1: Write the failing tests**

Append to the `tests` module in `crates/lingua-serpentis/src/latinity.rs`:

```rust
    use crate::ast::{ParsedFile, ParsedProgram};
    use std::collections::BTreeMap;

    fn pfile(path: &str, src: &str) -> ParsedFile {
        ParsedFile {
            path: path.into(),
            ast: crate::parser::parse_file(&tokenize(src).unwrap()).unwrap(),
        }
    }

    fn prog_with_package(pkg: &str) -> ParsedProgram {
        let mut packages = BTreeMap::new();
        packages.insert(pkg.to_string(), vec![
            pfile(&format!("{pkg}/intus.liss"), "munus quinque reddit ni { redde °V; }"),
        ]);
        ParsedProgram {
            root: vec![pfile("principale.liss",
                "munus initium reddit ni { ni result = °V; scribe(result); redde nullus; }")],
            packages,
        }
    }

    #[test]
    fn non_latin_package_name_warns_without_file_or_line() {
        let ws = lint_program(&prog_with_package("foobar"));
        let pkg_warning = ws.iter().find(|w| w.message.contains("fasciculi"))
            .expect("expected a package-name warning");
        assert!(pkg_warning.message.contains("foobar"));
        assert_eq!(pkg_warning.file, None);
        assert_eq!(pkg_warning.line, None);
    }

    #[test]
    fn latin_package_name_does_not_warn() {
        let ws = lint_program(&prog_with_package("mathematica"));
        assert!(ws.iter().all(|w| !w.message.contains("fasciculi")), "got {ws:?}");
    }

    #[test]
    fn file_warnings_carry_their_path() {
        // `result` inside initium is the only non-Latin identifier.
        let ws = lint_program(&prog_with_package("mathematica"));
        let w = ws.iter().find(|w| w.message.contains("result"))
            .expect("expected the 'result' warning");
        assert_eq!(w.file.as_deref(), Some("principale.liss"));
        assert_eq!(w.line, Some(1));
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `nix develop -c cargo test -p lingua-serpentis latinity::`
Expected: COMPILE ERROR (`lint_program` not found).

- [ ] **Step 3: Implement**

In `crates/lingua-serpentis/src/latinity.rs` (extend the `use crate::ast::…` line to include `ParsedProgram`):

```rust
/// Lint a whole program: package folder names (§4.2 — warning-only, like all
/// Latinity checks; file names are never checked because they never appear in
/// code), then every file's declarations attributed to their file. Package
/// warnings come first; within a file, warnings stay sorted by line.
pub fn lint_program(prog: &ParsedProgram) -> Vec<Warning> {
    let mut out = Vec::new();
    for name in prog.packages.keys() {
        if name.chars().count() > 1 && !latinitas::is_latin(name) {
            out.push(Warning {
                message: format!("nomen fasciculi '{name}' fortasse non Latinum est"),
                file: None,
                line: None,
            });
        }
    }
    let package_files = prog.packages.values().flatten();
    for file in prog.root.iter().chain(package_files) {
        let mut ws = lint(&file.ast.funcs);
        for w in &mut ws {
            w.file = Some(file.path.clone());
        }
        out.extend(ws);
    }
    out
}
```

- [ ] **Step 4: Run the full suite**

Run: `nix develop -c cargo test`
Expected: all green.

- [ ] **Step 5: Commit**

```bash
git add crates/lingua-serpentis/src/latinity.rs
git commit -m "feat(latinity): lint package names and attribute warnings to files

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>" -- crates/lingua-serpentis/src/latinity.rs
```

---

## Task 8: The package pipeline — `run()` flips, tests and examples move to `initium`

This task is atomic by necessity: rewiring `run`/`run_with_warnings` onto the strict model invalidates every script-style integration test and example, so they convert in the same commit. All expected *outputs* stay identical — only the wrapping changes.

**Files:**
- Modify: `crates/lingua-serpentis/src/lib.rs`
- Modify: `crates/lingua-serpentis/tests/programs.rs`
- Modify: `examples/salve.liss`, `examples/calc.liss`, `examples/praecisio.liss`, `examples/series.liss`

**Interfaces:**
- Consumes: everything from Tasks 1–7.
- Produces (in `lib.rs`):
  - `pub fn run_program_with_warnings(files: &[(&str, &str)]) -> Result<(String, u8, Vec<Warning>), ProgramError>` — the disk-free multi-file entry (paths are `/`-relative).
  - `pub fn run_dir_with_warnings(dir: &std::path::Path) -> Result<(String, u8, Vec<Warning>), ProgramError>`
  - `pub fn run_file_with_warnings(src: &str) -> Result<(String, u8, Vec<Warning>), LinguaError>` — single-file program; errors/warnings file-less (structural errors surface at line 1).
  - `pub fn run_with_warnings(src: &str) -> LResult<(String, Vec<Warning>)>` and `pub fn run(src: &str) -> LResult<String>` — same signatures as today, now enforcing the package model.
  - Re-export: `pub use error::{ProgramError, Warning};`

- [ ] **Step 1: Rewrite `lib.rs`**

```rust
pub mod error;
pub mod roman;
pub mod fraction;
pub mod token;
pub mod lexer;
pub mod ast;
pub mod parser;
pub mod checker;
pub mod value;
pub mod interp;
pub mod latinity;
pub mod program;

use ast::{ParsedFile, ParsedProgram};
use error::{LResult, LinguaError};
use program::{Program, SourceFile};
pub use error::{ProgramError, Warning};

fn parse_one(f: SourceFile) -> Result<ParsedFile, ProgramError> {
    let tokens = lexer::tokenize(&f.src).map_err(|e| ProgramError::in_file(&f.path, e))?;
    let ast = parser::parse_file(&tokens).map_err(|e| ProgramError::in_file(&f.path, e))?;
    Ok(ParsedFile { path: f.path, ast })
}

/// The full pipeline over a loaded program: parse every file, check the
/// whole program, lint, execute from `initium`. Returns the collected
/// `scribe` output, the exit status, and any Latinity warnings.
fn run_loaded(prog: Program) -> Result<(String, u8, Vec<Warning>), ProgramError> {
    let mut root = Vec::new();
    for f in prog.root {
        root.push(parse_one(f)?);
    }
    let mut packages = std::collections::BTreeMap::new();
    for (name, files) in prog.packages {
        let mut parsed = Vec::new();
        for f in files {
            parsed.push(parse_one(f)?);
        }
        packages.insert(name, parsed);
    }
    let mut parsed = ParsedProgram { root, packages };
    checker::check_program(&mut parsed)?;
    let warnings = latinity::lint_program(&parsed);
    let (output, code) = interp::execute_program(&parsed)?;
    Ok((output, code, warnings))
}

/// Run a program given as (relative path, source) pairs — the disk-free
/// entry integration tests use. Paths are '/'-separated and follow the
/// folder rules: `"a.liss"` is a root file, `"mathematica/summa.liss"` is in
/// the `mathematica` package, anything deeper is an error.
pub fn run_program_with_warnings(
    files: &[(&str, &str)],
) -> Result<(String, u8, Vec<Warning>), ProgramError> {
    let files = files.iter()
        .map(|(p, s)| SourceFile { path: (*p).to_string(), src: (*s).to_string() })
        .collect();
    run_loaded(program::assemble(files)?)
}

/// Run the program folder at `dir`.
pub fn run_dir_with_warnings(
    dir: &std::path::Path,
) -> Result<(String, u8, Vec<Warning>), ProgramError> {
    run_loaded(program::load_dir(dir)?)
}

/// Run a single-file program — the degenerate case of §4.1: same rules
/// (`initium` required, declarations only), but errors and warnings keep the
/// file-less phrasing the single-source API has always had. Structural
/// errors, which have no line of their own, surface at line 1.
pub fn run_file_with_warnings(src: &str) -> Result<(String, u8, Vec<Warning>), LinguaError> {
    let file = SourceFile { path: "fons.liss".into(), src: src.to_string() };
    let prog = Program { root: vec![file], packages: std::collections::BTreeMap::new() };
    match run_loaded(prog) {
        Ok((out, code, mut warnings)) => {
            for w in &mut warnings {
                w.file = None;
            }
            Ok((out, code, warnings))
        }
        Err(e) => Err(LinguaError { message: e.message, line: e.line.unwrap_or(1) }),
    }
}

/// Single-file pipeline, discarding the exit status (library convenience).
pub fn run_with_warnings(src: &str) -> LResult<(String, Vec<Warning>)> {
    run_file_with_warnings(src).map(|(output, _, warnings)| (output, warnings))
}

/// Single-file pipeline, discarding warnings too.
pub fn run(src: &str) -> LResult<String> {
    run_with_warnings(src).map(|(output, _)| output)
}
```

- [ ] **Step 2: Rewrite the examples**

`examples/salve.liss`:

```
munus initium reddit ni {
    scribe("Salve, munde!");
    redde nullus;
}
```

`examples/calc.liss`:

```
munus summare capit (ni a, ni b) reddit ni {
    ni c = a + b;
    redde c;
}

munus initium reddit ni {
    ni summa = summare(°XXV, °XXX.M.V.C.VII);
    si (summa minor_est °C) { summa = summa + °C; }
    scribe(summa);
    redde nullus;
}
```

`examples/praecisio.liss` (comments preserved, statements now inside `initium`, `iunge` stays a sibling function):

```
// Typi fluitantes distincti: mnr (granum unciae, 1/12) et nr (granum scripuli, 1/288).
// Eadem fractio Romana, granum tamen diversum.

// Munus quod nr accipit et reddit; litterae fractae typum contextus assumunt.
munus iunge capit (nr x, nr y) reddit nr {
    redde x + y;
}

munus initium reddit ni {
    // °I:I/III est unus plus tertia pars (1 + 1/3 = 4/12, in utroque grano exacta).
    mnr tertia_brevis = °I:I/III;   // granum unciae
    nr tertia_longa = °I:I/III;     // granum scripuli
    scribe(tertia_brevis);          // I et triens
    scribe(tertia_longa);           // I et triens

    // °I:I/XXIV (1 + 1/24) cadit infra granum unciae: mnr id abicit, nr servat.
    mnr minor = °I:I/XXIV;          // 1/24 infra unciam -> truncatur
    nr maior = °I:I/XXIV;           // 1/24 = semuncia, exacte
    scribe(minor);                  // I
    scribe(maior);                  // I et semuncia

    nr summa = iunge(°I:I/IV, °I:I/IV);   // 1¼ + 1¼ = 2½
    scribe(summa);                          // II et semis

    // Litterae fractae sine typo declarato ad nr recidunt.
    scribe(°I:I/III);                       // I et triens
    redde nullus;
}
```

`examples/series.liss`:

```
// Series (lista): aedifica, auge, itera, scribe.
munus initium reddit ni {
    ni lista numeri = [°X, °XX, °XXX];
    numeri = adde(numeri, °XL);   // cresce: [10, 20, 30, 40]
    numeri[°I] = °I;              // scribe primum elementum

    // Itera 1-index: dum i minor est longitudo + I.
    ni summa = nullus;
    dum (ni i = °I; i minor_est longitudo(numeri) + °I; i = i + °I) {
        summa = summa + numeri[i];
    }
    scribe(numeri);   // [I, XX, XXX, XL]
    scribe(summa);    // XCI
    redde nullus;
}
```

- [ ] **Step 3: Migrate `tests/programs.rs`**

Wrap every source in the entry function. Rules:
- Tests that assert an **error line** or **warning line** keep the whole program on ONE line so the line numbers stay 1. E.g.:
  - `type_error_reports_latin_and_line`: `let src = "munus initium reddit ni { ni x = \"salve\"; redde nullus; }";` — asserts unchanged (`err.line == 1`, message starts `"Error in linea 1:"`).
  - `non_latin_name_warns_but_program_still_runs`: `"munus initium reddit ni { ni result = °V; scribe(result); redde nullus; }"` — `warnings[0].line == Some(1)` (already updated in Task 1), output `"V\n"`.
  - `latin_program_produces_no_warnings`: `"munus initium reddit ni { ni numerus = °V; scribe(numerus); redde nullus; }"`.
- Multi-line `r#"…"#` sources wrap their statements inside `munus initium reddit ni { … redde nullus; }`, functions staying siblings of `initium`. E.g. `full_program_with_function_and_if` becomes:

```rust
    let src = r#"
        munus summare capit (ni a, ni b) reddit ni {
            ni c = a + b;
            redde c;
        }
        munus initium reddit ni {
            ni summa = summare(°XXV, °XXX.M.V.C.VII);
            si (summa minor_est °C) { summa = summa + °C; }
            scribe(summa);
            redde nullus;
        }
    "#;
```

- One-line `run(…)` assertions (the fraction/list batch from `mnr_truncates_below_its_uncia_grain` through `lists_have_value_semantics`) get the same one-line wrap, e.g. `run("munus initium reddit ni { mnr x = °I:I/XXIV; scribe(x); redde nullus; }")`. Error-expecting ones (`mixing_float_types_is_a_type_error`, `fraction_divide_by_zero_errors`, `fraction_modulo_is_a_type_error`) wrap identically — they only assert `is_err()`.
- The example-file tests (`calc_example_still_runs`, `praecisio_example_runs`, `series_example_runs`) keep their exact expected outputs — the rewritten examples must reproduce them verbatim.

Then append the new package-model integration tests:

```rust
use lingua_serpentis::run_program_with_warnings;

#[test]
fn multi_file_program_with_package_runs() {
    let files = [
        ("principale.liss",
         "affer mathematica;\nmunus initium reddit ni { scribe(mathematica.summa(°XXV, °XVII)); redde nullus; }"),
        ("mathematica/summa.liss",
         "munus summa capit (ni a, ni b) reddit ni { redde a + b; }"),
    ];
    let (output, code, warnings) = run_program_with_warnings(&files).unwrap();
    assert_eq!(output, "XLII\n");
    assert_eq!(code, 0);
    assert!(warnings.is_empty(), "unexpected warnings: {warnings:?}");
}

#[test]
fn same_folder_files_need_no_import() {
    let files = [
        ("auxilia.liss", "munus quinque reddit ni { redde °V; }"),
        ("principale.liss", "munus initium reddit ni { scribe(quinque()); redde nullus; }"),
    ];
    assert_eq!(run_program_with_warnings(&files).unwrap().0, "V\n");
}

#[test]
fn program_without_initium_is_a_bare_error() {
    let files = [("a.liss", "munus f reddit ni { redde nullus; }")];
    let e = run_program_with_warnings(&files).unwrap_err();
    assert_eq!(e.to_string(), "Error: munus 'initium' in programmate non inventum");
}

#[test]
fn multi_file_error_names_file_and_line() {
    let files = [
        ("principale.liss",
         "affer mathematica;\nmunus initium reddit ni { redde mathematica.summa(°I, °II); }"),
        ("mathematica/summa.liss",
         "// prima linea\nmunus summa capit (ni a, ni b) reddit ni { redde \"salve\"; }"),
    ];
    let e = run_program_with_warnings(&files).unwrap_err();
    assert!(e.to_string().starts_with("Error in 'mathematica/summa.liss', linea 2:"),
        "got: {e}");
}

#[test]
fn exit_status_surfaces_through_the_pipeline() {
    let files = [("a.liss", "munus initium reddit ni { redde °V; }")];
    assert_eq!(run_program_with_warnings(&files).unwrap().1, 5);
}

#[test]
fn import_cycle_runs() {
    let files = [
        ("a.liss", "affer par;\nmunus initium reddit ni { scribe(par.aequum(°X)); redde nullus; }"),
        ("par/p.liss",
         "affer impar;\nmunus aequum capit (ni n) reddit ni { si (n par_est nullus) { redde °I; } redde impar.imparum(n - °I); }"),
        ("impar/i.liss",
         "affer par;\nmunus imparum capit (ni n) reddit ni { si (n par_est nullus) { redde nullus; } redde par.aequum(n - °I); }"),
    ];
    // 10 is even → aequum returns I
    assert_eq!(run_program_with_warnings(&files).unwrap().0, "I\n");
}

#[test]
fn non_latin_package_name_warns_but_runs() {
    let files = [
        ("principale.liss",
         "affer foobar;\nmunus initium reddit ni { scribe(foobar.quinque()); redde nullus; }"),
        ("foobar/x.liss", "munus quinque reddit ni { redde °V; }"),
    ];
    let (output, _, warnings) = run_program_with_warnings(&files).unwrap();
    assert_eq!(output, "V\n");
    assert!(warnings.iter().any(|w| w.message.contains("fasciculi 'foobar'")),
        "got: {warnings:?}");
}

#[test]
fn top_level_statement_is_now_an_error() {
    let e = run("scribe(°I);").unwrap_err();
    assert!(e.message.contains("nihil nisi declarationes"), "got: {}", e.message);
    assert_eq!(e.line, 1);
}

#[test]
fn single_file_program_still_needs_initium() {
    let e = run("munus f reddit ni { redde °I; }").unwrap_err();
    assert!(e.message.contains("initium"), "got: {}", e.message);
}
```

- [ ] **Step 4: Run the full suite**

Run: `nix develop -c cargo test`
Expected: all green. If an example test's output differs, the example rewrite is wrong — fix the example, not the expectation.

- [ ] **Step 5: Commit**

```bash
git add crates/lingua-serpentis/src/lib.rs crates/lingua-serpentis/tests/programs.rs examples/salve.liss examples/calc.liss examples/praecisio.liss examples/series.liss
git commit -m "feat: the package pipeline — run(), tests, and examples on initium

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>" -- crates/lingua-serpentis/src/lib.rs crates/lingua-serpentis/tests/programs.rs examples/salve.liss examples/calc.liss examples/praecisio.liss examples/series.liss
```

---

## Task 9: CLI — run folders, surface the exit status; folder example

**Files:**
- Modify: `crates/lingua-serpentis/src/main.rs`
- Create: `examples/computatrum/principale.liss`, `examples/computatrum/mathematica/summa.liss`, `examples/computatrum/mathematica/multiplicatio.liss`
- Test: `crates/lingua-serpentis/tests/programs.rs` (the example is exercised disk-free via `include_str!`)

**Interfaces:**
- Consumes: `run_dir_with_warnings`, `run_file_with_warnings`, `Warning` (Task 8).
- Produces: `lingua-serpentis <via.liss | directorium>`; stdout = program output, stderr = warnings/errors, process exit status = `initium`'s return (or 1 on any error).

- [ ] **Step 1: Write the failing test**

Append to `crates/lingua-serpentis/tests/programs.rs`:

```rust
#[test]
fn computatrum_example_runs() {
    let files = [
        ("principale.liss", include_str!("../../../examples/computatrum/principale.liss")),
        ("mathematica/summa.liss", include_str!("../../../examples/computatrum/mathematica/summa.liss")),
        ("mathematica/multiplicatio.liss", include_str!("../../../examples/computatrum/mathematica/multiplicatio.liss")),
    ];
    let (output, code, warnings) = run_program_with_warnings(&files).unwrap();
    assert_eq!(output, "XLII\nXLII\n");
    assert_eq!(code, 0);
    assert!(warnings.is_empty(), "unexpected warnings: {warnings:?}");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `nix develop -c cargo test -p lingua-serpentis computatrum`
Expected: COMPILE ERROR (`include_str!` files missing).

- [ ] **Step 3: Create the example**

`examples/computatrum/principale.liss`:

```
affer mathematica;

munus initium reddit ni {
    scribe(mathematica.summa(°XXV, °XVII));
    scribe(mathematica.duplum(°XXI));
    redde nullus;
}
```

`examples/computatrum/mathematica/summa.liss`:

```
munus summa capit (ni a, ni b) reddit ni {
    redde a + b;
}
```

`examples/computatrum/mathematica/multiplicatio.liss`:

```
munus duplum capit (ni n) reddit ni {
    redde n * °II;
}
```

- [ ] **Step 4: Rewrite `main.rs`**

```rust
use lingua_serpentis::{run_dir_with_warnings, run_file_with_warnings, Warning};
use std::path::Path;
use std::process::exit;

fn finish<E: std::fmt::Display>(result: Result<(String, u8, Vec<Warning>), E>) -> ! {
    match result {
        Ok((output, code, warnings)) => {
            // Warnings go to stderr so they never contaminate program output.
            for w in &warnings {
                eprintln!("{w}");
            }
            print!("{output}");
            exit(code as i32);
        }
        Err(e) => {
            eprintln!("{e}");
            exit(1);
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("Usus: lingua-serpentis <via.liss | directorium>");
        exit(1);
    }
    let path = Path::new(&args[1]);
    if path.is_dir() {
        finish(run_dir_with_warnings(path));
    }
    if !args[1].ends_with(".liss") {
        eprintln!("Error: file must have .liss extension");
        exit(1);
    }
    let src = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => { eprintln!("Error reading file: {e}"); exit(1); }
    };
    finish(run_file_with_warnings(&src));
}
```

- [ ] **Step 5: Run the full suite, then smoke the CLI**

Run: `nix develop -c cargo test`
Expected: all green.

Run: `nix develop -c cargo run -p lingua-serpentis -- examples/computatrum; echo "exitus: $?"`
Expected: `XLII` twice, then `exitus: 0`.

Run: `nix develop -c cargo run -p lingua-serpentis -- examples/salve.liss; echo "exitus: $?"`
Expected: `Salve, munde!`, then `exitus: 0`.

Run: `nix develop -c cargo run -p lingua-serpentis -- examples 2>&1 | head -1`
Expected: an `Error:` line (the `examples` folder itself has root `.liss` files but no `initium`… it has `salve.liss` etc. at top level and the `computatrum` folder nested — `computatrum` directly contains `.liss` files so it is read as a package and `computatrum/mathematica` violates the flat rule). Expected message contains `fasciculus in fasciculo non licet`. This is correct behavior — `examples/` is not a program folder.

- [ ] **Step 6: Commit**

```bash
git add crates/lingua-serpentis/src/main.rs crates/lingua-serpentis/tests/programs.rs examples/computatrum
git commit -m "feat(cli): run program folders; exit status from initium

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>" -- crates/lingua-serpentis/src/main.rs crates/lingua-serpentis/tests/programs.rs examples/computatrum
```

---

## Task 10: Documentation amendments

**Files:**
- Modify: `docs/superpowers/specs/2026-07-19-lingua-serpentis-rust-interpreter-design.md`
- Modify: `docs/superpowers/specs/2026-08-01-programs-and-packages-design.md`

- [ ] **Step 1: Mark the superseded script model**

In `2026-07-19-lingua-serpentis-rust-interpreter-design.md`, append below the existing amendment blockquotes at the top:

```markdown
> **Amended 2026-08-01:** the script-style program model is superseded — a
> program is now a folder (or single file) that starts at `munus initium
> reddit ni`, and top level allows only declarations, so §5.5's top-level
> statements and §5.9's example no longer run as written. §9's "Imports /
> modules" v2 candidate is realized as Go-style packages (*fasciculi*). See
> `2026-08-01-programs-and-packages-design.md`.
```

- [ ] **Step 2: Update the new spec's status**

In `2026-08-01-programs-and-packages-design.md`, change the Status line to:

```markdown
- **Status:** Approved — implemented (plan: `../plans/2026-08-01-programs-and-packages.md`)
```

- [ ] **Step 3: Full suite one last time**

Run: `nix develop -c cargo test`
Expected: all green.

- [ ] **Step 4: Commit**

```bash
git add docs/superpowers/specs/2026-07-19-lingua-serpentis-rust-interpreter-design.md docs/superpowers/specs/2026-08-01-programs-and-packages-design.md
git commit -m "docs: mark the script-model passages superseded by packages

Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>" -- docs/superpowers/specs/2026-07-19-lingua-serpentis-rust-interpreter-design.md docs/superpowers/specs/2026-08-01-programs-and-packages-design.md
```
