# Refactor English names: plan

<!-- glance -->
```
Plan: Refactor English names: plan · 9 tasks · ~6550 lines · 108 files
 1  [1 Glossary and baselines]
 2  [2 Runtime crate ←1]
 3  [3 Abi crate ←2]
 4  [4 Ir and latinitas ←3]
 5  [5 Codegen crate ←4]
 6  [6 Lingua-serpentis source ←5]
 7  [7 Lingua-serpentis tests ←6]
 8  [8 Conformance corpus and … ←7]
 9  [9 Docs and the final sweep ←8]

#  does                                                          files  adds         size
1  Writes down which English word replaces each Latin word and…  1      nothing new  ~450
2  Renames the Latin names in `crates/runtime` and their use s…  15     nothing new  ~300
3  Renames the Latin Rust names in `crates/abi`, leaving every…  14     nothing new  ~800
4  Renames the names in `ir`, and renames the `latinitas` crat…  14     nothing new  ~250
5  Renames the Latin names in `crates/codegen`, its tests and …  13     nothing new  ~2000
6  Renames the Latin names in the lexer, parser, checker, CLI …  14     nothing new  ~1300
7  Renames the test files, test functions, helpers and snapsho…  23     nothing new  ~700
8  Renames the fixture directories, the category names and the…  6      nothing new  ~250
9  Brings the living documents up to the new names, adds one o…  8      nothing new  ~500
```
<!-- /glance -->

**Goal:** Every name in the project is English: Rust identifiers, file and directory names, test names, the `latinitas` crate, the fixture-harness vocabulary. The Latin language and everything the compiler says stays Latin.

**Architecture:** One glossary (Latin word → English word, plus a file and directory table) is written and approved first. A token-level rewrite script applies it crate by crate, bottom-up, and the workspace is green after every task. The script lives in the scratchpad and is not committed.

## Global Constraints

- Behaviour doesn't change: tests that passed before still pass.
- One implementation gets no interface; one caller gets no helper; no option nobody sets.
- Frozen, never renamed: string and char literals (diagnostics, `c"…"`, keywords, harness filenames inside literals are changed by hand in Task 8 only), the `liss_*` C symbols, `.liss` programs and their filenames (`principale.liss`, `examples/*`), the `liss` binary, crate `lingua-serpentis`, crate names `abi`, `runtime`, `ir`, `codegen`, CLI flags `--exi` and `--ad`, `D-NN` markers.
- One Latin word gets one English word everywhere (the glossary). A task never invents a mapping; a missing word is a `NEEDS_CONTEXT`.
- Never write a `/* … */` block comment under `crates/abi/src`; keep comment lines as they are (a comment line added or removed moves the catalogus snapshot).
- Crate rename: anchor on the whole crate name (`latinitas`), never a prefix.
- A file that only uses a renamed public item (a use-site fix, names only) is on the list of the task that renames the item; the plan's Files lists are the files that define the names, and the files check accepts these.
- Stack skill: rust-skills `rust-best-practices`.

## Test seams

No characterization test is added: the thin spots Ground listed are covered by existing tests (see flow.md Grounding), and the rest is prose. Tests that already guard a rename:

- `cargo nextest run` (whole workspace): 686 pass on `74ba79c`; catches a rename that breaks a build or a link, and any behaviour change. Misses prose and a rename that is wrong-but-consistent.
- `crates/lingua-serpentis/tests/catalogus.rs`: catches a changed, lost or reclassified diagnostic site. Misses nothing about names beyond the `:line`/path field it re-blesses.
- `crates/lingua-serpentis/tests/nuntii.rs`: catches a changed diagnostic sentence or line in the corpus. Misses file-name framing.
- `crates/abi/tests/abi.rs`: catches a renamed or missing `liss_*` symbol and a changed `c"…"` sentence.
- `crates/codegen/tests/pondus.rs`: catches landing pads and build-machine paths reaching the shipped runtime.
- `conformance` (`cargo nextest run --test conformance`): 123 trials, catches changed stdout, stderr or exit status.

Scratch baselines (not committed), recorded in Task 1 from the base:
the sorted set of `(message, status)` pairs in `catalogus__catalogus.snap`, and the sorted multiset of diagnostic sentences in `nuntii__nuntii.snap`. They are what "a re-bless changed only the name or `:line` field" is compared against.

### Task 1: Glossary and baselines
**Does:** Writes down which English word replaces each Latin word and file name, and what must not change, so every later task only applies it.
**Depends on:** none
**Adds:** nothing new
**Size:** ~450

**Files:**
- Create: `docs/features/refactor-english-names/glossary.md`

