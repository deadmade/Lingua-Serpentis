# Latin Identifier Checker (`latinitas`) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a warning-only pass to Lingua Serpentis that flags user-defined identifiers (function names, variables, parameters) which are not recognizable Latin words, backed by a real morphological analyzer over the public-domain Whitaker's WORDS dictionary.

**Architecture:** Convert the repo to a Cargo virtual workspace with two crates. A new leaf crate `latinitas` answers exactly one question — `is_latin(&str) -> bool` — by splitting a word into `stem + ending` and checking the stem against Whitaker's dictionary and the ending against Whitaker's inflection tables. The interpreter crate gains a `latinity::lint` AST pass that collects declaration sites and emits non-fatal `Warning`s, wired into the pipeline between the type checker and the interpreter.

**Tech Stack:** Rust (edition 2021), std only (no third-party crates), Cargo workspaces, Whitaker's WORDS data files (public domain).

## Global Constraints

- Rust edition **2021** for both crates (matches the existing `Cargo.toml`).
- `latinitas` has **zero third-party dependencies** — std only.
- `lingua-serpentis` depends on `latinitas` by path; the dependency is **one-way** (`latinitas` must never reference interpreter types).
- Warnings are **never fatal**: a program with warnings still runs, and the process exit code stays `0`.
- **Single-character identifiers are exempt** from the Latin check (`a`, `b`, `i`).
- Only **declaration sites** are checked (function names, parameters, `let` variables) — never uses/assignments.
- Warning text is Latin: `Monitum in linea {line}: nomen '{name}' fortasse non Latinum est`.
- Warnings print to **stderr**; program output stays on **stdout**.
- `examples/` stays at the **repository root** (not moved into any crate).
- Existing tests must keep passing **unchanged** — `run()` retains its `LResult<String>` signature.
- Vendored Whitaker data is public domain and must ship with a `NOTICE` file crediting William Whitaker.

## Deviation From the Spec (read before Task 4)

The spec (§6.3) called for a `build.rs` that preprocesses the raw Whitaker files into a compact table in `OUT_DIR`. This plan instead **embeds the raw data with `include_str!` and parses it lazily into a `OnceLock<Tables>` on first use**, with the parsing logic living in ordinary crate modules.

Rationale: `build.rs` code cannot be unit-tested directly, and the Whitaker fixed-width parsing is the single most error-prone part of this feature — exactly the part that most needs tests. Moving it into normal modules makes it pure, `&str`-in/data-out, and fully testable (Task 4 tests it against real sample lines). The cost is a one-time parse of ~40k lines on the first `is_latin` call (tens of milliseconds, once per process, and only when a program actually has multi-character identifiers). Everything else in the spec is unchanged.

If you prefer the `build.rs` approach, Tasks 4 and 5 are the only ones affected.

## File Structure

| File | Responsibility |
|---|---|
| `Cargo.toml` (root) | Virtual workspace manifest — members + default-members. No `[package]`. |
| `crates/lingua-serpentis/**` | The existing interpreter, moved verbatim. |
| `crates/lingua-serpentis/src/latinity.rs` | **New.** The AST lint pass: declaration sites → `Vec<Warning>`. |
| `crates/lingua-serpentis/src/error.rs` | **Modified.** Adds the `Warning` type. |
| `crates/lingua-serpentis/src/lib.rs` | **Modified.** Adds `run_with_warnings`; `run` becomes a wrapper. |
| `crates/lingua-serpentis/src/main.rs` | **Modified.** Prints warnings to stderr. |
| `crates/latinitas/Cargo.toml` | **New crate** manifest, no dependencies. |
| `crates/latinitas/src/model.rs` | **New.** `Pos`, `Group`, `Tables` data types. |
| `crates/latinitas/src/analyze.rs` | **New.** `normalize` + the `stem + ending` matching algorithm. |
| `crates/latinitas/src/whitaker.rs` | **New.** Pure parsers for `DICTLINE.GEN` / `INFLECTS.LAT` / `UNIQUES.LAT` → `Tables`. |
| `crates/latinitas/src/lib.rs` | **New.** Public API `is_latin`, lazy `OnceLock<Tables>` over embedded data. |
| `crates/latinitas/data/DICTLINE.GEN` | **New.** Vendored dictionary (public domain). |
| `crates/latinitas/data/INFLECTS.LAT` | **New.** Vendored inflection tables (public domain). |
| `crates/latinitas/data/UNIQUES.LAT` | **New.** Vendored irregular whole-word forms (public domain). |
| `crates/latinitas/data/NOTICE` | **New.** Attribution for the vendored data. |

---

### Task 1: Convert to a Cargo virtual workspace

**Files:**
- Create: `Cargo.toml` (new root workspace manifest)
- Move: `Cargo.toml` → `crates/lingua-serpentis/Cargo.toml`
- Move: `src/` → `crates/lingua-serpentis/src/`
- Move: `tests/` → `crates/lingua-serpentis/tests/`

**Interfaces:**
- Consumes: nothing (first task).
- Produces: a workspace where `cargo build`, `cargo test`, and `cargo run -- examples/calc.liss` all work from the repository root. Later tasks add `crates/latinitas` as a second member.

**No code changes.** The interpreter source is moved verbatim; the existing tests are the regression suite proving the move was clean.

- [ ] **Step 1: Verify the current build is green before touching anything**

Run: `cargo test`
Expected: PASS — all existing unit and integration tests pass. (If this fails, stop and fix before migrating.)

- [ ] **Step 2: Move the package into `crates/lingua-serpentis/`**

```bash
mkdir -p crates/lingua-serpentis
git mv Cargo.toml crates/lingua-serpentis/Cargo.toml
git mv src crates/lingua-serpentis/src
git mv tests crates/lingua-serpentis/tests
```

Note: `examples/`, `docs/`, `README.md`, and `Cargo.lock` stay at the repository root.

- [ ] **Step 3: Create the root workspace manifest**

Create `Cargo.toml` at the repository root with exactly this content. `latinitas` is deliberately **not** listed yet — it does not exist until Task 2, and listing a missing member makes every cargo command fail.

