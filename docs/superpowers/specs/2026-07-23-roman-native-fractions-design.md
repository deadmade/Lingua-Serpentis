# Lingua Serpentis — Roman-Native Fractions (`mnr` / `nr`) Design

- **Date:** 2026-07-23
- **Status:** Design approved in brainstorming; this written spec pending user review
- **Author:** Manuel (with Claude)
- **Amends:** `2026-07-22-nr-double-distinct-type-design.md` — the fractional types
  `mnr`/`nr` were backed by IEEE `f32`/`f64` and printed as decimals. Sequel to
  `2026-07-23-roman-native-integers-design.md`, which made `ni` Roman-native and
  explicitly deferred fractions ("a whole second engine", §9 there). This design
  builds that second engine. The integer engine (`roman.rs`, the `Roman` type) is
  **reused, not changed**.

## 1. Vision

The integer half of the language now tells the truth: you write Roman, compute in
Roman, get Roman back. The fractional half still doesn't. `mnr`/`nr` are IEEE
floats, so `°I:I/III` — one and a third, written in Roman fraction syntax — comes
out `1.3333334` (or `1.3333333333333333`). Roman in, binary float forever after,
**Arabic decimal out.** The same one-way street the integers just left.

This design makes the fraction the truth too, on the system the Romans actually
used: the **duodecimal `as`**, divided into twelve `unciae`, each `uncia` divided
down to the `scripulum` (1/288). A fractional value stops being a float wearing
Roman input syntax and becomes a **signed integer count of a fixed Roman grain
unit** — computed on directly (by the same carry/borrow numeral engine that backs
`ni`), and printed back in classical Latin fraction names.

```
scribe(°I:I/III);          → I et triens        (was: 1.3333333333333333)
scribe(°I:I/II + °I:I/II); → III                (½ + ½ carries to a whole; was 3)
mnr m = °I:I/XXIV;    scribe(m);  → I             (1/24 is below mnr's uncia grain — dropped)
nr   n = °I:I/XXIV;   scribe(n);  → I et semuncia (nr keeps it: 1/24 = semuncia)
```

After this change there is no decimal anywhere a user can see. The whole language
prints Roman: integers as numerals, fractions as `uncia`/`semis`/`triens`/…

The insight that makes this tractable — the "whole second engine" is **thin**. A
fractional value is just a `Roman` (the grain count) plus a fixed scale factor.
Add/subtract/negate/compare/equal are the *existing* `Roman` operations verbatim;
multiply and divide are the existing operations plus one rescaling `div_rem`
(which is where truncation lives); and the only genuinely new code is a duodecimal
**display decomposer** that names the grid points. The type system does not move.

## 2. Goals and Non-Goals

### Goals

- Replace the runtime backing of `mnr`/`nr` with a **Roman-native fixed-point
  value** (§3.1): a signed `Roman` count of a grain unit. No IEEE float ever
  appears on the value path again.
- Preserve `mnr` vs `nr` as a **real, enforced distinction** — expressed as
  *grid resolution* in the Roman idiom rather than IEEE mantissa width (§3.1):
  - `mnr` → grain = **uncia** (1/12);
  - `nr` → grain = **scripulum** (1/288), the finest classical subdivision.
- Compute directly on the grain count, reusing the `Roman` engine (§3.3):
  add/sub/neg/compare/equal are the existing operations unchanged; multiply and
  divide rescale then truncate via the existing truncating `div_rem`.
- **Truncate toward zero** on any off-grid result (§3.3), the same rule the
  integer engine already uses for division — one truncation convention across the
  whole numeric stack, no rounding logic anywhere.
- Print fractions as **classical Latin fraction names** (§3.4): the integer part
  as a Roman numeral, the fractional part as `uncia`/`sextans`/…/`deunx` plus the
  sub-uncia stack `semuncia`/`sicilicus`/`sextula`/`dimidia sextula`/`scripulum`,
  joined with `et`.
