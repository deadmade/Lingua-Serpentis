# Caret Diagnostics Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development
> (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use
> checkbox (`- [ ]`) syntax for tracking.

**Goal:** Put source spans through the front end and render a caret under the offending expression,
replacing the one-line `Error in 'f.liss', linea N: msg` with the three-level grammar of
`2026-08-09-caret-diagnostics-design.md` §3 — in the compiler, the back end, and the compiled image
alike.

**Architecture:** Positions enter at the lexer as byte offsets (`Spatium`), travel on `Token`, on
`Expr`, on `Stmt` and on declared names, and are turned into a `Fragmentum` — an already-extracted
source line plus caret column — at the point where the file's text is still in hand. The renderer
reads only the `Fragmentum`, so `render` keeps its signature and `strip`'s single-source rewrite
keeps working untouched. `liss-abi` adopts the header and drops `decimales`.

**Tech Stack:** Rust 2024. **No new dependency** — spec §11's table is not touched.

## Context

`Token` is `{ kind, line }`. `Expr` carries no position at all, so `checker.rs` threads a
`line: usize` parameter by hand through six methods and every diagnostic about a subexpression
reports the enclosing statement's line. Nothing in the workspace stores a byte offset or a column.

The design doc is `docs/superpowers/specs/2026-08-09-caret-diagnostics-design.md`; read its §3
(the grammar), §4 (the three units of position) and §5 (`Fragmentum` and every edge case) before
starting. Where this plan and that spec disagree, the spec governs.

**Almost all of the cost is test machinery.** 86 conformance fixtures pin stderr byte for byte, and
~50 more wording assertions live outside the corpus. Tasks 4–6 and 8–13 are larger than the compiler
work in 2, 3, 7 and 10.

---

## Global Constraints

Copied from `CLAUDE.md` and spec §11. Every task's requirements implicitly include this section.

- **Zero runtime dependencies is a rule** (spec §11). This plan adds none, to any table. Verify with
  `cargo tree -e normal`.
- **The source does not explain itself; `docs/internals/` does.** What stays in the source is
  `# Safety` clauses, fixture and test pointers, and a bare `D-NN`.
- **Comments move the catalogue snapshot.** Re-bless, then account for every moved row.
- Two test commands, both required: `cargo test`, and
  `cargo test --manifest-path crates/liss-abi/Cargo.toml`. The first does not reach the second
  workspace.
- Work inside `nix develop`.
- Commit subjects are `type(scope): a sentence in the repo's voice`. Scopes are the area
  (`diagnostica`, `conformance`, `spec`), not the crate name.

---

## Task 0: Baseline — DONE

Tree clean at `b2a67eb`. Both commands green: conformance 118, catalogus 5, cli 23, liss_cli 24,
robustness 29, generative 4; `liss-abi` 21 + 6 + 1. Snapshot header
`118 sites · 70 fixtured · 0 ambiguous · 46 uncovered · 2 unparsed`. No commit.

## Task 1: The design doc — DONE

`docs/superpowers/specs/2026-08-09-caret-diagnostics-design.md`, plus four `docs/README.md` edits:
the question-index row, the spec table row, the amends-chain entry, and the compiler-backend spec's
"Nothing amends it" line, which is no longer true.

---

## Task 2: `Spatium`, and the lexer produces it

**Files:** `crates/lingua-serpentis/src/token.rs`, `src/lexer.rs`.

- [x] Step 1: Write the failing tests — `°XXV` spans the whole numeral including `°`; a multi-line
      string spans open-quote to close-quote with `linea` at the opening line (D-03); a non-ASCII
      identifier's span is byte-correct; the separator give-back at `lexer.rs:40-42` shrinks the span.
- [x] Step 2: Run them and verify they fail to compile — `Token` has no `spatium`.
- [x] Step 3: Add `Spatium`; add `spatium` to `Token`, keeping `line`.
- [x] Step 4: Build the `offsets` prefix-sum table in `tokenize` and record spans at every push.
- [x] Step 5: Both test commands green. Hand-check `conformance/errores/linea-post-chordam`.

Pure addition; nothing reads `spatium` yet.

## Task 3: `Fragmentum` and `extract`

**Files:** new `crates/lingua-serpentis/src/fragmentum.rs`.

- [x] Step 1: Write the failing tests — the whole edge-case table of design §5, plus the alignment
      invariant `caret == textus[..byte].chars().count()`, plus `°` explicitly.
- [x] Step 2: Run and verify they fail.
- [x] Step 3: Implement `extract`. Pure, no I/O.
- [x] Step 4: Both commands green.

Zero wiring; cannot break anything.

## Task 4: The safety net, on the old grammar

**Files:** new test under `crates/lingua-serpentis/tests/`.

- [x] Step 1: Read all 86 `nuntii`, parse each line to `(genus, file, line, message)`,
      `insta`-snapshot the ~99 tuples.
- [x] Step 2: Commit that snapshot **on the old grammar**.

**This must land before Task 6.** If it slips, the whole safety argument of design §10 is gone.

## Task 5: `judge` learns the new grammar while still accepting the old

**Files:** `crates/lingua-serpentis/tests/support/diag.rs`.

- [x] Step 1: `Genus`, `Diagnostica`, `parse_stderr` returning blocks plus residue.
- [x] Step 2: `judge`'s six changes — last *block* not last line; the exit-0 branch, which currently
      requires every stderr line to start with `Monitum`; Roman `line in range` with a round-trip
      check; `file exists` from the location row; the new `caret aligns`; `no Arabic digits` widened
      per design §8.
- [x] Step 3: Keep `parse_diagnostic`'s rule that a *warning* yields `None`.
- [x] Step 4: Both commands green — nothing emits the new form yet.

Doing this before the emitter is what keeps the robustness suite from being red across two
consecutive tasks.

## Task 6: The header rename — framing only, no snippets

**Files:** `src/error.rs` (8 `Display` arms), `src/cli.rs` (9 raw `eprintln!` sites: `:113, 117,
191, 198, 252, 303, 404, 419, 425`), `crates/liss-codegen/src/lib.rs:21`,
`crates/lingua-serpentis/tests/catalogus.rs`, all 86 `nuntii`.

- [x] Step 1: `erratum:`/`monitum:`, the location row, Roman line numbers via
      `roman::Roman::from_u64(n).display()`.
- [x] Step 2: Fix `catalogus.rs::observed_messages` to collect only severity-prefixed lines — design
      §9's false-green, and it cannot wait for a later task.
- [x] Step 3: Fix the wording assertions that move: `error.rs:89-128`, `lib.rs:87,99,114`,
      `tests/cli.rs`, `tests/liss_cli.rs`, `tests/programs.rs`, `liss-codegen/tests/scalares.rs`,
      `vocationes.rs:13`, `aemulatio.rs:158,182`.
- [x] Step 4: `LISS_BLESS=1 cargo test --test conformance`, then the three diff checks and the Task 4
      snapshot.

Largest blast radius in the plan.

## Task 7: Declaration spans

**Files:** `src/ast.rs`, `src/parser.rs`, `src/latinity.rs`.

- [x] Step 1: `name_span` on `StmtKind::Let` and `FuncDecl`; per-parameter spans. `ParsedFile` gains
      `src`.
- [x] Step 2: Delete `FuncDecl::def_line` — three uses, all subsumed.
- [x] Step 3: Both commands green. Populated but unused.

## Task 8: Warning snippets

**Files:** `src/error.rs`, `src/latinity.rs`, `src/lib.rs`, `src/cli.rs`.

- [x] Step 1: `Warning.fragmentum`; `lint_program` attaches it; `warnings_to` emits the block — and
      so does the success path at `cli.rs:184-185`.
- [x] Step 2: Bless ~20 lines across ~8 files; the three checks and the Task 4 snapshot.

Warnings before errors: ~20 lines rather than 74, on a corpus where `monita/ordo-trans-scidas`
proves multi-block output and `monita/fons-unicus` proves the `strip()` path.

## Task 9: Lexer error spans, and `LinguaError` carries one

**Files:** `src/lexer.rs` (5 sites), and a mechanical `spatium: None` sweep across ~45 others.

- [x] Step 1: `LinguaError` gains `spatium: Option<Spatium>`; the sweep.
- [x] Step 2: Both commands green. Re-bless the catalogue; ~50 rows move by line only, and the
      pairing check must print nothing.

## Task 10: `Expr` becomes `{ kind, span }`

**Files:** `src/ast.rs`, `src/parser.rs` (58 sites), `src/checker.rs` (22), `tests/ir.rs`,
`tests/generative.rs`.

- [x] Step 1: One `Parser` helper — take the position before the production, close it after.
- [x] Step 2: Both commands green, **zero fixture movement**, and `tests/ir.rs`'s six snapshots
      unchanged.

Spans populated, still unused. Zero movement is what makes an 80-site mechanical change
independently verifiable: a compile error becomes the only way it can fail. Do not mix with
behaviour changes.

## Task 11: Error snippets

**Files:** `src/checker.rs`, `src/parser.rs`, `src/error.rs`, `src/lib.rs`.

- [x] Step 1: `checker.rs` drops the threaded `line: usize` from its six methods.
      `Parser::err()` uses the current token's span. `ProgramError::in_file` gains `&src`, 7 sites.
- [x] Step 2: **Record which fixtures move lines, before blessing**, and confirm each moved to a
      line that genuinely holds the offending expression.
- [x] Step 3: Turn on `caret aligns` and the widened `no Arabic digits` for real.
- [x] Step 4: Bless ~74 lines; the three checks and the Task 4 snapshot.

`ir::Stmt.line` keeps its present value — `emissio.rs:549` copies it into `Corpus::linea` and
`locus()` hands it to every fallible runtime entry point.

## Task 12: `liss-abi` adopts the header

**Files:** `crates/liss-abi/src/defectus.rs`, `src/ingressus.rs`, `tests/{abi,custos}.rs`.

- [x] Step 1: Reorder the writes; `Roman::from_u64(n).display()` for the line; delete `decimales`
      and its buffer; move `MONITUM_PRAEFIXUM`.
- [x] Step 2: **Force a rebuild of the embedded musl archives and confirm a runtime-origin fixture
      actually moved** — a stale staticlib is how this task fails silently with both commands green.
- [x] Step 3: Bless the 13 runtime fixtures. Both commands.

## Task 13: One grammar

**Files:** `tests/support/diag.rs`, `tests/robustness.rs:15-33`.

- [x] Step 1: Delete Task 5's compatibility branch; rewrite the pinned shapes.
- [x] Step 2: Both commands green.

## Task 14: Documentation

- [x] `docs/internals/rationale.md` §"Errors, warnings and rendering" — replaces the 4×2 table.
- [x] `docs/internals/testing.md` §"The robustness invariant" and the `judge` table.
- [x] `docs/internals/abi.md` §13 — the runtime's shapes.
- [x] `docs/defectus/2026-08-03-index.md` — a `D-NN` for the display-width limitation.
- [x] `CLAUDE.md` — the `VIA`-echoes-`examples/` coupling of design §11.
- [x] Re-verify the catalogue header; enumerate every moved row against the design doc.