```toml
[workspace]
resolver = "2"
members = ["crates/lingua-serpentis"]
default-members = ["crates/lingua-serpentis"]
```

- [ ] **Step 4: Verify the whole toolchain still works from the root**

Run: `cargo build`
Expected: PASS — compiles `lingua-serpentis` at its new path.

Run: `cargo test`
Expected: PASS — the same tests as Step 1, all green.

Run: `cargo run -- examples/calc.liss`
Expected: prints `30532`. (`default-members` makes `cargo run` pick the interpreter binary; `examples/` is resolved relative to the repository root, which is why it stays there.)

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "refactor: convert to cargo virtual workspace"
```

---

### Task 2: Scaffold the `latinitas` crate with its data model and normalizer

**Files:**
- Create: `crates/latinitas/Cargo.toml`
- Create: `crates/latinitas/src/model.rs`
- Create: `crates/latinitas/src/analyze.rs`
- Create: `crates/latinitas/src/lib.rs`
- Modify: `Cargo.toml` (root) — add `latinitas` to `members`

**Interfaces:**
- Consumes: the workspace from Task 1.
- Produces:
  - `pub enum Pos { Noun, Adj, Verb }` — derives `Debug, Clone, Copy, PartialEq, Eq, Hash`
  - `pub struct Group { pub pos: Pos, pub decl: u8 }` — derives `Debug, Clone, Copy, PartialEq, Eq, Hash`
  - `pub struct Tables { pub stems: HashMap<String, Vec<Group>>, pub endings: HashMap<Group, HashSet<String>>, pub whole: HashSet<String> }` — derives `Debug, Default`
  - `pub fn normalize(word: &str) -> String`

`Group` is the join key: a dictionary stem and an inflection ending belong together when they share a `Group`. `whole` holds indeclinable words (adverbs, prepositions, conjunctions) that are valid on their own with no ending.

- [ ] **Step 1: Create the crate manifest**

Create `crates/latinitas/Cargo.toml`:

```toml
[package]
name = "latinitas"
version = "0.1.0"
edition = "2021"
description = "Latin morphological word validation, backed by Whitaker's WORDS (public domain)"

[dependencies]
```

- [ ] **Step 2: Register the crate in the workspace**

Replace the root `Cargo.toml` with:

```toml
[workspace]
resolver = "2"
members = ["crates/lingua-serpentis", "crates/latinitas"]
default-members = ["crates/lingua-serpentis"]
```

- [ ] **Step 3: Write the data model**

Create `crates/latinitas/src/model.rs`:

```rust
use std::collections::{HashMap, HashSet};

/// The parts of speech whose forms we build from `stem + ending`.
/// Everything else (adverbs, prepositions, conjunctions, pronouns, numerals,
/// interjections) is treated as an indeclinable whole word instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Pos {
    Noun,
    Adj,
    Verb,
}

/// The join key between a dictionary stem and an inflection ending.
/// `decl` is the declension number for nouns/adjectives, or the conjugation
/// number for verbs. Whitaker's "variant" is deliberately collapsed away — see
/// the leniency note in the design spec.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Group {
    pub pos: Pos,
    pub decl: u8,
}

/// The compiled dictionary: which stems exist, which endings each group allows,
/// and which words stand alone with no ending at all.
#[derive(Debug, Default)]
pub struct Tables {
    pub stems: HashMap<String, Vec<Group>>,
    pub endings: HashMap<Group, HashSet<String>>,
    pub whole: HashSet<String>,
}

impl Tables {
    pub fn new() -> Self {
        Self::default()
    }
}
```

- [ ] **Step 4: Write the failing test for `normalize`**

Create `crates/latinitas/src/analyze.rs` containing only the tests for now:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lowercases() {
        assert_eq!(normalize("AMARE"), "amare");
    }

    #[test]
    fn maps_v_to_u_and_j_to_i() {
        // Classical Latin has no distinct v/j; the dictionary is stored in i/u
        // form, so both sides are folded the same way.
        assert_eq!(normalize("Verbum"), "uerbum");
        assert_eq!(normalize("Julius"), "iulius");
    }

    #[test]
    fn leaves_other_characters_alone() {
        // Underscores/digits survive normalization so they can fail lookup and
        // correctly produce a warning.
        assert_eq!(normalize("my_var2"), "my_uar2");
    }
}
```

- [ ] **Step 5: Wire up the crate root**

Create `crates/latinitas/src/lib.rs`. This must happen *before* running the test — without it the new files are never compiled, so the next step would not fail for the right reason:

```rust
pub mod analyze;
pub mod model;
```

- [ ] **Step 6: Run the test to verify it fails**

Run: `cargo test -p latinitas`
Expected: FAIL — compile error, `cannot find function 'normalize' in this scope`.

- [ ] **Step 7: Implement `normalize`**

Add to the **top** of `crates/latinitas/src/analyze.rs` (above the `mod tests` block):

```rust
/// Fold a word into the orthographic form the dictionary is stored in:
/// lowercase, with `v` written as `u` and `j` as `i`.
pub fn normalize(word: &str) -> String {
    word.chars()
        .map(|c| match c.to_ascii_lowercase() {
            'v' => 'u',
            'j' => 'i',
            other => other,
        })
        .collect()
}
```

- [ ] **Step 8: Run the tests to verify they pass**

Run: `cargo test -p latinitas`
Expected: PASS — 3 tests pass.

- [ ] **Step 9: Commit**

```bash
git add -A
git commit -m "feat(latinitas): add crate scaffold, data model, and normalizer"
```

---

### Task 3: Implement the `stem + ending` matching algorithm

**Files:**
- Modify: `crates/latinitas/src/analyze.rs`

**Interfaces:**
- Consumes: `Tables`, `Group`, `Pos` (Task 2); `normalize` (Task 2).
- Produces: `pub fn analyze(tables: &Tables, word: &str) -> bool`

This task tests the **algorithm** against small hand-built tables, so it is fully deterministic and does not depend on the Whitaker data (which arrives in Task 5).

