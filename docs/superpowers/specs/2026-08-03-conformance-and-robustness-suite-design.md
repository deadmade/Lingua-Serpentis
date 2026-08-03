# Lingua Serpentis — Conformance and Robustness Suite Design

- **Date:** 2026-08-03
- **Status:** Proposed
- **Author:** Manuel (with Claude)
- **Relationship:** Replaces the ad-hoc §6 "Testing Strategy" sections of the
  feature designs with one standing, program-level suite. Adds no language
  features and **changes no interpreter behaviour** — every defect it finds is
  recorded, not fixed (§8).

## 1. Vision

The language has 242 tests and they are the wrong shape. They test *functions*,
not *the language*: 211 of them live inside `#[cfg(test)]` modules and call
`tokenize`, `check`, `execute` directly, and the 31 integration tests call the
library API. Exactly **one** test in the workspace asserts a complete rendered
diagnostic end to end. **Zero** tests run the binary a user actually runs.

The consequence is not that the tests are weak in general — arithmetic, Roman
numerals and fractions are covered thoroughly. The consequence is specific:
**the error surface is nearly untested**, and it is the largest surface the
language has. Thirty-seven assertions say only `is_err()`. Of 78 distinct
diagnostics, ~32 have no test at all and ~17 are reached only by an `is_err()`
that never looks at the message. Two tests pass for the wrong reason — they
assert `is_err()` on a program that fails one line earlier than intended, so
both sites of `index seriei 'ni' esse debet` are in fact unreached.

A language is judged by its errors. When a user writes `°X/°II` and is told
`numerus Romanus invalidus: 'X/'`, or writes a multi-line string and gets every
later error attributed two lines too early, the implementation is wrong in the
way that matters most and no test notices.

This design adds a suite built around three principles:

- **Fixtures are programs, not snippets.** This language's error model is about
  file attribution (`Error in 'mathematica/summa.liss', linea 2:`). A test case
  must be able to be a folder.
- **Assert the whole diagnostic.** `contains("typ")` matches a dozen unrelated
  errors. The suite compares complete rendered output — stdout, stderr, exit
  code — or it isn't testing the diagnostic.
- **The catalogue is the ratchet.** A one-off audit rots. The suite carries an
  enumerated catalogue of every diagnostic the source can emit, and fails when a
  new one arrives without a fixture.

## 2. Goals and Non-Goals

### Goals

- **Cover the diagnostic surface**: a fixture per reachable diagnostic, asserting
  the full rendered message, the file attribution and the line number (§4).
- **Test the binary**, not just the library: argv, extensions, the stdout/stderr
  split, real process exit status (§6).
- **Establish an invariant against crashes**: no input may make the process
  abort, panic, hang, or emit a malformed diagnostic (§5).
- **Keep the surface from regrowing**: the catalogue snapshot fails on any new
  unfixtured diagnostic (§7).
- **Record every defect** with a repro, the responsible `file:line`, and a
  fixture pinning today's behaviour so a future fix has an acceptance test (§8).

### Non-Goals

- **No behaviour changes.** Not the stack overflows, not the exit-status
  truncation, not the two English strings in `main.rs`. Each is documented; the
  fix is a separate decision (§8).
- **No rewrite of the existing 242 tests.** They stay. The legacy-API problem
  they embody is recorded as a defect (D-19), not resolved here.
- **No performance benchmarking.** The unbounded `x = x * x` hang is recorded as
  a defect with a timeout guard, not turned into a benchmark suite.
- **No new language features**, including the ones the defects invite (numeral
  form validation, escape sequences, a recursion limit).

## 3. Key Decisions and Rationale

### 3.1 The suite lives in the `lingua-serpentis` package; the corpus does not

`CARGO_BIN_EXE_<name>` is set **only** for integration tests in the binary's own
package, and Cargo builds the binary artifact **only** because that package's
integration tests need it. Verified in a throwaway workspace:

```
sibling crate:   CARGO_BIN_EXE_app = None
                 target/debug/app exists = false   (even after `cargo test --workspace`)
same package:    CARGO_BIN_EXE_app = Some(".../target/debug/app")
```