- [ ] Extract every identifier word from `git ls-files '*.rs'` (outside string literals and comments) and every file and directory name under `crates`, `conformance` (test dirs and harness files only). Group the Latin ones; give each an English word. Where a word is also the language's own vocabulary (`munus`, `series`), pick the English concept name once and use it everywhere.
- [ ] Add the file table (old path → new path) for source files, test files, snapshot files, `crates/latinitas` → `crates/latin`, `crates/runtime/tests/data/scriptio-aurea.txt`, and the 123 fixture directories plus harness names.
- [ ] Add the frozen list from Global Constraints and any literal the first pass found that looks like a name but is a message.
- [ ] Record the two scratch baselines in the scratchpad.
- [ ] **Stop: the user reviews `glossary.md`.** Naming is the point of the refactor; the glossary is approved before any code moves. An edit the user asks for is made here.

### Task 2: Runtime crate
**Does:** Renames the Latin names in `crates/runtime` and their use sites across the workspace.
**Depends on:** Task 1
**Adds:** nothing new
**Size:** ~300

**Files:**
- Modify: `crates/runtime/src/dispositio.rs`
- Modify: `crates/runtime/src/exitus.rs`
- Modify: `crates/runtime/src/fraction.rs`
- Modify: `crates/runtime/src/genus.rs`
- Modify: `crates/runtime/src/lib.rs`
- Modify: `crates/runtime/src/proptests.rs`
- Modify: `crates/runtime/src/roman.rs`
- Modify: `crates/runtime/src/scriptio.rs`
- Modify: `crates/runtime/src/speculum.rs`
- Modify: `crates/runtime/tests/golden.rs`
- Modify: `crates/runtime/tests/data/scriptio-aurea.txt`
- Modify: `crates/abi/src/numeri.rs`
- Modify: `crates/abi/src/textus.rs`
- Modify: `crates/codegen/src/emissio.rs`
- Modify: `crates/codegen/src/target.rs`

- [ ] Write the token-level rename script in the scratchpad: reads the glossary, rewrites identifier tokens only (skips strings, chars, comments), applies workspace-wide for the names this crate defines, and `git mv`s files. Its first run is a dry run printing a diff summary.
- [ ] Run it for runtime; fix `mod` lines and `{inline}` format captures the compiler flags.
- [ ] `cargo nextest run` green. Commit: `refactor(runtime): the runtime's names are English`.

### Task 3: Abi crate
**Does:** Renames the Latin Rust names in `crates/abi`, leaving every `liss_*` symbol and every `c"…"` sentence alone.
**Depends on:** Task 2
**Adds:** nothing new
**Size:** ~800

**Files:**
- Modify: `crates/abi/src/custos.rs`
- Modify: `crates/abi/src/defectus.rs`
- Modify: `crates/abi/src/ingressus.rs`
- Modify: `crates/abi/src/lib.rs`
- Modify: `crates/abi/src/memoria.rs`
- Modify: `crates/abi/src/numeri.rs`
- Modify: `crates/abi/src/scriptio.rs`
- Modify: `crates/abi/src/series.rs`
- Modify: `crates/abi/src/textus.rs`
- Modify: `crates/abi/tests/abi.rs`
- Modify: `crates/abi/tests/custos.rs`
- Modify: `crates/abi/tests/emissio.rs`
- Modify: `crates/abi/Cargo.toml`
- Modify: `crates/codegen/build.rs`

- [ ] Run the script for abi. `custos.rs` test names also live in its `probationes` table: change both together.
- [ ] `[[test]] path` in `Cargo.toml` follows a renamed `tests/custos.rs`.
- [ ] `cargo nextest run` green; `abi.rs`'s `sententiae` still passes unedited. Commit: `refactor(abi): the ABI crate's names are English`.

### Task 4: Ir and latinitas
**Does:** Renames the names in `ir`, and renames the `latinitas` crate to `latin`.
**Depends on:** Task 3
**Adds:** nothing new
**Size:** ~250

**Files:**
- Modify: `crates/ir/src/lib.rs`
- Modify: `crates/ir/src/tests.rs`
- Modify: `crates/latinitas/Cargo.toml`
- Modify: `crates/latinitas/src/analyze.rs`
- Modify: `crates/latinitas/src/lib.rs`
- Modify: `crates/latinitas/src/model.rs`
- Modify: `crates/latinitas/src/whitaker.rs`
- Modify: `crates/latinitas/data/NOTICE`
- Modify: `Cargo.toml`
- Modify: `Cargo.lock`
- Modify: `crates/lingua-serpentis/Cargo.toml`
- Modify: `crates/lingua-serpentis/src/latinity.rs`
- Modify: `crates/lingua-serpentis/tests/catalogus.rs`
- Modify: `crates/lingua-serpentis/tests/robustness.rs`

- [ ] `git mv crates/latinitas crates/latin`; package name, workspace members, the dependency line, `use latinitas::` paths. `Cargo.lock` regenerates; check only the one package name moved.
- [ ] Rename `ir` and `latin` identifiers by the script.
- [ ] `catalogus` snapshot reorders because the path changed: re-bless, then check that the header still reads `121 sites · 73 fixtured · 0 ambiguous · 46 uncovered · 2 unparsed` and that the sorted `(message, status)` set equals the Task 1 baseline.
- [ ] `cargo nextest run` green. Commit: `refactor(latin): latinitas is latin, and the IR's names are English`.