- [ ] **Step 1: Write the failing tests**

Add to `crates/latinitas/src/analyze.rs`, inside the existing `mod tests` block:

```rust
    use crate::model::{Group, Pos, Tables};

    /// A tiny controlled dictionary: the 2nd-declension noun stem `numer-`,
    /// the 1st-conjugation verb stem `am-`, and the indeclinable `et`.
    fn fixture() -> Tables {
        let mut t = Tables::new();
        let noun2 = Group { pos: Pos::Noun, decl: 2 };
        let verb1 = Group { pos: Pos::Verb, decl: 1 };

        t.stems.insert("numer".to_string(), vec![noun2]);
        t.stems.insert("am".to_string(), vec![verb1]);

        t.endings.insert(
            noun2,
            ["us", "i", "o", "um", "e"].iter().map(|s| s.to_string()).collect(),
        );
        t.endings.insert(
            verb1,
            ["o", "as", "at", "are"].iter().map(|s| s.to_string()).collect(),
        );

        t.whole.insert("et".to_string());
        t
    }

    #[test]
    fn accepts_every_inflected_form_of_a_known_noun() {
        let t = fixture();
        for word in ["numerus", "numeri", "numero", "numerum", "numere"] {
            assert!(analyze(&t, word), "expected {word} to be Latin");
        }
    }

    #[test]
    fn accepts_inflected_verb_forms() {
        let t = fixture();
        for word in ["amo", "amas", "amat", "amare"] {
            assert!(analyze(&t, word), "expected {word} to be Latin");
        }
    }

    #[test]
    fn accepts_indeclinable_whole_words() {
        assert!(analyze(&fixture(), "et"));
    }

    #[test]
    fn is_case_insensitive() {
        assert!(analyze(&fixture(), "NUMERUS"));
    }

    #[test]
    fn rejects_non_latin_words() {
        let t = fixture();
        for word in ["result", "myVar", "getData", "xyz", "foobar"] {
            assert!(!analyze(&t, word), "expected {word} to be rejected");
        }
    }

    #[test]
    fn rejects_a_bare_stem_with_an_invalid_ending() {
        // `numer` is a real stem but `zzz` is not a 2nd-declension ending.
        assert!(!analyze(&fixture(), "numerzzz"));
    }

    #[test]
    fn rejects_empty_and_non_ascii_input() {
        let t = fixture();
        assert!(!analyze(&t, ""));
        assert!(!analyze(&t, "café"));
    }
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p latinitas`
Expected: FAIL — compile error, `cannot find function 'analyze' in this scope`.

- [ ] **Step 3: Implement `analyze`**

Add to `crates/latinitas/src/analyze.rs`, directly below `normalize`:

```rust
use crate::model::Tables;

/// True when `word` is a recognizable Latin word form: either an indeclinable
/// listed whole, or some split into `stem + ending` where the stem is in the
/// dictionary and the ending is valid for one of that stem's groups.
pub fn analyze(tables: &Tables, word: &str) -> bool {
    let w = normalize(word);
    if w.is_empty() {
        return false;
    }
    // Byte-slicing below assumes one byte per character. Latin words are ASCII,
    // so anything else is not Latin (and would otherwise risk a panic).
    if !w.is_ascii() {
        return false;
    }
    if tables.whole.contains(&w) {
        return true;
    }
    for i in 0..=w.len() {
        let (stem, ending) = (&w[..i], &w[i..]);
        let Some(groups) = tables.stems.get(stem) else {
            continue;
        };
        for g in groups {
            if tables.endings.get(g).is_some_and(|ends| ends.contains(ending)) {
                return true;
            }
        }
    }
    false
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p latinitas`
Expected: PASS — 10 tests pass.

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "feat(latinitas): add stem+ending matching algorithm"
```

---

### Task 4: Parse Whitaker's `DICTLINE`, `INFLECTS`, and `UNIQUES` formats

**Files:**
- Create: `crates/latinitas/src/whitaker.rs`
- Modify: `crates/latinitas/src/lib.rs` (add `mod whitaker;`)

**Interfaces:**
- Consumes: `Tables`, `Group`, `Pos` (Task 2); `normalize` (Task 2).
- Produces: `pub fn build_tables(dictline: &str, inflects: &str, uniques: &str) -> Tables`

**The three formats, confirmed against the real data files:**

`DICTLINE.GEN` is fixed-width. Four stem fields of 19 characters each occupy columns 1–76; from column 77 the part of speech and its codes begin, followed by five flag letters and the English gloss:

```
abac               abac                                                     N      2 1 M T          E E X C E small table for cruets...
abaestu            abaestu            abaestuav          abaestuat          V      1 1 INTRANS      D X X F S wave down...
abact              abact                                                    ADJ    1 1 POS          X X X E S driven away/off/back...
```

The token right after the part of speech is the declension number (nouns/adjectives) or conjugation number (verbs).

`INFLECTS.LAT` is whitespace-delimited: `POS declension variant <features…> stem-key ending-length ending flags`. The feature count varies by part of speech, so the ending is located **from the right**: the last two tokens are always the two flag letters; before them, the ending is the final alphabetic token, or absent entirely when the ending-length field is `0`.

```
N     1 1 NOM S C  1 1 a         X A
N     1 0 GEN P C  2 4 arum      X A
V     1 1 PRES  ACTIVE  IND  1 S  1 1 o             X A
```

`UNIQUES.LAT` holds irregular forms that no stem-plus-ending rule generates (forms of *esse* and friends). Each entry is two lines — the word form flush at column 1, then an indented line of codes and meaning — so the flush-left lines are exactly the word forms:

```
agantur
      V      3 1 PRES PASSIVE SUB 3 P  IMPERS             F  X  X  E  E