- Keep the literal syntax **unchanged** (`°W:N/D`) and adapt the existing
  elaborating-checker machinery (§3.5): the literal carries an exact rational and
  is narrowed to a grain count at the demand site, exactly as `FloatLit` is
  narrowed to `f32`/`f64` today.
- Keep the type checker essentially untouched (§4.5); `mnr`/`nr` stay
  `Type::Float`/`Type::Double`, mixing stays a static error.

### Non-Goals (v1)

- **Casts / conversions between `mnr` and `nr`.** Mixing the two stays the static
  type error it is today (no implicit widening) — carried over from the
  `nr`-distinct-type design's deferral list. There is no `mnr`→`nr` promotion even
  though it would be lossless, and no `nr`→`mnr` truncation operator.
- **Whole-number fraction literals.** `mnr x = °V;` still does not type-check —
  `°V` is an `ni` literal (`RomanNumber::Int`), and there is still no way to write
  a fractional literal without the `:N/D` fraction part. Unchanged from before;
  still deferred.
- **Zero-integer-part fraction literals.** The lexer's `is_roman_body` admits only
  `I V X L C D M . : /`, so a `°…` literal's integer part can never be empty or the
  `nullus` keyword — a bare sub-1 fraction like `1/24` is **not writable** as a
  literal. Write `1 + 1/24` as `°I:I/XXIV` instead. (Sub-1 *values* still arise at
  runtime from arithmetic — e.g. `°I:I/XXIV - °I:I/XLVIII` → `sicilicus` — and the
  display handles a zero whole part by omitting it, so this is purely a literal-
  grammar limitation.) Lifting it (a `°:N/D` or `°nullus:N/D` form) is deferred.
- **A rounding mode.** Off-grid results truncate toward zero (§3.3); there is no
  round-to-nearest and therefore no tie-break rule.
- **Arbitrary (non-duodecimal) rationals.** Every value lives on the grain grid by
  construction, so there is never a value that cannot be named. There is no
  general `p/q` runtime type and no ratio fallback in display.
- **A user-facing decimal notation.** As with integers, there is intentionally no
  way to write or print an Arabic decimal.
- Everything already deferred elsewhere (inference, imports, `>=`/`<=`, for-each,
  maps, …).

## 3. Semantics

### 3.1 The fractional value — a grain count

An `mnr`/`nr` value is a **signed `Roman` count of a fixed grain unit**. The grain
is a property of the *type*, so the value stores only the count; the grain is
implied by which `Value` variant holds it:

```rust
// value.rs
Value::Float(Roman)   // mnr: count of unciae   — value = count / 12
Value::Double(Roman)  // nr:  count of scripula  — value = count / 288
```

Let `G` be the grain **denominator**: `G = 12` for `mnr`, `G = 288` for `nr`. The
mathematical value is `count / G`. Because the count is a `Roman`, it is unbounded
(no overflow, §3.6) and carries its own sign — reusing every invariant the integer
engine already guarantees.

**Why these two grains.** The `scripulum` (1/288) is the finest unit in the
classical Roman system, and 288 is the least common multiple of every named
subdivision, so *every* classical fraction is a whole number of scripula:
`uncia` = 24, `semuncia` = 12, `sicilicus` = 6, `sextula` = 4, `dimidia sextula`
= 2, `scripulum` = 1. `nr` at scripulum grain therefore represents the entire
system exactly. `mnr` at uncia grain represents only twelfths — the everyday Roman
fraction — giving a genuine coarse-vs-fine precision ladder that mirrors
`f32`-vs-`f64` in spirit while being exact on its own grid.

