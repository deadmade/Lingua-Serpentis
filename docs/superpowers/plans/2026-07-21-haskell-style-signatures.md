# Separate Function Signatures Implementation Plan

> **Superseded by** `2026-07-22-capit-reddit-function-headers.md`. Kept as a
> historical record.

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace Lingua Serpentis's C-style inline function types with a standalone ML/Haskell-style signature line (`munus summare :: ni -> ni -> ni`).

**Architecture:** A syntax-layer change confined to the lexer and parser. `FuncDecl` in `ast.rs` already stores *resolved* types (`params: Vec<(Type, String)>`, `ret: Type`), so the parser simply fills those same fields from two syntactic sources instead of one. Nothing downstream can tell the difference — `checker.rs` and `interp.rs` are not modified at all.

**Tech Stack:** Rust 2021, cargo workspace. No new dependencies.

## Global Constraints

- **Run cargo through PowerShell on this machine, not Git Bash** — Git Bash hits a linker bug. Every command below is written for PowerShell.
- **Error messages are Latin**, matching the parser's existing terse style (`typus reditus expectatus`, `nomen expectatum`).
- **The Latin wording in this plan is a first draft.** Task 6 reviews it against the `latinitas` crate. Implement the messages verbatim as written, then let Task 6 revise.
- **No changes to `ast.rs`, `checker.rs`, or `interp.rs` source** (their inline `#[cfg(test)]` modules do change, in Task 3).
- **Arrow types are notation only.** `Type` stays a flat five-variant `Copy` enum. No currying, no function values.
- Spec: `docs/superpowers/specs/2026-07-21-haskell-style-signatures-design.md`

## File Structure

| File | Change |
|---|---|
| `crates/lingua-serpentis/src/token.rs` | Add `ColonColon` and `Arrow` variants |
| `crates/lingua-serpentis/src/lexer.rs` | Lex `::` and `->`; test module gains lexer tests |
| `crates/lingua-serpentis/src/parser.rs` | Rewrite `func_stmt`; one-line rescue in `statement`; test module rewritten/extended |
| `crates/lingua-serpentis/src/checker.rs` | **Test module only** — migrate 5 occurrences |
| `crates/lingua-serpentis/src/interp.rs` | **Test module only** — migrate 1 occurrence |
| `crates/lingua-serpentis/tests/programs.rs` | Migrate 1 occurrence; add end-to-end tests |
| `examples/calc.liss` | Migrate 1 function |
| `docs/superpowers/specs/2026-07-19-lingua-serpentis-rust-interpreter-design.md` | Amend two stale code samples in place |

## Task Ordering Note

**Task 3 is deliberately large and cannot be split.** It lands the breaking parser change *and* migrates all eight code call sites in one commit. Any commit that changes `func_stmt` without migrating leaves the suite red, so these must be atomic. Tasks 1, 2, 4, 5, and 6 are each independently green.

---

### Task 1: Lex `::` and `->`

**Files:**
- Modify: `crates/lingua-serpentis/src/token.rs:20-22`
- Modify: `crates/lingua-serpentis/src/lexer.rs:24-29`
- Test: `crates/lingua-serpentis/src/lexer.rs` (inline `mod tests`, from line 115)

**Interfaces:**
- Consumes: nothing.
- Produces: `TokenKind::ColonColon` and `TokenKind::Arrow`, used by `func_stmt` in Task 3.

This task is purely additive — no existing syntax changes meaning, so the suite stays green.

- [ ] **Step 1: Write the failing tests**

Add to the `mod tests` block in `crates/lingua-serpentis/src/lexer.rs` (after the existing `symbols_and_operators` test):

```rust
    #[test]
    fn signature_symbols() {
        assert_eq!(
            kinds(":: ->"),
            vec![TokenKind::ColonColon, TokenKind::Arrow]
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

    // `:` is a Roman-numeral body char (vinculum separator), so a numeral
    // butted directly against `::` swallows the colons. Unreachable through
    // valid syntax — `::` only ever follows an identifier — but pinned so the
    // behaviour is a decision rather than an accident.
    #[test]
    fn numeral_swallows_adjacent_colons() {
        assert!(tokenize("°X::ni").is_err());
    }
```