```

**Two deliberate simplifications** (sanctioned by the spec's leniency clause): Whitaker's *variant* number is collapsed (all variants of a declension share one `Group`), and the *stem-key* is ignored (any stem of an entry may take any ending of its group). Both only ever make the checker more accepting, never less — every genuine form is still matched, which is what protects against false warnings.

- [ ] **Step 1: Write the failing tests**

Create `crates/latinitas/src/whitaker.rs` containing only the tests for now:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyze::analyze;
    use crate::model::{Group, Pos};

    /// Build a DICTLINE line with correct 19-character stem columns, so the
    /// tests never depend on hand-counted spacing.
    fn dict_line(stems: &[&str], tail: &str) -> String {
        let mut s = String::new();
        for i in 0..4 {
            let stem = stems.get(i).copied().unwrap_or("");
            s.push_str(stem);
            for _ in stem.len()..19 {
                s.push(' ');
            }
        }
        s.push_str(tail);
        s
    }

    #[test]
    fn parses_a_noun_entry_and_its_endings() {
        let dict = dict_line(&["numer", "numer"], "N      2 1 M T          X X X A O number;");
        let infl = "N     2 1 NOM S M  1 2 us        X A\n\
                    N     2 1 GEN S M  2 1 i         X A";
        let t = build_tables(&dict, infl, "");

        let g = Group { pos: Pos::Noun, decl: 2 };
        assert_eq!(t.stems.get("numer"), Some(&vec![g]));
        assert!(t.endings[&g].contains("us"));
        assert!(analyze(&t, "numerus"));
        assert!(analyze(&t, "numeri"));
    }

    #[test]
    fn parses_a_verb_entry_with_four_stems() {
        let dict = dict_line(
            &["am", "am", "amav", "amat"],
            "V      1 1 TRANS        X X X A O love, like;",
        );
        let infl = "V     1 1 PRES  ACTIVE  IND  1 S  1 1 o         X A\n\
                    V     1 1 PRES  ACTIVE  INF  0 X  2 3 are       X A\n\
                    V     1 1 PERF  ACTIVE  IND  1 S  3 1 i         X A";
        let t = build_tables(&dict, infl, "");

        assert!(analyze(&t, "amo"));
        assert!(analyze(&t, "amare"));
        assert!(analyze(&t, "amavi")); // perfect stem + perfect ending
    }

    #[test]
    fn treats_indeclinable_parts_of_speech_as_whole_words() {
        let dict = format!(
            "{}\n{}",
            dict_line(&["et"], "CONJ                    X X X A O and;"),
            dict_line(&["ab"], "PREP   ABL              X X X A O by, from;"),
        );
        let t = build_tables(&dict, "", "");
        assert!(t.whole.contains("et"));
        assert!(analyze(&t, "et"));
        assert!(analyze(&t, "ab"));
    }

    #[test]
    fn handles_zero_length_endings() {
        // 3rd-declension nominatives attach a zero-length ending, so the bare
        // stem is itself a valid word.
        let dict = dict_line(&["rex", "reg"], "N      3 1 M T          X X X A O king;");
        let infl = "N     3 1 NOM S M  1 0           X A\n\
                    N     3 1 GEN S M  2 2 is        X A";
        let t = build_tables(&dict, infl, "");

        assert!(analyze(&t, "rex"));
        assert!(analyze(&t, "regis"));
    }

    #[test]
    fn declension_nine_means_indeclinable() {
        let dict = dict_line(&["a"], "N      9 8 M N          X X X C G year, abb. a.;");
        let t = build_tables(&dict, "", "");
        assert!(t.whole.contains("a"));
    }

    #[test]
    fn skips_placeholder_and_empty_stems() {
        let dict = dict_line(&["numer", "zzz"], "N      2 1 M T          X X X A O number;");
        let t = build_tables(&dict, "N     2 1 NOM S M  1 2 us        X A", "");
        assert!(t.stems.contains_key("numer"));
        assert!(!t.stems.contains_key("zzz"));
    }

    #[test]
    fn skips_comments_and_blank_lines_in_inflects() {
        let t = build_tables("", "-- a comment line\n\nN     2 1 NOM S M  1 2 us        X A", "");
        assert!(t.endings[&Group { pos: Pos::Noun, decl: 2 }].contains("us"));
    }

    #[test]
    fn parses_uniques_as_whole_words() {
        // UNIQUES entries are two lines: the word form flush left, then an
        // indented line of grammatical codes and the English meaning.
        let uniques = "est\n\
                       \x20     V      5 1 PRES ACTIVE IND 3 S     X X X A O he/she/it is;\n\
                       agantur\n\
                       \x20     V      3 1 PRES PASSIVE SUB 3 P     F X X E E they are driven;";
        let t = build_tables("", "", uniques);

        assert!(t.whole.contains("est"));
        assert!(t.whole.contains("agantur"));
        // The indented codes line must never be mistaken for a word form.
        assert!(!t.whole.contains("V"));
        assert!(analyze(&t, "est"));
    }

    #[test]
    fn normalizes_stems_to_dictionary_orthography() {
        let dict = dict_line(&["uerb", "verb"], "N      2 2 N T          X X X A O word;");
        let t = build_tables(&dict, "N     2 2 NOM S N  1 2 um        X A", "");
        // Both spellings fold to the same normalized stem.
        assert!(analyze(&t, "verbum"));
        assert!(analyze(&t, "uerbum"));
    }
}
```

- [ ] **Step 2: Register the module**

Replace `crates/latinitas/src/lib.rs` with the following. This must happen *before* running the test — an unregistered file is never compiled, so the next step would not fail for the right reason:

```rust
pub mod analyze;
pub mod model;
pub mod whitaker;
```

- [ ] **Step 3: Run the tests to verify they fail**

Run: `cargo test -p latinitas`
Expected: FAIL — compile error, `cannot find function 'build_tables' in this scope`.

- [ ] **Step 4: Implement the parsers**

Add to the **top** of `crates/latinitas/src/whitaker.rs` (above the `mod tests` block):