**Accepted consequence — `mnr` is deliberately coarse.** At uncia grain, `mnr`
cannot represent 1/8, 1/5, 1/24, or anything off the twelfths grid; such a value
truncates (§3.3), sometimes dropping the sub-uncia part entirely — e.g. `1 + 1/24`
prints `I` as `mnr` (the 1/24 vanishes) but `I et semuncia` as `nr`. This is the
honest analogue of float precision loss and is the intended, approved behaviour,
not a bug. (The grain floor for `mnr` is
a single constant; moving it finer, e.g. to `semuncia`, is a one-line change if
ever wanted — but uncia is the chosen default: most authentic, sharpest contrast.)

### 3.2 Zero, negatives, and exactness

- **Zero** is `Roman::zero()` as the count (an empty numeral); it renders `nullus`
  (§3.4), the same word integers use for zero.
- **Negatives** are the count's own sign flag; a negative fraction renders with a
  single leading `-` in front of the whole rendering (§3.4).
- Comparisons and equality are now **exact and total** — no NaN, no
  `0.1 + 0.2 ≠ 0.3` surprises. Two values are equal iff their counts are equal
  (same grain), which is exactly `Roman` structural equality, so `Value` keeps
  deriving `PartialEq`.

### 3.3 Arithmetic — the `Roman` engine plus one rescale

Every operation on values of grain denominator `G`, counts `c1`, `c2`, reduces to
existing `Roman` operations:

| Op (Latin) | count result | note |
|---|---|---|
| `+` | `c1.add(&c2)` | exact; same grain only |
| `-` | `c1.sub(&c2)` | exact |
| unary `-` | `c.neg()` | exact |
| `*` | `(c1.mul(&c2)).div_rem(G).0` | rescale by `G`, **truncates** |
| `/` | `(c1.mul(&G)).div_rem(&c2).0` | rescale by `G`, **truncates**; `c2` zero → runtime error |
| `<` `>` (`minor_est`/`maior_est`) | `c1.cmp(&c2)` | total order |
| `==` `!=` (`par_est`/`par_non_est`) | `c1 == c2` | structural |

The two rescales are the whole of the "new" arithmetic, and each derivation is a
one-liner:

- **Multiply.** `(c1/G)·(c2/G) = c1·c2 / G²`; expressed back in grain units that is
  `c1·c2 / G`, truncated — `(c1·c2) div G`.
- **Divide.** `(c1/G)/(c2/G) = c1/c2`; in grain units that is `c1·G / c2`,
  truncated — `(c1·G) div c2`. A zero divisor count is a runtime error (§5),
  **replacing** the old IEEE `÷0 → inf` behaviour.

Truncation is not new logic — it is the existing truncating `div_rem` toward zero.
Mixing `mnr` with `nr` never reaches these routines: it is a **static type error**
in the checker (§4.5), unchanged.

### 3.4 Display — duodecimal decomposition into Latin names

`fraction::display(count, G)` renders a value. The algorithm converts the count to
scripula, splits it into whole / twelfths / sub-uncia, names each part, and joins
with ` et `:

1. **Sign & magnitude.** If `count` is negative, emit a leading `-`; work with the
   magnitude `m = |count|` thereafter.
2. **Whole and grain-remainder.** `(whole, rem) = m.div_rem(G)` — `whole` is a
   `Roman` (unbounded; rendered by `Roman::display`), `rem` is the fractional part
   in grain units, `0 ≤ rem < G` (so `rem` is small — `< 288` — and safely fits a
   host integer via `to_i64` for the naming step).
3. **To scripula.** `rem_scr = rem · (288 / G)` (× 24 for `mnr`, × 1 for `nr`),
   giving `0 ≤ rem_scr < 288`.
4. **Twelfths.** `twelfths = rem_scr / 24` (0..11) names one part from the table;
   `sub = rem_scr % 24` (0..23) is the sub-uncia remainder.
5. **Sub-uncia.** Greedy over `{semuncia: 12, sicilicus: 6, sextula: 4,
   dimidia sextula: 2, scripulum: 1}` scripula, each unit used at most once
   (verified exact and repeat-free for every remainder 0..23).
