# Lingua Serpentis — Magnitude Cap Design

- **Date:** 2026-09-15
- **Status:** Proposed
- **Author:** Manuel (with Claude)
- **Relationship:** Amends `2026-07-23-roman-native-integers-design.md` §1 (the "unbounded vinculum
  numeral" of `°MMM * °MMM`, :49), §3.6 "No overflow" (:236-245) and §5, which **removed**
  `numerus nimis magnus (excessus arithmeticus)` (:374-376). Amends
  `2026-07-23-roman-native-fractions-design.md` §3.6 "No overflow" (:266-270), and with it §3.1's
  "it is unbounded (no overflow, §3.6)" (:123-124). Amends `2026-08-07-compiler-backend-design.md`
  §5.4's "The runtime's own vocabulary" (:464-494) and §10.3's list of runtime sentences
  (:1229-1233). Closes deferred defect D-20 (`defectus/2026-08-03-index.md`). Cites, and does not
  amend, `2026-09-15-roman-numeral-form-design.md` §10 (:392-397), whose per-chunk rule is what
  makes an over-cap literal unwritable (§6), and the compiler-backend spec's §2 Non-Goal "No
  resolution of D-13 or D-20" (:94), which was that milestone's scope and not a language rule. Adds
  **no** dependency: §11's table is untouched.

## 1. Vision

`Roman` is arbitrary precision (`crates/runtime/src/roman.rs`: little-endian decimal digits in a
`Vec<u8>`), and the compiled image stores whatever it computes (`int_novus`,
`crates/abi/src/numeri.rs:48`). Nothing is ever too big. With the binary built from `738918f`:

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

```
$ timeout 20 liss run quadratum; echo $?
124
```

Twenty seconds, then killed: no byte on stdout, no byte on stderr, no diagnostic. The same loop
bounded at k squarings, built with `liss build` and timed on the same machine:

| k | digits of 2^(2^k) | run time |
|---|---|---|
| XII | 1,234 | 0.011 s |
| XIII | 2,467 | 0.030 s |
| XIV | 4,933 | 0.145 s |
| XV | 9,865 | 0.544 s |
| XVI | 19,729 | 1.814 s |

Each squaring doubles the digits and multiplies the time by about four, so the unbounded loop is
not a slow program, it is an unrepresentable one. The register filed it as deferred until a spec
chose a limit (D-20, and `via.md` milestone 2). This document chooses: **a stored number has at
most XXXVIII decimal digits, and a program that would store more ends.**

```
erratum: numerus nimis magnus est
```

The loop above stores 2^64 (twenty digits) after its sixth squaring and is refused at its seventh,
2^128 being thirty-nine digits. It exits 1 at once.

What it buys beyond the diagnostic: every operation's inputs are bounded, so every operation's cost
is bounded; `terminates` stops being tolerated for D-20's sake (§7); and a program's output for one
`scribe` of a number is bounded too.

## 2. Goals and Non-Goals

### Goals

- **One bound, stated three ways** — a digit count on the stored `LissInt`, and the value range it
  gives `ni`, `mnr` and `nr` (§3).
- **One new sentence**, inline at its `liss_panic` call site, unique in the workspace, every word
  passing `latinitas::is_latin`, no Arabic digit and no hole (§4).
- **One check, at the one chokepoint** every stored number passes through, so no entry point can
  forget it (§5).
- **No compile-time site**, because after D-13 an over-cap literal cannot be written; the argument is
  computed, not asserted (§6).
- **`RECORDED_RULES` loses `terminates`**, or keeps it under an honest new name if the fuzzer shows a
  loop no cap can end (§7).
- **The squaring loop becomes a fixture** and a curated pathology, and the boundary is pinned on both
  sides in `abi`'s own tests (§9).

### Non-Goals

- **Not a representation change.** `LissInt` stays `{hdr, len, neg}` plus a digit payload
  (`abi/src/lib.rs`), `ONUS` and the codegen digit blob are unchanged, every `ni` is still boxed, and
  `pondus.rs` sees the same objects. A 38-digit cap would fit an `i128`, but nothing here stores one;
  `internals/codegen.md`'s boxing rationale ("the language's integers are unbounded and no immediate
  could carry one") is re-worded, not acted on.
- **Not a location.** `liss_int_add`, `liss_int_sub`, `liss_int_mul`, `liss_int_neg` and
  `liss_frac_mul` take no `(file, line)`. Giving them one changes five exports, five inline call
  signatures at codegen's `voca_unum` sites — which nothing cross-checks against `numeri.rs`, so an
  arity slip is silent undefined behaviour, not a build error — and every `abi.rs` call. The
  sentence has the shape of `memoria exhausta est`: a sentence and nothing else.
- **Not a cap in `runtime`.** `Roman::add`, `mul` and `div_rem` stay total and unbounded. Returning
  `Option` there would turn the checker's `expect("a literal's denominator is never zero")` and
  `convert_int_roman`'s arithmetic into reachable English panics, the thing D-28's close forbids.
  `roman.rs`'s `multiply_is_unbounded` and `fraction.rs`'s `display_handles_unbounded_whole_part`
  stay true and stay as they are. The cap is a property of the compiled image.
- **Not a value cap for fractions.** The cap applies to the stored grain count (§3). A cap on the
  value would need the grain inside `liss_int_add` and `liss_int_sub`, which `ni`, `mnr` and `nr`
  share and which never see it.
- **Not a machine width.** No `i64`, `i128` or `usize` bound: the check is a digit count, which is
  sign-symmetric, so `-x` is total for every stored `x` and both musl targets agree by construction.
- **Not a faster `mag_divmod`.** Division is roughly cubic in digits, which matters at thousands of
  digits and not at thirty-eight.
- **Not a warning, and not configurable.** A limit a program can raise is a limit a program can
  reintroduce D-20 with.

## 3. The bound

> **A stored number has at most XXXVIII decimal digits: its magnitude is at most 10³⁸ − 1, whatever
> its sign.**

"Stored" means boxed as a `LissInt` by `int_novus`. For `ni` the stored number is the value; for
`mnr` and `nr` it is the **grain count** (`2026-07-23-roman-native-fractions-design.md` §3.1: an
`mnr` counts unciae, twelve to the `as`; an `nr` counts scripula, 288 to the `as`), so the three
value ranges differ:

| type | stored | largest magnitude | largest whole part |
|---|---|---|---|
| `ni` | the value | 10³⁸ − 1 = 99 999 999 999 999 999 999 999 999 999 999 999 999 (38 digits) | the same |
| `mnr` | unciae | (10³⁸ − 1)/12 | 8 333 333 333 333 333 333 333 333 333 333 333 333 (37 digits), and 3/12 |
| `nr` | scripula | (10³⁸ − 1)/288 | 347 222 222 222 222 222 222 222 222 222 222 222 (36 digits), and 63/288 |

A number whose digit count is 38 is stored; one whose digit count is 39 is not. `nullus` has zero
digits. Nothing is rounded, saturated or wrapped: the operation that would store the 39-digit number
ends the program instead.

**The milestone plan wrote the bound as `|x| < 10^XXXIX`.** That is thirty-nine digits and is off by
one against the decision it records, "at most XXXVIII digits". This document is the authority:
`|x| < 10^XXXVIII`, and the boundary tests (§9) are 38 accepted and 39 refused.

**Why thirty-eight.**

- **The corpus does not move.** The largest number any program or test in the repository stores is
  3999⁶ = 4 089 859 838 720 239 976 001, twenty-two digits (`currunt/arithmetica-illimitata`,
  `abi.rs`'s `arithmetic_is_unbounded`). Nineteen digits, `i64`'s range, would break both.
- **Every operation is cheap at the bound.** An operand is at most 38 digits, so `mag_mul`'s
  intermediate is at most 76, `fraction::mul`'s `c1·c2` at most 76 before the grain divides it, and
  `fraction::div`'s `c1·288` at most 41. One `scribe` of the largest `ni` writes 38 glyph runs with
  at most twelve overlines each — on the order of a kilobyte.
- **It is a Roman quantity.** "At most XXXVIII tiers" is sayable in the language's own terms, and no
  vinculum numeral of the bound has to be printed or quoted (§4).

## 4. The sentence

```
numerus nimis magnus est
```

*The number is too large.*

| word | `latinitas::is_latin` |
|---|---|
| `numerus` | true |
| `nimis` | true |
| `magnus` | true |
| `est` | true |

Measured on 2026-09-15 with the binary built from `738918f`: a program declaring `ni numerus`, `ni
nimis`, `ni magnus` and `ni est` compiled with no `monitum`, while the control name `zzqx` in the
same program drew `nomen 'zzqx' fortasse non Latinum est`. The Latinity lint is `latinitas::is_latin`
on each declared name (`latinity.rs`).

- **No hole.** The bound as a numeral is a vinculum run no reader can check, and a hole must be a
  `LissInt` (spec §5.4); the offending value is by definition one the program could not store. The
  bound is stated here and in `internals/abi.md`, not in the sentence. No Arabic digit.
- **Unique.** No literal in `crates/*/src` contains `nimis magnus`; the only other `nimis` is the
  parser's `expressio nimis alte implicata`. A shared pattern would read `ambiguous`.
- **Inline.** The `c"…"` literal is the argument of `liss_panic` at its call site, per spec §10.3,
  inside a `-> !` helper in `defectus.rs` as every other runtime sentence is.
- **The register it joins.** Subject, adjective, `est` — the family of `memoria exhausta est` and
  `acervus vocationum exhaustus est`, the two other sentences about a limit of the machine. It is
  the interpreter's `numerus nimis magnus (excessus arithmeticus)`, which the integers spec §5
  removed, returned without its parenthetical: `excessus arithmeticus` named an `i64` overflow, and
  there is no `i64`.

The output is one line on stderr, `erratum: numerus nimis magnus est`, after `liss_panic` has flushed
stdout (D-08), and the exit status is 1.

## 5. Where the check lives

**In `int_novus`, before it allocates.** Every `LissInt` the image creates from a value is created
there; the one other constructor, `liss_int_copy` (`numeri.rs:106`), copies a number that is already
stored. `allocare(…).cast::<LissInt>()` occurs once in `crates/abi/src` (`numeri.rs:53`).

```rust
pub(crate) unsafe fn int_novus(kind: u32, r: &Roman) -> *mut LissInt {
    let digiti = r.digits();
    // D-20
    if digiti.len() > 38 {
        numerus_nimis_magnus()
    }
    // … unchanged
```

and, appended after `memoria_exhausta` at the end of `crates/abi/src/defectus.rs`:

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

Every caller of `int_novus`, measured at `738918f`, and whether it can reach the bound:

| caller | site | can it store 39 digits | why |
|---|---|---|---|
| `liss_int_add` | `numeri.rs:142` | yes | a sum gains a digit |
| `liss_int_sub` | `numeri.rs:156` | yes | a difference of opposite signs gains a digit |
| `liss_int_mul` | `numeri.rs:171` | yes | the loop in §1 |
| `liss_int_neg` | `numeri.rs:185` | no | the magnitude is unchanged, and the cap is sign-symmetric |
| `liss_int_div` | `numeri.rs:207` | no | \|a / b\| ≤ \|a\| for `ni`; it passes the check at no cost |
| `liss_int_mod` | `numeri.rs:230` | no | \|a % b\| < \|b\| |
| `liss_frac_mul` | `numeri.rs:265` | yes | `c1·c2/g` |
| `liss_frac_div` | `numeri.rs:290` | yes | `c1·g/c2`, and `c2` may be one grain |
| `liss_int_new` | `numeri.rs:94` | not from compiled code (§6) | a literal's digit blob |
| `liss_list_len` | `series.rs:210` | no | a `u64` has at most twenty digits |
| `index_extra_fines` | `defectus.rs:169` | no | a `u64`; so the check never re-enters `liss_panic` while it is raising |

**A check after the arithmetic is enough.** Every operand was stored, so it has at most 38 digits,
and the work done before `int_novus` sees the result is bounded by one 38-by-38 multiply and one
divide by a grain. The loop of §1 stops at the first result that would exceed the bound; it never
computes the next.

**A future entry point that builds a number must call `int_novus`**, and nothing else may cast an
allocation to `LissInt`. That is a coupling (§11), not a new rule: it is how every entry point is
written today.

**Both targets are identical.** The check compares a `Vec` length with a constant; `x86_64` and
`aarch64` musl are both 64-bit, and neither the helper nor the comparison can panic, so `pondus.rs`'s
no-landing-pad property is untouched.

## 6. Why no literal can exceed the cap

A literal reaches the image as a digit blob handed to `liss_int_new`, which calls `int_novus`. If a
literal could exceed the bound, a compile-time error would become a run-time one with no line. It
cannot, because of D-13, and the argument is the one `2026-09-15-roman-numeral-form-design.md` §10
promised this document would make.

**Integer literals.** `convert_int_roman` (`roman.rs:201`) splits the literal at `.`, multiplies
adjacent chunks in pairs, adds the pairs, and adds a trailing odd chunk. Under D-13 every chunk is a
classical numeral in 1..=3999. So a literal's value is a sum of products `a·b` of two such chunks,
and each product costs the bytes `a.b.` — the two chunks and two dots.

The most value per byte any pair buys was enumerated over all 3999 values on 2026-09-15: the
longest-value classical numeral of each length is `M`, `MM`, `MMM`, `MMMD`, `MMMCM`, `MMMCML`,
`MMMCMXC`, … `MMMCMXCIX` (3999, nine letters), and `a·b / (len a + len b + 2)` peaks at
`MMMCM.MMMCM.` — 3900², 15 210 000 for twelve bytes, **1 267 500 per byte**. The trailing odd
chunk is at most 3999 and changes nothing. Hence:

> **value ≤ 1 267 500 × (bytes of the literal) + 3999**

Reaching 10³⁸ takes at least **7.9 × 10³¹ bytes** of literal.

**Fraction literals.** `parse_roman` (`roman.rs:244`) builds `numer = W·D + N` from the whole part,
numerator and denominator, and the checker stores `nr`'s count `trunc(numer · 288 / D)`
(`checker.rs:253`), re-narrowed onto `mnr`'s grid by dividing again (`checker.rs:752`). With `D` ≥ 1
(D-25), the count is at most `288 · (W + N)`, and `W` and `N` are dot chains bounded as above.
Reaching 10³⁸ takes at least **2.7 × 10²⁹ bytes**.

**Both exceed any source file that can exist.** The source is read into a Rust `String`, whose
length is a `usize`: at most 2⁶⁴ − 1 ≈ 1.8 × 10¹⁹ bytes on either target. The shortest over-cap
literal is ten billion times longer than that.

**Nothing else computes a stored number at compile time.** The only constant fold is a negated
literal, one level deep (`emissio.rs`'s `Unary{Neg}` over a `Const`), and negation keeps the digit
count. A list literal is its elements, each its own literal.

So `liss_int_new` never receives an over-cap blob from compiled code, and the check it passes
through is reachable only by a test that calls it directly — which is how its boundary is pinned
(§9). Before D-13, `°M…M.M…M` multiplied two unbounded chunks, and a literal of about six megabytes
exceeded `i64`; this argument could not have been made, and a compile-time site would have been
needed.

## 7. `RECORDED_RULES`

`crates/lingua-serpentis/tests/support/diag.rs:153`:

```rust
pub const RECORDED_RULES: [&str; 1] = [
    "terminates", // D-20
];
```

It is consulted in two places: the mutation fuzzer (`robustness.rs:330`, 300 draws at seed
`0x5EED`, deadline `T` = 60 s) and the generated-program property (`generative.rs:107`, 64 draws,
10 s). The curated `pathologies()` do not consult it.

D-20 is the reason the entry exists, and after §5 no program can grow a number without bound. But a
mutation can also write a loop that never ends for a reason no cap touches — delete a step, flip a
`+` — and today `terminates` absorbs that silently. Whether the fuzzer's own stream contains one is
an empirical question, so the entry is not simply deleted:

1. **After the fix and the fixture land** — the new `.liss` under `conformance/` shifts the fuzz
   stream (`support/mutate.rs`'s `seed_corpus` sorts by path), and the decision must be taken on the
   stream the suite will run — make it `pub const RECORDED_RULES: [&str; 0] = [];`.
2. Run the suite's own draws: `cargo nextest run --test robustness --test generative`.
3. Run a wider draw against the release binary:
   `LISS_FUZZ_CASES=3000 cargo nextest run --release --test robustness no_mutation_of_any_known_program_escapes_the_error_model`.
4. Decide:
   - **Both clean:** the array stays `[&str; 0]`. The two `RECORDED_RULES.contains(&rule)` arms stay:
     they compile over an empty array and are the slot a future entry would need. They are removed
     only if the compiler forces it.
   - **A `terminates` violation:** take the printed source and run it with
     `timeout 600 target/release/liss run`. Whether it ends slowly or never, it is **not D-20** — no
     number in it can grow past the cap. Restore the entry as `"terminates", // D-31` and file
     **D-31** in the register with the source, the measured behaviour and its cause. D-20's name is
     not kept on it.
   - **Any other rule:** a new defect unrelated to D-20. It is filed, not allowlisted, and the
     implementation stops to report it.

`generative.rs` emits one expression inside `computa` and no loop (`program()`), so it cannot write
a non-terminating program; it is run because it consults the constant.

Which of the three happened is recorded in the commit that edits `diag.rs` and in D-20's **Fixed**
bullet.

## 8. What this does to the catalogue

Measured from `crates/lingua-serpentis/tests/snapshots/catalogus__catalogus.snap` at `738918f`:

```
before:  120 sites · 72 fixtured · 0 ambiguous · 46 uncovered · 2 unparsed
after:   121 sites · 73 fixtured · 0 ambiguous · 46 uncovered · 2 unparsed
```

Every row difference, for the helper appended exactly as §5 writes it — a blank line after
`memoria_exhausta`'s closing brace at `defectus.rs:209`, the doc line at `:211`, `liss_panic(` at
`:214`:

| row | change | cause |
|---|---|---|
| `crates/abi/src/defectus.rs:214  numerus nimis magnus est  → fixtured` | **added** | §4; fixtured by `errores/numerus-nimis-magnus` (§9) |

**No row moves.** The helper sits below every existing `defectus.rs` site, and `numeri.rs` holds no
site: its two added lines call a helper and carry no literal. The extractor counts the site because
`liss_panic` is one of its call markers (`catalogus.rs:336`) and the literal is written at the call,
exactly like `memoria exhausta est` at `:201`. If the helper is written in a different number of
lines, only the new row's `:line` differs; the check that does not depend on it: **the sorted set of
`(pattern, status)` pairs after equals the set before plus exactly one pair**,
`(numerus nimis magnus est, fixtured)`.

## 9. Fixtures, tests, and what the counts become

**One fixture is added: `conformance/errores/numerus-nimis-magnus/`.**

- `programma/principale.liss` is §1's program, byte for byte. Its first line is a comment, as every
  `errores/` program's is; this diagnostic carries no line, so it could not land on line 1 anyway.
- `exitus` = `1`.
- `nuntii` is blessed (§10), and is exactly one line:
  ```
  erratum: numerus nimis magnus est
  ```
  the shape of `errores/acervus-exhaustus/nuntii`. No `emissio`: the program prints nothing.
- `NOTA.md`: D-20 is fixed; the program used to run until killed, with no output and no diagnostic;
  a stored number has at most XXXVIII decimal digits, grain counts for `mnr` and `nr`; 2^64 is stored
  and 2^128 is refused at the seventh squaring; the check is in `int_novus`, by symbol, and carries
  no line because the operations that reach it take none.

| count | before | after |
|---|---|---|
| `conformance/` fixtures | 122 | 123 |
| `errores/` | 83 | 84 |
| `nuntii__nuntii.snap` header | `91 fixtures · 104 diagnostics` | `92 fixtures · 105 diagnostics` |
| catalogue header | §8 | §8 |
| conformance trials | — | `errores::numerus-nimis-magnus` |

The one new `nuntii` row, between `errores::numerus-fractus-in-ni` and
`errores::numerus-non-canonicus`, is the **only** difference in `nuntii__nuntii.snap`:

```
errores::numerus-nimis-magnus
  erratum  -  numerus nimis magnus est
```

**Tests that change.**

- `crates/abi/tests/abi.rs`:
  - `sententiae`'s expected list gains `"numerus nimis magnus est"`, last, after
    `"modulus per nullum"`.
  - **New** `a_number_of_xxxviii_digits_is_stored`: `liss_int_new` with thirty-eight 9s, positive and
    negative, and `nullus` minus that number, each consume back to thirty-eight 9s with the right
    sign; the live count returns to zero.
  - **New** `a_number_of_xxxix_digits_raises`: two new `scaena` arms — `numerus-xxxix`,
    `liss_int_new` with the blob of 10³⁸; `summa-xxxix`, `liss_int_add` of thirty-eight 9s and one —
    each give `(Some(1), "erratum: numerus nimis magnus est\n")`.
  - `arithmetic_is_unbounded` is renamed `arithmetic_passes_a_machine_word`. Its body, twenty-two
    digits, is unchanged and still passes; the old name now lies.
  - `scaena`'s `index-immanis` arm, behind `an_index_outside_the_bounds_raises_with_roman_numerals`,
    built its subscript "far past `usize`" from **forty** 9s through `liss_int_new`. The cap refuses
    that before the index is read, and the test failed with `numerus nimis magnus est`. It is
    thirty-eight 9s now, still eighteen digits past any `usize`, and asserts what it did. This list
    missed it when the spec was written; the implementation found it.
- `crates/lingua-serpentis/tests/robustness.rs` — `pathologies()` gains `quadratum sine fine`, §1's
  program on one line, under `MORA_ARTA` with `None`, with a bare `// D-20`. It times out today and
  must satisfy the invariant after §5.
- `crates/lingua-serpentis/tests/support/diag.rs` — §7.
- `crates/runtime` — nothing. `Roman` stays unbounded (§2).

The test total rises by three: the two `abi.rs` tests and the conformance trial.

`currunt/arithmetica-illimitata` keeps its name, program and expectations; its `NOTA.md`, which says
the point is that `Roman` is arbitrary-precision, gains one sentence saying twenty-two digits is
inside the cap. Renaming the fixture would rename a trial and shift the fuzz stream for no
behavioural change.

The restated counts — `CLAUDE.md`'s catalogue header and corpus counts, `README.md:80`,
`docs/README.md`'s fixture count, `docs/internals/testing.md`, `crates/lingua-serpentis/tests/nuntii.rs:3`
— are updated where milestone 2 closes, not in the fix commit, as D-26's and D-13's were.

## 10. Blessing

One `nuntii` file is *new*; none is rewritten.

1. **Before the code changes,** confirm the fixture hangs with a deadline of your own:
   `timeout 10 target/debug/liss run conformance/errores/numerus-nimis-magnus/programma; echo $?`
   → `124`. **Do not run the conformance trial yet**: the fixture runner's backstop is 120 s
   (`support/mod.rs`'s `MORA`), which is exactly why the register called D-20 "not fixture-shaped".
2. **After the code changes,** the same command prints §9's line to stderr, nothing to stdout, and
   exits 1 in well under a second.
3. **Bless with a filter**: `LISS_BLESS=1 cargo test --test conformance -- numerus-nimis-magnus`. No
   other trial's name contains the substring.
4. **`git status --porcelain conformance/`** lists only the new directory's files.
5. **Read the `nuntii`**: §9's one line, byte for byte.
6. **`cargo nextest run --test nuntii`** fails with exactly one added row and no removed or changed
   row; accept with `INSTA_UPDATE=always`, re-run green.

## 11. Couplings

- **Every `LissInt` built from a value goes through `int_novus`.** A new entry point that allocates
  and fills a `LissInt` by hand bypasses the cap, and D-20 returns for that operation.
- **A future constant folder must respect the cap.** §6's argument covers literals and the one
  negation fold. A pass that folds `°M.M * °M.M` at compile time would produce digit blobs that
  `liss_int_new` refuses at run time, with no line — it must fold only results within XXXVIII digits
  and leave the rest to run time, or emit a located compile-time diagnostic of its own.
- **The cap is a number in Rust source only.** `38` appears in `numeri.rs` and `abi.rs`; the
  sentence, the docs a user reads and every diagnostic say XXXVIII or nothing.
- **The new fixture shifts the fuzz seed stream** (`support/mutate.rs`'s `seed_corpus`). Say so in the
  fix commit; §7's decision is taken after it.
- **Documentation that says "unbounded" about the image changes with the fix:**
  `internals/abi.md` §9 (`int_novus`) and §13's catalogue table, `internals/codegen.md`'s
  `Corpus::numerus` boxing sentence, `internals/rationale.md`'s "Deferred elsewhere" (D-20 is no
  longer deferred, and the section had nothing else), `internals/testing.md`'s `RECORDED_RULES`
  paragraph (per §7's outcome).
- **The integers spec now carries two 2026-09-15 notices**, D-13's and this one, stacked.
- **D-20's register entry, status row, deferred table and header prose close with this change**;
  the deferred table is then empty. `docs/via.md`'s D-20 row is updated then; the milestone 2
  heading and "Current position" are milestone 2's close, not this change's.

## 12. Success criteria

1. `target/debug/liss run conformance/errores/numerus-nimis-magnus/programma` writes
   `erratum: numerus nimis magnus est` and a newline to stderr, nothing to stdout, and exits 1 in
   under a second.
2. `a_number_of_xxxviii_digits_is_stored` and `a_number_of_xxxix_digits_raises` pass; the latter
   fails before §5 (the scenario returns and hits `unreachable!`, exit 101).
3. `every_latin_sentence_this_crate_can_say_is_listed_here` passes with nine sentences.
4. `every_curated_pathology_matches_its_recorded_verdict` passes with `quadratum sine fine` → `None`.
5. `crates/lingua-serpentis/tests/snapshots/catalogus__catalogus.snap` opens with
   `121 sites · 73 fixtured · 0 ambiguous · 46 uncovered · 2 unparsed`, and its sorted
   `(pattern, status)` set is §8's before-set plus one pair.
6. `crates/lingua-serpentis/tests/snapshots/nuntii__nuntii.snap` opens with
   `92 fixtures · 105 diagnostics`, and its diff against `738918f` is §9's one added row.
7. `ls conformance/errores | wc -l` prints `84`; `git diff --stat 738918f -- conformance/` touches
   only `errores/numerus-nimis-magnus/` and `currunt/arithmetica-illimitata/NOTA.md`.
8. `RECORDED_RULES` is `[&str; 0]`, or it holds `"terminates"` attributed to D-31 and D-31 exists in
   the register (§7); `grep -n 'D-20' crates/lingua-serpentis/tests/support/diag.rs` prints nothing.
9. `cargo nextest run` green; the total is Task 0's baseline plus three.
10. `cargo tree -e normal` shows no new package; `codegen/tests/pondus.rs` passes.
11. `grep -rn 'numerus nimis magnus est' crates/*/src` prints exactly one line, in `defectus.rs`.
12. `docs/defectus/2026-08-03-index.md` shows D-20 **fixed** and no deferred defect; `docs/README.md`
    reads `29 fixed, 0 deferred, 1 open` (or the D-31 count, if §7 filed it).