```rust
use crate::analyze::normalize;
use crate::model::{Group, Pos, Tables};

/// Width of each of the four stem columns in DICTLINE.GEN.
const STEM_WIDTH: usize = 19;
/// Total width of the four stem columns; the part of speech starts here.
const STEMS_END: usize = STEM_WIDTH * 4;
/// Whitaker's placeholder for "this stem slot is unused".
const NO_STEM: &str = "zzz";
/// Whitaker's declension code for an indeclinable entry.
const INDECLINABLE_DECL: u8 = 9;

/// Map a Whitaker part-of-speech code onto the three inflecting classes we
/// model. Everything else is indeclinable as far as this checker is concerned.
fn inflecting_pos(code: &str) -> Option<Pos> {
    match code {
        "N" => Some(Pos::Noun),
        "ADJ" => Some(Pos::Adj),
        "V" => Some(Pos::Verb),
        _ => None,
    }
}

/// Compile the three Whitaker data files into lookup tables.
pub fn build_tables(dictline: &str, inflects: &str, uniques: &str) -> Tables {
    let mut t = Tables::new();
    for line in inflects.lines() {
        parse_inflect_line(line, &mut t);
    }
    for line in dictline.lines() {
        parse_dict_line(line, &mut t);
    }
    for line in uniques.lines() {
        parse_unique_line(line, &mut t);
    }
    t
}

/// One line of UNIQUES.LAT. Entries are two lines: the irregular word form
/// flush at column 1, then an indented line of grammatical codes and meaning.
/// Only the flush-left lines are word forms.
fn parse_unique_line(line: &str, t: &mut Tables) {
    if line.starts_with(char::is_whitespace) {
        return;
    }
    let word = line.trim();
    if word.is_empty() || word.starts_with("--") {
        return;
    }
    if word.chars().all(|c| c.is_ascii_alphabetic()) {
        t.whole.insert(normalize(word));
    }
}

/// One DICTLINE entry: four fixed-width stems, then the part of speech and its
/// declension/conjugation number.
fn parse_dict_line(line: &str, t: &mut Tables) {
    if line.len() < STEMS_END || !line.is_ascii() {
        return;
    }
    let stems: Vec<String> = (0..4)
        .map(|i| &line[i * STEM_WIDTH..(i + 1) * STEM_WIDTH])
        .map(|s| s.trim())
        .filter(|s| !s.is_empty() && *s != NO_STEM)
        .map(normalize)
        .collect();
    if stems.is_empty() {
        return;
    }

    let mut toks = line[STEMS_END..].split_whitespace();
    let Some(pos_code) = toks.next() else {
        return;
    };

    // Indeclinable part of speech: the stem is the entire word.
    let Some(pos) = inflecting_pos(pos_code) else {
        t.whole.extend(stems);
        return;
    };

    let Some(decl) = toks.next().and_then(|d| d.parse::<u8>().ok()) else {
        return;
    };
    if decl == INDECLINABLE_DECL {
        t.whole.extend(stems);
        return;
    }

    let group = Group { pos, decl };
    for stem in stems {
        let groups = t.stems.entry(stem).or_default();
        if !groups.contains(&group) {
            groups.push(group);
        }
    }
}

/// One INFLECTS rule. Layout is
/// `POS decl variant <features...> stem-key ending-length ending flag flag`,
/// with a variable number of feature columns — so we read from the right.
fn parse_inflect_line(line: &str, t: &mut Tables) {
    let line = line.trim();
    if line.is_empty() || line.starts_with("--") {
        return;
    }
    let toks: Vec<&str> = line.split_whitespace().collect();
    // Shortest real rule is: POS decl variant stem-key length flag flag.
    if toks.len() < 7 {
        return;
    }
    let Some(pos) = inflecting_pos(toks[0]) else {
        return;
    };
    let Ok(decl) = toks[1].parse::<u8>() else {
        return;
    };

    // Drop the two trailing flag columns (age, frequency).
    let core = &toks[..toks.len() - 2];
    // The ending is the last token when it is alphabetic; when the ending-length
    // column is 0 there is no ending token at all, so the last token is numeric.
    let ending = match core.last() {
        Some(last) if last.chars().all(|c| c.is_ascii_alphabetic()) => normalize(last),
        Some(_) => String::new(),
        None => return,
    };

    t.endings.entry(Group { pos, decl }).or_default().insert(ending);
}
```

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo test -p latinitas`
Expected: PASS — 19 tests pass (10 from earlier tasks, 9 new).

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "feat(latinitas): parse Whitaker DICTLINE and INFLECTS formats"
```

---

### Task 5: Vendor the Whitaker data and expose `is_latin`

**Files:**
- Create: `crates/latinitas/data/DICTLINE.GEN` (downloaded)
- Create: `crates/latinitas/data/INFLECTS.LAT` (downloaded)
- Create: `crates/latinitas/data/UNIQUES.LAT` (downloaded)
- Create: `crates/latinitas/data/NOTICE`
- Modify: `crates/latinitas/src/lib.rs`

**Interfaces:**
- Consumes: `build_tables` (Task 4), `analyze` (Task 3).
- Produces: `pub fn is_latin(word: &str) -> bool` — the crate's entire public contract, and the only thing the interpreter uses.

- [ ] **Step 1: Download the public-domain data files**

```bash
mkdir -p crates/latinitas/data
curl -L -o crates/latinitas/data/DICTLINE.GEN \
  https://raw.githubusercontent.com/mk270/whitakers-words/master/DICTLINE.GEN
curl -L -o crates/latinitas/data/INFLECTS.LAT \
  https://raw.githubusercontent.com/mk270/whitakers-words/master/INFLECTS.LAT
curl -L -o crates/latinitas/data/UNIQUES.LAT \
  https://raw.githubusercontent.com/mk270/whitakers-words/master/UNIQUES.LAT
```

Verify all three downloaded correctly:

Run: `wc -l crates/latinitas/data/*.GEN crates/latinitas/data/*.LAT`
Expected: `DICTLINE.GEN` has roughly 39,000 lines; `INFLECTS.LAT` roughly 1,500; `UNIQUES.LAT` a few hundred. If any is only a handful of lines, the download failed (likely an HTML error page) — stop and re-fetch.

- [ ] **Step 2: Add the attribution notice**