6. **Assemble.** Join the non-empty parts — `[whole numeral] [twelfth name]
   [sub-uncia names…]` — with ` et `. If *all* parts are empty the value is zero →
   `nullus`.

Because a `mnr` value is always a whole number of unciae, `rem_scr` is always a
multiple of 24, so `sub` is always empty: **`mnr` renders as at most
`NUMERAL et TWELFTH-NAME`** (short). Only `nr` ever stacks sub-uncia parts.

The twelfths table (fractional part `k/12`, `k = 1..11`):

| k/12 | name | k/12 | name |
|---|---|---|---|
| 1/12 | uncia | 7/12 | septunx |
| 2/12 | sextans | 8/12 | bes |
| 3/12 | quadrans | 9/12 | dodrans |
| 4/12 | triens | 10/12 | dextans |
| 5/12 | quincunx | 11/12 | deunx |
| 6/12 | semis | | |

Worked examples:

| value | as `mnr` | as `nr` |
|---|---|---|
| 1 + 1/3 (= 4/12) | `I et triens` | `I et triens` |
| 5/12 | `quincunx` | `quincunx` |
| 5 + 1/2 | `V et semis` | `V et semis` |
| 1/24 | `nullus` | `semuncia` |
| 1/8 (36 scr) | `uncia` | `uncia et semuncia` |
| 1/7 | `uncia` | `uncia et semuncia et sextula et scripulum` |
| -(1 + 1/2) | `-I et semis` | `-I et semis` |

For `1/7` the grains diverge sharply: as `mnr`, `1/7 · 12 = 12/7 = 1.71 unciae`
truncates to **1 uncia** (`uncia`); as `nr`, `1/7 · 288 = 41.1 scripula` truncates
to **41 scripula** = 24 + 12 + 4 + 1 = `uncia et semuncia et sextula et scripulum`.

### 3.5 Literals & checker elaboration — carry an exact rational

The literal syntax is unchanged (`°W:N/D`, e.g. `°I:I/III`). What changes is that
the fraction is **not** collapsed to a float at parse time; it is carried as an
*exact rational* and narrowed to a grain count at the demand site, reusing the
adaptive-`FloatLit` machinery already built for the `f32`/`f64` split:

- `parse_roman`'s fraction branch stops computing `int_val.to_f64() + …`. Instead
  it folds the whole part into a single **combined numerator** and keeps the
  denominator, producing the exact rational `numer / den`, where
  `numer = int_val·den + frac_num` (signed `Roman`, `frac_num` being the literal's
  own numerator) and `den` is the positive `Roman` denominator. The value is
  exactly `numer / den`. `RomanNumber::Float(f64)` becomes
  `RomanNumber::Frac { numer: Roman, den: Roman }`.
