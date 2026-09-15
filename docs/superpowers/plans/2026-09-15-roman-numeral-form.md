# Roman Numeral Form Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development
> (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use
> checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make every `.`, `:` and `/` chunk of a Roman literal equal the numeral the language prints
for its value, rejected in the lexer's call to `convert_single` with
`numerus Romanus non canonicus: '{}'`, closing D-13, then move the D-13 fixture to `errores/` and keep
D-25's zero-denominator argument and the huge-literal stress alive.

**Architecture:** One equality check at the end of `convert_single` (`crates/runtime/src/roman.rs`):
`total.display() == s`. `display` already renders the classical form and emits no U+0305 below
4000, so the check alone confines a chunk to 1..=3999 with classical repetition and pairs. Every
chunk of every literal goes through `convert_single`, so nothing else changes: the lexer, the token,
the AST and every stage after them see the same value for every literal still accepted.

**Tech Stack:** Rust 2024. **No new dependency.** Spec §11's table is not touched.

## Context

`convert_single` accepts any string over `I V X L C D M` by one positional add-or-subtract rule, so
`°IIX + °VV + °IM` prints `MXIX` and exits 0.

The design doc is `docs/superpowers/specs/2026-09-15-roman-numeral-form-design.md`; read §3 (the
rule), §5 (what is rejected and accepted), §6 (D-25 restated) and §7 (catalogue impact) before
starting. Where this plan and that spec disagree, the spec governs.

Line numbers below are at `6634fcd`, and each task's edits move the lines the next task names. Find
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
  argument of `format!` at its call site in `crates/runtime/src/roman.rs`. It must not reuse the text
  of `numerus Romanus invalidus: '{}'`. No Arabic digits.
- **The source does not explain itself; `docs/internals/` does.** What stays in the source is
  `# Safety` clauses, fixture and test pointers, and a bare `D-NN`.
- **Comments move the catalogue snapshot.** Re-bless, then check the header against spec §7 and
  account for every moved row.
- **Bless narrowly, then run `tests/nuntii.rs`.** Never bless the whole corpus for this change; a
  `nuntii` that changes rather than appears is a regression.
- Commit subjects are `type(scope): a sentence in the repo's voice`. Scopes are the area (`runtime`,
  `robustness`, `conformance`, `defectus`, `internals`, `spec`), not the crate name. Every commit
  message ends with `Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>`.

---

## Task 0: Baseline

No commit.

- [ ] Step 1: `git status` clean; `git log -1` is the spec commit or later.
- [ ] Step 2: `cargo nextest run` green. Record the total (682 at `6634fcd`) and the per-binary counts
      for `conformance`, `robustness`, and the `runtime` lib tests.
- [ ] Step 3: Record `head -5 crates/lingua-serpentis/tests/snapshots/catalogus__catalogus.snap` — it
      must read `119 sites · 71 fixtured · 0 ambiguous · 46 uncovered · 2 unparsed` — and
      `nuntii__nuntii.snap`'s `90 fixtures · 103 diagnostics`.
- [ ] Step 4: Save the before-set for spec §7's check:
      `sed -n '7,$p' crates/lingua-serpentis/tests/snapshots/catalogus__catalogus.snap | sed -E 's/^[^ ]+  //' | sort > $SCRATCH/catalogus-ante.txt`
      (`$SCRATCH` is any directory outside the repository).
- [ ] Step 5: `for g in currunt errores monita defectus; do echo $g $(ls conformance/$g | wc -l); done`
      → `35 82 4 1`.

If a count differs from the spec, record the measured number here and carry on against it; do not
edit the spec's before-header.

## Task 1: The design doc — DONE

`docs/superpowers/specs/2026-09-15-roman-numeral-form-design.md`; the amendment notices at the top of
`2026-07-19-lingua-serpentis-rust-interpreter-design.md` and
`2026-07-23-roman-native-integers-design.md`; this plan; and `docs/README.md`: the question-table
row, the spec-table row and "Fifteen documents", "amended six times", the integers row's
"Amended once", the sixth amends-chain entry, the numeric-types note, the reading list's "six
amendments" and "Sixteen plans".

---

## Task 2: The fixture moves, failing

**Files:** `conformance/defectus/D-13-numerus-non-canonicus/` → `conformance/errores/numerus-non-canonicus/`
(`emissio` deleted; `exitus` new; `NOTA.md` rewritten).

**Interfaces:** produces the trial `errores::numerus-non-canonicus`, which Task 3 turns green. No
`nuntii` yet; Task 3 blesses it.

- [ ] Step 1: `git mv conformance/defectus/D-13-numerus-non-canonicus conformance/errores/numerus-non-canonicus`.
      `programma/principale.liss` is not edited: its bad literal is on line 2.
- [ ] Step 2: `git rm conformance/errores/numerus-non-canonicus/emissio`; write `exitus` = `1`.
- [ ] Step 3: Rewrite `NOTA.md`: a `# Numerus Romanus non canonicus` heading; "D-13 is fixed." then
      what the program did before (stdout `MXIX`, exit 0: `IIX` read as 10, `VV` as 10, `IM` as 999);
      that each chunk split at `.`, `:` and `/` must be written the way its value prints, 1..=3999;
      that only the first bad literal is reported, so `'IIX'` is named and `°VV`, `°IM` are not; and
      where the rule lives (`convert_single`, by symbol).
- [ ] Step 4: Confirm the old behaviour: `cargo build && target/debug/liss run conformance/errores/numerus-non-canonicus/programma; echo $?`
      → `MXIX`, `0`.
- [ ] Step 5: `cargo nextest run --no-fail-fast --test conformance`. Expected: exactly
      `errores::numerus-non-canonicus` fails, on stdout (`MXIX` against none) and exit (0 against 1);
      `defectus::D-13-numerus-non-canonicus` is no longer listed; every other trial passes.

Not committed alone — a commit with a red suite would break `git bisect`. Carry into Task 3.

## Task 3: The form check, its tests, and the bless

**Files:** `crates/runtime/src/roman.rs` (`convert_single` `:178-197`; tests
`convert_single_is_never_zero_or_negative` `:514-539`), `conformance/errores/numerus-non-canonicus/nuntii`,
`crates/lingua-serpentis/tests/snapshots/{catalogus,nuntii}__*.snap`.

**Interfaces:** `convert_single(s: &str) -> Result<Roman, String>` keeps its signature; it now also
returns `Err("numerus Romanus non canonicus: '<s>'")`. Consumes Task 2's fixture.

- [ ] Step 1: Write the failing tests in `roman.rs`'s test module. Replace
      `convert_single_is_never_zero_or_negative` with:

      ```rust
      #[test]
      fn convert_single_accepts_exactly_the_canonical_numerals() {
          // D-25, D-13
          const SYMS: [char; 7] = ['I', 'V', 'X', 'L', 'C', 'D', 'M'];
          let canonical: std::collections::HashSet<String> =
              (1..=3999).map(|v| Roman::from_u64(v).display()).collect();
          let zero = Roman::zero();
          let mut words: Vec<String> = vec![String::new()];
          let (mut checked, mut accepted) = (0usize, 0usize);
          for _ in 0..6 {
              let mut next = Vec::with_capacity(words.len() * SYMS.len());
              for w in &words {
                  for c in SYMS {
                      let w = format!("{w}{c}");
                      checked += 1;
                      match convert_single(&w) {
                          Ok(v) => {
                              accepted += 1;
                              assert!(canonical.contains(&w), "{w:?} accepted but not canonical");
                              assert!(v > zero, "convert_single({w:?}) = {} is not >= 1", v.display());
                              assert_eq!(v.display(), w);
                          }
                          Err(_) => assert!(!canonical.contains(&w), "{w:?} canonical but rejected"),
                      }
                      next.push(w);
                  }
              }
              words = next;
          }
          assert_eq!(checked, 137_256, "every string of length 1..=6 over the seven symbols");
          assert_eq!(accepted, 1_308, "the numerals in 1..=3999 of at most six letters");
      }
      ```

      and add beside it:

      ```rust
      #[test]
      fn a_non_canonical_chunk_is_named() {
          // D-13
          let non = |c: &str| Err::<RomanNumber, String>(format!("numerus Romanus non canonicus: '{c}'"));
          for (lit, chunk) in [
              ("°IIII", "IIII"), ("°XXXX", "XXXX"), ("°MMMM", "MMMM"), ("°VV", "VV"),
              ("°LL", "LL"), ("°DD", "DD"), ("°IIX", "IIX"), ("°IM", "IM"), ("°IC", "IC"),
              ("°IL", "IL"), ("°XM", "XM"), ("°VX", "VX"), ("°IIV", "IIV"), ("°IXI", "IXI"),
              ("°VIV", "VIV"), ("°MCMXCIXIII", "MCMXCIXIII"), ("°V.IIX", "IIX"),
              ("°IIX.M", "IIX"), ("°IIX.VV", "IIX"), ("°IIII:I/II", "IIII"),
              ("°I:IIX/XII", "IIX"), ("°I:I/IIX", "IIX"),
          ] {
              assert_eq!(parse_roman(lit), non(chunk), "{lit}");
          }
          for (lit, v) in [
              ("°MMMCMXCIX", 3999), ("°XXX.M.V.C.VII", 30507), ("°IV.M", 4000),
              ("°M.IV", 4000), ("°II.MM", 4000), ("°C.C.C", 10100), ("°M.M.M.M.M", 2_001_000),
          ] {
              assert_eq!(parse_roman(lit), Ok(RomanNumber::Int(Roman::from_u64(v))), "{lit}");
          }
          assert_eq!(parse_roman("°I..I"), Err("numerus Romanus vacuus".into()));
          assert_eq!(parse_roman("°I:I/II/III"), Err("numerus Romanus invalidus: 'II/III'".into()));
      }
      ```

- [ ] Step 2: `cargo nextest run -p runtime --lib convert_single a_non_canonical`. Expected: both fail —
      the enumeration on the first non-canonical string it walks, `"IL" accepted but not
      canonical`, and `a_non_canonical_chunk_is_named` on `°IIII` (`Ok(Int(…))` against `Err`).
- [ ] Step 3: In `convert_single`, between the loop's closing brace (`:195`) and `Ok(total)`, three
      lines:
      ```rust
      if total.display() != s {
          return Err(format!("numerus Romanus non canonicus: '{}'", s));
      }
      ```
      The literal inline. No comment.
- [ ] Step 4: `cargo nextest run -p runtime`. Green, including `single_rejects_garbage`,
      `int_multiplicative_pairs`, `parse_fraction_literal` and
      `parse_rejects_malformed_or_zero_fraction`, whose inputs are canonical or fail earlier.
- [ ] Step 5: `cargo build && target/debug/liss run conformance/errores/numerus-non-canonicus/programma; echo $?`.
      Expected, on stderr and with nothing on stdout:
      ```
      erratum: numerus Romanus non canonicus: 'IIX'
        in 'principale.liss', linea II
      ```
      and `1`.
- [ ] Step 6: Bless the one fixture, and only it:
      `LISS_BLESS=1 cargo test --test conformance -- numerus-non-canonicus`.
- [ ] Step 7: `git status --porcelain conformance/` — only paths under `errores/numerus-non-canonicus/`
      (and the old `defectus/` paths as renamed or deleted). `nuntii` is Step 5's two lines exactly.
- [ ] Step 8: `cargo nextest run --test nuntii`. Expected: fails with one added row,
      `errores::numerus-non-canonicus` / `erratum  principale.liss:2  numerus Romanus non canonicus: 'IIX'`,
      nothing removed or changed. Accept: `INSTA_UPDATE=always cargo nextest run --test nuntii`; header
      now `91 fixtures · 104 diagnostics`.
- [ ] Step 9: `cargo nextest run --test catalogus`. Expected: fails — one added `roman.rs:197` row, and
      `roman.rs:226` → `:229`, `:238` → `:241`. Accept with `INSTA_UPDATE=always`; header
      `120 sites · 72 fixtured · 0 ambiguous · 46 uncovered · 2 unparsed`.
- [ ] Step 10: Spec §7's set check: regenerate the sorted `(pattern, status)` list as in Task 0 Step 4
      and `diff $SCRATCH/catalogus-ante.txt -`. Expected: exactly one `>` line,
      `numerus Romanus non canonicus: '{}'  → fixtured`.
- [ ] Step 11: `cargo nextest run`. Green; total = Task 0 + 1.
- [ ] Step 12: Commit:

```bash
git add crates/runtime/src/roman.rs conformance/ crates/lingua-serpentis/tests/snapshots/
git commit -m "fix(runtime): a Roman numeral is written the way it is printed, one chunk at a time

D-13's fixture moves to errores/numerus-non-canonicus and names the
first bad chunk, 'IIX'. The enumeration D-25 rested on now proves
accepted exactly when canonical, and every accepted value at least I.
Moving the fixture's .liss shifts the fuzz seed stream:
support/mutate.rs sorts the corpus by path.

Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>"
```

## Task 4: The huge literal, accepted and rejected

**Files:** `crates/lingua-serpentis/tests/robustness.rs` (`pathologies()`, `numerus longissimus`
`:228-235`).

**Interfaces:** none. Moves **no fixture** and no snapshot; `tests/` is invisible to the catalogue.

- [ ] Step 1: Above `numerus longissimus`'s tuple, a bare `// D-13`. Its source and its `None` stay: it
      is now a rejection pathology, and still must satisfy the invariant.
- [ ] Step 2: After it, add:
      ```rust
      (
          "catena numeri longissima",
          format!(
              "munus initium reddit ni {{ scribe(°{}); redde nullus; }}",
              vec!["MMMCMXCIX"; 500].join(".")
          ),
          MORA_ARTA,
          None,
      ),
      ```
- [ ] Step 3: `cargo nextest run --test robustness every_curated_pathology`. Green. Record both cases'
      wall time from `judge_source_intra` (add a temporary `eprintln!` and remove it, or time the
      two sources with `target/debug/liss run`) for Task 5's table.
- [ ] Step 4: `cargo nextest run` green. Commit:

```bash
git commit -am "test(robustness): five thousand M is a rejection now, and a long chain of canonical chunks still runs

Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>"
```

## Task 5: Internals documentation

**Files:** `docs/internals/abi.md`, `rationale.md`, `testing.md`.

- [ ] Step 1: `abi.md:502-514` — D-25's paragraph: `convert_single` is at least 1 for every string it
      accepts **because** an accepted chunk equals `display` of its value, and
      `convert_single_accepts_exactly_the_canonical_numerals` walks the same 137,256 strings, accepts
      1,308, and checks both. Add a sentence to the `Roman` section that parsing accepts exactly the
      classical form per chunk, 1..=3999, and that the dot form is multiplication, not spelling (spec
      §2 Non-Goals).
- [ ] Step 2: `rationale.md:94-99` (the lexer's `°` bullet) — every leaf that reaches `convert_single`
      must also be canonical, and the error quotes the chunk, not the literal; `:1070-1076` (Part 2,
      `parse_roman`) — rename the test and restate the argument as Step 1; `:1409-1411` — D-13 is no
      longer deferred: the paragraph goes, and "Deferred elsewhere" names D-20 alone.
- [ ] Step 3: `testing.md:437` — the `numerus longissimus` timing row is a rejection now; add the
      `catena numeri longissima` row with Task 4's measurement. `:463-466` — every fixed defect's
      fixture moved out of `conformance/defectus/`, which is now empty and absent from git, and
      `discover()` skips it.
- [ ] Step 4: `cargo nextest run` green (nothing here is read by a test, but the `catalogus` snapshot
      must not have moved). Commit:

```bash
git commit -am "docs(internals): a numeral chunk is its own printed form, and D-25's argument says why a denominator is never nullus

Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>"
```

## Task 6: Close D-13 in the register

**Files:** `docs/defectus/2026-08-03-index.md`, `docs/README.md`, `docs/via.md`.

- [ ] Step 1: The entry (`:427-443`) gains `- **Fixed** \`<Task 3 sha>\`, 2026-09-15 — …` stating the
      rule and the sentence, that "Expected: `numerus Romanus invalidus` for each" shipped as a new
      sentence for the **first** bad chunk (the lexer stops at its first error, and `invalidus` stays
      the message for a non-letter), and **Covered by**: `errores/numerus-non-canonicus`,
      `roman.rs`'s `convert_single_accepts_exactly_the_canonical_numerals` and
      `a_non_canonical_chunk_is_named`. The as-found text stays; its stale `roman.rs:204` is left as
      found.
- [ ] Step 2: Status row `:126` → `**fixed** \`<sha>\` → \`errores/numerus-non-canonicus\``; the D-13
      row of the deferred table (`:83`) goes.
- [ ] Step 3: Header prose — `:6` → `28 of the 30 are fixed; 1 is deferred; 1 is open`; `:48`
      (`conformance/defectus/` holds D-13's directory) → it is empty; `:74` "the eleven curated
      pathologies" → twelve; `:77-79` "The two deferred defects are language changes …" → one, with
      D-13 beside D-26 in the parenthetical as closed by `2026-09-15-roman-numeral-form-design.md`.
- [ ] Step 4: `docs/README.md:251` — `28 fixed, 1 deferred, 1 open`.
- [ ] Step 5: `docs/via.md:33` — one deferred language decision (`D-20`); `D-13` and `D-26` are
      decided. `:74` — the D-13 row → `**Shipped** 2026-09-15 (\`<sha>\`): every chunk of a literal is
      written the way its value prints, 1..=3999 (\`2026-09-15-roman-numeral-form-design.md\`).` The
      milestone heading waits for D-20.
- [ ] Step 6: Commit:

```bash
git commit -am "docs(defectus): D-13 closes, and a numeral is written the way it is printed

Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>"
```

The restated corpus counts (`CLAUDE.md`'s conformance section, including "`defectus/` (one deferred
defect)", `README.md:80`, `docs/README.md:220`, `testing.md:55-57`, `nuntii.rs:3`, the catalogue
header quoted in `CLAUDE.md` and `testing.md:284`) are updated when milestone 2 closes (spec §8), not
here.

---

## Verification

1. `cargo nextest run` — green. Total = Task 0 + 1 (`a_non_canonical_chunk_is_named`).
2. `head -5 crates/lingua-serpentis/tests/snapshots/catalogus__catalogus.snap` →
   `120 sites · 72 fixtured · 0 ambiguous · 46 uncovered · 2 unparsed`.
3. `diff $SCRATCH/catalogus-ante.txt <(sed -n '7,$p' …catalogus__catalogus.snap | sed -E 's/^[^ ]+  //' | sort)`
   → one added line, the new sentence `→ fixtured`.
4. `head -5 crates/lingua-serpentis/tests/snapshots/nuntii__nuntii.snap` →
   `91 fixtures · 104 diagnostics`; `git diff 6634fcd -- …nuntii__nuntii.snap` adds one row and
   removes none.
5. `ls conformance/errores | wc -l` → `83`; `ls conformance/defectus` → no such directory;
   `git diff --stat -M 6634fcd -- conformance/` touches only the moved directory.
6. `target/debug/liss run conformance/errores/numerus-non-canonicus/programma` reproduces `nuntii` and
   exits 1.
7. `grep -n 'every string over the symbols parses' crates/runtime/src/roman.rs` → nothing.
8. `cargo tree -e normal` → no new package.
9. `grep -n 'D-13' docs/defectus/2026-08-03-index.md docs/README.md docs/via.md docs/internals/*.md`
   → no "deferred" wording for D-13.
