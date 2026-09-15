# Magnitude Cap Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development
> (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use
> checkbox (`- [ ]`) syntax for tracking.

**Goal:** Refuse to store a number of more than XXXVIII decimal digits — for `mnr` and `nr`, a grain
count — in the compiled image, raising `numerus nimis magnus est` with no location, closing D-20;
then make the squaring loop a fixture and a pathology, and take `terminates` out of `RECORDED_RULES`
or re-attribute it honestly.

**Architecture:** One length check at the top of `int_novus` (`crates/abi/src/numeri.rs`), the
function every stored `LissInt` is built by, calling one new `-> !` helper in
`crates/abi/src/defectus.rs` that holds the inline `c"…"` sentence. No ABI signature, layout, codegen
or `runtime` change; the compiler's `Roman` stays unbounded, and D-13 makes an over-cap literal
unwritable, so there is no compile-time site.

**Tech Stack:** Rust 2024. **No new dependency.** Spec §11's table is not touched.

## Context

`x = x * x` in a loop runs until killed: `Roman` is arbitrary precision and `int_novus` stores
anything. The design doc is `docs/superpowers/specs/2026-09-15-magnitude-cap-design.md`; read §3
(the bound), §5 (the check), §7 (`RECORDED_RULES`) and §8 (catalogue impact) before starting. Where
this plan and that spec disagree, the spec governs.

Line numbers below are at `738918f`, and each task's edits move the lines the next task names. Find
the code by symbol when a number has drifted.

---

## Global Constraints

Copied from `CLAUDE.md` and spec §11. Every task's requirements implicitly include this section.

- **`cargo nextest run` is the one test command.** It runs the whole workspace, `abi` included.
  `cargo test --test conformance` is used only to bless, because `LISS_BLESS` goes through it.
- Work inside `nix develop`; if `cargo` is missing, prefix commands with `nix develop -c`.
- **Zero runtime dependencies is a rule** (spec §11). This plan adds none. Verify with
  `cargo tree -e normal`.
- **Diagnostics are Latin, inline and unique.** The one new sentence is the `c"…"` argument of
  `liss_panic` at its call site in `crates/abi/src/defectus.rs`. No Arabic digits in it.
- **Never write a `/* … */` block comment under `crates/abi/src`.** `abi.rs`'s `sententiae` greps
  that source for `c"…"` literals and skips only whole-line `//` comments.
- **The embedded runtime's profile lives in `codegen/build.rs`.** Nothing here touches it;
  `codegen/tests/pondus.rs` must stay green.
- **The source does not explain itself; `docs/internals/` does.** What stays in the source is
  `# Safety` clauses, fixture and test pointers, and a bare `D-NN`.
- **Comments move the catalogue snapshot.** Re-bless, then check the header against spec §8 and
  account for every moved row.
- **Bless narrowly, then run `tests/nuntii.rs`.** A `nuntii` that changes rather than appears is a
  regression.
- **Do not run the new conformance trial before the fix.** It hangs until the runner's 120 s
  backstop. Use `timeout 10 target/debug/liss run …` instead.
- Commit subjects are `type(scope): a sentence in the repo's voice`. Scopes are the area (`abi`,
  `robustness`, `conformance`, `defectus`, `internals`, `spec`), not the crate name. Commit with
  `git -c user.name=deadmade -c user.email=manuel.schuelein@proton.me commit`; every message ends
  with `Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>`.

---

## Task 0: Baseline

No commit.

- [ ] Step 1: `git status` clean; `git log -1` is the spec commit or later.
- [ ] Step 2: `cargo nextest run` green. Record the total (683 at `738918f`) and the per-binary
      counts for `abi` (`abi.rs`), `conformance` and `robustness`.
- [ ] Step 3: `head -5 crates/lingua-serpentis/tests/snapshots/catalogus__catalogus.snap` → 
      `120 sites · 72 fixtured · 0 ambiguous · 46 uncovered · 2 unparsed`; `nuntii__nuntii.snap` →
      `91 fixtures · 104 diagnostics`.
- [ ] Step 4: Save the before-set for spec §8's check:
      `sed -n '7,$p' crates/lingua-serpentis/tests/snapshots/catalogus__catalogus.snap | sed -E 's/^[^ ]+  //' | sort > $SCRATCH/catalogus-ante.txt`
      (`$SCRATCH` is any directory outside the repository).
- [ ] Step 5: `for g in currunt errores monita; do echo $g $(ls conformance/$g | wc -l); done`
      → `35 83 4`.

If a count differs from the spec, record the measured number here and carry on against it; do not
edit the spec's before-header.

## Task 1: The design doc — DONE

`docs/superpowers/specs/2026-09-15-magnitude-cap-design.md`; the amendment notices at the top of
`2026-07-23-roman-native-integers-design.md`, `2026-07-23-roman-native-fractions-design.md` and
`2026-08-07-compiler-backend-design.md`; this plan; and `docs/README.md`: the question-table row, the
spec-table row and "Sixteen documents", the integers and fractions rows' amendment counts, the
numeric-types and compiler entries of the amends chain, the reading list, and "Seventeen plans".

---

## Task 2: The fixture, hanging

**Files:** `conformance/errores/numerus-nimis-magnus/{programma/principale.liss,exitus,NOTA.md}`.

**Interfaces:** produces the trial `errores::numerus-nimis-magnus`, which Task 3 turns green. No
`nuntii` yet; Task 3 blesses it.

- [ ] Step 1: Write `programma/principale.liss`, spec §1's program exactly:
      ```liss
      // D-20: 'x = x * x' sine fine.
      munus initium reddit ni {
          ni x = °II;
          indem (verus) {
              x = x * x;
          }
          redde nullus;
      }
      ```
- [ ] Step 2: `exitus` = `1`.
- [ ] Step 3: `NOTA.md`: a `# Numerus nimis magnus` heading; "D-20 is fixed." then what the program
      did before (ran until killed; no output, no diagnostic; each squaring doubles the digits and
      quadruples the time); that a stored number has at most XXXVIII decimal digits, and for `mnr`
      and `nr` that is the grain count; that 2^64 is stored and 2^128, thirty-nine digits, is refused
      at the seventh squaring; that the check is in `int_novus`, and carries no line because
      `liss_int_mul` takes none.
- [ ] Step 4: `cargo build && timeout 10 target/debug/liss run conformance/errores/numerus-nimis-magnus/programma; echo $?`
      → nothing printed, `124`.

Not committed alone — a commit with a hanging trial would break `git bisect`. Carry into Task 3.

## Task 3: The cap, its tests, and the bless

**Files:** `crates/abi/src/numeri.rs` (`int_novus` `:48`, the `use crate::defectus::…` import `:8`),
`crates/abi/src/defectus.rs` (append after `memoria_exhausta`, `:199-209`), `crates/abi/tests/abi.rs`
(`sententiae` expected list `:194-206`, `arithmetic_is_unbounded` `:345`, `scaena` `:518`),
`crates/lingua-serpentis/tests/robustness.rs` (`pathologies()` `:206`),
`conformance/errores/numerus-nimis-magnus/nuntii`,
`crates/lingua-serpentis/tests/snapshots/{catalogus,nuntii}__*.snap`.

**Interfaces:** `int_novus(kind: u32, r: &Roman) -> *mut LissInt` keeps its signature and now never
returns for `r.digits().len() > 38`. New `pub(crate) fn numerus_nimis_magnus() -> !` in `defectus`.
No exported symbol changes. Consumes Task 2's fixture.

- [ ] Step 1: Write the failing tests.

      In `abi.rs`, add `"numerus nimis magnus est",` as the last entry of the list in
      `every_latin_sentence_this_crate_can_say_is_listed_here`, after `"modulus per nullum",`.

      Rename `arithmetic_is_unbounded` to `arithmetic_passes_a_machine_word`; body unchanged.

      After it, add:

      ```rust
      #[test]
      fn a_number_of_xxxviii_digits_is_stored() {
          // D-20
          solus(|| unsafe {
              assert_eq!(liss_live_count(), 0);
              let novem = [9u8; 38];
              assert_eq!(
                  consume(liss_int_new(KIND_INT, 0, novem.as_ptr(), 38)),
                  (false, vec![9; 38]),
              );
              assert_eq!(
                  consume(liss_int_new(KIND_INT, 1, novem.as_ptr(), 38)),
                  (true, vec![9; 38]),
              );
              let maximus = liss_int_new(KIND_INT, 0, novem.as_ptr(), 38);
              assert_eq!(consume(liss_int_sub(ni(0), maximus)), (true, vec![9; 38]));
              assert_eq!(liss_live_count(), 0);
          });
      }
      ```

      In `scaena`'s `match`, before `other => panic!(…)`, add:

      ```rust
      "numerus-xxxix" => {
          let mut decem = [0u8; 39];
          decem[38] = 1;
          liss_int_new(KIND_INT, 0, decem.as_ptr(), 39);
      }
      "summa-xxxix" => {
          let novem = [9u8; 38];
          liss_int_add(liss_int_new(KIND_INT, 0, novem.as_ptr(), 38), ni(1));
      }
      ```

      and after `an_exhausted_heap_is_a_latin_exit_and_never_an_abort`:

      ```rust
      #[test]
      fn a_number_of_xxxix_digits_raises() {
          // D-20
          for scaena in ["numerus-xxxix", "summa-xxxix"] {
              assert_eq!(
                  erratum(scaena),
                  (Some(1), "erratum: numerus nimis magnus est\n".to_string()),
                  "{scaena}",
              );
          }
      }
      ```

      In `robustness.rs`'s `pathologies()`, after `catena numeri longissima`'s tuple, add:

      ```rust
      // D-20
      (
          "quadratum sine fine",
          "munus initium reddit ni { ni x = °II; indem (verus) { x = x * x; } redde nullus; }"
              .to_string(),
          MORA_ARTA,
          None,
      ),
      ```

- [ ] Step 2: Run them and verify they fail.
      `cargo nextest run -p abi --test abi` — expected failures:
      `every_latin_sentence_this_crate_can_say_is_listed_here` ("a Latin sentence appeared or
      changed", the expected list longer than the found one) and `a_number_of_xxxix_digits_raises`
      (`Some(101)` against `Some(1)`: the scenario returns and reaches `unreachable!`).
      `a_number_of_xxxviii_digits_is_stored` passes — it pins the side of the bound that must not
      move. `cargo nextest run --test robustness every_curated_pathology` — fails with
      `quadratum sine fine: expected None, got Violation { rule: "terminates", … }` after
      `MORA_ARTA`.
- [ ] Step 3: Append to `crates/abi/src/defectus.rs`, after the blank line that follows
      `memoria_exhausta`'s closing brace, exactly:

      ```rust
      /// D-20
      pub(crate) fn numerus_nimis_magnus() -> ! {
          unsafe {
              liss_panic(
                  ptr::null(),
                  0,
                  c"numerus nimis magnus est".as_ptr(),
                  ptr::null(),
                  ptr::null(),
              )
          }
      }
      ```

      `liss_panic(` lands on `:214`. No `/* */`.
- [ ] Step 4: In `numeri.rs`, add `numerus_nimis_magnus` to the `use crate::defectus::{…}` import,
      and make `int_novus` open:

      ```rust
      let digiti = r.digits();
      // D-20
      if digiti.len() > 38 {
          numerus_nimis_magnus()
      }
      ```

      before the `ONUS.checked_add` line.
- [ ] Step 5: `cargo nextest run -p abi`. Green, including `arithmetic_passes_a_machine_word`
      (22 digits) and
      `fractions_share_the_integer_engine_and_differ_only_where_they_must`.
- [ ] Step 6: `cargo build && target/debug/liss run conformance/errores/numerus-nimis-magnus/programma; echo $?`.
      Expected: stderr exactly `erratum: numerus nimis magnus est`, nothing on stdout, `1`, well under
      a second.
- [ ] Step 7: Bless the one fixture, and only it:
      `LISS_BLESS=1 cargo test --test conformance -- numerus-nimis-magnus`.
- [ ] Step 8: `git status --porcelain conformance/` — only `errores/numerus-nimis-magnus/` paths.
      `nuntii` is Step 6's one line.
- [ ] Step 9: `cargo nextest run --test nuntii`. Expected: fails with one added row,
      `errores::numerus-nimis-magnus` / `erratum  -  numerus nimis magnus est`, nothing removed or
      changed. Accept: `INSTA_UPDATE=always cargo nextest run --test nuntii`; header now
      `92 fixtures · 105 diagnostics`.
- [ ] Step 10: `cargo nextest run --test catalogus`. Expected: fails — one added
      `crates/abi/src/defectus.rs:214  numerus nimis magnus est  → fixtured` row, no other row
      moved. Accept with `INSTA_UPDATE=always`; header
      `121 sites · 73 fixtured · 0 ambiguous · 46 uncovered · 2 unparsed`.
- [ ] Step 11: Spec §8's set check: regenerate the sorted list as in Task 0 Step 4 and
      `diff $SCRATCH/catalogus-ante.txt -`. Expected: exactly one `>` line,
      `numerus nimis magnus est  → fixtured`.
- [ ] Step 12: `cargo nextest run --test robustness every_curated_pathology`. Green.
- [ ] Step 13: `cargo nextest run`. Green; total = Task 0 + 3. `cargo tree -e normal` — no new
      package.
- [ ] Step 14: Commit:

```bash
git add crates/abi/ crates/lingua-serpentis/tests/robustness.rs conformance/ crates/lingua-serpentis/tests/snapshots/
git -c user.name=deadmade -c user.email=manuel.schuelein@proton.me commit -m "fix(abi): a number past XXXVIII digits is not stored, and the squaring loop ends

int_novus refuses to box more than thirty-eight decimal digits, grain
counts for mnr and nr, with numerus nimis magnus est and no location.
errores/numerus-nimis-magnus is D-20's loop, refused at 2^128; abi.rs
pins both sides of the bound, and quadratum sine fine is a pathology.
The new .liss shifts the fuzz seed stream: support/mutate.rs sorts the
corpus by path.

Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>"
```

## Task 4: `RECORDED_RULES`

**Files:** `crates/lingua-serpentis/tests/support/diag.rs` (`RECORDED_RULES` `:152-155`).

**Interfaces:** `pub const RECORDED_RULES: [&str; N]`, consumed by `robustness.rs:330` and
`generative.rs:107`. Moves no fixture and no snapshot.

- [ ] Step 1: Replace the constant, keeping its doc comment:
      ```rust
      pub const RECORDED_RULES: [&str; 0] = [];
      ```
- [ ] Step 2: `cargo nextest run --test robustness --test generative`. Record the result of
      `no_mutation_of_any_known_program_escapes_the_error_model` (300 draws, seed `0x5EED`) and
      `generated_programs_never_escape_the_error_model`.
- [ ] Step 3: `LISS_FUZZ_CASES=3000 cargo nextest run --release --test robustness no_mutation_of_any_known_program_escapes_the_error_model`.
      Record the wall time and the result.
- [ ] Step 4: Decide, per spec §7:
      - **Both clean** → keep `[&str; 0]`. Leave the two `RECORDED_RULES.contains(&rule)` arms and
        their imports unless the build warns or fails; if it does, remove only what it names.
      - **A `terminates` violation** → save the printed source, run it with
        `timeout 600 target/release/liss run`, and record whether and when it ends. Restore
        `pub const RECORDED_RULES: [&str; 1] = ["terminates", // D-31\n];` and carry the source and
        measurement to Task 6 Step 5.
      - **Any other rule** → stop and report it as a new defect; do not allowlist it.
- [ ] Step 5: `cargo nextest run` green. Commit, with the subject for the outcome:

```bash
git commit -am "test(robustness): terminates is tolerated no longer, because nothing grows without bound

Three hundred draws at 0x5EED and three thousand in release are clean
with RECORDED_RULES empty.

Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>"
```

or

```bash
git commit -am "test(robustness): terminates is tolerated for a loop a mutation writes, and D-20 no longer holds it

<the draw, its source's shape, and how long it ran>.

Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>"
```

(both with `git -c user.name=deadmade -c user.email=manuel.schuelein@proton.me`).

## Task 5: Internals documentation

**Files:** `docs/internals/abi.md`, `codegen.md`, `rationale.md`, `testing.md`;
`conformance/currunt/arithmetica-illimitata/NOTA.md`.

- [ ] Step 1: `abi.md` §9, the `int_novus` bullet (`:449-454`) — it refuses a number of more than
      XXXVIII digits before allocating, with `numerus nimis magnus est` and no location; for `mnr`
      and `nr` that is the grain count, so the value ranges are the cap over XII and CCLXXXVIII; every
      caller and whether it can reach the bound (spec §5's table, by symbol); and why no literal can
      (spec §6, one paragraph). §13's catalogue table (`:1025-1034`) gains
      `| numerus nimis magnus est | defectus::numerus_nimis_magnus | int_novus would store more than XXXVIII digits |`,
      and the text says it is the one row of the table a fixture provokes.
- [ ] Step 2: `codegen.md` `Corpus::numerus` (`:786-791`) — "the language's integers are unbounded and
      no immediate could carry one" → a stored number has up to XXXVIII digits, which no machine word
      carries, and boxing is uniform ownership rather than width (spec §2 Non-Goals).
- [ ] Step 3: `rationale.md` "Deferred elsewhere" (`:1405-1413`) — D-20 is no longer deferred; the
      paragraph and its heading go, and a Part 2 note under `abi`'s `numeri.rs` records what the loop
      did and why the check is in `int_novus` and not in `Roman` (spec §2).
- [ ] Step 4: `testing.md:421-425` — `RECORDED_RULES` per Task 4: "currently none" and six entries
      retired, `terminates` (D-20) last; or "currently one, `terminates` (D-31)". The curated
      pathology count, wherever it is stated, rises by one.
- [ ] Step 5: `arithmetica-illimitata/NOTA.md` — one sentence: twenty-two digits is inside the
      XXXVIII-digit cap D-20 set; the fixture name is history.
- [ ] Step 6: `cargo nextest run` green (the catalogue snapshot must not have moved). Commit:

```bash
git commit -am "docs(internals): a stored number has at most XXXVIII digits, and nothing is deferred elsewhere

Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>"
```

## Task 6: Close D-20 in the register

**Files:** `docs/defectus/2026-08-03-index.md`, `docs/README.md`, `docs/via.md`.

- [ ] Step 1: The entry (`:637-656`) gains `- **Fixed** \`<Task 3 sha>\`, 2026-09-15 — …`: a stored
      number has at most XXXVIII decimal digits, grain counts for `mnr` and `nr`, refused in
      `int_novus` with `numerus nimis magnus est` and no location; the entry's "Expected: … a
      diagnostic once a value exceeds some documented magnitude" shipped as that; the repro is
      fixture-shaped now; and Task 4's outcome. **Covered by**: `errores/numerus-nimis-magnus`,
      `abi.rs`'s `a_number_of_xxxviii_digits_is_stored` and `a_number_of_xxxix_digits_raises`,
      `robustness.rs`'s `quadratum sine fine`. The as-found text, including its stale `roman.rs`
      line numbers and D-23's "already allowlisted via D-20", stays as found.
- [ ] Step 2: Status row `:135` → `**fixed** \`<sha>\` → \`errores/numerus-nimis-magnus\``. The
      deferred table (`:84-86`) and its lead-in (`:78-82`) go: no defect is deferred, and the
      sentence naming D-26's and D-13's specs gains this one.
- [ ] Step 3: Header prose — `:6` → `29 of the 30 are fixed; 1 is open` (or the D-31 count);
      `:51-52` "Only `terminates` remains, held open by the deferred D-20" → Task 4's outcome;
      `:75` "the twelve curated pathologies" → thirteen.
- [ ] Step 4: `docs/README.md`'s register sentence → `29 fixed, 0 deferred, 1 open` (or the D-31
      count).
- [ ] Step 5: **Only if Task 4 restored the entry:** file D-31 after D-30 — severity, the mutated
      source (or its shape and draw number), Observed (Task 4 Step 4's measurement), Expected, and
      Cause: a mutation can write a loop that no bound on numbers ends; `terminates` is tolerated for
      that, attributed to D-31, not D-20. Add its status row.
- [ ] Step 6: `docs/via.md` — `:33` "One deferred language decision (`D-20`) sits in …" → all three
      decisions are taken; `:75` the D-20 row → `**Shipped** 2026-09-15 (\`<sha>\`): a stored number
      has at most XXXVIII decimal digits, and past it the program ends with a Latin sentence
      (\`2026-09-15-magnitude-cap-design.md\`).` The milestone heading waits for milestone 2's close.
- [ ] Step 7: Commit:

```bash
git commit -am "docs(defectus): D-20 closes, and a number has at most XXXVIII digits

Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>"
```

The restated counts (`CLAUDE.md`'s catalogue header and corpus counts, `README.md:80`,
`docs/README.md`'s fixture count, `testing.md`, `nuntii.rs:3`) are updated when milestone 2 closes
(spec §9), not here.

---

## Verification

1. `cargo nextest run` — green. Total = Task 0 + 3.
2. `head -5 crates/lingua-serpentis/tests/snapshots/catalogus__catalogus.snap` →
   `121 sites · 73 fixtured · 0 ambiguous · 46 uncovered · 2 unparsed`.
3. `diff $SCRATCH/catalogus-ante.txt <(sed -n '7,$p' …catalogus__catalogus.snap | sed -E 's/^[^ ]+  //' | sort)`
   → one added line, `numerus nimis magnus est  → fixtured`.
4. `head -5 crates/lingua-serpentis/tests/snapshots/nuntii__nuntii.snap` →
   `92 fixtures · 105 diagnostics`; `git diff 738918f -- …nuntii__nuntii.snap` adds one row and
   removes none.
5. `ls conformance/errores | wc -l` → `84`.
6. `time target/debug/liss run conformance/errores/numerus-nimis-magnus/programma` → the `nuntii`
   line on stderr, exit 1, under a second.
7. `grep -rn 'numerus nimis magnus est' crates/*/src` → one line, `crates/abi/src/defectus.rs`.
8. `grep -rn 'D-20' crates/lingua-serpentis/tests/support/diag.rs` → nothing.
9. `cargo tree -e normal` → no new package; `cargo nextest run -p codegen --test pondus` green.
10. `grep -n 'D-20' docs/defectus/2026-08-03-index.md docs/README.md docs/via.md docs/internals/*.md`
    → no "deferred" wording for D-20.