- [ ] **Step 2: Run tests to verify they fail**

```powershell
cargo test -p lingua-serpentis --lib lexer
```

Expected: FAIL — `no variant named ColonColon found for enum TokenKind` (compile error).

- [ ] **Step 3: Add the token variants**

In `crates/lingua-serpentis/src/token.rs`, change the symbol-operator and punctuation section:

```rust
    // symbol operators
    Assign, Plus, Minus, Star, Slash, Percent,
    // signature notation
    ColonColon, Arrow,                         // :: | ->
    // punctuation
    LParen, RParen, LBrace, RBrace, Semicolon, Comma,
```

- [ ] **Step 4: Lex the two symbols**

In `crates/lingua-serpentis/src/lexer.rs`, inside the `match c` block. The `->` arm **must** come before the plain `'-'` arm, mirroring how the `//` comment arm precedes `'/'`. Replace the existing `'-'` line with these three lines:

```rust
            '-' if chars.get(i + 1) == Some(&'>') => {
                tokens.push(Token { kind: TokenKind::Arrow, line }); i += 2;
            }
            ':' if chars.get(i + 1) == Some(&':') => {
                tokens.push(Token { kind: TokenKind::ColonColon, line }); i += 2;
            }
            '-' => { tokens.push(Token { kind: TokenKind::Minus, line }); i += 1; }
```

A lone `:` still falls through to the catch-all `signum ignotum` arm, which is what `lone_colon_is_still_unknown` asserts.

- [ ] **Step 5: Run tests to verify they pass**

```powershell
cargo test -p lingua-serpentis
```

Expected: PASS — all 52 pre-existing tests plus 4 new ones.

- [ ] **Step 6: Commit**

```powershell
git add crates/lingua-serpentis/src/token.rs crates/lingua-serpentis/src/lexer.rs
git commit -m "feat(lexer): add '::' and '->' tokens for function signatures"
```

---

### Task 2: Parse the signature form (new test, red)

**Files:**
- Test: `crates/lingua-serpentis/src/parser.rs` (inline `mod tests`, replacing `function_declaration_and_return` at lines 455-467)

**Interfaces:**
- Consumes: `TokenKind::ColonColon`, `TokenKind::Arrow` (Task 1).
- Produces: nothing new — asserts the existing `FuncDecl { name, params, ret, body }` shape is populated identically from the new syntax.

This task writes the red test only. Task 3 makes it green. Splitting the test from the implementation is deliberate: it proves the new syntax genuinely fails before the parser changes, so the test cannot silently pass for the wrong reason.

- [ ] **Step 1: Replace the existing function-declaration test**

In `crates/lingua-serpentis/src/parser.rs`, replace the whole `function_declaration_and_return` test (lines 455-467) with:

```rust
    #[test]
    fn function_declaration_and_return() {
        let s = parse_all("munus adde :: ni -> ni -> ni\nadde(a, b) { redde a + b; }");
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

    #[test]
    fn zero_argument_function() {
        let s = parse_all("munus duo :: ni\nduo() { redde °II; }");
        match &s[0].kind {
            StmtKind::Func(f) => {
                assert_eq!(f.name, "duo");
                assert_eq!(f.ret, Type::Int);
                assert!(f.params.is_empty());
            }
            other => panic!("expected func, got {:?}", other),
        }
    }

    #[test]
    fn mixed_parameter_types() {
        let s = parse_all("munus test :: voc -> ni -> veritas\ntest(v, n) { redde verus; }");
        match &s[0].kind {
            StmtKind::Func(f) => {
                assert_eq!(f.ret, Type::Bool);
                assert_eq!(f.params, vec![(Type::Text, "v".into()), (Type::Int, "n".into())]);
            }
            other => panic!("expected func, got {:?}", other),
        }
    }
```

- [ ] **Step 2: Run tests to verify they fail**

```powershell
cargo test -p lingua-serpentis --lib parser
```