A separate `crates/conformance` package would therefore need hand-rolled path
discovery from `current_exe()` and would run against a stale or absent binary
under `cargo test -p conformance` — green because it tested nothing. The usual
reason to split (keeping `insta`/`proptest`/`libtest-mimic` out of the graph)
does not apply: dev-dependencies never reach consumers.

The **fixture corpus** is a different matter. It is a language artifact, not a
Rust one, and belongs at the workspace root where it can be read without
knowing Rust.

### 3.2 In-process by default; subprocess only where required

Running a fixture through `run_dir_with_warnings` is ~100× faster than spawning
the binary and yields the same stdout, exit status and warnings. A subprocess is
*required* only for what the library cannot observe:

| needs a subprocess | reason |
|---|---|
| stack overflow, allocation failure | abort kills the test process; `catch_unwind` does not catch `SIGABRT` |
| broken pipe | Rust ignores `SIGPIPE`; `print!` panics |
| real process exit status | `run_with_warnings` discards the `u8` (`lib.rs:88`) |
| stdout/stderr routing | only `main.rs` splits the streams |
| argv, file extension, unreadable file | `main.rs` only |

So `conformance.rs` runs in-process, and `cli.rs` and `robustness.rs` spawn the
binary. Curated fixtures are safe in-process because they are curated; the
fuzzer, which is not, is isolated per input.

### 3.3 Characterization vs. specification, marked explicitly

Some assertions pin behaviour we want (`Error in 'a.liss', linea 2: …`). Others
pin behaviour we consider wrong but are not fixing (`expectatum: Semicolon`).
Freezing both in the same tree without distinction would make the suite an
argument that the bugs are correct. The `defectus/` category exists to keep the
two separate: a fixture there asserts today's output and carries a `NOTA.md`
stating what it *should* be and which defect ID it belongs to.

### 3.4 Fixtures are data, and blessing them is one command

`LISS_BLESS=1 cargo test --test conformance` rewrites every expected file from
actual output. Adding a case means adding a folder and blessing it, then reading
the diff — not writing Rust. This is what makes covering ~78 diagnostics
tractable, and it is why the expected output lives in files next to the program
rather than in `insta` snapshots keyed by test name.

### Vocabulary

The corpus uses the language's own Latin, consistent with `examples/`:

| directory | meaning |
|---|---|
| `currunt` | programs that run — "they run" |
| `errores` | programs that fail |
| `monita` | programs that warn but run |
| `defectus` | known defects, pinning today's behaviour |
| `programma/` | the program folder handed to the runner |
| `emissio` | expected stdout |
| `nuntii` | expected stderr — "messages", covering both `Monitum` and `Error` lines |
| `exitus` | expected exit status |

`nuntii` rather than the obvious `errores` because `errores` is already a
*category* directory; a file of that name inside it would read as a nested
category.

## 4. The Conformance Corpus

### 4.1 Layout

```
Lingua-Serpentis/
├── conformance/                       ← the corpus, at workspace root
│   ├── currunt/
│   │   └── <casus>/
│   │       ├── programma/             ← handed to the runner as a program folder
│   │       │   ├── principale.liss
│   │       │   └── mathematica/summa.liss
│   │       ├── emissio                ← expected stdout   (absent ⇒ empty)
│   │       ├── nuntii                 ← expected stderr   (absent ⇒ empty)
│   │       ├── exitus                 ← expected exit code (absent ⇒ 0)
│   │       └── NOTA.md                ← optional prose: why this case exists
│   ├── errores/
│   ├── monita/
│   └── defectus/
└── crates/lingua-serpentis/tests/
    ├── support/mod.rs
    ├── conformance.rs
    ├── cli.rs
    ├── robustness.rs
    └── catalogus.rs
```

Corpus path: `concat!(env!("CARGO_MANIFEST_DIR"), "/../../conformance")`.
Binary path: `env!("CARGO_BIN_EXE_lingua-serpentis")`.

A single-file case is a `programma/` containing one file. There is no separate
single-file format — the degenerate case stays degenerate.

### 4.2 The runner

`conformance.rs` uses `libtest-mimic` so each fixture is a **named test**:

```
$ cargo test --test conformance errores::
    errores::typus-mixtus ......... ok
    errores::index-extra-fines .... FAILED
```