### Task 5: Codegen crate
**Does:** Renames the Latin names in `crates/codegen`, its tests and its source files.
**Depends on:** Task 4
**Adds:** nothing new
**Size:** ~2000

**Files:**
- Modify: `crates/codegen/build.rs`
- Modify: `crates/codegen/src/emissio.rs`
- Modify: `crates/codegen/src/lib.rs`
- Modify: `crates/codegen/src/nexus.rs`
- Modify: `crates/codegen/src/target.rs`
- Modify: `crates/codegen/tests/aemulatio.rs`
- Modify: `crates/codegen/tests/pondus.rs`
- Modify: `crates/codegen/tests/regimen.rs`
- Modify: `crates/codegen/tests/salve.rs`
- Modify: `crates/codegen/tests/scalares.rs`
- Modify: `crates/codegen/tests/series.rs`
- Modify: `crates/codegen/tests/support/mod.rs`
- Modify: `crates/codegen/tests/vocationes.rs`

- [ ] Script for codegen; the harness literals (`"VIA"`, `"emissio"`, `"nuntii"`, `"programma"`) in `aemulatio.rs` are left until Task 8.
- [ ] Use sites in `lingua-serpentis` of codegen's public names are fixed in this task's commit (moved names only).
- [ ] `cargo nextest run` green; `pondus.rs` unedited apart from names. Commit: `refactor(codegen): the back end's names are English`.

### Task 6: Lingua-serpentis source
**Does:** Renames the Latin names in the lexer, parser, checker, CLI and the rest of the front end, and the source files that carry them.
**Depends on:** Task 5
**Adds:** nothing new
**Size:** ~1300

**Files:**
- Modify: `crates/lingua-serpentis/src/ast.rs`
- Modify: `crates/lingua-serpentis/src/bin/liss.rs`
- Modify: `crates/lingua-serpentis/src/checker.rs`
- Modify: `crates/lingua-serpentis/src/cli.rs`
- Modify: `crates/lingua-serpentis/src/error.rs`
- Modify: `crates/lingua-serpentis/src/fragmentum.rs`
- Modify: `crates/lingua-serpentis/src/latinity.rs`
- Modify: `crates/lingua-serpentis/src/lexer.rs`
- Modify: `crates/lingua-serpentis/src/lib.rs`
- Modify: `crates/lingua-serpentis/src/parser.rs`
- Modify: `crates/lingua-serpentis/src/pictura.rs`
- Modify: `crates/lingua-serpentis/src/program.rs`
- Modify: `crates/lingua-serpentis/src/spatium.rs`
- Modify: `crates/lingua-serpentis/src/token.rs`

- [ ] Script for the front end. `src/bin/liss.rs` stays a shim that says nothing. No string literal moves (diagnostics are written inline, spec §10.3).
- [ ] Fix the test files' use sites of the renamed public API (names only); test-private names wait for Task 7.
- [ ] Re-bless `catalogus` as in Task 4 and check its header and sorted set. `cargo nextest run` green. Commit: `refactor(front-end): the compiler front end's names are English`.

### Task 7: Lingua-serpentis tests
**Does:** Renames the test files, test functions, helpers and snapshots in `crates/lingua-serpentis/tests`.
**Depends on:** Task 6
**Adds:** nothing new
**Size:** ~700