Expected: FAIL — all three panic inside `parse_all`'s `.unwrap()`, because `func_stmt` currently wants a type keyword right after `munus` and errors with `typus reditus expectatus`.

- [ ] **Step 3: Commit the red test**

Committing a known-red test is intentional here — Task 3 is the atomic breaking change, and this commit records the target behaviour separately from the machinery that satisfies it.

```powershell
git add crates/lingua-serpentis/src/parser.rs
git commit -m "test(parser): specify signature-line syntax (red)"
```

---

### Task 3: Rewrite `func_stmt` and migrate every call site

**Files:**
- Modify: `crates/lingua-serpentis/src/parser.rs:229-247` (`func_stmt`)
- Modify: `crates/lingua-serpentis/src/checker.rs:285,290,295,300` (test module)
- Modify: `crates/lingua-serpentis/src/interp.rs:310` (test module)
- Modify: `crates/lingua-serpentis/tests/programs.rs:7`

**Interfaces:**
- Consumes: `TokenKind::ColonColon`, `TokenKind::Arrow` (Task 1); the red tests from Task 2.
- Produces: `func_stmt` accepting `'munus' IDENT '::' type ('->' type)* IDENT '(' [names] ')' block`, still returning `StmtKind::Func(FuncDecl)` unchanged.

**This task is atomic by necessity.** The moment `func_stmt` changes, every old-syntax test breaks. Implementation and migration land in one commit so the tree is never red.

- [ ] **Step 1: Rewrite `func_stmt`**

In `crates/lingua-serpentis/src/parser.rs`, replace the entire `func_stmt` function (lines 229-247) with:

```rust
    fn func_stmt(&mut self) -> LResult<Stmt> {
        let line = self.line();
        self.advance(); // munus

        // Old inline form `munus ni nomen(...)`: a type keyword sits where the
        // name belongs. Caught here so it gets real advice, not `nomen expectatum`.
        if matches!(
            self.peek(),
            Some(TokenKind::TyInt | TokenKind::TyFloat | TokenKind::TyText
                 | TokenKind::TyChar | TokenKind::TyBool)
        ) {
            return Err(self.err("typus in signatura declarandus, non in definitione"));
        }

        let sig_name = self.ident()?;
        self.eat(&TokenKind::ColonColon)?;

        // At least one type; the last is the return type, the rest are parameters.
        let mut types = vec![
            self.type_keyword().ok_or_else(|| self.err("typus post '::' expectatus"))?
        ];
        while self.peek() == Some(&TokenKind::Arrow) {
            self.advance();
            types.push(
                self.type_keyword().ok_or_else(|| self.err("typus post '->' expectatus"))?
            );
        }
        let ret = types.pop().expect("vec built with at least one element above");

        let def_name = self.ident()?;
        if def_name != sig_name {
            return Err(self.err(&format!(
                "nomen definitionis '{}' non congruit cum signatura '{}'",
                def_name, sig_name
            )));
        }

        self.eat(&TokenKind::LParen)?;
        let mut names = Vec::new();
        if self.peek() != Some(&TokenKind::RParen) {
            loop {
                names.push(self.ident()?);
                if self.peek() == Some(&TokenKind::Comma) { self.advance(); } else { break; }
            }
        }
        self.eat(&TokenKind::RParen)?;

        if names.len() != types.len() {
            return Err(self.err(&format!(
                "signatura {} parametros promittit, definitio {} habet",
                types.len(), names.len()
            )));
        }

        let params = types.into_iter().zip(names).collect();
        let body = self.block()?;
        Ok(Stmt { kind: StmtKind::Func(FuncDecl { name: sig_name, params, ret, body }), line })
    }
```

Note `types.pop()` happens *before* the arity check, so `types.len()` at that point is the parameter count, not the total.

`statement()` needs **no change** — `munus` still leads, so the existing `Some(TokenKind::Func) => return self.func_stmt()` dispatch stands and the grammar stays LL(1).

- [ ] **Step 2: Run the parser tests to verify Task 2's tests now pass**

```powershell
cargo test -p lingua-serpentis --lib parser
```

Expected: PASS for `function_declaration_and_return`, `zero_argument_function`, `mixed_parameter_types`.

