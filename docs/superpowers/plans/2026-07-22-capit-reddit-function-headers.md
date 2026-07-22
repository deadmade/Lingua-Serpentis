# `capit` / `reddit` Function Headers Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the Haskell-style `munus summare :: ni -> ni -> ni` signature with a single-line Latin header framed by two verbs — `munus summare capit (ni a, ni b) reddit ni { … }`.

**Architecture:** A syntax-layer change confined to the lexer and parser. `FuncDecl` in `ast.rs` already stores *resolved* types (`params: Vec<(Type, String)>`, `ret: Type`), so the parser fills those same fields from the new syntax. `checker.rs` and `interp.rs` are not modified. The change is breaking, so the parser rewrite and the migration of every program-text call site land in one commit (Task 2).

**Tech Stack:** Rust (cargo workspace, two crates: `lingua-serpentis`, `latinitas`). No new dependencies.

**Spec:** `docs/superpowers/specs/2026-07-22-capit-reddit-function-headers-design.md`

## Global Constraints

- **New grammar:** `func := 'munus' IDENT [ 'capit' '(' params ')' ] 'reddit' type block`, where `params := type IDENT (',' type IDENT)*` (one or more). `capit` is present **iff** the function has parameters; a zero-argument function writes neither `capit` nor `()`. A `capit ()` with empty parentheses is a parse error. `reddit` and the return type are always present.
- **No runtime-logic changes outside the lexer and parser.** Production changes are confined to `lexer.rs`, `token.rs`, and `parser.rs` (`func_stmt`). `ast.rs`, `checker.rs`, and `interp.rs` keep every struct field, enum variant, and line of logic exactly as-is — their only permitted edits are the `ast.rs` `def_line` doc-comment and migrated **test-program strings** inside `#[cfg(test)]` modules. `latinity.rs` changes only one comment plus its test-program strings. Test files (`tests/programs.rs` and the inline test modules) and `examples/calc.liss` are migrated to the new syntax; that is test data, not logic.
- **Error messages** are terse Latin, mirroring the parser's existing style (`nomen expectatum`, `typus … expectatus`, `parametra`). Exact strings are given per step; do not paraphrase them — tests assert on substrings.
- **Do not touch the Cargo manifests.** `Cargo.toml`, `crates/latinitas/Cargo.toml`, and `crates/lingua-serpentis/Cargo.toml` carry unrelated uncommitted edition-drift edits. Stage only the files each task's commit names — never `git add -A`.
- **Running tests (this machine):** `cargo` is not on `PATH` and the MSVC linker must be imported first. Run every cargo command through this wrapper:
  ```
  cmd /c 'call "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat" && "%USERPROFILE%\.cargo\bin\cargo.exe" test --workspace'
  ```
  Wherever a step says `cargo test …`, run it through that wrapper. Baseline before starting: **117 tests pass** (35 `latinitas` + 75 `lingua-serpentis` lib + 7 integration). Only the `lingua-serpentis` lib count moves in this plan; the 35 `latinitas` and 7 integration tests are unaffected in count.

---

## Task 1: Add `capit` / `reddit` keyword tokens (lexer, additive)

Purely additive: the two new keywords start tokenizing, the old `::`/`->` notation still works, and every existing test stays green. This isolates the low-risk lexer change from the breaking parser change in Task 2.

**Files:**
- Modify: `crates/lingua-serpentis/src/token.rs:15`
- Modify: `crates/lingua-serpentis/src/lexer.rs` (keyword table `word_to_kind`, and the lexer test module)

**Interfaces:**
- Produces: `TokenKind::Takes` (from the word `capit`) and `TokenKind::Yields` (from the word `reddit`), consumed by Task 2's parser.

- [ ] **Step 1: Add the two token variants**

In `crates/lingua-serpentis/src/token.rs`, extend the keyword line (currently line 15):

```rust
    // keywords
    If, Else, While, For, Func, Return, Break, Continue, Print,
    Takes, Yields,                             // capit | reddit
```

- [ ] **Step 2: Map the two keywords in the lexer**