- `Expr::FloatLit(f64)` becomes `Expr::FloatLit { numer: Roman, den: Roman }` — the
  abstract, pre-elaboration fractional literal (variant name kept, payload swapped,
  mirroring the integer design's "keep the variant name" decision).
- `resolve_floatlit(e, ty)` narrows it: for grain denominator `G` (12 for
  `Type::Float`, 288 for `Type::Double`), the count is `(numer · G) div den`
  (truncated toward zero), rewriting the node to `Expr::Float(Roman)` /
  `Expr::Double(Roman)` (payloads now `Roman` counts, not `f32`/`f64`).
- Unconstrained literals still **default to `nr`** (`Type::Double`), unchanged.

The checker's *type* rules are payload-blind and do not move: `InferTy::FloatLit`,
the `Float`/`Double`/mixing arms, and the "float literal adopts context type" sink
logic all read types, not values. Only `resolve_floatlit`'s rewrite body and the
literal payload types change.

### 3.6 No overflow

The grain count is a `Roman`, so fractional arithmetic **cannot overflow** — the
count vector simply grows. Any residual float-overflow / `inf` reasoning from the
IEEE design is gone: there is no `inf`, no `NaN`, no `f32`/`f64` range. Division by
a zero fraction is a clean runtime error (§5), not `inf`.

## 4. Implementation

### 4.1 Module layout — a new `fraction.rs`

Add `crates/lingua-serpentis/src/fraction.rs`, depending one-way on `roman.rs`
(`use crate::roman::Roman;`). It houses the fixed-point operations and the
duodecimal display, keeping `roman.rs` the pure integer engine (already ~540
lines). Per the user's approved decision, **no new `Fraction`/`Duodecimal`
struct** — the value is a bare `Roman` and the grain is passed in. The module
exposes free functions parameterized by the grain denominator `G` (itself a
`Roman`):

- `frac_mul(c1: &Roman, c2: &Roman, g: &Roman) -> Roman`
- `frac_div(c1: &Roman, c2: &Roman, g: &Roman) -> Result<Roman, RuntimeError>`
  (or return an `Option`/enum the interp maps to `divisio per nullum`)
- `frac_from_rational(numer: &Roman, den: &Roman, g: &Roman) -> Roman` — the
  `(numer · g) div den` narrowing used by `resolve_floatlit`
- `display(count: &Roman, g: &Roman) -> String` — the §3.4 decomposer
- two grain constants, `UNCIA_GRAIN = Roman::from(12)` and
  `SCRIPULUM_GRAIN = Roman::from(288)` (or small helper accessors)

The decomposer needs the count's sign and magnitude; if `Roman` lacks a public
`is_negative`/`abs`, add thin helpers (or use `*count < Roman::zero()` and
`neg()`), and reuse `div_rem`, `to_i64` (on the small `< 288` remainder), and
`Roman::display` for the whole part. The sub-uncia naming works on a plain host
integer 0..23.

### 4.2 `roman.rs` — parsing changes output type only

- `RomanNumber::Float(f64)` → `RomanNumber::Frac { numer: Roman, den: Roman }`. The
  `Int(Roman)` arm is unchanged.
- The fraction branch of `parse_roman` builds `numer = int_val·den + frac_num` and
  keeps `den`; it drops the `to_f64()` computation. The existing
  `den.is_zero()` → `denominator est nullus` guard stays (a literal `/nullus` is
  still rejected at parse time).
- `to_f64` on `Roman` is no longer needed by the fraction path; keep it only if
  another caller uses it (audit — likely removable, but out of scope to force).

### 4.3 Tokens — `token.rs`

- `TokenKind::Float(f64)` → `TokenKind::Frac { numer: Roman, den: Roman }` (keep a
  short variant name; it still means "a fractional literal").
- `From<RomanNumber> for TokenKind`: `RomanNumber::Int(r) => TokenKind::Int(r)`,
  `RomanNumber::Frac { numer, den } => TokenKind::Frac { numer, den }`.

### 4.4 Lexer — `lexer.rs`

The `°…` branch is unchanged in shape — it calls `parse_roman` and pushes
`value.into()`; only the carried payload type differs. No other lexer change.

### 4.5 Checker — `checker.rs` (almost nothing)

The float *type* rules are payload-blind and stay. `Expr::FloatLit { .. }` →
`InferTy::FloatLit`; `Expr::Float(_)`/`Expr::Double(_)` → their `Known` types;
every `FloatLit`/`Float`/`Double` mixing arm and the untyped-sink resolution are
untouched. The only edits:

- `resolve_floatlit` reads `Expr::FloatLit { numer, den }` and, per target grain,
  writes `Expr::Float(frac_from_rational(numer, den, &UNCIA_GRAIN))` /
  `Expr::Double(frac_from_rational(numer, den, &SCRIPULUM_GRAIN))` (§3.5).
- The recursive `resolve_floatlit` arms for `Unary`/`Binary` are unchanged in
  shape.

`mnr`/`nr` stay `Type::Float`/`Type::Double`; `type_name`, assignability, and the
strict no-mix rule do not change.

### 4.6 AST — `ast.rs`

- `Expr::FloatLit(f64)` → `Expr::FloatLit { numer: Roman, den: Roman }`.
- `Expr::Float(f32)` → `Expr::Float(Roman)`; `Expr::Double(f64)` →
  `Expr::Double(Roman)`. `Expr` is already `Clone`-not-`Copy`, so `Roman` payloads
  change no derive. `Type` (`Float`/`Double` fieldless variants) is untouched.

### 4.7 Value & interpreter — `value.rs`, `interp.rs`

- `value.rs`: `Value::Float(f32)` → `Value::Float(Roman)`, `Value::Double(f64)` →
  `Value::Double(Roman)`. `Value::display`'s two arms become
  `Value::Float(c) => fraction::display(c, &UNCIA_GRAIN)` and
  `Value::Double(c) => fraction::display(c, &SCRIPULUM_GRAIN)`. `Value` keeps
  deriving `Clone, PartialEq` (structural `Roman` equality is numeric, §3.2).
- `interp.rs`:
  - `Expr::Float(c) => Value::Float(c.clone())`, `Expr::Double(c) =>
    Value::Double(c.clone())`.
  - Unary `Neg` on `Float`/`Double` → `c.neg()`.
  - `eval_binary` `Float` arms use `frac_*` with `UNCIA_GRAIN`; `Double` arms with
    `SCRIPULUM_GRAIN`. `Add`/`Sub` call `Roman::add`/`sub`; `Mul`/`Div` call
    `frac_mul`/`frac_div`; a zero divisor from `frac_div` maps to the existing
    `divisio per nullum` runtime error.
  - **Decision: fractions support `+ - * /` and comparisons only; `%` (`Mod`) is
    not defined on `mnr`/`nr`.** This design changes representation, not the
    fractional operator set — with one deliberate exception: if `Mod` is currently
    accepted on `Float`/`Double`, migration turns a fractional `%` into the same
    static "operator not defined for this type" error the checker already raises
    for other type-invalid binops, and a test pins that. (Modulo on a duodecimal
    grid has no clean, obviously-correct meaning; excluding it keeps the surface
    honest. Task 0 of the plan should confirm whether float `%` type-checks today
    so this is a no-op or a one-arm removal.)
  - Comparison arms (`cmp`, `Eq`/`Ne`, `Lt`/`Gt`) on `Float`/`Double` use
    `Roman`'s `Ord`/`PartialEq` on the counts — now total (no NaN paths to worry
    about).
  - The defensive/unreachable `Expr::FloatLit` interp arm (if present) updates to
    the new payload shape or stays a guarded arm.