Create `crates/latinitas/data/NOTICE`:

```
The files DICTLINE.GEN, INFLECTS.LAT and UNIQUES.LAT in this directory are data
files from WORDS, a Latin dictionary and morphological analyser by William
Whitaker (1936-2010).

William Whitaker placed WORDS and its data files in the public domain.

Obtained from the maintained mirror at:
    https://github.com/mk270/whitakers-words

They are vendored here unmodified so that `latinitas` builds offline and
reproducibly. All morphological analysis code in this crate is original.
```

- [ ] **Step 3: Write the failing smoke tests**

Replace `crates/latinitas/src/lib.rs` with the module declarations plus this test block (the implementation comes in the next step):

```rust
pub mod analyze;
pub mod model;
pub mod whitaker;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_common_latin_nouns_in_several_cases() {
        for word in ["numerus", "numeri", "numerum", "rex", "regis"] {
            assert!(is_latin(word), "expected {word} to be recognized as Latin");
        }
    }

    #[test]
    fn accepts_common_latin_verbs_and_adjectives() {
        for word in ["amo", "amare", "bonus", "bona"] {
            assert!(is_latin(word), "expected {word} to be recognized as Latin");
        }
    }

    #[test]
    fn accepts_words_used_in_the_example_programs() {
        for word in ["summa", "pecunia"] {
            assert!(is_latin(word), "expected {word} to be recognized as Latin");
        }
    }

    #[test]
    fn accepts_irregular_forms_from_the_uniques_table() {
        // Forms of `esse` are irregular and come from UNIQUES.LAT, not from
        // any stem + ending rule.
        assert!(is_latin("est"));
    }

    #[test]
    fn is_case_insensitive() {
        assert!(is_latin("AMO"));
        assert!(is_latin("Numerus"));
    }

    #[test]
    fn rejects_english_programmer_names() {
        for word in ["result", "getData", "myVar", "counter", "foobar"] {
            assert!(!is_latin(word), "expected {word} to be rejected");
        }
    }

    #[test]
    fn rejects_empty_input() {
        assert!(!is_latin(""));
    }
}
```

- [ ] **Step 4: Run the tests to verify they fail**

Run: `cargo test -p latinitas`
Expected: FAIL — compile error, `cannot find function 'is_latin' in this scope`.

- [ ] **Step 5: Implement `is_latin`**

Add to `crates/latinitas/src/lib.rs`, between the module declarations and the `mod tests` block:

```rust
use model::Tables;
use std::sync::OnceLock;

/// Whitaker's WORDS data, embedded so the crate is self-contained at runtime.
/// See `data/NOTICE` for provenance and licensing.
const DICTLINE: &str = include_str!("../data/DICTLINE.GEN");
const INFLECTS: &str = include_str!("../data/INFLECTS.LAT");
const UNIQUES: &str = include_str!("../data/UNIQUES.LAT");

/// The compiled dictionary, built once on first use.
fn tables() -> &'static Tables {
    static TABLES: OnceLock<Tables> = OnceLock::new();
    TABLES.get_or_init(|| whitaker::build_tables(DICTLINE, INFLECTS, UNIQUES))
}

/// True when `word` is a recognizable Latin word form — any inflection of a
/// dictionary entry, or an indeclinable word. Case- and orthography-insensitive
/// (`v`/`u` and `j`/`i` are equivalent). Empty or non-ASCII input is `false`.
pub fn is_latin(word: &str) -> bool {
    analyze::analyze(tables(), word)
}
```

- [ ] **Step 6: Run the tests to verify they pass**

Run: `cargo test -p latinitas`
Expected: PASS — 26 tests pass.

If a specific word in Steps 3's *positive* tests fails, check it against the real dictionary before assuming the parser is broken:

```bash
grep -c "^numer" crates/latinitas/data/DICTLINE.GEN
```

If the stem genuinely is not in Whitaker, swap that word for another common one — the tests exist to prove the mechanism works, not to assert any particular word's presence. A failing *negative* test (an English word accepted) is more serious: it means the matching is too lenient and the leniency simplifications need revisiting.

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "feat(latinitas): vendor Whitaker dictionary and expose is_latin"
```

---

### Task 6: Add the `Warning` type

**Files:**
- Modify: `crates/lingua-serpentis/src/error.rs`

**Interfaces:**
- Consumes: nothing.
- Produces: `pub struct Warning { pub message: String, pub line: usize }`, deriving `Debug, Clone, PartialEq`, with a `Display` impl rendering `Monitum in linea {line}: {message}`.

`Warning` is deliberately a separate type from `LinguaError` so a non-fatal style note can never be mistaken for a fatal error by either the compiler or a reader.

- [ ] **Step 1: Write the failing test**

Add to the existing `mod tests` block in `crates/lingua-serpentis/src/error.rs`:

```rust
    #[test]
    fn warning_formats_with_line_and_message() {
        let w = Warning { message: "nomen 'result' fortasse non Latinum est".into(), line: 3 };
        assert_eq!(
            w.to_string(),
            "Monitum in linea 3: nomen 'result' fortasse non Latinum est"
        );
    }
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p lingua-serpentis --lib error`
Expected: FAIL — compile error, `cannot find struct 'Warning' in this scope`.

- [ ] **Step 3: Implement `Warning`**

Add to `crates/lingua-serpentis/src/error.rs`, after the `LResult` type alias and before the `mod tests` block:

```rust
/// A non-fatal style note. Unlike `LinguaError`, a `Warning` never stops a
/// program from running.
#[derive(Debug, Clone, PartialEq)]
pub struct Warning {
    pub message: String,
    pub line: usize,
}