In `crates/lingua-serpentis/src/lexer.rs`, add two arms to `word_to_kind`, next to the other keywords (after the `"scribe" => TokenKind::Print,` line):

```rust
        "scribe" => TokenKind::Print,
        "capit" => TokenKind::Takes,
        "reddit" => TokenKind::Yields,
```

- [ ] **Step 3: Write the lexer test**

Add this test inside the `#[cfg(test)] mod tests` block in `crates/lingua-serpentis/src/lexer.rs`:

```rust
    #[test]
    fn header_keywords() {
        assert_eq!(kinds("capit reddit"), vec![TokenKind::Takes, TokenKind::Yields]);
        // A longer word that merely starts with a header keyword stays one ident.
        assert_eq!(kinds("capitulum"), vec![TokenKind::Ident("capitulum".into())]);
        assert_eq!(kinds("redditus"), vec![TokenKind::Ident("redditus".into())]);
    }
```

- [ ] **Step 4: Run the tests**

Run: `cargo test --workspace`
Expected: PASS — 118 tests pass (117 plus `header_keywords`). Nothing regresses because the change is additive.

- [ ] **Step 5: Commit**

```bash
git add crates/lingua-serpentis/src/token.rs crates/lingua-serpentis/src/lexer.rs
git commit -m "feat: lex capit and reddit as keywords

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

## Task 2: Switch the parser to the `capit` / `reddit` grammar and migrate all program text

The breaking change. First migrate every test and example to the new syntax (turning the suite red under the old parser), then rewrite `func_stmt` (turning it green). One commit.

**Files:**
- Modify: `crates/lingua-serpentis/src/parser.rs` (`func_stmt` at lines 229–292, and the test module)
- Modify: `crates/lingua-serpentis/src/checker.rs:285,290,295,300` (test programs)
- Modify: `crates/lingua-serpentis/src/interp.rs:310` (test program)
- Modify: `crates/lingua-serpentis/src/latinity.rs` (three test programs + one comment)
- Modify: `crates/lingua-serpentis/src/ast.rs:29-33` (the `def_line` doc-comment)
- Modify: `crates/lingua-serpentis/tests/programs.rs:6-18`
- Modify: `examples/calc.liss`

**Interfaces:**
- Consumes: `TokenKind::Takes`, `TokenKind::Yields` (Task 1); the existing `self.type_keyword() -> Option<Type>`, `self.ident() -> LResult<String>`, `self.eat(&TokenKind) -> LResult<()>`, `self.err(&str) -> LinguaError`, `self.peek()`, `self.advance()`, `self.line()`, `self.block()`.
- Produces: a `StmtKind::Func(FuncDecl { name, params, ret, body, def_line })` identical in shape to today's — so nothing downstream changes.

- [ ] **Step 1: Migrate the three positive parser tests**

In `crates/lingua-serpentis/src/parser.rs`, replace the source strings in these three tests (assertions unchanged). `function_declaration_and_return`:

```rust
    #[test]
    fn function_declaration_and_return() {
        let s = parse_all("munus adde capit (ni a, ni b) reddit ni { redde a + b; }");
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

`zero_argument_function`:

```rust
    #[test]
    fn zero_argument_function() {
        let s = parse_all("munus duo reddit ni { redde °II; }");
        match &s[0].kind {
            StmtKind::Func(f) => {
                assert_eq!(f.name, "duo");
                assert_eq!(f.ret, Type::Int);
                assert!(f.params.is_empty());
            }
            other => panic!("expected func, got {:?}", other),
        }
    }
```

`mixed_parameter_types`:

```rust
    #[test]
    fn mixed_parameter_types() {
        let s = parse_all("munus test capit (voc v, ni n) reddit veritas { redde verus; }");
        match &s[0].kind {
            StmtKind::Func(f) => {
                assert_eq!(f.ret, Type::Bool);
                assert_eq!(f.params, vec![(Type::Text, "v".into()), (Type::Int, "n".into())]);
            }
            other => panic!("expected func, got {:?}", other),
        }
    }
```

- [ ] **Step 2: Delete the five obsolete error tests**

In `crates/lingua-serpentis/src/parser.rs`, delete these five test functions entirely — they test errors that no longer exist (no repeated name, one param list, no arrows, no `::`):

- `signature_name_must_match_definition`
- `too_few_parameters_rejected`
- `too_many_parameters_rejected`
- `dangling_arrow_rejected`
- `dangling_colon_colon_rejected`

Leave `old_inline_form_is_rejected` and `definition_without_signature_gets_real_advice` in place (they are handled in Steps 3–4).

- [ ] **Step 3: Add the new error tests**

In `crates/lingua-serpentis/src/parser.rs`, add these five tests where the deleted block was:

```rust
    #[test]
    fn empty_capit_is_rejected() {
        // With no parameters, `capit ()` must be omitted entirely.
        let m = parse_err("munus f capit () reddit ni { redde °I; }");
        assert!(m.contains("sine parametris"), "got: {m}");
    }

    #[test]
    fn parameters_without_capit_are_rejected() {
        let m = parse_err("munus f(ni a) reddit ni { redde a; }");
        assert!(m.contains("ante parametra"), "got: {m}");
    }

    #[test]
    fn missing_reddit_is_rejected() {
        let m = parse_err("munus f capit (ni a) { redde a; }");
        assert!(m.contains("typus reditus"), "got: {m}");
    }

    #[test]
    fn missing_return_type_is_rejected() {
        let m = parse_err("munus f reddit { redde °I; }");
        assert!(m.contains("post 'reddit'"), "got: {m}");
    }

    #[test]
    fn parameter_without_a_type_is_rejected() {
        let m = parse_err("munus f capit (a) reddit ni { redde °I; }");
        assert!(m.contains("typus parametri"), "got: {m}");
    }
```

- [ ] **Step 4: Update the old-inline-form test's expected message**

In `crates/lingua-serpentis/src/parser.rs`, `old_inline_form_is_rejected` currently asserts `m.contains("in signatura declarandus")`. Change it to the new message and update the comment:

```rust
    // Load-bearing: the old syntaxes were removed deliberately. Without this,
    // a future change could quietly restore a dual-syntax parser.
    #[test]
    fn old_inline_form_is_rejected() {
        let m = parse_err("munus ni adde(ni a, ni b) { redde a + b; }");
        assert!(m.contains("ante nomen"), "got: {m}");
    }
```

- [ ] **Step 5: Migrate the checker test programs**

In `crates/lingua-serpentis/src/checker.rs`, replace the four test source strings (assertions unchanged):

```rust
    #[test]
    fn function_return_type_checked() {
        assert!(check_src("munus adde capit (ni a, ni b) reddit ni { redde a + b; } scribe(adde(°I, °II));").is_ok());
    }

    #[test]
    fn returning_wrong_type_fails() {
        assert!(check_src("munus f reddit ni { redde \"salve\"; }").is_err());
    }

    #[test]
    fn calling_with_wrong_arg_type_fails() {
        assert!(check_src("munus f capit (ni a) reddit ni { redde a; } scribe(f(\"x\"));").is_err());
    }

    #[test]
    fn nested_function_rejected() {
        let err = check_src("munus outer reddit ni { munus inner reddit ni { redde °I; } redde °I; }").unwrap_err();
        assert!(err.message.contains("nidificat"));
    }
```

- [ ] **Step 6: Migrate the interpreter test program**

In `crates/lingua-serpentis/src/interp.rs`, `function_call_returns`:

```rust
    #[test]
    fn function_call_returns() {
        let src = "munus adde capit (ni a, ni b) reddit ni { redde a + b; } scribe(adde(°IV, °VI));";
        assert_eq!(run(src), "10\n");
    }
```

- [ ] **Step 7: Migrate the integration test and the example**

In `crates/lingua-serpentis/tests/programs.rs`, `full_program_with_function_and_if`:

```rust
#[test]
fn full_program_with_function_and_if() {
    let src = r#"
        munus summare capit (ni a, ni b) reddit ni {
            ni c = a + b;
            redde c;
        }
        ni summa = summare(°XXV, °XXX.M.V.C.VII);
        si (summa minor_est °C) { summa = summa + °C; }
        scribe(summa);
    "#;
    assert_eq!(run(src).unwrap(), "30532\n"); // 25 + 30507 = 30532 (not < 100, so unchanged)
}
```

Replace the entire contents of `examples/calc.liss` with:

```
munus summare capit (ni a, ni b) reddit ni {
    ni c = a + b;
    redde c;
}
ni summa = summare(°XXV, °XXX.M.V.C.VII);
si (summa minor_est °C) { summa = summa + °C; }
scribe(summa);
```

- [ ] **Step 8: Migrate the latinity test programs**

In `crates/lingua-serpentis/src/latinity.rs`, `flags_a_non_latin_function_name_and_parameter` — replace only the source string (line 103), keeping its comment and assertions:

```rust
        let w = lint_src("munus foobar capit (ni widget) reddit ni { redde widget; }");
```

`reaches_declarations_inside_function_bodies` (line 149):

```rust
        let w = lint_src("munus summa reddit ni { ni result = °V; redde result; }");
```

`parameters_are_reported_on_the_definition_line` — rewrite the comment and the multi-line source so `munus foobar` is line 1 and the `capit (...)` clause (where the parameter is written) is line 2. Assertions unchanged:

```rust
    #[test]
    fn parameters_are_reported_on_the_definition_line() {
        // The header may be split across lines: `munus foobar` on line 1 and the
        // `capit (...)` clause — where the parameter is actually written — on
        // line 2. Reporting the parameter at the enclosing `Stmt.line` would
        // blame line 1. Every other lint test keeps the header on one line,
        // where the two coincide and the bug would be invisible.
        let src = "munus foobar\n\
                   capit (ni widget) reddit ni {\n\
                   ni result = widget;\n\
                   redde result;\n\
                   }";
        let w = lint_src(src);

        let param = w.iter().find(|x| x.message.contains("widget"))
            .expect("expected a warning for the parameter 'widget'");
        assert_eq!(param.line, 2, "parameter must be reported on the line it is written");

        // The function name still belongs to the `munus` line, and the body
        // declaration to its own line — so this really is pinning the parameter,
        // not a blanket line shift.
        let name = w.iter().find(|x| x.message.contains("foobar"))
            .expect("expected a warning for the function name 'foobar'");
        assert_eq!(name.line, 1);
        let local = w.iter().find(|x| x.message.contains("result"))
            .expect("expected a warning for the local 'result'");
        assert_eq!(local.line, 3);
    }
```

- [ ] **Step 9: Run the tests to confirm they now fail**

Run: `cargo test --workspace`
Expected: FAIL. The migrated programs use `capit`/`reddit`, which the old `func_stmt` cannot parse (after reading the name it tries to `eat` `::` and hits `capit`), so positive tests panic on `.unwrap()` and negative tests see the wrong message. This red state confirms the tests exercise the new syntax before the parser supports it.

- [ ] **Step 10: Rewrite `func_stmt`**

In `crates/lingua-serpentis/src/parser.rs`, replace the whole `func_stmt` function (lines 229–292) with:

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
            return Err(self.err("typus reditus post 'reddit' ponendus, non ante nomen"));
        }

        let name = self.ident()?;

        // `capit (...)` introduces a non-empty parameter clause; its absence
        // means the function takes nothing. `def_line` marks where the
        // parameters are written, which for a header split across lines differs
        // from the `munus` line.
        let def_line = self.line();
        let mut params: Vec<(Type, String)> = Vec::new();
        if self.peek() == Some(&TokenKind::Takes) {
            self.advance(); // capit
            self.eat(&TokenKind::LParen)?;
            if self.peek() == Some(&TokenKind::RParen) {
                return Err(self.err("munus sine parametris 'capit' omittat"));
            }
            loop {
                let ty = self.type_keyword()
                    .ok_or_else(|| self.err("typus parametri expectatus"))?;
                let pname = self.ident()?;
                params.push((ty, pname));
                if self.peek() == Some(&TokenKind::Comma) { self.advance(); } else { break; }
            }
            self.eat(&TokenKind::RParen)?;
        } else if self.peek() == Some(&TokenKind::LParen) {
            return Err(self.err("'capit' ante parametra expectatum"));
        }

        // `reddit <type>` — always present, even for an `ni` return.
        if self.peek() != Some(&TokenKind::Yields) {
            return Err(self.err("'reddit' et typus reditus expectati"));
        }
        self.advance(); // reddit
        let ret = self.type_keyword()
            .ok_or_else(|| self.err("typus post 'reddit' expectatus"))?;

        let body = self.block()?;
        Ok(Stmt {
            kind: StmtKind::Func(FuncDecl { name, params, ret, body, def_line }),
            line,
        })
    }
