# Lingua Serpentis — Latin Identifier Checker (`latinitas`) Design

- **Date:** 2026-07-19
- **Status:** Approved (design) — pending implementation plan
- **Author:** Manuel (with Claude)

## 1. Vision

Lingua Serpentis already speaks Latin at the *keyword* level (`si`, `munus`,
`scribe`, `redde`). But the names the programmer invents — function names,
variables, parameters — can be anything: `summa` sits next to `a`, `b`, `c`, and
nothing stops `getData` or `myVar`.

This feature closes that gap with a **warning-only Latinity checker**: a pass
that flags user-defined identifiers which are not recognizable Latin words, so
the whole program *reads* as Latin, not just its keywords. It never blocks
execution — it nudges.

The interesting part is *how* it decides a word is Latin. Rather than a flat word
list, it uses a real **morphological analyzer**: it understands Latin
inflection, so `numerus`, `numerum`, and `numeri` all resolve to the same stem
and are all accepted. The analyzer is backed by **William Whitaker's WORDS**, the
public-domain ~39,000-entry Latin dictionary, and lives in its own reusable
crate, `latinitas`.

## 2. Goals and Non-Goals

### Goals
- Warn on user-defined identifiers that are not valid Latin words, with the line
  number, phrased in Latin.
- Never fail a program because of a naming warning (warning-only).
- Understand inflection: accept any declined/conjugated form of a real Latin
  word, not just dictionary lemmas.
- Keep warnings **trustworthy** — a warning should almost always mean a genuinely
  non-Latin name, so the signal is worth listening to. (This is the whole reason
  for backing it with the full Whitaker dictionary rather than a small list.)
- Ship the morphology engine as a **self-contained, reusable crate** with a
  minimal `&str -> bool` interface and no dependency on the interpreter.

### Non-Goals (v1)
- Blocking / hard-error enforcement. (Possible later; deliberately not now.)
- Checking anything other than user-defined identifiers — keywords, types, and
  Roman numerals are out of scope (keywords are Latin by construction).
- Suggesting corrections ("did you mean `numerus`?").
- Full grammatical validation of a name against context (case agreement, etc.).
  We check *"is this a Latin word form?"*, not *"is it the right form here?"*.
- A configuration system for the allow-list beyond the single-letter exemption.

These are recorded as **v2 candidates** in §9, not rejected forever.

## 3. Key Decisions and Rationale

