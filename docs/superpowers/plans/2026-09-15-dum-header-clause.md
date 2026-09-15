# `dum` Header Clause Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development
> (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use
> checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make a `dum` init clause a declaration or an assignment and nothing else, rejected in the
parser with `clausula prima capitis 'dum' declaratio vel assignatio esse debet`, closing D-26 and the
`munus`-in-header hole, then delete the code and tests that only existed for the undecided shape.

**Architecture:** One lookahead guard in `Parser::for_stmt` before it calls `statement()`. Nothing
downstream changes shape: the AST still holds a `Let` or an `Assign`. The checker's `Func` arm, the
codegen header-walk guard, one `regimen.rs` test and the `generative.rs` guard lose their only
input and are deleted.

**Tech Stack:** Rust 2024. **No new dependency.** Spec §11's table is not touched.

## Context

The parser reads a `dum` init clause with `self.statement()?` (`parser.rs:323`), so `redde`,
`abrumpe`, `continua`, `si`, `indem`, `dum`, `munus`, `scribe` and expression statements all parse
there, and the first three silently return or leave an enclosing loop.

The design doc is `docs/superpowers/specs/2026-09-15-dum-header-clause-design.md`; read §3 (the
grammar), §5 (every shape rejected), §6 (what is deleted) and §7 (catalogue impact) before starting.
Where this plan and that spec disagree, the spec governs.

Line numbers below are at `cd516b3`, and each task's edits move the lines the next task names. Find
the code by symbol when a number has drifted.

---

## Global Constraints

Copied from `CLAUDE.md` and spec §11. Every task's requirements implicitly include this section.

- **`cargo nextest run` is the one test command.** It runs the whole workspace, `abi` included.
  `cargo test --test conformance` is used only to bless, because `LISS_BLESS` goes through it.
- Work inside `nix develop`; if `cargo` is missing, prefix commands with `nix develop -c`.
- **Zero runtime dependencies is a rule** (spec §11). This plan adds none. Verify with
  `cargo tree -e normal`.
- **Diagnostics are Latin, inline and unique.** The one new sentence is written as the literal
  argument at its call site in `crates/lingua-serpentis/src/parser.rs`. No Arabic digits.
- **The source does not explain itself; `docs/internals/` does.** What stays in the source is
  `# Safety` clauses, fixture and test pointers, and a bare `D-NN`.
- **Comments move the catalogue snapshot.** Re-bless, then check the header against spec §7 and
  account for every moved row.
- **Bless narrowly, then run `tests/nuntii.rs`.** Never bless the whole corpus for this change; a
  `nuntii` that changes rather than appears is a regression.
- Commit subjects are `type(scope): a sentence in the repo's voice`. Scopes are the area (`parser`,
  `checker`, `codegen`, `conformance`, `defectus`, `spec`), not the crate name. Every commit message
  ends with `Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>`.

---

## Task 0: Baseline

No commit.

- [ ] Step 1: `git status` clean; `git log -1` is the spec commit or later.
- [ ] Step 2: `cargo nextest run` green. Record the total and the per-binary counts for
      `conformance`, `generative`, `regimen`, and the `lingua-serpentis` lib tests.
- [ ] Step 3: Record `head -5 crates/lingua-serpentis/tests/snapshots/catalogus__catalogus.snap` — it
      must read `118 sites · 70 fixtured · 0 ambiguous · 46 uncovered · 2 unparsed` — and
      `nuntii__nuntii.snap`'s `86 fixtures · 99 diagnostics`.
- [ ] Step 4: Save the before-set for spec §7's check:
      `sed -n '7,$p' crates/lingua-serpentis/tests/snapshots/catalogus__catalogus.snap | sed -E 's/^[^ ]+  //' | sort > $SCRATCH/catalogus-ante.txt`
      (`$SCRATCH` is any directory outside the repository).

If a count differs from the spec, record the measured number here and carry on against it; do not
edit the spec's before-header.

## Task 1: The design doc — DONE

`docs/superpowers/specs/2026-09-15-dum-header-clause-design.md`, the amendment notice at the top of
`2026-07-19-lingua-serpentis-rust-interpreter-design.md`, and `docs/README.md`: the question-table
row, the spec-table row and "Fourteen documents", "amended five times", the fifth amends-chain entry
and the reading list's "five amendments". This plan and README's "Fifteen plans" land in the commit
after.

---

## Task 2: The fixtures, failing

**Files:** new `conformance/errores/{redde,abrumpe,si,munus}-in-capite-dum/{programma/principale.liss,exitus,NOTA.md}`.

**Interfaces:** produces four trials `errores::<name>` that Task 3 turns green. No `nuntii` yet; Task
3 blesses it.

- [ ] Step 1: Write the four programs. Each opens with a comment line; identifiers are Latin.

      `redde-in-capite-dum/programma/principale.liss`:
      ```liss
      // D-26: 'redde' in capite 'dum'.
      munus initium reddit ni {
          ni k = nullus;
          dum (redde nullus; verus; k = k + °I) { scribe("numquam"); }
      }
      ```

      `abrumpe-in-capite-dum/programma/principale.liss`:
      ```liss
      // D-26: 'abrumpe' in capite 'dum', intra iterationem exteriorem.
      munus initium reddit ni {
          ni i = nullus;
          indem (i minor_est °II) {
              i = i + °I;
              dum (abrumpe; verus; i = i) { scribe("intus"); }
              scribe("extra");
          }
          redde nullus;
      }
      ```

      `si-in-capite-dum/programma/principale.liss`:
      ```liss
      // D-26: 'si' cum 'redde' in capite 'dum'.
      munus initium reddit ni {
          ni k = nullus;
          dum (si (verus) { redde °I; } k minor_est °II; k = k + °I) { scribe(k); }
          redde nullus;
      }
      ```

      `munus-in-capite-dum/programma/principale.liss`:
      ```liss
      // D-26: 'munus' in capite 'dum'.
      munus initium reddit ni {
          ni k = nullus;
          dum (munus unum reddit ni { redde °I; } k minor_est °II; k = k + °I) { scribe(k); }
          redde nullus;
      }
      ```

- [ ] Step 2: `exitus` = `1` in each.
- [ ] Step 3: `NOTA.md` in each: "D-26 is fixed." then what the program did before (spec §8's table:
      exit 0 silent; exit 0 silent with the outer loop left; exit 1 silent; ran with the `munus`
      checked and dropped), then that a `dum` init clause is now a declaration or an assignment, and
      that the program opens with a comment so the error does not land on line 1.
- [ ] Step 4: Confirm the old behaviour against the current binary before changing anything:
      `cargo build && target/debug/liss run conformance/errores/<name>/programma; echo $?` for each.
      Expected: no stderr, exits `0`, `0`, `1`, `0`; the `munus` case prints `nullus` and `I`.
- [ ] Step 5: `cargo nextest run --test conformance`. Expected: exactly the four new trials fail, each
      on a stderr/exit mismatch (empty expected stderr is missing, `exitus` 1 against 0 or 1); every
      other trial passes.

Not committed alone — a commit with a red suite would break `git bisect`. Carry into Task 3.

## Task 3: The parser rule, the parser test, and the first bless

**Files:** `crates/lingua-serpentis/src/parser.rs` (`for_stmt` `:319-330`; tests module near
`while_for_break_continue` `:664-667`), the four `nuntii` files,
`crates/lingua-serpentis/tests/snapshots/{catalogus,nuntii}__*.snap`.

**Interfaces:** consumes Task 2's fixtures. Produces a parse error for every spec §5 shape; the AST
for legal clauses is unchanged.

- [ ] Step 1: Write the failing parser test beside `while_for_break_continue`:

      ```rust
      #[test]
      fn dum_init_is_a_declaration_or_an_assignment() {
          for init in [
              "ni i = °I;", "ni lista l = [°I];", "i = °I;",
          ] {
              let src = format!("dum ({init} verus; i = i + °I) {{ }}");
              assert!(matches!(parse_all(&src)[0].kind, StmtKind::For { .. }), "{src}");
          }
          for init in [
              "redde °I;", "abrumpe;", "continua;",
              "si (verus) { redde °I; }", "indem (verus) { abrumpe; }",
              "dum (redde °I; verus; i = i) { }", "munus f reddit ni { redde °I; }",
              "scribe(°I);", "f(°I);", "p.f(°I);", "x[°I] = °I;",
          ] {
              let src = format!("dum ({init} verus; i = i + °I) {{ }}");
              assert_eq!(
                  parse_err(&src),
                  "clausula prima capitis 'dum' declaratio vel assignatio esse debet",
                  "{src}"
              );
          }
          // Spec §5 row 11: an empty init clause.
          assert_eq!(
              parse_err("dum (; verus; i = i + °I) { }"),
              "clausula prima capitis 'dum' declaratio vel assignatio esse debet"
          );
      }
      ```

- [ ] Step 2: `cargo nextest run -p lingua-serpentis --lib dum_init`. Expected: fails at the first
      rejected entry — `redde °I;` parses today, so `parse_err` panics `expected a parse error`.
- [ ] Step 3: In `for_stmt`, after `self.eat(&TokenKind::LParen)?;` and before `statement()`:
      a lookahead that is true when `self.peek()` is one of the six type-keyword tokens
      (`TyInt TyFloat TyDouble TyText TyChar TyBool`, the set `func_stmt` names at `:347-348`) or
      an `Ident` whose next token (`self.toks.get(self.pos + 1)`) is `Assign`. When it is false,
      `return Err(self.err("clausula prima capitis 'dum' declaratio vel assignatio esse debet"));`
      — the literal inline. Keep `Box::new(self.statement()?)` for the true case.
- [ ] Step 4: `cargo nextest run -p lingua-serpentis --lib`. Green.
- [ ] Step 5: Hand-run each fixture: `target/debug/liss run conformance/errores/<name>/programma`.
      Expected, for `redde-in-capite-dum`:
      ```
      erratum: clausula prima capitis 'dum' declaratio vel assignatio esse debet
        in 'principale.liss', linea IV
          |
       IV |     dum (redde nullus; verus; k = k + °I) { scribe("numquam"); }
          |          ^^^^^
      ```
      and exit 1. The others the same, caret under `abrumpe` (linea VI), `si` and `munus` (linea IV).
- [ ] Step 6: Bless the four, and only the four:
      `LISS_BLESS=1 cargo test --test conformance -- in-capite-dum`.
- [ ] Step 7: `git status --porcelain conformance/` — only the four new directories, now with `nuntii`.
      Each `nuntii` is one block (spec §9 step 4).
- [ ] Step 8: `cargo nextest run --test nuntii`. Expected: fails with four added rows, nothing removed
      or changed. Accept: `INSTA_UPDATE=always cargo nextest run --test nuntii`; header now
      `90 fixtures · 103 diagnostics`.
- [ ] Step 9: `cargo nextest run --test catalogus`. Expected: fails — one added `parser.rs` row, and
      `parser.rs` rows from `:350` shifted. Accept with `INSTA_UPDATE=always`; header
      `119 sites · 71 fixtured · 0 ambiguous · 46 uncovered · 2 unparsed`.
- [ ] Step 10: Spec §7's set check: regenerate the sorted `(pattern, status)` list as in Task 0 Step 4
      and `diff $SCRATCH/catalogus-ante.txt -`. Expected: exactly one `>` line, the new sentence
      `→ fixtured`.
- [ ] Step 11: `cargo nextest run`. Expected red in exactly one place: `regimen`'s
      `caput_dum_terminatum_componitur`, whose programs no longer parse. (`generative`'s self-test
      exercises a text scanner, not the parser, and still passes.) Delete `caput_dum_terminatum_componitur`
      (`crates/codegen/tests/regimen.rs:296-344`, doc comment included) in this commit so the tree is
      green; the rest of the dead code is Task 4.
- [ ] Step 12: `cargo nextest run` green. Commit:

```bash
git add crates/lingua-serpentis/src/parser.rs crates/codegen/tests/regimen.rs \
  conformance/errores/*-in-capite-dum crates/lingua-serpentis/tests/snapshots/
git commit -m "fix(parser): a dum header opens with a declaration or an assignment

The four errores/ fixtures are the first D-26 has had. The fuzz seed
stream shifts: support/mutate.rs sorts the corpus by path.

Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>"
```

## Task 4: What the rule made dead

**Files:** `crates/lingua-serpentis/src/checker.rs` (`:594-595`, `:613-618`, `:726-737`),
`crates/codegen/src/emissio.rs` (`:736-749`), `crates/lingua-serpentis/tests/generative.rs`
(`:91-142`, `:154-159`, `:200`, `:257-280`), `catalogus__catalogus.snap`.

**Interfaces:** none. Moves **no fixture** and changes no diagnostic text; the catalogue moves by
`:line` and one row reorders.

- [ ] Step 1: `checker.rs:594-595` — the two-line comment becomes `// D-26`.
- [ ] Step 2: `checker.rs` `StmtKind::Func(f)` arm in `check_stmt` — replace its body with the
      nested-function `Err(LinguaError { message: format!("functio '{}' intra corpus definiri non
      potest; functiones nidificatae non sustinentur", f.name), line, spatium: Some(spatium) })`,
      and delete the `if let StmtKind::Func(f) = &s.kind { … }` block from `check_block_inner`.
      Move, do not copy: the sentence must exist at exactly one site or the catalogue reads it
      `ambiguous`.
- [ ] Step 3: `emissio.rs` `dum_intus` — delete the five-line comment and both `terminatus` checks; the
      header walk is `for stmt in &header.stmts { self.sententia(stmt)?; }`. Leave the step guard at
      `:776-778` (spec §2 Non-Goals).
- [ ] Step 4: `generative.rs` — delete `caput_dum_terminatum`, `clausula_initialis`, `verbum_est`, the
      two assertions that call it, and
      `the_guard_recognises_the_shape_it_excludes_and_nothing_else`. Remove any import this leaves
      unused.
- [ ] Step 5: `cargo nextest run --test conformance`. Green, and `git status --porcelain conformance/`
      empty — `errores/munus-nidificatum` is the check that the moved rejection still fires.
- [ ] Step 6: `cargo nextest run --test catalogus` fails on `:line` only. Accept with
      `INSTA_UPDATE=always`. The header is unchanged from Task 3. Run spec §7's set check again against
      the Task 3 list: **no** difference. Confirm the rows that moved are the 14 `checker.rs` rows from
      `:623` (with the `munus-nidificatum` row now above them) and `emissio.rs:813`, and nothing else.
- [ ] Step 7: `grep -rn 'caput_dum_terminatum\|clausula_initialis' crates/` prints nothing.
- [ ] Step 8: `cargo nextest run` green. Commit:

```bash
git commit -am "refactor(checker): a munus in a dum header cannot parse, so nothing waits for one

The nested-function rejection moves into the Func arm it made dead;
codegen's header walk loses its terminator guard, and generative.rs
its D-26 guard and the self-test that held it up.

Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>"
```

## Task 5: Internals documentation

**Files:** `docs/internals/codegen.md`, `rationale.md`, `testing.md`.

- [ ] Step 1: `codegen.md:626-631` — the claim that the header walk owes the terminator guard goes;
      `:733-765`, §7 "A terminator in the header (D-26)", is rewritten to say the parser makes the
      shape unconstructible, so `dum_intus` walks the header plainly.
- [ ] Step 2: `rationale.md:346-353` (parser: the `dum` init clause is a guarded `statement()`, and why
      a grammar rule rather than a checker flow rule — spec §2's rejected alternative) and
      `:1353-1387` (Part 2: what the undecided shape used to do in each backend; keep it as the
      counterfactual record, add the close).
- [ ] Step 3: `testing.md:537-566` — the generative guard section goes; say in one sentence why there is
      no guard.
- [ ] Step 4: `cargo nextest run` green (nothing here is read by a test, but the `catalogus` snapshot
      must not have moved). Commit:

```bash
git commit -am "docs(internals): the dum header is a grammar rule, and the guards that waited for D-26 go

Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>"
```

## Task 6: Close D-26 in the register

**Files:** `docs/defectus/2026-08-03-index.md`.

- [ ] Step 1: The entry (`:913-1013`) gains `- **Fixed** \`<Task 3 sha>\`, 2026-09-15 — …` stating the
      rule and the sentence, and **Covered by** names the four fixtures and
      `parser.rs`'s `dum_init_is_a_declaration_or_an_assignment`. The as-found text stays.
- [ ] Step 2: Status row `:139` → `**fixed** \`<sha>\` → \`errores/redde-in-capite-dum\`` (and the
      three siblings); the D-26 row of the deferred table (`:84`) goes.
- [ ] Step 3: Header prose `:32-40` and `:77`, which describe D-26 as deferred; and the status line at
      `:6`, which was already stale before this change (it predates D-28 and D-29 closing) — make it
      match `docs/README.md:251`'s count with D-26 moved from deferred to fixed.
- [ ] Step 4: `docs/README.md:251` — `27 fixed, 2 deferred, 1 open`.
- [ ] Step 5: Commit:

```bash
git commit -am "docs(defectus): D-26 closes, and a dum header is a declaration or an assignment

Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>"
```

The restated corpus counts (`CLAUDE.md`, `README.md:80`, `docs/README.md:220`, `testing.md`,
`nuntii.rs:3`, the catalogue header quoted in `CLAUDE.md` and `testing.md:284`) and `docs/via.md`'s
milestone 2 row are updated when milestone 2 closes (spec §8), not here.

---

## Verification

1. `cargo nextest run` — green. Total = Task 0 + 1 (parser test) + 4 (conformance) − 1
   (`caput_dum_terminatum_componitur`) − 1 (the generative self-test).
2. `head -5 crates/lingua-serpentis/tests/snapshots/catalogus__catalogus.snap` →
   `119 sites · 71 fixtured · 0 ambiguous · 46 uncovered · 2 unparsed`.
3. `diff $SCRATCH/catalogus-ante.txt <(sed -n '7,$p' …catalogus__catalogus.snap | sed -E 's/^[^ ]+  //' | sort)`
   → one added line, the new sentence `→ fixtured`.
4. `head -5 crates/lingua-serpentis/tests/snapshots/nuntii__nuntii.snap` →
   `90 fixtures · 103 diagnostics`; `git diff cd516b3 -- …nuntii__nuntii.snap` adds four rows and
   removes none.
5. `ls conformance/errores | wc -l` → `82`; `git diff --stat cd516b3 -- conformance/` touches only the
   four `*-in-capite-dum` directories.
6. `target/debug/liss run conformance/errores/<name>/programma` reproduces each `nuntii` and exits 1.
7. `grep -rn 'caput_dum_terminatum\|clausula_initialis' crates/` → nothing.
8. `cargo tree -e normal` → no new package.
9. `grep -n 'D-26' docs/defectus/2026-08-03-index.md docs/README.md` → no "deferred" wording for D-26.