```

- [ ] **Step 11: Update the two stale comments**

In `crates/lingua-serpentis/src/ast.rs`, replace the `def_line` doc-comment (lines 29–32):

```rust
    /// Line where the parameters are written. Usually the same as the enclosing
    /// `Stmt.line`, but a header split across lines (`munus` on one line, the
    /// `capit (...)` clause on the next) puts the parameters lower, and anything
    /// pointing at a parameter must use this instead.
    pub def_line: usize,
```

In `crates/lingua-serpentis/src/latinity.rs`, replace the comment inside the `StmtKind::Func` arm (lines 39–41):

```rust
                // Parameters are written in the `capit (...)` clause, which for a
                // header split across lines sits below the `munus`/name line;
                // report them at `def_line`, not the enclosing `Stmt.line`.
```

- [ ] **Step 12: Run the tests to confirm green**

Run: `cargo test --workspace`
Expected: PASS — 118 tests pass (unchanged total: the 5 deleted error tests are balanced by the 5 new ones, on top of Task 1's `header_keywords`). Every migrated program now parses and runs; `calc_example_still_runs` and `full_program_with_function_and_if` both print `30532`.

- [ ] **Step 13: Commit**

```bash
git add crates/lingua-serpentis/src/parser.rs crates/lingua-serpentis/src/checker.rs crates/lingua-serpentis/src/interp.rs crates/lingua-serpentis/src/latinity.rs crates/lingua-serpentis/src/ast.rs crates/lingua-serpentis/tests/programs.rs examples/calc.liss
git commit -m "feat: switch function headers to capit/reddit syntax

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