| Decision | Choice | Why |
|---|---|---|
| Enforcement | **Warning-only** (never blocks) | Keeps the language playful and writable; the user opted for a nudge, not a gate. |
| Validation method | **Morphological analysis** (stem + ending), not a flat word list | Latin is heavily inflected; a lemma list would wrongly flag `numerum`/`numeri`. The analyzer understands inflection. |
| Dictionary | **Full Whitaker's WORDS (~39k)**, public domain | Coverage is what makes warnings trustworthy. A small curated seed produces frequent false "not Latin" warnings, which trains the user to ignore *all* warnings. |
| Packaging | **Separate `latinitas` crate** in a Cargo **virtual workspace** | The engine has a clean `&str -> bool` interface with zero interpreter dependency. Isolating it gives build isolation for the ~MBs of data, independent testing, and a genuinely publishable artifact. |
| Which identifiers | **Functions, variables, and parameters** — all declaration sites | The user wants the whole program to read as Latin. |
| Short-name exemption | **Single-character names exempt** (`a`, `b`, `i`) | Conventional math/loop counters; checking them would be pure noise. |
| Data delivery | **Vendor raw Whitaker files + `build.rs` preprocessor** | Reproducible and self-contained (no build-time network, no submodule). Fiddly fixed-width parsing happens once at build time, off the runtime path. |
| Matching strictness | **Start lenient** (stem exists + ending valid for that stem's class); tighten with Whitaker stem-slot keys only if false positives appear | Warning-only means slight over-acceptance is harmless; correctness-of-rejection matters more than correctness-of-acceptance. |

## 4. Architecture

The checker is a **new walk over the AST**, inserted after the type checker
(walk #1) and before the interpreter (walk #2):

```
source (.liss)
   │  lexer → parser
   ▼
AST ── Vec<Stmt>
   │  walk #1: type checker    ──►  Latin error + line (halts on failure)
   ▼
AST (validated)
   │  NEW walk: latinity::lint ──►  Vec<Warning> (printed to stderr, NON-fatal)
   ▼
AST (validated)
   │  walk #2: interpreter     ──►  program output
   ▼
program output
```

- Type errors still abort first: you fix real errors before seeing style nags.
- The lint pass only runs on a **type-valid** program, and its result never
  changes whether the program runs — warnings are printed, then execution
  proceeds exactly as today.

### 4.1 Two-part split

The feature spans the workspace along its natural seam:

| Concern | Where | Depends on |
|---|---|---|
| *"Is this string a Latin word?"* — `is_latin()`, the engine, the dictionary tables | `latinitas` crate | nothing (leaf crate) |
| *"Which names in this program should I check, and where?"* — `lint()`, AST walk, `Warning` emission | `lingua-serpentis` crate (`src/latinity.rs`) | `latinitas` + `ast` |

Dependency direction is strictly one-way: `lingua-serpentis` → `latinitas`.

## 5. Workspace Layout

Convert the repository to a **virtual workspace**: the root `Cargo.toml` contains
only `[workspace]` (no `[package]`, no root `src/`), and both crates live under
`crates/`. Documentation, examples, and sample data stay at the repository root.

```
Lingua-Serpentis/
  Cargo.toml                 # [workspace] only; default-members = interpreter
  Cargo.lock
  README.md, docs/, .superpowers/
  examples/                  # *.liss — STAY AT ROOT (language samples, not crate examples)
  crates/
    lingua-serpentis/        # the interpreter (moved here from root, unchanged)
      Cargo.toml
      src/
        main.rs, lib.rs, lexer.rs, token.rs, ast.rs, parser.rs,
        checker.rs, interp.rs, value.rs, roman.rs, error.rs
        latinity.rs          # NEW: the lint pass (AST walk → Vec<Warning>)
      tests/
        programs.rs          # existing integration tests, unchanged
    latinitas/               # NEW crate: the Latin morphology engine
      Cargo.toml
      build.rs               # parses vendored Whitaker data → OUT_DIR table
      data/
        DICTLINE.GEN         # vendored, public domain
        INFLECTS.LAT         # vendored, public domain
        NOTICE               # attribution: William Whitaker's WORDS (public domain)
      src/
        lib.rs               # public API: is_latin()
        analyze.rs           # the matching algorithm
        model.rs             # Class / Declension / Conjugation enums, table types
```

### 5.1 Root `Cargo.toml`

```toml
[workspace]
resolver = "2"
members = ["crates/lingua-serpentis", "crates/latinitas"]
default-members = ["crates/lingua-serpentis"]
```

`default-members` ensures `cargo run -- examples/calc.liss` and `cargo test` from
the repository root behave exactly as they do today.

### 5.2 Migration

Moving the existing package into `crates/lingua-serpentis/` is a mechanical
`git mv` of `src/`, `tests/`, and `Cargo.toml`. The interpreter code does not
change — the integration tests already drive it through the public library API.
The examples remain at the root; `examples/*.liss` are passed to the binary by
path (they are sample programs, not Cargo `examples/` targets). Verification:
`cargo build`, `cargo test`, and `cargo run -- examples/calc.liss` all pass after
the move, before any new code is written.

## 6. The `latinitas` Crate

### 6.1 Public API

```rust
/// Returns true if `word` is a recognizable Latin word form (any inflection of
/// a dictionary entry, an indeclinable, or a known irregular/unique form).
/// Input is normalized internally (see §6.4). Empty input returns false.
pub fn is_latin(word: &str) -> bool;
```

One function, `&str -> bool`. No external dependencies. This is the entire
contract the interpreter depends on; everything else in the crate is private and
free to change (dictionary internals, table format, matching heuristics).

A richer `analyze(word) -> Option<Analysis>` (returning stem, part of speech,
possible cases) is a natural future extension but is **not** part of v1 — `bool`
is all the checker needs.

### 6.2 The Whitaker data

- **Source:** William Whitaker's WORDS, released into the **public domain**.
  Vendored from a maintained mirror (e.g. `github.com/mk270/whitakers-words`).
  Two files: `DICTLINE.GEN` (dictionary stems + part-of-speech/inflection codes)
  and `INFLECTS.LAT` (the inflection endings and the classes they apply to).
- **Attribution:** a `data/NOTICE` file credits William Whitaker and records the
  public-domain status and the mirror URL.
- **Size:** a few MB of text committed to git. This is the one real cost of the
  feature and is accepted deliberately in exchange for warning trustworthiness.

### 6.3 Build-time preprocessing (`build.rs`)

Whitaker's files are fixed-width, Ada-era formats. Parsing them at runtime would
be slow and couple the runtime to their quirks. Instead, `build.rs` runs **once
at build time** (re-run only when the data or `build.rs` changes, via
`cargo:rerun-if-changed`) and:

1. Parses `DICTLINE.GEN` into stem entries. Each entry carries up to four stems
   and a part-of-speech + declension/conjugation code.
2. Parses `INFLECTS.LAT` into endings, each tagged with the part of speech and
   the declension/conjugation (+ variant) it applies to.
3. Maps Whitaker's POS and declension/conjugation codes onto our compact
   `Class` model (§6.5).
4. Emits a compact table (Rust source or a parseable static blob) into
   `OUT_DIR`, `include!`-ed by the crate at compile time.

Emitted tables:

- `STEMS`: `stem → { InflectionGroup }` — every dictionary stem and the
  inflection group(s) it can take.
- `ENDINGS`: `InflectionGroup → { ending string }`.
- `UNIQUES`: whole-word irregular forms Whitaker lists explicitly (e.g. forms of
  *esse*), matched directly.
- Indeclinables (adverbs, prepositions, conjunctions, interjections, some
  numerals) are modeled as a group whose only ending is `""`, so the stem *is*
  the whole word — this unifies them with the general algorithm.

### 6.4 Normalization

Because the dictionary is stored in classical orthography, `is_latin` normalizes
input to match before lookup:

- lowercase;
- `j → i` and `v → u` (classical Latin has no distinct j/v).

Normalization is applied identically to input and to the generated tables, so the
two always agree. (Underscores and digits, which never appear in Latin words,
therefore cause a normalized word to fail lookup — correctly flagged.)

### 6.5 Data model

```rust
// model.rs
enum Class {
    Noun(Declension),        // 1..5
    Adjective(Declension),   // declension family used for endings
    Verb(Conjugation),       // 1..4
    Indeclinable,            // adverb / prep / conj / interj / indeclinable numeral
}
enum Declension  { D1, D2, D3, D4, D5 }
enum Conjugation { C1, C2, C3, C4 }
```

`InflectionGroup` is the join key between `STEMS` and `ENDINGS` (derived from
`Class` plus Whitaker's variant number). The exact shape is an internal detail of
the generator and the tables; the public API exposes none of it.

### 6.6 Matching algorithm (`analyze.rs`)

```
is_latin(word):
    w = normalize(word)
    if w is empty: return false
    if w in UNIQUES: return true
    for split point i in 0..=len(w):
        stem   = w[..i]
        ending = w[i..]
        for group g in STEMS.get(stem):
            if ending in ENDINGS[g]:
                return true
    return false
```

- Identifiers are short, so the brute-force scan over split points is trivially
  fast; `STEMS`/`ENDINGS` are hash lookups.
- **Lenient by design:** we accept if *some* stem of *some* entry matches with a
  valid ending for that entry's class. We do not (in v1) enforce Whitaker's
  stem-slot key that says *which* of the up-to-four stems a given ending group
  attaches to. This can over-accept a few malformed combinations, which is
  acceptable for a warning-only checker. Tightening to stem-slot precision is a
  localized future change (§9).

## 7. The Lint Pass (`src/latinity.rs`)

### 7.1 `Warning` type (`src/error.rs`)

A new type parallel to `LinguaError`, so warnings never masquerade as errors:

```rust
pub struct Warning { pub message: String, pub line: usize }

impl Display for Warning {  // "Monitum in linea {line}: {message}"
    // e.g. Monitum in linea 3: nomen 'result' fortasse non Latinum est
}
```

`Monitum` = "warning/reminder"; `nomen '…' fortasse non Latinum est` = "the name
'…' is perhaps not Latin".

### 7.2 `lint`

```rust
pub fn lint(program: &[Stmt]) -> Vec<Warning>;
```

Walks the AST collecting **declaration sites only** (never uses, to avoid
duplicate warnings for the same name):

- `StmtKind::Func` → the function name **and** each parameter name;
- `StmtKind::Let` → the variable name;
- recursion into every nested block: `if`/`else`, `while`, `for` (including the
  `for` init `let`), and function bodies.

For each collected `(name, line)`:

```
if name.chars().count() > 1 && !latinitas::is_latin(name):
    push Warning { "nomen '{name}' fortasse non Latinum est", line }
```

Single-character names are skipped (the exemption). The returned vector is sorted
by line for stable, readable output. De-duplication of repeated names is **not**
required in v1 (each declaration site is reported once; the same name declared in
two scopes yields two warnings, which is acceptable and arguably correct).

## 8. Pipeline Wiring

### 8.1 Library (`src/lib.rs`)

```rust
/// Full pipeline, returning program output plus any Latinity warnings.
pub fn run_with_warnings(src: &str) -> LResult<(String, Vec<Warning>)> {
    let tokens  = lexer::tokenize(src)?;
    let program = parser::parse(&tokens)?;
    checker::check(&program)?;
    let warnings = latinity::lint(&program);   // never fails
    let output   = interp::execute(&program)?;
    Ok((output, warnings))
}

/// Backwards-compatible: output only, warnings discarded.
pub fn run(src: &str) -> LResult<String> {
    run_with_warnings(src).map(|(out, _)| out)
}
```

Keeping `run` as a thin wrapper means **every existing unit and integration test
continues to pass unchanged**.

### 8.2 Binary (`src/main.rs`)

Switch the driver to `run_with_warnings`; print each warning to **stderr** (so it
never pollutes captured program output on stdout), then print program output to
stdout:

```rust
match run_with_warnings(&src) {
    Ok((output, warnings)) => {
        for w in &warnings { eprintln!("{w}"); }
        print!("{output}");
    }
    Err(e) => { eprintln!("{e}"); exit(1); }
}
```

Warnings on stderr, output on stdout, exit code unchanged (0 even with warnings).

## 9. Testing Strategy (TDD)

Tests are written before the implementation they cover.

### 9.1 `latinitas` unit tests
- **Positives:** an inflected noun set (`numerus`, `numerum`, `numeri`), a verb
  form (e.g. an infinitive and a conjugated form), an adjective, an indeclinable
  (e.g. an adverb), and a *unique*/irregular form.
- **Negatives:** `result`, `myVar`, `getData`, `xyz`, `foobar`, and a name with a
  digit/underscore.
- **Normalization:** a word written with `v`/`j` resolves the same as its `u`/`i`
  form.
- **Edge:** empty string → false.

### 9.2 `latinity::lint` unit tests
- Single-letter names (`a`, `b`, `i`) produce **no** warnings.
- A non-Latin function name, parameter, and `let` variable each produce exactly
  one warning at the correct line.
- Names inside nested blocks (if/while/for/function bodies) are reached.
- A fully-Latin program produces zero warnings.

### 9.3 Integration tests (`tests/programs.rs`)
- A program containing a non-Latin identifier, run via `run_with_warnings`,
  yields the expected warning **and still produces the correct output** — the
  proof that the feature is warning-only.
- Existing tests remain green (they call `run`, unaffected).

Run via `cargo test` from the workspace root.

## 10. Relationship to Existing Code

- The interpreter, checker, lexer, parser, and Roman-numeral logic are
  **unchanged** except for their new home under `crates/lingua-serpentis/` and
  the small `lib.rs`/`main.rs` wiring in §8.
- No existing test is modified; the only additions are new tests.
- Example programs (`examples/*.liss`) are unchanged and stay at the root. (Note:
  `calc.liss` uses `a`, `b`, `c`, which are exempt, and `summare`/`summa`, which
  the dictionary should accept — so it should lint cleanly.)

## 11. Roadmap

### v1 milestones (build order)
1. Convert to virtual workspace; move interpreter into `crates/lingua-serpentis/`;
   verify `build`/`test`/`run` all pass with no code changes.
2. Scaffold the `latinitas` crate with a stub `is_latin` and its data model;
   write the analyzer unit tests (red).
3. Vendor Whitaker `DICTLINE.GEN` / `INFLECTS.LAT` + `NOTICE`.
4. `build.rs`: parse the data → emit `STEMS` / `ENDINGS` / `UNIQUES` into
   `OUT_DIR`.
5. Implement `analyze.rs` matching + normalization until analyzer tests pass
   (green).
6. Add `Warning` to `error.rs`; implement `latinity::lint` with its unit tests.
7. Wire `run_with_warnings` / `run` (lib) and the driver (main).
8. Integration test proving warning-only behavior; full `cargo test` green.

### v2 candidates (deferred, not rejected)
- Hard-error / configurable enforcement mode.
- Stem-slot–precise matching (tighten §6.6 leniency).
- `analyze(word) -> Option<Analysis>` richer API (stem, POS, suggested lemma).
- "Did you mean …?" suggestions for near-miss names.
- Configurable allow-list beyond single letters.
- Publishing `latinitas` to crates.io.

## 12. Open Questions
- Exact Whitaker mirror/commit to vendor from (cosmetic; settled during impl).
- Whether a handful of common programmer-Latin infinitives used as function names
  (e.g. `summare`) are present in Whitaker or need a tiny supplemental list
  (verify during impl; add a small `data/supplement.txt` only if a real example
  fails).
- Final Latin wording of the warning message (cosmetic; settled during impl).