- [ ] **Step 3: Run the full suite to see exactly what the breaking change broke**

```powershell
cargo test -p lingua-serpentis
```

Expected: FAIL — 6 tests in `checker`, `interp`, and `programs` still use the old syntax. This is the expected mid-task state; Steps 4-6 fix it.

- [ ] **Step 4: Migrate the `checker.rs` test module**

In `crates/lingua-serpentis/src/checker.rs`, replace lines 285, 290, 295, and 300 respectively:

```rust
        assert!(check_src("munus adde :: ni -> ni -> ni adde(a, b) { redde a + b; } scribe(adde(°I, °II));").is_ok());
```

```rust
        assert!(check_src("munus f :: ni f() { redde \"salve\"; }").is_err());
```

```rust
        assert!(check_src("munus f :: ni -> ni f(a) { redde a; } scribe(f(\"x\"));").is_err());
```

```rust
        let err = check_src("munus outer :: ni outer() { munus inner :: ni inner() { redde °I; } redde °I; }").unwrap_err();
```

The nested case still produces one `StmtKind::Func` inside another's body, so the checker's existing nesting rejection is untouched — which is exactly what this test now confirms.

- [ ] **Step 5: Migrate the `interp.rs` test module**

In `crates/lingua-serpentis/src/interp.rs`, replace line 310:

```rust
        let src = "munus adde :: ni -> ni -> ni adde(a, b) { redde a + b; } scribe(adde(°IV, °VI));";
```

- [ ] **Step 6: Migrate the integration test**

In `crates/lingua-serpentis/tests/programs.rs`, replace the `src` literal in `full_program_with_function_and_if` (lines 6-14):

```rust
    let src = r#"
        munus summare :: ni -> ni -> ni
        summare(a, b) {
            ni c = a + b;
            redde c;
        }
        ni summa = summare(°XXV, °XXX.M.V.C.VII);
        si (summa minor_est °C) { summa = summa + °C; }
        scribe(summa);
    "#;
```

- [ ] **Step 7: Run the full suite to verify green**

```powershell
cargo test -p lingua-serpentis
```

Expected: PASS — all tests green, no old-syntax call sites remain in code.

- [ ] **Step 8: Commit**

```powershell
git add crates/lingua-serpentis/src/parser.rs crates/lingua-serpentis/src/checker.rs crates/lingua-serpentis/src/interp.rs crates/lingua-serpentis/tests/programs.rs
git commit -m "feat(parser)!: move function types to a standalone signature line

BREAKING: 'munus ni f(ni a)' is replaced by 'munus f :: ni -> ni' followed
by 'f(a) { ... }'. FuncDecl is unchanged, so checker and interpreter are
untouched; only their test fixtures move to the new syntax."
```

---

### Task 4: Signature/definition disagreement errors

**Files:**
- Test: `crates/lingua-serpentis/src/parser.rs` (inline `mod tests`)

**Interfaces:**
- Consumes: `func_stmt` from Task 3.
- Produces: nothing — pins the error behaviour Task 3 already implements.

The error paths were written in Task 3 but nothing exercises them yet. This task adds the tests. If any fails, fix `func_stmt`, not the test.

- [ ] **Step 1: Add a parse-error helper and the error tests**

Add to `mod tests` in `crates/lingua-serpentis/src/parser.rs`, next to the existing `parse_all` helper:

```rust
    fn parse_err(src: &str) -> String {
        parse(&tokenize(src).unwrap())
            .expect_err("expected a parse error")
            .message
    }

    #[test]
    fn signature_name_must_match_definition() {
        let m = parse_err("munus adde :: ni -> ni\nsubtrahe(a) { redde a; }");
        assert!(m.contains("non congruit"), "got: {m}");
    }

    #[test]
    fn too_few_parameters_rejected() {
        let m = parse_err("munus adde :: ni -> ni -> ni\nadde(a) { redde a; }");
        assert!(m.contains("2 parametros promittit"), "got: {m}");
        assert!(m.contains("1 habet"), "got: {m}");
    }

    #[test]
    fn too_many_parameters_rejected() {
        let m = parse_err("munus adde :: ni -> ni\nadde(a, b) { redde a; }");
        assert!(m.contains("1 parametros promittit"), "got: {m}");
        assert!(m.contains("2 habet"), "got: {m}");
    }

    #[test]
    fn dangling_arrow_rejected() {
        let m = parse_err("munus adde :: ni ->\nadde(a) { redde a; }");
        assert!(m.contains("post '->'"), "got: {m}");
    }

    #[test]
    fn dangling_colon_colon_rejected() {
        let m = parse_err("munus adde ::\nadde(a) { redde a; }");
        assert!(m.contains("post '::'"), "got: {m}");
    }

    // Load-bearing: the old syntax was removed deliberately. Without this,
    // a future change could quietly restore a dual-syntax parser.
    #[test]
    fn old_inline_form_is_rejected() {
        let m = parse_err("munus ni adde(ni a, ni b) { redde a + b; }");
        assert!(m.contains("in signatura declarandus"), "got: {m}");
    }
```

- [ ] **Step 2: Run tests**

```powershell
cargo test -p lingua-serpentis --lib parser
```

Expected: PASS — all six. `dangling_arrow_rejected` is the one most likely to surprise: `->` followed by a newline then `adde` means `type_keyword()` returns `None` on the `Ident`, producing the `typus post '->' expectatus` error rather than a confusing downstream failure.

- [ ] **Step 3: Commit**

```powershell
git add crates/lingua-serpentis/src/parser.rs
git commit -m "test(parser): pin signature/definition disagreement errors"
```

---

### Task 5: Rescue the missing-signature error message

**Files:**
- Modify: `crates/lingua-serpentis/src/parser.rs:301-305` (the fall-through arm of `statement`)
- Test: `crates/lingua-serpentis/src/parser.rs` (inline `mod tests`)

**Interfaces:**
- Consumes: `parse_err` helper from Task 4.
- Produces: nothing.

A definition written without its signature — `adde(a, b) { ... }` — parses cleanly as a *call* expression (bare identifiers are valid expressions), then fails wanting `;` where `{` sits. The default `expectatum: Semicolon` points at the right line with useless advice.

- [ ] **Step 1: Write the failing test**

Add to `mod tests` in `crates/lingua-serpentis/src/parser.rs`:

```rust
    #[test]
    fn definition_without_signature_gets_real_advice() {
        let m = parse_err("adde(a, b) { redde a + b; }");
        assert!(m.contains("signatura ante definitionem"), "got: {m}");
    }
```

- [ ] **Step 2: Run test to verify it fails**

```powershell
cargo test -p lingua-serpentis --lib definition_without_signature
```

Expected: FAIL — message is `expectatum: Semicolon`, not the advice.

- [ ] **Step 3: Add the rescue**

In `crates/lingua-serpentis/src/parser.rs`, replace the fall-through arm at the end of `statement()` (lines 301-305):