## Task 3: Remove the retired `::` / `->` notation

Nothing references `TokenKind::ColonColon` or `TokenKind::Arrow` after Task 2, so delete them and their lexer arms. `::` and `->` now fail at the lexer as `signum ignotum`.

**Files:**
- Modify: `crates/lingua-serpentis/src/lexer.rs` (remove two arms; adjust the test module)
- Modify: `crates/lingua-serpentis/src/token.rs:22` (remove two variants)

- [ ] **Step 1: Remove the two lexer arms**

In `crates/lingua-serpentis/src/lexer.rs`, delete the `->` and `::` arms (currently lines 25–30), so the `'+'` arm is followed directly by the `'-'` arm:

```rust
            '+' => { tokens.push(Token { kind: TokenKind::Plus, line }); i += 1; }
            '-' => { tokens.push(Token { kind: TokenKind::Minus, line }); i += 1; }
            '*' => { tokens.push(Token { kind: TokenKind::Star, line }); i += 1; }
```

- [ ] **Step 2: Remove the two token variants**

In `crates/lingua-serpentis/src/token.rs`, delete the signature-notation line (currently line 22):

```rust
    // signature notation
    ColonColon, Arrow,                         // :: | ->
```

Remove both the comment and the variants.

- [ ] **Step 3: Replace the retired lexer tests**