For each fixture: load `programma/` via `program::assemble` (paths derived from
the folder, `/`-separated), run `run_loaded`, render stdout / stderr / exit code
exactly as `main.rs` does, and compare all three against the expected files. The
rendering must be shared with `main.rs` rather than reimplemented — see §6.1.

Failure output shows a unified diff of each stream that differs.

### 4.3 What the corpus must cover

Derived from the survey; the catalogue (§7) is the authority on completeness.

- **`errores/`** — one case per reachable diagnostic across lexer, parser,
  checker, interpreter and program assembly. Each asserts the complete rendered
  line, including the file attribution and line number. Multi-file cases assert
  that the error names the *right* file, not merely some file.
- **Line-number cases specifically.** No test in the workspace currently asserts
  a parser or checker line number on a multi-line source. Every `errores/` case
  that produces a line number places it on a line other than 1. (Structural
  errors — `Error: munus 'initium' in programmate non inventum` — carry no line
  by design and are exempt.)
- **`monita/`** — warning cases, including the ordering across files that
  `lint_program` produces emergently (package-name warnings with no file and no
  line first, then root files by path, then package files), which no test has
  ever exercised because no test has warnings from two files at once.
- **`currunt/`** — every shipped example, including `salve.liss`, which is
  currently referenced by nothing. `computatrum/` is picked up as a folder, so a
  new file added to it joins the suite automatically — unlike today's
  file-by-file `include_str!`.
- **`defectus/`** — §8.

## 5. The Robustness Invariant

`robustness.rs` asserts one property over many inputs, each in its own process:

> Within a timeout, the process terminates with either exit 0 and a stderr
> containing only `Monitum` lines, or exit 1 and a stderr whose final line is a
> **well-formed diagnostic**. Any other outcome is a defect.

Well-formed means all of:

1. matches `^Error( in '<path>')?(, linea \d+)?: .+$`;
2. if a line number is present, that line exists in the named file;
3. if a path is present, that file is part of the program;
4. the message contains no ASCII digits — roman-native-integers §1 promises "no
   Arabic number anywhere a user can see";
5. the message contains no Rust type name (`Semicolon`, `RParen`, `LBrace`, …);
6. the message contains no English (denylist: `file`, `must`, `error reading`,
   `extension`, …).

Conditions 4–6 fail against today's implementation on known defects (D-02, D-11,
D-12). They are expressed as invariants with a **defect allowlist keyed by ID**,
so the allowlist shrinks as fixes land rather than the assertion being weakened.

Input sources:

1. **Corpus** — every `.liss` under `examples/` and `conformance/`.
2. **Mutations** of the corpus, deterministic under a fixed seed: byte
   delete/duplicate/swap; token delete/duplicate/swap; truncation at each token
   boundary; bracket unbalancing; `°` removal; digit substitution.
3. **`proptest` generators** — (a) token soup, to drive the lexer and parser;
   (b) well-typed-ish ASTs rendered to source, which is the only way to reach
   the checker and interpreter in depth. Shrinking is the reason `proptest`
   earns its place: a failing generated program shrinks to a minimal repro.
4. **Curated pathologies** — the confirmed crashes plus depth and size sweeps:
   recursion depth, expression nesting, numeral length, list length, file count,
   package count, identifier length, import-cycle depth.

Budget: a few hundred inputs at a fixed seed by default so `cargo test` stays
fast; `LISS_FUZZ_CASES` and `LISS_FUZZ_SEED` for a soak.

## 6. The CLI Suite

`cli.rs` covers what the fixture format cannot express, one named test each,
every one spawning the binary:

- argv arity (0, 1, 2, 3 arguments);
- `.liss` extension enforcement, and its interaction with `is_dir()` — a
  *directory* named `x.liss` runs as a program folder, and a nonexistent
  directory argument is reported as `Error: file must have .liss extension`;
- missing file, unreadable file (`chmod 000`), non-UTF-8 file content, non-UTF-8
  argv (`main.rs:23` uses `env::args()`, which panics);
- the **stdout/stderr split** under separate redirection — the stated contract
  is that warnings never contaminate piped output;
- **real process exit status**: 0, 1, and `initium`'s return value, including
  out-of-range and negative;