impl std::fmt::Display for Warning {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Monitum in linea {}: {}", self.line, self.message)
    }
}
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `cargo test -p lingua-serpentis --lib error`
Expected: PASS — 2 tests pass.

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "feat: add non-fatal Warning type"
```

---

### Task 7: Implement the `latinity` lint pass

**Files:**
- Create: `crates/lingua-serpentis/src/latinity.rs`
- Modify: `crates/lingua-serpentis/Cargo.toml` (add the `latinitas` dependency)
- Modify: `crates/lingua-serpentis/src/lib.rs` (add `pub mod latinity;`)

**Interfaces:**
- Consumes: `latinitas::is_latin` (Task 5); `Warning` (Task 6); `Stmt`, `StmtKind`, `FuncDecl` from `crate::ast`.
- Produces: `pub fn lint(program: &[Stmt]) -> Vec<Warning>` — warnings sorted by line.

Only **declaration sites** are visited: function names, parameter names, and `let` variable names. Assignments and expression uses refer to names declared elsewhere, so visiting them would double-report. Parameters have no line of their own, so they are reported on their function's line.

- [ ] **Step 1: Add the dependency**

In `crates/lingua-serpentis/Cargo.toml`, add a `[dependencies]` section (or extend the existing one):

```toml
[dependencies]
latinitas = { path = "../latinitas" }
```

- [ ] **Step 2: Write the failing tests**

Create `crates/lingua-serpentis/src/latinity.rs` containing only the tests for now:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::tokenize;
    use crate::parser::parse;

    fn lint_src(src: &str) -> Vec<Warning> {
        lint(&parse(&tokenize(src).unwrap()).unwrap())
    }

    #[test]
    fn single_letter_names_are_exempt() {
        let w = lint_src("ni a = °V; ni b = °I;");
        assert!(w.is_empty(), "single letters should not warn, got {w:?}");
    }

    #[test]
    fn flags_a_non_latin_variable() {
        let w = lint_src("ni result = °V;");
        assert_eq!(w.len(), 1);
        assert_eq!(w[0].line, 1);
        assert!(w[0].message.contains("result"));
    }

    #[test]
    fn accepts_a_latin_variable() {
        assert!(lint_src("ni numerus = °V;").is_empty());
    }

    #[test]
    fn flags_a_non_latin_function_name_and_parameter() {
        // `foobar` and `widget` are invented words. Avoid plausible-looking
        // English here: `compute` and `value` would be poor choices, because
        // `computare` and `valere` are real Latin verbs whose stems are in the
        // dictionary.
        let w = lint_src("munus ni foobar(ni widget) { redde widget; }");
        let names: Vec<&str> = w.iter().map(|x| x.message.as_str()).collect();
        assert_eq!(w.len(), 2, "expected function + param warnings, got {names:?}");
        assert!(names.iter().any(|m| m.contains("foobar")));
        assert!(names.iter().any(|m| m.contains("widget")));
    }

    #[test]
    fn reaches_declarations_inside_nested_blocks() {
        let src = "si (verus) { indem (verus) { ni result = °V; } }";
        let w = lint_src(src);
        assert_eq!(w.len(), 1);
        assert!(w[0].message.contains("result"));
    }

    #[test]
    fn reaches_declarations_inside_function_bodies() {
        let w = lint_src("munus ni summa() { ni result = °V; redde result; }");
        assert!(w.iter().any(|x| x.message.contains("result")));
    }

    #[test]
    fn ignores_assignments_to_already_declared_names() {
        // `result` is declared once and assigned once: exactly one warning.
        let w = lint_src("ni result = °V; result = °VI;");
        assert_eq!(w.len(), 1);
    }

    #[test]
    fn warnings_are_sorted_by_line() {
        let w = lint_src("ni alpha = °I;\nni result = °V;\nni beta = °II;");
        let lines: Vec<usize> = w.iter().map(|x| x.line).collect();
        let mut sorted = lines.clone();
        sorted.sort();
        assert_eq!(lines, sorted);
    }
}
```

- [ ] **Step 3: Register the module**

Add to `crates/lingua-serpentis/src/lib.rs`, with the other `pub mod` declarations:

```rust
pub mod latinity;
```

- [ ] **Step 4: Run the tests to verify they fail**

Run: `cargo test -p lingua-serpentis --lib latinity`
Expected: FAIL — compile error, `cannot find function 'lint' in this scope`.

- [ ] **Step 5: Implement the lint pass**

Add to the **top** of `crates/lingua-serpentis/src/latinity.rs` (above the `mod tests` block):

```rust
use crate::ast::{Stmt, StmtKind};
use crate::error::Warning;

/// Collect a warning for every declared identifier that is not a Latin word.
/// Only declaration sites are visited, so each name is reported at most once
/// per declaration. Results are sorted by line.
pub fn lint(program: &[Stmt]) -> Vec<Warning> {
    let mut out = Vec::new();
    for s in program {
        lint_stmt(s, &mut out);
    }
    out.sort_by_key(|w| w.line);
    out
}

/// Single-character names (`a`, `b`, `i`) are conventional counters and are
/// exempt; everything longer must be recognizable Latin.
fn check_name(name: &str, line: usize, out: &mut Vec<Warning>) {
    if name.chars().count() > 1 && !latinitas::is_latin(name) {
        out.push(Warning {
            message: format!("nomen '{name}' fortasse non Latinum est"),
            line,
        });
    }
}

fn lint_block(body: &[Stmt], out: &mut Vec<Warning>) {
    for s in body {
        lint_stmt(s, out);
    }
}

fn lint_stmt(s: &Stmt, out: &mut Vec<Warning>) {
    match &s.kind {
        StmtKind::Let { name, .. } => check_name(name, s.line, out),
        StmtKind::Func(f) => {
            check_name(&f.name, s.line, out);
            for (_, param) in &f.params {
                // Parameters carry no line of their own; report them on the
                // function's line.
                check_name(param, s.line, out);
            }
            lint_block(&f.body, out);
        }
        StmtKind::If { then, else_, .. } => {
            lint_block(then, out);
            if let Some(eb) = else_ {
                lint_block(eb, out);
            }
        }
        StmtKind::While { body, .. } => lint_block(body, out),
        StmtKind::For { init, step, body, .. } => {
            lint_stmt(init, out);
            lint_stmt(step, out);
            lint_block(body, out);
        }
        // Assignments and expressions only *use* names declared elsewhere.
        StmtKind::Assign { .. }
        | StmtKind::Print(_)
        | StmtKind::ExprStmt(_)
        | StmtKind::Return(_)
        | StmtKind::Break
        | StmtKind::Continue => {}
    }
}
```