```rust
            _ => {
                let e = self.expr()?;
                // A call followed by `{` is a function definition whose
                // signature line is missing — the only way that shape arises.
                if matches!(e, Expr::Call { .. }) && self.peek() == Some(&TokenKind::LBrace) {
                    return Err(self.err("signatura ante definitionem expectata"));
                }
                self.eat(&TokenKind::Semicolon)?;
                Ok(Stmt { kind: StmtKind::ExprStmt(e), line })
            }
```

The `Expr::Call` guard keeps this precise — a non-call expression followed by `{` is a different mistake and keeps its existing message.

- [ ] **Step 4: Run the full suite**

```powershell
cargo test -p lingua-serpentis
```

Expected: PASS — including every test from Tasks 1-4.

- [ ] **Step 5: Commit**

```powershell
git add crates/lingua-serpentis/src/parser.rs
git commit -m "feat(parser): advise on a missing signature line"
```

---

### Task 6: Migrate examples and docs, review the Latin

**Files:**
- Modify: `examples/calc.liss`
- Modify: `docs/superpowers/specs/2026-07-19-lingua-serpentis-rust-interpreter-design.md:172-175,194-197`
- Test: `crates/lingua-serpentis/tests/programs.rs`

**Interfaces:**
- Consumes: the finished parser from Tasks 1-5.
- Produces: nothing.

- [ ] **Step 1: Write a failing test that runs the example file**

`examples/calc.liss` is currently verified by nobody. Add to `crates/lingua-serpentis/tests/programs.rs`:

```rust
#[test]
fn calc_example_still_runs() {
    let src = include_str!("../../../examples/calc.liss");
    assert_eq!(run(src).unwrap(), "30532\n");
}
```

- [ ] **Step 2: Run it to verify it fails**

```powershell
cargo test -p lingua-serpentis --test programs calc_example
```

Expected: FAIL — `calc.liss` still uses the old inline syntax, so it errors with `typus in signatura declarandus, non in definitione`.

- [ ] **Step 3: Migrate `examples/calc.liss`**

Replace the whole file:

```
munus summare :: ni -> ni -> ni
summare(a, b) {
    ni c = a + b;
    redde c;
}
ni summa = summare(°XXV, °XXX.M.V.C.VII);
si (summa minor_est °C) { summa = summa + °C; }
scribe(summa);
```

- [ ] **Step 4: Run to verify it passes**

```powershell
cargo test -p lingua-serpentis
```

Expected: PASS — full suite green.

- [ ] **Step 5: Amend the two stale code samples in the interpreter spec**

`docs/superpowers/specs/2026-07-19-lingua-serpentis-rust-interpreter-design.md` contains two **identical** samples, at lines 172-175 and 194-197. Both currently read:

```
munus ni summare(ni a, ni b) {
    ni c = a + b;
    redde c;
}
```

Replace both with:

```
munus summare :: ni -> ni -> ni
summare(a, b) {
    ni c = a + b;
    redde c;
}
```

**Do not rewrite the prose or restructure the document** — it is a historical record of shipped work, and only these samples are stale. Add one line under its `**Status:**` field:

```markdown
- **Amended:** 2026-07-21 — the two code samples were updated for the signature
  syntax introduced in `2026-07-21-haskell-style-signatures-design.md`. Prose
  unchanged.
```

`docs/superpowers/specs/2026-07-19-latin-identifier-checker-design.md` needs **no change** — it names `munus` only in prose (line 9), as an example of a Latin keyword, which is still accurate. Do not touch it.

- [ ] **Step 6: Review the Latin against `latinitas`**

Every message introduced by this plan is a first-draft guess. Check each noun and verb form against the `latinitas` crate's Whitaker-backed dictionary and correct any that are wrong, updating both the source string and the test that asserts on it:

| Message | Where |
|---|---|
| `typus in signatura declarandus, non in definitione` | `parser.rs` `func_stmt` |
| `nomen definitionis '{}' non congruit cum signatura '{}'` | `parser.rs` `func_stmt` |
| `signatura {} parametros promittit, definitio {} habet` | `parser.rs` `func_stmt` |
| `typus post '::' expectatus` | `parser.rs` `func_stmt` |
| `typus post '->' expectatus` | `parser.rs` `func_stmt` |
| `signatura ante definitionem expectata` | `parser.rs` `statement` |

Note that Task 4's assertions match on substrings (`non congruit`, `post '->'`) precisely so small wording changes don't cascade — but any change to those exact fragments does require updating the test.

- [ ] **Step 7: Run the full suite one last time**

```powershell
cargo test
```

Expected: PASS — the whole workspace, both crates.

- [ ] **Step 8: Commit**

```powershell
git add examples/calc.liss crates/lingua-serpentis/tests/programs.rs docs/superpowers/specs/ crates/lingua-serpentis/src/parser.rs
git commit -m "docs: migrate examples and specs to the signature syntax"
```

---

## Verification

After Task 6, confirm the whole feature end to end:

```powershell
cargo test
cargo run -p lingua-serpentis -- examples/calc.liss
```

Expected: all tests pass; the example prints `30532`.

## Out of Scope

Recorded in the spec's §9, deliberately not built here: signature without definition (forward declaration), real arrow types with currying and function values, and type inference.