**Files:**
- Modify: `crates/lingua-serpentis/Cargo.toml`
- Modify: `crates/lingua-serpentis/tests/catalogus.rs`
- Modify: `crates/lingua-serpentis/tests/cli.rs`
- Modify: `crates/lingua-serpentis/tests/conformance.rs`
- Modify: `crates/lingua-serpentis/tests/generative.rs`
- Modify: `crates/lingua-serpentis/tests/instrumenta.rs`
- Modify: `crates/lingua-serpentis/tests/ir.rs`
- Modify: `crates/lingua-serpentis/tests/liss_cli.rs`
- Modify: `crates/lingua-serpentis/tests/nuntii.rs`
- Modify: `crates/lingua-serpentis/tests/programs.rs`
- Modify: `crates/lingua-serpentis/tests/robustness.rs`
- Modify: `crates/lingua-serpentis/tests/snapshots/catalogus__catalogus.snap`
- Modify: `crates/lingua-serpentis/tests/snapshots/ir__basics.snap`
- Modify: `crates/lingua-serpentis/tests/snapshots/ir__conditionals.snap`
- Modify: `crates/lingua-serpentis/tests/snapshots/ir__fractions.snap`
- Modify: `crates/lingua-serpentis/tests/snapshots/ir__lists.snap`
- Modify: `crates/lingua-serpentis/tests/snapshots/ir__loops.snap`
- Modify: `crates/lingua-serpentis/tests/snapshots/ir__packages.snap`
- Modify: `crates/lingua-serpentis/tests/snapshots/nuntii__nuntii.snap`
- Modify: `crates/lingua-serpentis/tests/support/diag.rs`
- Modify: `crates/lingua-serpentis/tests/support/mod.rs`
- Modify: `crates/lingua-serpentis/tests/support/mutate.rs`
- Modify: `crates/lingua-serpentis/tests/support/proc.rs`

- [ ] Script for the test crate. A renamed test file or fn renames its `.snap` (`<file>__<fn>`); `git mv` the snapshot and keep its body.
- [ ] A test's asserted Latin literal stays; a Latin `expect`/`panic!` message that is the test's own wording becomes English.
- [ ] `cargo nextest run` green. Commit: `refactor(tests): the front-end tests' names are English`.

### Task 8: Conformance corpus and harness
**Does:** Renames the fixture directories, the category names and the files inside each fixture, and the harness code that reads them.
**Depends on:** Task 7
**Adds:** nothing new
**Size:** ~250

**Files:**
- Modify: `conformance`
- Modify: `crates/lingua-serpentis/tests/support/mod.rs`
- Modify: `crates/lingua-serpentis/tests/conformance.rs`
- Modify: `crates/lingua-serpentis/tests/nuntii.rs`
- Modify: `crates/lingua-serpentis/tests/snapshots/nuntii__nuntii.snap`
- Modify: `crates/codegen/tests/aemulatio.rs`

- [ ] By hand, because these are string literals: `CATEGORIES`, `"emissio"`, `"nuntii"`, `"exitus"`, `"programma"`, `"VIA"`, `"NOTA.md"`, and the same in `aemulatio.rs`. `.liss` file names inside fixtures do not move.
- [ ] `git mv` each fixture directory and file by the glossary's table. A `VIA` file's content (a path to a program elsewhere) is updated only where it names a moved path; `examples/` does not move.
- [ ] `nuntii__nuntii.snap` is re-blessed; the sorted multiset of diagnostic sentences must equal the Task 1 baseline, only fixture names differ.
- [ ] `LISS_BLESS` is not used on the corpus: no `nuntii`/`emissio`/`exitus` file's content changes.
- [ ] `cargo nextest run` green (123 conformance trials). Commit: `refactor(conformance): the corpus names its fixtures in English`.

### Task 9: Docs and the final sweep
**Does:** Brings the living documents up to the new names, adds one old→new table for readers of the dated history, and proves nothing Latin was left behind.
**Depends on:** Task 8
**Adds:** nothing new
**Size:** ~500

**Files:**
- Modify: `README.md`
- Modify: `CLAUDE.md`
- Modify: `docs/README.md`
- Modify: `docs/via.md`
- Modify: `docs/internals/abi.md`
- Modify: `docs/internals/codegen.md`
- Modify: `docs/internals/rationale.md`
- Modify: `docs/internals/testing.md`

- [ ] Replace old identifiers, paths, fixture names and test IDs in those eight files by the glossary. Dated specs, plans and `docs/defectus` are untouched. `docs/README.md` gets one section: old name → new name for crate, source files and the fixture layout.
- [ ] `CLAUDE.md`: new paths in its invariants (`cli.rs` stays), the catalogus header line, the fixture layout names.
- [ ] Sweep: a script lists remaining identifier tokens, filenames and directory names that the glossary's Latin side still contains; the only hits allowed are the frozen list.
- [ ] Sweep: `git diff 74ba79c -- '*.snap' conformance` shows only renames and moved names; the catalogus pair set and the nuntii sentence multiset equal the baselines.
- [ ] Full `cargo nextest run`: 686 pass. Commit: `docs(rename): the living docs use the English names`.