`Value::display` and `eval_binary`/`cmp` are exhaustive matches, so the compiler
forces every float/double arm to be revisited.

### 4.8 `lib.rs`, `main.rs`, `latinity.rs` — verify only

None handle fractional payloads directly. Confirm each still compiles against the
new `Expr`/`Value`/`TokenKind`; no logic change expected. Any new Latin error or
name string introduced (there should be none beyond the fixed fraction-name table,
which is fixed vocabulary) must pass the `latinitas` `is_latin` check per project
convention — note that the fraction *names* themselves (`uncia`, `semuncia`, …)
are data in `fraction.rs`, not identifiers, so they are not linted, but they are
genuine Latin regardless.

## 5. Errors

No new user-facing error *conditions*; one behaviour change and one reused
message:

| Condition | Message | Change |
|---|---|---|
| fraction divide by zero | `divisio per nullum` | **new error where IEEE gave `inf`**; reuses the integer divide-by-zero message |
| literal `/nullus` denominator | `denominator est nullus` | unchanged (parse-time guard) |

The IEEE `÷0 → inf` / `NaN` behaviours are **removed** — they can no longer occur.
This retires the "document float ÷0 → inf" follow-up from the earlier review by
eliminating the case.

## 6. Migration

The representation change is shallow but its output reaches every fraction that is
printed.