- [ ] **Step 6: Run the tests to verify they pass**

Run: `cargo test -p lingua-serpentis --lib latinity`
Expected: PASS — 8 tests pass.

If `accepts_a_latin_variable` fails, confirm the word directly with `latinitas` before changing the lint code — the failure is in the dictionary layer, not here.

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "feat: add latinity lint pass over declaration sites"
```

---

### Task 8: Wire warnings into the pipeline and the CLI

**Files:**
- Modify: `crates/lingua-serpentis/src/lib.rs`
- Modify: `crates/lingua-serpentis/src/main.rs`
- Modify: `crates/lingua-serpentis/tests/programs.rs`

**Interfaces:**
- Consumes: `latinity::lint` (Task 7); `Warning` (Task 6).
- Produces:
  - `pub fn run_with_warnings(src: &str) -> LResult<(String, Vec<Warning>)>`
  - `pub fn run(src: &str) -> LResult<String>` (unchanged signature — a wrapper)
  - `pub use error::Warning;` re-export for test and CLI convenience

- [ ] **Step 1: Write the failing integration tests**

Add to `crates/lingua-serpentis/tests/programs.rs`:

```rust
use lingua_serpentis::run_with_warnings;

#[test]
fn non_latin_name_warns_but_program_still_runs() {
    let src = "ni result = °V; scribe(result);";
    let (output, warnings) = run_with_warnings(src).unwrap();
    // The warning must not change the program's behaviour.
    assert_eq!(output, "5\n");
    assert_eq!(warnings.len(), 1);
    assert_eq!(warnings[0].line, 1);
    assert!(warnings[0].message.contains("result"));
}

#[test]
fn latin_program_produces_no_warnings() {
    let src = "ni numerus = °V; scribe(numerus);";
    let (output, warnings) = run_with_warnings(src).unwrap();
    assert_eq!(output, "5\n");
    assert!(warnings.is_empty(), "unexpected warnings: {warnings:?}");
}

#[test]
fn type_errors_still_halt_before_any_linting() {
    // A type error aborts the pipeline, so no warnings are produced at all.
    assert!(run_with_warnings("ni x = \"salve\";").is_err());
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p lingua-serpentis --test programs`
Expected: FAIL — compile error, `unresolved import 'lingua_serpentis::run_with_warnings'`.

- [ ] **Step 3: Add `run_with_warnings` to the library**

In `crates/lingua-serpentis/src/lib.rs`, replace the `use error::LResult;` line and the `run` function with:

```rust
use error::LResult;
pub use error::Warning;

/// Lex → parse → type-check → lint → interpret.
/// Returns the collected `scribe` output together with any Latinity warnings.
/// Linting never fails and never affects whether the program runs.
pub fn run_with_warnings(src: &str) -> LResult<(String, Vec<Warning>)> {
    let tokens = lexer::tokenize(src)?;
    let program = parser::parse(&tokens)?;
    checker::check(&program)?;
    let warnings = latinity::lint(&program);
    let output = interp::execute(&program)?;
    Ok((output, warnings))
}

/// Lex → parse → type-check → interpret, discarding Latinity warnings.
pub fn run(src: &str) -> LResult<String> {
    run_with_warnings(src).map(|(output, _)| output)
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p lingua-serpentis --test programs`
Expected: PASS — all integration tests pass, including the three pre-existing ones that call `run`.

- [ ] **Step 5: Print warnings from the CLI**

In `crates/lingua-serpentis/src/main.rs`, change the import on line 1 and replace the `match run(&src)` block:

```rust
use lingua_serpentis::run_with_warnings;
```

```rust
    match run_with_warnings(&src) {
        Ok((output, warnings)) => {
            // Warnings go to stderr so they never contaminate program output.
            for w in &warnings {
                eprintln!("{w}");
            }
            print!("{output}");
        }
        Err(e) => { eprintln!("{e}"); exit(1); }
    }
```

- [ ] **Step 6: Verify the whole suite and the CLI end to end**

Run: `cargo test`
Expected: PASS — every test in both crates.

Run: `cargo run -- examples/calc.liss`
Expected: prints `30532` on stdout. Any `Monitum in linea ...` lines appear on stderr; the exit code is still `0`. Confirm the exit code:

Run: `cargo run -- examples/calc.liss > /dev/null; echo "exit=$?"`
Expected: `exit=0` — warnings must never change the exit status.

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "feat: report Latin identifier warnings from the pipeline and CLI"
```

---

## Verification Checklist

After Task 8, confirm the feature end to end:

- [ ] `cargo test` passes for both crates.
- [ ] `cargo run -- examples/calc.liss` prints `30532` and exits `0`.
- [ ] A program with an English identifier prints a `Monitum in linea N: nomen '...' fortasse non Latinum est` line on **stderr** and still produces correct output on **stdout**.
- [ ] Redirecting stdout alone (`> out.txt`) captures only program output, never warnings.
- [ ] The three original integration tests in `tests/programs.rs` are unmodified and passing.

## Notes for the Implementer

- **The leniency is intentional.** `analyze` accepts any stem of an entry combined with any ending of that entry's group, ignoring Whitaker's variant and stem-key precision. This can accept a malformed combination like `rexis`. That is a deliberate trade: for a warning-only checker, wrongly *accepting* a nonsense word is harmless, while wrongly *rejecting* a real Latin word produces a false warning that erodes trust in every warning.
- **If real Latin words get flagged**, the fix is almost always in the parsing (Task 4), not the algorithm (Task 3) — check that the word's part of speech is one of `N`/`ADJ`/`V` and that its declension parsed correctly.
- **`examples/calc.liss` uses `summare`.** If Whitaker lacks that late/medieval verb, the example will emit one warning. That is correct behaviour, not a bug — the spec's open question anticipated it. Do not add special cases to silence it.
