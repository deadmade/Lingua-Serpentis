---
flow: refactor
phase: build
now: Task 1 (the glossary) is being written by a subagent on branch refactor-english-names; it stops for your review before any code moves.
---

# Refactor English names: flow

## Request

Could we make a plan to essentialy rename and make sure to just use english for everything in the project?

## Artifacts

- Plan: [plan.md](plan.md)

## Gates

- Gate 1, plan: 2026-11-11, execution: subagent-driven

## Skipped

- plan review: not run, because the size is local (a rename), and plan review is for restructurings only.

## Grounding

- Base: `74ba79c` on `milestone-2`, tree clean. `nix develop --command cargo nextest run`: **686 passed, 0 failed** (1 slow, 90 s). Measured.
- Source comments and docs prose are already English (sampled 388 comment lines in 5 files: no Latin). The Latin lives in
  identifiers, filenames, directory names, test names, one crate name, and the fixture-harness vocabulary. Verified by sampling.
- Mixed already: `liss_int_add`, `div_rem` beside `series_vacuae_sunt_res_verae`, `nucleus_nostra_sine_pontibus`. 1079 distinct fn names, 77 types. Measured.
- Size of the Latin surface: 1 894 lines hold an identifier from a 90-word seed list of Latin words (abi 344, codegen 862,
  ir 18, latinitas 1, lingua-serpentis 598, runtime 71). A lower bound, since the seed list is hand-made. Measured, with that caveat.
- Rust files: 19 370 lines, 503 tracked files in crates/tests/examples/conformance. Conformance is 123 fixtures (104 currunt, 288 files errores, 15 monita by file count).
- **Frozen, because it is behaviour** (scope answer: "Names only"): every string literal the compiler emits or a test asserts
  (Latin diagnostics, `c"…"` in abi, keywords), the `liss_*` C symbols (spec §5.4), `.liss` programs and their filenames (`principale.liss`
  appears in diagnostics, `examples/praecisio.liss` is echoed in a conformance `nuntii`), the `liss` binary, crate `lingua-serpentis`, CLI flags `--exi`/`--ad`.
- Nets that already cover a rename (read from the tests, inferred to be sufficient): `tests/catalogus.rs` (121 diagnostic sites, snapshot records `file:line`
  and sorts by full path), `tests/nuntii.rs` (snapshot of all corpus sentences), `crates/abi/tests/abi.rs` (198 `liss_` mentions),
  `crates/codegen/tests/pondus.rs` (greps section names for `3abi`/`7runtime`, so crates `abi`/`runtime` must keep their names),
  conformance (123 trials).
- Hazards found while reading: (1) `custos.rs` owns `main` and lists tests in `probationes`; (2) insta snapshot names are `<file>__<fn>`, so renaming a test
  file or fn renames its `.snap`; (3) `{name}` inline format captures sit inside string literals, which a token-level rename won't see (the compiler will);
  (4) nextest test IDs change, so anything quoting them in docs moves; (5) `Cargo.lock` changes when `latinitas` is renamed.

### Coverage thin spots

- Prose in README, CLAUDE.md, docs/internals, docs/README.md, docs/via.md: no test reads it. Covered by a grep sweep in the last task.
- Literal drift: a script that renames identifiers could touch a literal. Covered by catalogus + nuntii + abi `sententiae` + conformance, no new test needed.
- Exported `liss_*` symbols: covered by `crates/abi/tests/abi.rs` and every compiled-program test (a mismatch fails to link).
- No behaviour the refactor moves lacks a test, so no characterization test is written (see plan Test seams); instead Task 1 records two scratch baselines to diff against.

## Findings

## Options

### Shape

Rename every Latin Rust identifier, source and test filename, test name, fixture directory and harness file name to English, one word
list for the whole project; keep everything frozen above.

- Crate dir and package `latinitas` → `latin`.
- Source files, proposed (Task 1 fixes the table): runtime `dispositio→layout`, `exitus→exit_status`, `genus→kind`, `scriptio→script`;
  abi `custos→guard`, `defectus→defect`, `ingressus→entry`, `memoria→memory`, `numeri→numbers`, `series→list`, `textus→text`;
  codegen `emissio→emit`, `nexus→link`; lingua-serpentis `fragmentum→fragment`, `pictura→picture`, `spatium→span`.
- Fixture harness: `currunt/errores/monita → run/reject/warn`, `emissio/nuntii/exitus/programma/VIA/NOTA.md → stdout/stderr/exit/program/PATH/NOTE.md`.
- Fixture directory names become English slugs.
- Living docs (README, CLAUDE.md, docs/README.md, docs/internals, docs/via.md) follow. Dated specs, plans and the defect register are history and keep the old names; docs/README.md gets one old→new table.
- Seams: none. A rename adds no concern and moves no boundary.

Picked: the shape above (names only; living docs updated, dated history untouched; `latinitas` → `latin`).

### Size

Recommend **local**: a rename, no module or boundary changes. The diff is large (about 1 900+ lines of Rust, hundreds of file moves) but its shape is one mechanical transformation, so a seam analysis and plan review don't apply.

Picked: local.

## Plan review

Not run, see Skipped.

## Tasks

Task 1: base 74ba79c, untracked at start: docs/features/refactor-english-names/flow.md, plan.md; run folder only, no repo files.

## Rulings

## Parking lot

## Deviations

## Walkthrough

| goal | plan task | file:line | test |
| ---- | --------- | --------- | ---- |

## Review

## Learned

## Run verdict