- warnings emitted before program output, and their ordering;
- broken pipe (`| head -1` on large output);
- `run_dir_with_warnings` end to end — currently one caller, zero tests: hidden
  directories not skipped, `.liss` filtering existing only in `load_dir` and not
  in `assemble`, no path deduplication, and structural validation running before
  the sorts (so with two structural problems, *which* error surfaces depends on
  filesystem iteration order).

### 6.1 One extraction refactor

`main.rs` currently inlines the rendering of warnings, output and exit status
into `finish()`. The suite must assert exactly what a user sees, so that
rendering moves to a small library function that both `main.rs` and the suite
call. This is the **only** production-code change in this design, it is a pure
extraction, and it is what keeps §4.2's in-process runner honest.

## 7. The Catalogue Ratchet

`catalogus.rs` extracts every message-producing site from `crates/*/src/*.rs`
(constructions of `LinguaError`, `ProgramError`, `Warning`, and the `eprintln!`s
in `main.rs`) and snapshots the catalogue via `insta`:

```
78 diagnostics · 61 fixtured · 13 uncovered · 4 dead
  checker.rs:45   functio '{}' numerum argumentorum falsum accepit
                    → errores/numerus-argumentorum
  parser.rs:28    expectatum: {:?}
                    → defectus/D-02-expectatum-nomen-rusticum
  checker.rs:118  fasciculus '{}' non inventus
                    → DEAD: no input reaches this branch
  lexer.rs:57     littera invalida
                    → UNCOVERED
```

A new diagnostic added without a fixture changes the snapshot and fails review.
`UNCOVERED` and `DEAD` are legal states, but they are *visible* states — the
count in the header is the number the project is choosing to live with.

The extraction is a grep, so it is approximate by construction: it can miss an
unusual construction shape. That is acceptable because its failure mode is a
visible snapshot diff, not a silent pass. Sites it cannot classify are listed
as `UNPARSED` rather than dropped.

## 8. Defect Handling

Every problem found is recorded in `docs/defectus/2026-08-03-index.md`:

```markdown
### D-07 · `abrumpe` extra iterationem tacite ignoratur
- **Severity:** medium — silently wrong behaviour, no diagnostic
- **Repro:** `conformance/defectus/D-07-abrumpe-extra-iterationem/`
- **Observed:** prints `I` then `II`; exit 0
- **Expected:** an error naming `abrumpe` outside a loop
- **Cause:** `checker.rs:449` — `StmtKind::Break | StmtKind::Continue => Ok(())`,
  no loop-depth tracking; `interp.rs:165` matches only `Flow::Return`.
```

Each gets a `defectus/` fixture pinning **current** behaviour, so `cargo test`
is green and the defect is executable. Fixing one makes its fixture fail loudly;
the fix then moves the fixture into `errores/` with the correct expectation. The
defect list is the fix's acceptance criteria.

### 8.1 Confirmed defects

All reproduced against a fresh `target/debug/lingua-serpentis` on 2026-08-03.