- **Fraction-literal / value construction in tests.** `roman.rs`'s
  `parse_fraction_literal` test (`°V:I/II` → `RomanNumber::Float(5.5)`) is rewritten
  to assert the new `RomanNumber::Frac { numer, den }` (here `11/2`). Any
  `Expr::Float(..)`/`Expr::Double(..)`/`FloatLit(..)` built with bare floats in
  parser/checker tests move to `Roman` counts / rational payloads (provide
  `From<i64> for Roman` reuse and a local helper as the integer migration did).
- **Interpreter / integration / example expected output.** Every fractional
  `scribe` changes from a decimal string to Latin names. **`examples/praecisio.liss`
  is rewritten**: its entire point was `f32` vs `f64` decimal digit divergence
  (`1.3333334` vs `1.3333333333333333`), which no longer exists — 1/3 is exact on
  both grids (`I et triens`). The new file demonstrates the **grain** difference
  instead, e.g.:
  - `mnr` `°I:I/XXIV` → `I` vs `nr` `°I:I/XXIV` → `I et semuncia`
    (the new canonical `mnr` vs `nr` demonstration — 1/24 is below the uncia grid,
    so `mnr` drops it and prints just the whole `I`);
  - a `°I:I/II + °I:I/II` → `III` carry example;
  - the exact-on-both case `°I:I/III` → `I et triens` for both types.
- **The old canonical test flips.** Any test asserting `°I:I/III` prints
  `1.3333334`/`1.3333333333333333` now asserts `I et triens` for both types; a
  *new* test pins the grain-difference demo above so the `mnr`/`nr` distinction
  stays observably covered.
- **Integer tests are untouched.** The `ni` Roman-native work stands; `calc.liss`
  and other integer-only output do not change.

Documentation:

- Amend `2026-07-22-nr-double-distinct-type-design.md` and the fractions Non-Goal
  in `2026-07-23-roman-native-integers-design.md` **in place** to point here (the
  "fractions stay IEEE / print decimal" statements are now superseded). Do not
  rewrite them.
- Update the top-level `README` if it shows decimal fractional output or claims
  `mnr`/`nr` are IEEE floats.

## 7. Testing

`fraction.rs` unit tests (the new engine — the load-bearing coverage):

- **Grain-count parity.** For a table of rationals and both grains, assert
  `frac_from_rational` equals the independently computed truncated count
  `trunc(num·G / den)` (reference via `i128`), including negatives and values that
  truncate to zero.
- **Arithmetic parity.** For representative counts and both grains, assert
  `frac_mul`/`frac_div` equal `(c1·c2) div G` / `(c1·G) div c2` computed against an
  `i128`/rational reference (truncating toward zero); assert add/sub/neg equal the
  plain `Roman` ops; assert `frac_div` by a zero count errors.
- **Display — every twelfth.** `k/12` for `k = 0..12` renders the right single
  name (`0 → nullus`, `12 → I`), integer part included where present.
- **Display — sub-uncia stacks.** Representative `nr` remainders (e.g. `1/24 →
  semuncia`, `1/8 → uncia et semuncia`, `1/7 → uncia et semuncia et sextula et
  scripulum`, and the maximal `23`-scripula stack) render the correct greedy
  composition; assert **no `mnr` value ever emits a sub-uncia name**.
- **Sign & zero.** Negative renders a single leading `-`; zero renders `nullus`;
  whole-valued fraction renders just the numeral (`II`, not `II et …`).
- **Unbounded.** A count beyond `i64` range still computes and displays a correct
  (large whole-part) rendering — no panic.

Interpreter / integration:

- Existing float tests with Latin-name expected strings (§6) stay green.
- The rewritten `praecisio.liss` end-to-end and a new integration test pinning the
  `mnr` `nullus` vs `nr` `semuncia` demonstration.
- Fraction divide-by-zero raises `divisio per nullum`.

