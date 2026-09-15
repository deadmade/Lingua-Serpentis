# Lingua Serpentis — Roman Numeral Form Design

- **Date:** 2026-09-15
- **Status:** Proposed
- **Author:** Manuel (with Claude)
- **Relationship:** Amends `2026-07-19-lingua-serpentis-rust-interpreter-design.md` §5.3 — and
  **supersedes** two of its clauses outright: "ported from the existing `RomanNumeralConverter.py`,
  whose exact semantics must be preserved" (:154-156) and the porting note's "Preserve existing
  behaviour in v1; broaden subtractive support only if a test demands it (YAGNI)" (:172-176). Amends
  `2026-07-23-roman-native-integers-design.md` §3.5 ("Input notation is untouched", :223-225), §4.2
  ("parsing keeps its shape", :262-277) and §7, whose round-trip bullet (:414-416) names the
  multiplicative `°VII.M` and is false for it. Closes deferred defect D-13
  (`defectus/2026-08-03-index.md`). Cites, and does not amend, `2026-07-23-roman-native-fractions-design.md`
  ("literal syntax unchanged", :66, :232 — the `°W:N/D` shape is unchanged; its parts now obey §3),
  the integers spec's §3.1 canonical-form invariant (:113-125, a rule about the stored value, not
  about source text), the compiler-backend spec's §2 Non-Goal "No resolution of D-13 or D-20" (:86,
  that milestone's scope and not a language rule), and the conformance design's §2 Non-Goals, which
  listed form validation as out of that suite's scope. Adds **no** dependency: §11's table is
  untouched.

## 1. Vision

`convert_single` (`crates/runtime/src/roman.rs:178-197`) reads a string over `I V X L C D M` by one
positional rule — subtract a letter if the next one is bigger, otherwise add it — and accepts every
such string. Nothing limits repetition, nothing restricts which subtractive pairs exist, and the
result is never compared with the numeral that would print it. With the binary built from `6634fcd`:

```liss
munus initium reddit ni {
  scribe(°IIX + °VV + °IM);
  redde nullus;
}
```

```
$ liss run conformance/defectus/D-13-numerus-non-canonicus/programma
MXIX
$ echo $?
0
```

10 + 10 + 999. None of the three is a Roman numeral. Measured one literal at a time, the same way:

| literal | prints | read as |
|---|---|---|
| `°IIX` | `X` | 10 |
| `°VV` | `X` | 10 |
| `°IM` | `CMXCIX` | 999 |
| `°IIII` | `IV` | 4 |
| `°XXXX` | `XL` | 40 |
| `°MMMM` | `MV̅` | 4000 |
| `°VX` | `V` | 5 |
| `°IC` | `XCIX` | 99 |
| `°LL` | `C` | 100 |
| `°XM` | `CMXC` | 990 |
| `°IXI` | `X` | 10 |
| `°MCMXCIXIII` | `MMII` | 2002 |
| `°V.IIX` | `L` | 5 × 10 |
| `°IIX.M` | `X̅` | 10 × 1000 |
| `°I:I/IIX` | `I et uncia et sextula` | 1 + 1/10 |
| `°I:IIX/XII` | `I et dextans` | 1 + 10/12 |

The register filed this as deferred so that a spec would choose a form (D-13, and `via.md`
milestone 2). This document chooses: **every chunk of a literal is written exactly the way the
language prints its value, and nothing else lexes.**

```
erratum: numerus Romanus non canonicus: 'IIX'
  in 'principale.liss', linea II
```

What it buys beyond the diagnostic: a literal has one spelling per chunk, the zero-denominator
argument D-25 rests on gets simpler rather than weaker (§6), and a literal can no longer grow a
value without growing the source proportionally, which is what lets D-20's cap be a run-time rule
alone (§10).

## 2. Goals and Non-Goals

### Goals

- **One form rule per chunk** — a chunk is canonical when it equals `Roman::display` of its own
  value, which confines it to 1..=3999 (§3).
- **One new sentence**, inline at its call site, unique in the workspace, every word passing
  `latinitas::is_latin`, no Arabic digits (§4).
- **The rule reaches every chunk**: each `.` chunk of the whole part, and the numerator and
  denominator of a fraction, with no second check anywhere (§3, §5).
- **D-25's zero-denominator argument is restated, not lost**: the enumeration that backs it changes
  from "every string is accepted and positive" to "accepted exactly when canonical, and every
  accepted value is at least 1" (§6).