In `crates/lingua-serpentis/src/lexer.rs`, delete the `signature_symbols` test and the `numeral_swallows_adjacent_colons` test (with its comment). Add this in their place:

```rust
    #[test]
    fn retired_signature_symbols_no_longer_tokenize() {
        // `::` and `->` were the old function-signature notation; both are gone,
        // so each now fails as an unknown symbol.
        assert!(tokenize("::").is_err());
        assert!(tokenize("->").is_err());
    }
```

Leave `arrow_does_not_swallow_plain_minus` and `lone_colon_is_still_unknown` unchanged — both still pass and still test valid behavior.

- [ ] **Step 4: Run the tests**

Run: `cargo test --workspace`
Expected: PASS — 117 tests pass (118 from Task 2 − `signature_symbols` − `numeral_swallows_adjacent_colons` + `retired_signature_symbols_no_longer_tokenize`). A compile error naming `ColonColon` or `Arrow` here means a reference was missed — grep for both across `crates/lingua-serpentis/src` and remove it.

- [ ] **Step 5: Commit**

```bash
git add crates/lingua-serpentis/src/lexer.rs crates/lingua-serpentis/src/token.rs
git commit -m "refactor: drop the retired :: and -> signature tokens

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

## Task 4: Update documentation

Doc-only. No code or tests change.

**Files:**
- Modify: `docs/superpowers/specs/2026-07-19-lingua-serpentis-rust-interpreter-design.md` (two code samples)
- Modify: `docs/superpowers/specs/2026-07-21-haskell-style-signatures-design.md` (superseded banner)
- Modify: `docs/superpowers/plans/2026-07-21-haskell-style-signatures.md` (superseded banner)

- [ ] **Step 1: Re-amend the interpreter-design samples**

In `docs/superpowers/specs/2026-07-19-lingua-serpentis-rust-interpreter-design.md`, the §5.6 sample (lines 175–179) and the §5.9 sample (lines 198–202) are identical. Replace each occurrence of:

```
munus summare :: ni -> ni -> ni
summare(a, b) {
    ni c = a + b;
    redde c;
}
```

with:

```
munus summare capit (ni a, ni b) reddit ni {
    ni c = a + b;
    redde c;
}
```

Amend the samples in place; do not rewrite the surrounding prose.

- [ ] **Step 2: Mark the 2026-07-21 design doc superseded**

In `docs/superpowers/specs/2026-07-21-haskell-style-signatures-design.md`, add a banner immediately under the `# …` title line:

```markdown
> **Superseded by** `2026-07-22-capit-reddit-function-headers-design.md`. The `::`
> / `->` signature described here was replaced by the `capit` / `reddit` header
> before it reached production. Kept as a historical record.
```

- [ ] **Step 3: Mark the 2026-07-21 plan superseded**

In `docs/superpowers/plans/2026-07-21-haskell-style-signatures.md`, add the same banner under its title line:

```markdown
> **Superseded by** `2026-07-22-capit-reddit-function-headers.md`. Kept as a
> historical record.
```

- [ ] **Step 4: Commit**

```bash
git add docs/superpowers/specs/2026-07-19-lingua-serpentis-rust-interpreter-design.md docs/superpowers/specs/2026-07-21-haskell-style-signatures-design.md docs/superpowers/plans/2026-07-21-haskell-style-signatures.md
git commit -m "docs: update samples for capit/reddit; mark Haskell-signature docs superseded

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

## Notes for the implementer

- **README needs no change** — its `->` characters are German prose arrows ("Lazy Way -> Replace"), not function-header syntax. Verified during planning.
- **`statement()` is intentionally not touched.** Its call-followed-by-`{` guard still emits `signatura ante definitionem expectata`, and the `definition_without_signature_gets_real_advice` test still passes. This is per the spec (§4.3: "statement() needs no change").
- **The Latin error strings** were chosen to match the parser's existing vocabulary: `parametra` (neuter plural, accusative after `ante`) and `parametris` (ablative after `sine`) mirror the old `parametra promittit`; `typus … expectatus/expectati` mirror existing messages. If any word later reads wrong to the author, it is a one-line string change with a matching test-substring update.
```