Whole suite: the current **176** tests stay green after string updates, plus the
new `fraction.rs` engine tests.

## 8. Design Notes

**Why fixed-point (grain count), not an exact `p/q` rational.** A general
numerator/denominator rational is exact for *arbitrary* fractions, but then (a) the
`mnr`/`nr` distinction — which the user just built as a real precision difference —
collapses, because exactness has no resolution gradient; and (b) display of an
arbitrary ratio as a Roman fraction needs rounding or a ratio fallback,
reintroducing the very ambiguity the named system avoids. Fixed-point keeps a
genuine coarse/fine `mnr`/`nr` distinction, guarantees every value is exactly
nameable (it lives on the grid by construction), and — crucially — reuses the
existing truncating integer engine wholesale. It is the fractional analogue of the
integer decision (compute on the numeral, truncate like `i64`).

**Why the value is a bare `Roman`, not a `Fraction` struct.** The grain is fixed
per type and already encoded in the `Value::Float`/`Value::Double` variant, so a
struct pairing `count` with a grain would carry redundant state and touch every
match arm for no gain. Passing `G` into the handful of `fraction::` functions keeps
the value identical in shape to an integer (a `Roman`) and the diff minimal.

**Why truncate, not round.** One truncation convention (toward zero) now spans the
whole stack — integer division and fractional multiply/divide behave the same, with
no tie-break rule to specify. It is also the historically honest choice (Roman
reckoning had no rounding convention), and it falls directly out of the existing
`div_rem`.

**Why classical Latin names, not the dot/`S` symbols.** The authentic dot-and-`S`
notation is visually Roman but its sub-uncia glyphs (`semuncia`, `sicilicus`,
`scripulum`) are inconsistent across sources and render poorly or not at all in a
terminal — fatal at `nr`/scripulum depth. Latin names are terminal-safe, exactly
on-theme for a Latin-keyword language, unambiguous, and compose additively with
`et` the way the Romans wrote them out in words. (The symbolic form remains a v2
display option if ever wanted; the representation does not depend on notation.)

**Why keep the variant names `Float`/`Double`/`FloatLit`.** `mnr`/`nr` are still
`Type::Float`/`Type::Double`; keeping the runtime/AST variant names (payloads now
`Roman`/rational) keeps the payload-blind checker arms and exhaustive matches
minimal, exactly as the integer design kept `Int`. The precision meaning now lives
in the grain, not the Rust float width, but the names still read correctly (`mnr` =
lesser precision = `Float`, `nr` = greater = `Double`).

**Alternatives rejected.** (a) *Display-only — keep `f32`/`f64`, print Roman
fractions* — the "float in a costume" mirror of the integer costume the user
rejected; inconsistent with the direction chosen. (b) *Exact `p/q` rational* — see
the first note; collapses `mnr`/`nr` and complicates display. (c) *Round to
nearest* — adds a convention the rest of the stack lacks. (d) *A finer `mnr` grain
by default (semuncia/sicilicus)* — softens the `1/24 → nullus` edge but blurs the
coarse/fine contrast and is less authentic than the everyday `uncia`; left as a
one-constant knob.

## 9. v2 Candidates

Not rejected, just not now:

- **Casts / conversions between `mnr` and `nr`** (lossless `mnr`→`nr`, truncating
  `nr`→`mnr`) — the natural next relaxation once both are Roman-native.
- **Whole-number fraction literals** (`mnr x = °V;`) — still needs a literal form
  for a fractional whole; shared with the integer design's deferral.
- **The dot/`S` symbolic display** as an alternate notation, and a finer `mnr`
  grain, both designed-for above.
- **A `Fraction` newtype** if pairing count+grain ever proves clearer than passing
  `G` (e.g. if fractions gain many more operations).
- **`SmallVec` grain counts** (shared with the integer engine's optimization note)
  to avoid heap allocation on clone for common magnitudes.