| ID | defect | observed |
|---|---|---|
| D-01 | No all-paths-return analysis; the type system is unsound | `munus nomen reddit voc { si (falsus) { redde "a"; } }` → prints `nullus`, exit 0. `interp.rs:167` fabricates `Int(zero)` for any missing return |
| D-02 | Parser leaks Rust type names | `expectatum: Semicolon` — `parser.rs:28` prints `TokenKind` `Debug`. Renaming a variant silently changes user-facing text in 7 places |
| D-03 | Line numbers wrong after a multi-line string | `lexer.rs:43` never increments `line` inside string bodies; every later diagnostic *and warning* is attributed N lines early |
| D-04 | Unbounded recursion aborts | `stack overflow, aborting`, SIGABRT, exit 134 — outside the error model entirely; an embedder cannot survive it |
| D-05 | Deep expression nesting aborts | ~20k nested parens → stack overflow in the parser |
| D-06 | Broken pipe panics | `prog | head` → `failed printing to stdout: Broken pipe`, exit 101. `main.rs:12` uses `print!`; Rust ignores `SIGPIPE` |
| D-07 | `abrumpe`/`continua` outside a loop is a silent no-op | prints both sides; `checker.rs:449` has no loop-depth tracking |
| D-08 | A runtime error discards everything already printed | `scribe("ante")` then divide-by-zero → **empty** stdout; `scribe` accumulates into `Interp::out` and the `Err` arm never prints it |
| D-09 | Any error discards all Latinity warnings | `lint_program` runs at `lib.rs:43`; warnings return only on the `Ok` arm at `:45` |
| D-10 | Exit status silently truncated | `redde °CCC` → exit 116; `redde -°I` → exit 255 |
| D-11 | Two English diagnostics | `Error: file must have .liss extension`, `Error reading file: {e}` — one line below the Latin `Usus:`. The second is also *wrong* for a nonexistent **directory** argument |
| D-12 | Arabic numerals in a user-visible message | `index extra fines: III (longitudo 2)` — index via `Roman::display()`, length via `usize` |
| D-13 | Numeral form never validated | `°IIX + °VV + °IM` → `MXIX` (10 + 10 + 999) |
| D-14 | `°X/°II` lexes as one numeral | `numerus Romanus invalidus: 'X/'`; spaced, it prints `V`. Whitespace is silently load-bearing. Same for `°V.summa` |
| D-15 | Arabic digits get the catch-all | `ni x = 5;` → `signum ignotum: '5'`, with no hint that `°V` exists — the likeliest beginner error |
| D-16 | Duplicate parameter names accepted | `capit (ni a, voc a)` — both checker and interpreter keep the last binding; the body silently sees the `voc` |
| D-17 | A stray hidden directory kills the program | `.git/x.liss` → `Error: nomen fasciculi '.git' non validum est`; hidden directories are not skipped |
| D-18 | `examples/calc.liss` warns | on `summare`. latin-identifier-checker §12 leaves open whether programmer-Latin infinitives need a supplemental word list, to be added "only if a real example fails" — one does, and the supplement was never added. `programs.rs` uses `run()`, which discards warnings, so nothing notices |
| D-19 | ~50 unit tests run against the legacy `parser::parse` / `checker::check` / `interp::execute` API | which still accepts top-level statements and `initium`-less scripts, has no duplicate-function check and no `initium` requirement — a laxer language than the compiler implements. Recorded only; not resolved (§2) |
| D-20 | `x = x * x` in a loop runs forever | `Roman` is arbitrary-precision, so there is no overflow; `mag_mul` is super-quadratic and `display` quadratic again |
| D-21 | `mag_divmod`'s zero guard is a `debug_assert` | `roman.rs:110` — in release a zero divisor spins the doubling loop forever. Every caller guards today |

Further candidates surfaced by the survey — unreachable branches, duplicated
Latin phrasings for one condition, the `dum` init-slot holes, Unicode
identifiers, missing escape sequences, builtin shadowing — are triaged into this
table during implementation as each is reproduced.

## 9. Dependencies

Added to `crates/lingua-serpentis/Cargo.toml` under `[dev-dependencies]` only:

| crate | why |
|---|---|
| `libtest-mimic` | one named test per fixture; `cargo test --test conformance errores::` filters |
| `insta` | the catalogue snapshot, with `cargo insta review` |
| `proptest` | shrinking — a hand-rolled generator finds the bug but hands back a 400-line program |

Runtime dependencies stay at zero. Fixture walking, diffing, blessing and the
subprocess runner are hand-rolled against `std`.

## 10. Success Criteria

- `cargo test` is green, including all `defectus/` fixtures.
- The catalogue header shows every diagnostic either fixtured, `UNCOVERED` with
  a reason, or `DEAD` with a defect ID.
- Every `errores/` fixture asserts a complete rendered diagnostic; every one
  that carries a line number places it on a line other than 1.
- `docs/defectus/2026-08-03-index.md` has an entry with a repro for every defect
  in §8.1, and every entry has a matching fixture.
- The robustness invariant holds over the default budget, with every exception
  carrying a defect ID.
- No interpreter behaviour changed; the only production edit is the §6.1
  rendering extraction.

## 11. Open Questions

None blocking. Two to settle during implementation:

1. **Catalogue extraction fidelity** — whether the grep needs to become a real
   parse. Start with the grep; if `UNPARSED` entries accumulate, revisit.
2. **Fuzz budget in CI** — the default seed keeps `cargo test` fast, but nothing
   here schedules a soak. Out of scope until there is CI to schedule it in.