- **The D-13 fixture moves to `errores/`** and becomes the rule's fixture (§8).
- **The huge-literal stress survives** as a long canonical dot chain (§8).

### Non-Goals

- **Not a canonical dot form.** `°IV.M`, `°M.IV` and `°II.MM` all stay legal and equal (4000). A
  rule for pairs (`a.M`, `a` < 1000, tiers descending) would rewrite `examples/calc.liss:7`
  (`°XXX.M.V.C.VII`, the `exemplum-calc` fixture and the spec's own example), `cli.rs:118,266`,
  `programs.rs:58` and `roman.rs`'s own tests, and would make 10⁶ and above unwritable unless `M.M`
  were allowed again. The dot form is multiplication; it is not a spelling. A round-tripping dot
  *output* stays where the integers spec §9 (:498-500) put it, a v2 display option.
- **Not a fraction rule beyond chunks.** No lowest terms (`codegen/tests/series.rs:78` writes
  `°I:VI/XII`), no numerator below denominator, no denominator dividing the grain
  (`tests/ir.rs:308-311,605,637` write `°I:I/VII` and `°II:III/V` to test truncation). A fraction's
  whole part still cannot be zero; that is the fractions spec's own limit (:83-89).
- **Not vinculum input.** Display emits U+0305 from 4000 up and the lexer rejects it
  (`signum ignotum`, `lexer.rs:114-118`). Accepting it touches the byte/char span rules of
  `2026-08-09-caret-diagnostics-design.md` and the gutter width; it is not needed to write any value,
  because the dot form writes all of them.
- **Not a caret on the literal.** Lexer errors carry `spatium: None` (`lexer.rs:62`), so this
  diagnostic, like `numerus Romanus invalidus` and `vacuus`, has a location line and no source echo.
  Spans for all lexer sites are one change to three existing fixtures' shapes, and it is not this
  one.
- **Not every bad literal in a file.** `tokenize` returns at the first `Err` (`lexer.rs:62`'s `?`),
  so `°IIX + °VV + °IM` reports `'IIX'` and stops. The register's "Expected: … for each" (index
  :450) cannot hold for any lexer error today, and is corrected when D-13 closes (§10).
- **Not a warning.** A `monitum` would need a lexer warning channel that does not exist
  (`tokenize` returns `LResult<Vec<Token>>`; the token keeps the value, not the lexeme), and would
  freeze the hole into 1.0 as a warning.
- **Not a new sentence for `vacuus` or `invalidus`.** Both keep their text, their sites and their
  fixtures (§5).

## 3. The rule

A literal is `°` then a body; `parse_roman` (`roman.rs:219-246`) splits the body at the first `:`
into a whole part and an optional fraction, splits the fraction at the first `/` into numerator and
denominator, and `convert_int_roman` (`:199-211`) splits each of those three at every `.` into
chunks. Every chunk reaches `convert_single`. The rule is one clause of `convert_single`:

> **A chunk `s` is accepted exactly when `s == Roman::display(v)`, where `v` is the value the
> positional rule computes for `s`.**

That one equality is the whole classical rule, because `display` is the whole classical renderer:

- `v` must be in **1..=3999**. `display(0)` is `nullus` and a negative value prints a leading `-`,
  neither of which is a string over `I V X L C D M`; from 4000 up `display` emits U+0305, which no
  chunk contains.
- `I`, `X`, `C` and `M` appear at most three times in a row; `V`, `L` and `D` at most once.
- The only subtractive pairs are `IV IX XL XC CD CM`.
- Letters descend by tier: `MDCLXVI`-order, with each tier's digit written as `render_digit`
  (`roman.rs:269-278`) writes it.

The check runs **after** the existing loop, so a chunk holding a letter outside `I V X L C D M`
still fails with `numerus Romanus invalidus` first, and an empty chunk still fails with
`numerus Romanus vacuus` before either. `convert_int_roman`'s `collect` stops at the first failing
chunk, and `parse_roman` evaluates whole part, numerator and denominator in that order, so **the
chunk reported is the first non-canonical one from the left**.

The value, the AST, the token and every stage after the lexer see exactly what they see today for
every literal that is still accepted. Nothing downstream changes.

`nullus` is a keyword that lexes to zero through `word_to_kind` and never reaches `convert_single`;
it is unaffected.

## 4. The sentence

```
numerus Romanus non canonicus: '{}'
```

*Roman numeral not canonical: '…'*, the hole quoting the offending **chunk**, not the literal.

| word | `latinitas::is_latin` |
|---|---|
| `numerus` | true |
| `Romanus` | true |
| `non` | true |
| `canonicus` | true |

Measured on 2026-09-15 against `crates/latinitas` at `6634fcd`.

- **A hole, and why.** `°V.IIX` is canonical in its first chunk; naming `'IIX'` says which part to
  fix. The quoted text is a run of Roman letters, never an Arabic digit, and the robustness suite's
  no-Arabic-digits rule strips quoted spans anyway.
- **Unique.** No other literal in `crates/*/src` contains `canonicus`. It must **not** reuse
  `numerus Romanus invalidus: '{}'` (`roman.rs:192`): two sites sharing a pattern both read
  `ambiguous` (`catalogus.rs`), and the header would fall to `69 fixtured · 2 ambiguous`.
- **Inline.** Written as the literal argument of `format!` at the call site, per spec §10.3; a
  forwarded constant would read `UNPARSED`.
- **`non canonicus`, not `invalidus`.** `invalidus` already means "a character that is not a Roman
  letter"; this is a string of Roman letters in the wrong form, and the two failures should read
  differently.

## 5. What is rejected, and what is not

### Rejected — by the new sentence

| # | literal | chunk quoted | value today | why |
|---|---|---|---|---|
| 1 | `°IIII` | `IIII` | 4 | `I` four times; `display(4)` is `IV` |
| 2 | `°XXXX` | `XXXX` | 40 | `X` four times |
| 3 | `°MMMM` | `MMMM` | 4000 | out of 1..=3999; `display(4000)` is `MV̅` |
| 4 | `°VV` | `VV` | 10 | `V` twice |
| 5 | `°LL`, `°DD` | `LL`, `DD` | 100, 1000 | `L`, `D` twice |
| 6 | `°IIX` | `IIX` | 10 | `II` before `X` is no pair |
| 7 | `°IM`, `°IC`, `°IL`, `°XM` | the literal | 999, 99, 49, 990 | `IM IC IL XM` are not among the six pairs |
| 8 | `°VX` | `VX` | 5 | `V` before `X` is no pair |
| 9 | `°IIV`, `°IXI`, `°VIV` | the literal | 5, 10, 9 | letters out of tier order |
| 10 | `°MCMXCIXIII` | `MCMXCIXIII` | 2002 | letters out of tier order |
| 11 | `°V.IIX` | `IIX` | 50 | second `.` chunk |
| 12 | `°IIX.M` | `IIX` | 10 000 | first `.` chunk |
| 13 | `°IIX.VV` | `IIX` | 20 | the first bad chunk from the left |
| 14 | `°IIII:I/II` | `IIII` | 4½ | whole part |
| 15 | `°I:IIX/XII` | `IIX` | 1 + 10/12 | numerator |
| 16 | `°I:I/IIX` | `IIX` | 1 + 1/10 | denominator |
| 17 | `°` then 5 000 `M` | the 5 000 `M`s | 5 000 000 | out of 1..=3999 (`robustness.rs`'s `numerus longissimus`) |

### Rejected — by an existing sentence, unchanged

| literal | message | why |
|---|---|---|
| `°` | `numerus Romanus vacuus` | empty whole part; `errores/numerus-romanus-vacuus` |
| `°I..I` | `numerus Romanus vacuus` | empty `.` chunk |
| `°I:I/II/III` | `numerus Romanus invalidus: 'II/III'` | `/` inside the denominator is not a letter, found by the loop before the form check; `errores/numerus-romanus-invalidus` |
| `°I:I` | `fractio sine denominatore` | `errores/fractio-sine-denominatore` |

### Accepted — unchanged

| literal | value | note |
|---|---|---|
| `°I`, `°IV`, `°IX`, `°XL`, `°XC`, `°CD`, `°CM` | 1 … 900 | the six pairs, and `I` |
| `°MMMCMXCIX` | 3999 | the largest chunk; `currunt/arithmetica-illimitata` writes it |
| `°XXX.M.V.C.VII` | 30 507 | `examples/calc.liss:7`; every chunk canonical |
| `°IV.M`, `°M.IV`, `°II.MM` | 4000 | three spellings of one value; §2 Non-Goals |
| `°V.M` | 5000 | `currunt/recursio-profunda` |
| `°C.C.C` | 10 100 | `cli.rs:118` |
| `°M.M.M.M.M` | 2 001 000 | `cli.rs:266` |
| `°V:I/II` | 5½ | |
| `°I:VI/XII` | 1½ | not in lowest terms; `codegen/tests/series.rs:78` |
| `°I:XIII/XII` | 2 + 1/12 | improper |
| `°I:I/VII`, `°II:III/V` | | off the twelfths grid; `tests/ir.rs` |
| `nullus` | 0 | a keyword; never reaches `convert_single` |

**No program in the repository changes.** Every `°[IVXLCDM.:/]+` in `git ls-files` at `6634fcd` was
split at `.`, `:` and `/` and each chunk compared with the classical form of its value: the only
non-canonical chunks are `IIX`, `VV` and `IM`, in the D-13 fixture and in documents that quote it.
The only non-canonical literal built by test code is `robustness.rs`'s `"M".repeat(5_000)` (§8).
`generative.rs`'s `roman()` emits the standard table over `0..200`, always canonical.

## 6. D-25's argument, restated

D-25 (`a148f3f`) deleted `denominator est nullus` because `convert_int_roman` cannot return zero:
every value it builds is a sum of products of `convert_single` values, and `convert_single` is at
least 1 for every string it accepts. `roman.rs:241` carries the bare pointer; `abi.md:502-514` and
`rationale.md:1070-1076` carry the argument. It rested on
`convert_single_is_never_zero_or_negative` (`roman.rs:514-539`), which calls
`.expect("every string over the symbols parses")` on all **137,256** strings of length 1..=6 over the
seven letters. Under §3 that `expect` panics on `IIII`, so the test must change, and the argument
with it.

**The argument after this change is shorter.** An accepted chunk equals `display(v)`, which is a
string of Roman letters only when `v` ≥ 1 (§3). A product of values ≥ 1 is ≥ 1, and a non-empty sum
of them is ≥ 1, so `convert_int_roman` still cannot return zero, and `den` is never zero. The
checker's `expect("a literal's denominator is never zero")` (`checker.rs:254`) holds for the same
reason: this rule only narrows what is accepted.

**The enumeration stays as the proof.** It is replaced by one that walks the same 137,256 strings
and asserts, for each:

1. `convert_single(w).is_ok()` **exactly when** `w` is `Roman::from_u64(v).display()` for some `v`
   in 1..=3999;
2. every accepted `w` has a value ≥ 1 and displays as `w`;
3. **1,308** of the 137,256 are accepted — the numbers in 1..=3999 whose classical form is at most
   six letters, counted independently on 2026-09-15 by a regular expression
   (`^M{0,3}(CM|CD|D?C{0,3})(XC|XL|L?X{0,3})(IX|IV|V?I{0,3})$`) over the same strings.

It keeps D-25's count, so the prose that quotes "137,256" stays true, and it gains the count that
says how many of them are numerals.

## 7. What this does to the catalogue

Measured from `crates/lingua-serpentis/tests/snapshots/catalogus__catalogus.snap` at `6634fcd`:

```
before:  119 sites · 71 fixtured · 0 ambiguous · 46 uncovered · 2 unparsed
after:   120 sites · 72 fixtured · 0 ambiguous · 46 uncovered · 2 unparsed
```

Every row difference, with its cause, for the check written as three lines replacing `Ok(total)`
after the loop's closing brace at `roman.rs:195`:

```rust
    Some(total)
        .filter(|t| t.display() == s)
        .ok_or_else(|| format!("numerus Romanus non canonicus: '{}'", s))
```

**The shape is load-bearing.** The catalogue's extractor counts `.ok_or_else(|| format!(…))` as a
site, but counts `Err(…)` only when its argument *opens* with a string literal
(`catalogus.rs`'s `incipit_literale`), so `if total.display() != s { return Err(format!(…)); }` —
the form this section first named — is no site at all, and the header would not move. That was
found by running it; the corrected form is what shipped.

| row | change | cause |
|---|---|---|
| `crates/runtime/src/roman.rs:198  numerus Romanus non canonicus: '{}'  → fixtured` | **added** | §4; fixtured by `errores/numerus-non-canonicus` (§8) |
| `crates/runtime/src/roman.rs:226  numerus sine '°': '{}'  → UNCOVERED` | `:line` only → `:228` | the two lines added above it |
| `crates/runtime/src/roman.rs:238  fractio sine denominatore  → fixtured` | `:line` only → `:240` | the same |

`roman.rs:180` (`vacuus`) and `:192` (`invalidus`) do not move. No row in any other file moves.
`ambiguous`, `uncovered` and `unparsed` do not move. If the check is written in a different number
of lines, the two `:line` values move by that number instead; the check that does not depend on it:
**the sorted set of `(pattern, status)` pairs after equals the set before plus exactly one pair**,
`(numerus Romanus non canonicus: '{}', fixtured)`.

## 8. Fixtures, and what the corpus counts become

**The D-13 fixture moves; no fixture is added.**

`git mv conformance/defectus/D-13-numerus-non-canonicus conformance/errores/numerus-non-canonicus`,
the `D-13-` prefix dropped as every earlier close dropped it. Then:

- `programma/principale.liss` is **unchanged**. Its bad literal is on line 2, so the error does not
  land on line 1, which an `errores/` program must not do.
- `emissio` (`MXIX`) is **deleted**: an empty stream has no file.
- `exitus` = `1`.
- `nuntii` is blessed (§9), and is exactly:
  ```
  erratum: numerus Romanus non canonicus: 'IIX'
    in 'principale.liss', linea II
  ```
  Two lines, no echo and no caret, the shape of `errores/numerus-romanus-invalidus/nuntii`.
- `NOTA.md` is rewritten: D-13 is fixed; the program used to print `MXIX` and exit 0; a chunk must be
  written the way its value prints; only the first bad literal is reported, so `°VV` and `°IM` are
  not; the rejection lives in `convert_single`.

`conformance/defectus/` is then empty and git drops it. `support/mod.rs:70` keeps `"defectus"` in
`CATEGORIES`, and `discover()` skips a category directory that does not exist
(`support/mod.rs:135-137`), so nothing needs editing there.

| count | before | after |
|---|---|---|
| `conformance/` fixtures | 122 | 122 |
| `errores/` | 82 | 83 |
| `defectus/` | 1 | 0 |
| `nuntii__nuntii.snap` header | `90 fixtures · 103 diagnostics` | `91 fixtures · 104 diagnostics` |
| catalogue header | §7 | §7 |
| conformance trials | `defectus::D-13-numerus-non-canonicus` | `errores::numerus-non-canonicus` |

The one new `nuntii` row, between `errores::numerus-fractus-in-ni` and
`errores::numerus-romanus-invalidus`, is the **only** difference in `nuntii__nuntii.snap`:

```
errores::numerus-non-canonicus
  erratum  principale.liss:2  numerus Romanus non canonicus: 'IIX'
```

**Tests that change.**

- `crates/runtime/src/roman.rs` — `convert_single_is_never_zero_or_negative` is replaced by the §6
  enumeration, and one new unit test pins the §5 "Rejected" table's sentence and chunk for rows 1-16
  and the "Accepted" table's non-fraction integer rows.
- `crates/lingua-serpentis/tests/robustness.rs` — `numerus longissimus` (`°` + 5 000 `M`, `:228-235`)
  keeps its source and its `None`; it is now a rejection pathology, with a bare `// D-13`. **A new
  pathology, `catena numeri longissima`**, writes `°` then `MMMCMXCIX` joined by `.` 500 times — a
  4 999-byte literal of canonical chunks, value 250 × 3999² — with `MORA_ARTA` and `None`, so a huge
  literal that is *accepted* still goes through the lexer, checker and compiler under the deadline.
- `crates/lingua-serpentis/tests/support/diag.rs` `numerus` (`:42-48`) reads diagnostic line numbers
  with `parse_roman`. A location numeral that is not canonical now reads `None` and fails
  `numeral round-trips` as "is not a numeral" rather than "is not how N is written". Both are
  violations; the rule is already canonical-only, and nothing it passes today changes.

The restated counts — `CLAUDE.md` (the conformance section's counts and "`defectus/` (one deferred
defect)"), `README.md:80`, `docs/README.md:220`, `docs/internals/testing.md:55-57`,
`crates/lingua-serpentis/tests/nuntii.rs:3` — are updated where milestone 2 closes, not in the fix
commit, as D-26's were.

## 9. Blessing

One `nuntii` file is *new*; none is rewritten. The narrow case, kept narrow mechanically:

1. **Before the code changes, after the move,** `cargo nextest run --no-fail-fast --test conformance`
   fails on exactly one trial, `errores::numerus-non-canonicus`: stdout `MXIX` against none, exit 0
   against 1. `defectus::D-13-numerus-non-canonicus` no longer exists.
2. **Bless with a filter**: `LISS_BLESS=1 cargo test --test conformance -- numerus-non-canonicus`.
   `libtest-mimic` matches the substring against `errores::<casus>`, and no other trial contains it.
3. **`git status --porcelain conformance/`** lists the rename, the deleted `emissio` and the new
   `exitus`, `nuntii` and `NOTA.md`, under the one directory, and nothing else.
4. **Read the `nuntii`**: §8's two lines, byte for byte.
5. **`cargo nextest run --test nuntii`** fails with exactly one added row and no removed or changed
   row; accept with `INSTA_UPDATE=always`, re-run green.

## 10. Couplings

- **A Roman literal written anywhere in the repository — a `.liss` example, a test string in any
  crate, a documentation sample that a test reads — must be canonical per chunk.** `examples/` are
  reached from `conformance/` through `VIA` files and several crates compile programs from strings,
  so a stray `°IIII` now fails a test far from `roman.rs`. §5's scan says none exists today.
- **The fixture's move shifts the fuzz seed stream.** `support/mutate.rs:67-74` sorts the corpus by
  path, so renaming `conformance/defectus/D-13-…/programma/principale.liss` to
  `conformance/errores/numerus-non-canonicus/…` changes which mutations run, even though no `.liss`
  byte changes. Say so in the fix commit.
- **D-20's cap relies on this.** After §3 a chunk is at most 3999, a `.` pair at most 3999² =
  15 992 001, and pairs only add, so a literal's value grows at most linearly in its length:
  exceeding 10³⁸ would take on the order of 10³¹ bytes of source. An over-cap literal is therefore
  unwritable, and D-20's spec can put its cap in the run-time image alone, stating this argument the
  way D-25 stated its own. Before this change `°M…M.M…M` multiplied two unbounded chunks, and a
  literal a few megabytes long exceeded any fixed cap.
- **The integers spec gains two amendment notices.** D-20's spec amends
  `2026-07-23-roman-native-integers-design.md` too (§1, §3.6, §5); its notice stacks beside this one.
- **D-13's register entry, status row, deferred-table row and header prose close with this change**;
  the entry's "Expected: `numerus Romanus invalidus` for each" is recorded as having shipped as a new
  sentence for the first bad chunk. `docs/via.md`'s D-13 row and its "Current position" line are
  updated then; the milestone 2 heading waits for D-20.

## 11. Success criteria

1. Every "Rejected — by the new sentence" row of §5 fails with `numerus Romanus non canonicus: '<chunk>'`
   and the chunk named there; every "Accepted" row parses to the value named there; the three
   "existing sentence" rows keep their message — the `roman.rs` unit test enumerates them.
2. The `roman.rs` enumeration walks 137,256 strings, accepts exactly 1,308, and asserts accepted ⇔
   canonical and every accepted value ≥ 1; `grep -n 'every string over the symbols parses'
   crates/runtime/src/roman.rs` prints nothing.
3. `crates/lingua-serpentis/tests/snapshots/catalogus__catalogus.snap` opens with
   `120 sites · 72 fixtured · 0 ambiguous · 46 uncovered · 2 unparsed`, and its sorted
   `(pattern, status)` set is §7's before-set plus one pair.
4. `crates/lingua-serpentis/tests/snapshots/nuntii__nuntii.snap` opens with
   `91 fixtures · 104 diagnostics`, and its diff against `6634fcd` is §8's one added row.
5. `ls conformance/errores | wc -l` prints `83`; `ls conformance/defectus` fails; `git diff --stat
   -M 6634fcd -- conformance/` touches only the moved directory.
6. `target/debug/liss run conformance/errores/numerus-non-canonicus/programma` prints §8's `nuntii` to
   stderr, nothing to stdout, and exits 1.
7. `robustness.rs`'s `every_curated_pathology_matches_its_recorded_verdict` passes with
   `numerus longissimus` and `catena numeri longissima` both `None`.
8. `cargo nextest run` green; the test count is Task 0's baseline plus one (the new `roman.rs` unit
   test; the enumeration and the conformance trial are replaced one for one).
9. `cargo tree -e normal` shows no new package; §11's table is unchanged.
10. `docs/defectus/2026-08-03-index.md` shows D-13 **fixed** in its status table and no longer lists it
    as deferred; `docs/README.md` reads `28 fixed, 1 deferred, 1 open`.
