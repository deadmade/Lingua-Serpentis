# Lingua Serpentis — Roman-Native Integers (`ni`) Design

- **Date:** 2026-07-23
- **Status:** Design approved in brainstorming; this written spec pending user review
- **Author:** Manuel (with Claude)
- **Amends:** `2026-07-19-lingua-serpentis-rust-interpreter-design.md` — the handling
  of Roman literals and the `ni` type (Roman numerals were converted to `i64` at
  lex time and never seen again). Interacts with, but does not change,
  `2026-07-22-nr-double-distinct-type-design.md` (the fractional types `mnr`/`nr`
  stay IEEE floats — see §2 Non-Goals).

## 1. Vision

Today a Roman numeral is pure input syntax. `°XXV` is converted to `i64` in the
lexer (`roman.rs` → `token.rs`), and from the parser onward there is no trace it
was ever Roman — it is the same `25` you would get from an Arabic literal (which
the language cannot even write). At the other end, `value.rs` prints
`Value::Int(i) => i.to_string()`, so it comes out `25`. The conversion is a
one-way street: **Roman in, integer forever after, Arabic out.**

For a language whose whole identity is Latin keywords and Roman numerals, that is
the wrong center of gravity. This design makes the numeral **the truth**: the
integer type `ni` stops being an `i64` wearing Roman input syntax and becomes a
genuinely Roman value — a structured numeral the interpreter computes on directly
(by carrying and borrowing, the way an abacus does), and that prints back as a
Roman numeral. After this change there is no Arabic number anywhere a user can
see: you write Roman, you compute in Roman, you get Roman back.

```
scribe(°V + °III * °II);   → XI          (was: 11)
ni x = °X; x = x + °II;    scribe(x);  → XII
scribe(°MMM * °MMM);       → an unbounded vinculum numeral (was: overflow error)
```

The change is deliberately narrow where it counts: **the type system does not
move.** `ni` stays one type (`Type::Int`); the checker's type rules are identical
whether an integer is backed by `i64` or by a numeral. What changes is the
runtime *representation*, the *literal* the parser builds, the *arithmetic*, and
*display* — concentrated in `roman.rs`, `value.rs`, and `interp.rs`. The one
place a machine integer survives is the boundary with Rust's own `Vec` (list
indices and lengths), and it is invisible to the language.

## 2. Goals and Non-Goals

### Goals

- Replace the runtime backing of `ni` with a **Roman-native value** (§3.1): a
  sign plus a bag of numeral symbols, structured as decimal tiers, so it is the
  numeral rather than an integer in a costume.
- Compute directly on that representation. `+ - * / %`, comparison, negation, and
  equality are defined on numerals by **carry / borrow** and operations built
  only on those (§3.4) — never by converting the value to a host `i64` to do the
  math.
- Represent **arbitrarily large** numbers with small, honest per-symbol counts,
  via a generalized tier ladder (§3.1). This retires overflow for `ni` entirely
  (§3.6).
- Support **zero** (the empty numeral, printed `nullus`) and **negatives** (a
  sign flag) so subtraction, unary negate, loops, and indexing keep working
  exactly as they do today (§3.2).
- Print numerals back: standard letters through 3999, **vinculum** (overline =
  ×1000) above that (§3.5).
- Keep integer arithmetic **results identical** to today's `i64` semantics
  (truncating division toward zero, remainder sign follows the dividend), so only
  *output notation* changes, not *values* (§3.4, §6).
- Keep the type checker essentially untouched (§4.5).

### Non-Goals (v1)

- **Roman-native fractions.** `mnr`/`nr` stay `f32`/`f64` exactly as
  `2026-07-22-nr-double-distinct-type-design.md` left them, and keep printing as
  decimals. This means a mixed look for now — integers Roman, fractions decimal
  (e.g. `1.3333334`). Rebuilding fractions on the Roman duodecimal system
  (`uncia` = 1/12, …) is a whole second engine and is deferred (§9). This is the
  float question raised in brainstorming and set aside, not decided here.

> **Superseded 2026-07-23:** that deferred second engine now exists —
> `2026-07-23-roman-native-fractions-design.md`. Fractions are Roman-native too;
> the "integers Roman, fractions decimal" mixed look described here no longer
> applies.

- **Casts / conversions** between `ni` and the float types. Mixing stays the same
  type error it is today.
- **A user-facing Arabic notation.** There is intentionally still no way to write
  or print an Arabic integer.
- **Changing arithmetic results.** No new rounding, no new overflow policy beyond
  "there is no overflow"; negative division/modulo keep today's truncation
  semantics (§3.4).
- Everything already deferred to v2 elsewhere (inference, imports, `>=`/`<=`,
  for-each, maps, …).

## 3. Semantics

### 3.1 The Roman value

An `ni` value is a **sign plus a bag of numeral symbols, grouped into decimal
tiers**:

```rust
// roman.rs
pub struct Roman {
    negative: bool,     // invariant: always false when the value is zero
    tiers: Vec<Tier>,   // little-endian: tiers[0] = ones, tiers[1] = tens, …
}
struct Tier { units: u8 /* 0..=4 */, five: u8 /* 0..=1 */ }
```

Each tier `k` contributes `units × 10^k` plus `five × 5 × 10^k`. Tier 0 is I/V,
tier 1 is X/L, tier 2 is C/D, tier 3 is M/V̄, tier 4 is X̄/L̄, and up with no
ceiling (§3.5 gives the symbol ladder). The seven fixed symbols `I V X L C D M`
were only the first 2½ turns of a wheel that repeats `×5, ×2` forever; making the
tiers a `Vec` captures the whole wheel and makes the value unbounded.

**Canonical form (an enforced invariant).** Every `Roman` produced by any
constructor or operation satisfies:

1. each tier has `units ≤ 4` and `five ≤ 1`;
2. there are no trailing all-zero tiers (the `Vec` ends on a non-zero tier);
3. the empty `Vec` is the unique representation of **zero**, and zero is never
   `negative`.

Because the canonical form is unique for each mathematical value, structural
equality of two `Roman`s is exactly numeric equality — which is what lets
`Value` keep deriving `PartialEq` and lets list equality (`par_est`) keep working
unchanged (§4.6). Every operation normalizes before returning; no non-canonical
`Roman` is ever handed out.

The per-tier counts are the only integers in the whole structure, and they are
tiny (`units` is `0..=4`, `five` is `0..=1`). That is the honest "Roman is the
truth" line: the value is a collection of numeral symbols, exactly as you would
name it in Latin ("duo decem et quinque" — two tens and a five), not a binary
integer with a Roman mask.

### 3.2 Zero and negatives

Both were chosen (brainstorming) as **practical: allowed**, because loops,
subtraction, unary negate, and index math already produce them.

- **Zero** is the empty tier vector — the natural additive identity, needing no
  special case in arithmetic. It renders as `nullus` (§3.5). The `nullus` literal
  and the bare `0`-producing paths map to it.
- **Negatives** are the `negative` flag; the magnitude lives in the tiers.
  Subtraction that crosses zero flips the flag; unary `Neg` toggles it (except on
  zero, which stays non-negative per the invariant). A negative renders with a
  leading `-` (§3.5) — chosen over the word `minus` in brainstorming.

### 3.3 Types are unchanged

`ni` is still exactly one type, `Type::Int`. Nothing about assignability, the
strict no-implicit-conversion rule, comparison-yields-`veritas`, or
`ni`-vs-`mnr`/`nr` mixing changes. The checker's integer rules are payload-blind
(§4.5). This is what keeps the blast radius off the type system.

### 3.4 Arithmetic — on numerals, never on `i64`

All operations are defined on the tier representation and reduce to **carry** and
**borrow**:

- **Add.** Sum the two tier vectors position-by-position, then normalize by
  carrying: `5 units → 1 five` (5 I = V), `2 fives → 1 unit of the next tier`
  (2 V = X). Opposite signs become a magnitude subtraction.
- **Subtract.** Compare magnitudes (§ below), borrow (the carry rules in
  reverse), and take the sign from the larger side; equal magnitudes give zero.
- **Multiply.** A doubling ladder (Russian-peasant / Egyptian): compose from
  repeated `add` (doubling) and halving, so it is `O(log)` and still never does a
  host multiply. Result sign is the XOR of operand signs.
- **Divide / modulo.** Doubling long division built on `add`, `subtract`, and
  magnitude comparison. **Semantics match today's `i64`:** quotient truncates
  toward zero, remainder's sign follows the dividend. Division or modulo by zero
  stays a runtime error (`divisio per nullum` / `modulus per nullum`), unchanged.
- **Compare** (`maior_est`, `minor_est`, and the ordering used by `cmp`). Sign
  first (negative < zero < positive); within a sign, the longer canonical tier
  vector is larger; on equal length, compare tiers from the highest down by that
  tier's digit `five×5 + units` (0..9). Yields a total order (`Ord`).
- **Equality** (`par_est` / `par_non_est`). Structural on the canonical form
  (§3.1); no separate numeric path needed.

The magnitude routines (add/sub/mul/divmod on the unsigned tier vectors) are the
core; the signed operations are thin wrappers that decide the result sign and
delegate. This keeps every path built out of carrying, which is the point.

### 3.5 Display — vinculum

`Roman::display()` produces the printed form:

- **Zero** → `nullus`.
- **Negative** → `-` then the magnitude.
- **Magnitude**: walk tiers high → low; render each tier's digit (`five×5 +
  units`, 0..9) with the **classic subtractive rules** using that tier's *one*
  and *five* glyphs plus the next tier's *one* glyph — i.e. `4 → I·V` (IV),
  `9 → I·X` (IX), etc., lifted to the tier's symbols.

The glyph ladder (a bar = overline = ×1000):

| tier | one | five | value of "one" |
|---|---|---|---|
| 0 | I | V | 1 |
| 1 | X | L | 10 |
| 2 | C | D | 100 |
| 3 | M | V̄ | 1000 |
| 4 | X̄ | L̄ | 10 000 |
| 5 | C̄ | D̄ | 100 000 |
| 6 | M̄ | V̿ | 1 000 000 |
| … | … | … | … |

The pattern repeats every three tiers with one additional overline. **1000 prints
as the familiar `M`** (not `Ī`), so `MMM` = 3000 as expected; this is the one
irregularity in an otherwise uniform ladder and is confined to the renderer (the
representation stays uniform). The overline is emitted as the base letter followed
by U+0305 COMBINING OVERLINE (repeated for higher bar counts). *Note:* combining
overlines render inconsistently across terminals/fonts; on the target Windows
Terminal they display acceptably. If they ever prove unreadable, the ASCII bar
fallback and the round-tripping dot-notation form remain available as v2 display
options (§9) — the representation does not depend on the chosen notation.

Input notation is untouched: `°XXV`, `°VII.M` (multiplicative dot form), and
`nullus` all keep lexing as before. Output uses vinculum regardless of how a
number was written.

### 3.6 No overflow

With a `Vec` of tiers there is no fixed width, so `ni` arithmetic **cannot
overflow** — the vector simply grows a tier. The `checked_add/sub/mul/div/rem`
calls and the `numerus nimis magnus (excessus arithmeticus)` error in
`eval_binary` disappear for the integer arms (§4.7). This is a deliberate,
observable behavior change and it retires the "checked arithmetic in roman.rs"
follow-up for `ni`. (Float overflow to `inf` is a separate `mnr`/`nr` concern and
is unchanged.) The existing `arithmetic_overflow_errors` test is repurposed
(§6, §7).

## 4. Implementation

### 4.1 Module layout — `roman.rs` grows into the engine

`roman.rs` gains the `Roman` type, its constructors, the arithmetic/compare/
display implementations, and their unit tests, alongside the existing parsing
functions. If the file grows unwieldy, split it into a `roman/` module
(`value.rs` for the type + constructors, `arith.rs` for the magnitude routines
and signed wrappers, `display.rs` for vinculum, `parse.rs` for the existing
`parse_roman`/`convert_*`), re-exported from `roman/mod.rs`. Start as one file;
split only if it becomes hard to hold in view.

The public surface `roman.rs` must expose to the rest of the crate:

- `Roman` (with `Debug, Clone, PartialEq, Eq`) and a manual `Ord`/`PartialOrd`.
- Constructors: `Roman::zero()`, `Roman::from_u64(u64)`, `Roman::from_i64(i64)`,
  `Roman::from_usize(usize)` (used by `longitudo`), and `From<i64>` for terse test
  literals (see §6).
- Operations: `add`, `sub`, `mul`, `div_rem` (returns `(quotient, remainder)`),
  `neg`, `is_zero`.
- Boundary conversions (the only `i64`/`usize` seams, §4.8):
  `to_i64() -> Option<i64>`, `to_usize() -> Option<usize>`, `to_f64() -> f64`.
- `display(&self) -> String`.

### 4.2 `roman.rs` — parsing keeps its shape, changes its output type

- `RomanNumber::Int(i64)` becomes `RomanNumber::Int(Roman)`. The `Float(f64)` arm
  is unchanged (fractions stay IEEE).
- `convert_single` and `convert_int_roman` return `Roman` instead of `i64`. They
  build the value **through the engine** (add/subtract for the subtractive
  I-before-V rule, multiply for the `.`-separated multiplicative pairs), so the
  parser exercises the arithmetic engine on every literal.
  - *Acceptable fallback for the literal path only:* if building via the engine
    complicates bring-up, compute the literal's magnitude as `i64` (as today) and
    finish with `Roman::from_i64`. This is safe **only** for literals — human-
    typed numerals fit `i64` — and never appears on the runtime value path, so it
    does not weaken "Roman is the truth". Prefer the engine build; keep this in
    reserve.
- The fraction branch computes its `f64` from `int_val.to_f64()`,
  `num.to_f64()`, `den.to_f64()`; the `den.is_zero()` guard replaces `den == 0`.

### 4.3 Tokens — `token.rs`

- `TokenKind::Int(i64)` → `TokenKind::Int(Roman)`. Keeping the variant **name**
  `Int` (payload now `Roman`) minimizes churn: it still means "an `ni` literal".
- `From<RomanNumber> for TokenKind`: `RomanNumber::Int(r) => TokenKind::Int(r)`,
  `Float(f) => TokenKind::Float(f)`.

### 4.4 Lexer — `lexer.rs`

- `word_to_kind("nullus")` returns `TokenKind::Int(Roman::zero())` instead of
  `TokenKind::Int(0)`.
- The `°…` branch is unchanged in shape — it still calls `parse_roman` and pushes
  `value.into()`; only the carried payload type differs.

### 4.5 Checker — `checker.rs` (almost nothing)

The integer rules are payload-blind. `Expr::Int(_)` → `InferTy::Known(Type::Int)`
(the `_` ignores the new payload), `Type::Int` and its `type_name` are untouched,
`resolve_floatlit` only ever rewrites `Float`/`Double` nodes, index-must-be-`ni`
and all mixing rules are unchanged. Expected work here: **none beyond compiling**
against the new `Expr::Int` payload. Its own tests construct `Expr` via source, so
they are unaffected except where they assert on printed numbers (they do not — the
checker tests assert `is_ok`/`is_err`).

### 4.6 AST — `ast.rs`

- `Expr::Int(i64)` → `Expr::Int(Roman)`. `Expr` is already `Clone`-not-`Copy`
  (it holds `String`/`Box`/`Vec`), so a `Vec`-bearing `Roman` payload changes no
  derive. `Type` is untouched (`Int` stays a fieldless variant).

### 4.7 Value & interpreter — `value.rs`, `interp.rs`

- `value.rs`: `Value::Int(i64)` → `Value::Int(Roman)`; `Value::display` becomes
  `Value::Int(r) => r.display()`. `Value` keeps deriving `Clone, PartialEq` —
  `PartialEq` on the canonical `Roman` is numeric equality (§3.1), so `BinOp::Eq`
  (`l == r`) and list equality keep working.
- `interp.rs`:
  - `Expr::Int(i) => Ok(Value::Int(*i))` becomes a clone of the `Roman` payload.
  - Unary `(UnOp::Neg, Value::Int(r)) => Value::Int(r.neg())`.
  - `eval_binary` integer arms call `Roman::add/sub/mul` and `div_rem` (quotient
    for `Div`, remainder for `Mod`), keep the zero-divisor errors, and **drop the
    `overflow` closure and every `checked_*` call** for the `Int` arms (§3.6).
    The `Float`/`Double`/`Text` arms are unchanged.
  - `cmp` integer arm uses `Roman`'s `Ord` (`a.cmp(&b)`).
  - The implicit "function fell off the end" return `Ok(Value::Int(0))` becomes
    `Ok(Value::Int(Roman::zero()))`.
  - `longitudo` returns `Value::Int(Roman::from_usize(items.len()))`.
  - Defensive `Expr::FloatLit` arm is unchanged.

`Value::display` and `interp::eval`/`eval_binary` are exhaustive `match`es on
`Value`/`Expr`, so the compiler forces the display and eval arms to be updated;
the catch-all arms in `eval_binary`/`cmp` will compile but be wrong if the `Int`
arms are not converted, so they are called out here explicitly.

### 4.8 The one machine-integer boundary — list indices & lengths

Rust's `Vec` indexes with `usize`, so a `Roman` index must convert at the edge —
the only place `i64`/`usize` touches a numeral:

- **Index read** (`interp.rs` `Expr::Index`) and **index write**
  (`StmtKind::IndexAssign`): replace the `Value::Int(idx)` match with
  `idx.to_usize()`. A `None` (negative, or larger than `usize`) or a `0` or an
  out-of-range value raises the existing `index extra fines: {n} (longitudo {len})`
  error, with `{n}` now formatted from `idx.display()` (Roman). The 1-based rule
  and the "index 0 / bare `nullus` is out of range" behavior are preserved.
- **Length** flows the other way: `items.len()` (a `usize`) becomes a `Roman`
  via `Roman::from_usize` (`longitudo`, §4.7).

These are the "invisible plumbing" conversions — the user never sees an `i64`;
they only ever see the `Roman` on either side.

### 4.9 `lib.rs`, `main.rs`, `latinity.rs` — verify only

None handle integer payloads. `check(&mut program)` / `interp::execute` / the
`latinity` lint keep their signatures. Confirm each still compiles against the new
`Expr`/`Value`/`TokenKind`; no logic change is expected.

## 5. Errors

No new user-facing error *conditions*. Two shifts, both already-existing messages:

| Condition | Message | Change |
|---|---|---|
| `ni` divide/modulo by zero | `divisio per nullum` / `modulus per nullum` | unchanged; the zero test is now `Roman::is_zero()` |
| index out of range | `index extra fines: {n} (longitudo {len})` | `{n}` now prints Roman via `idx.display()` |

The `numerus nimis magnus (excessus arithmeticus)` overflow error is **removed**
for `ni` (§3.6) — it can no longer occur. Any new internal error wording added
while implementing (there should be none needed) must be validated against the
`latinitas` dictionary (`is_latin`) before landing, per the project convention.

## 6. Migration

The representation change is shallow but its *output* reaches broadly: **every
place a number is printed now prints Roman.** That is the feature, but it means a
wide, mechanical sweep of expected strings.

- **`Expr::Int` / `Value::Int` / `TokenKind::Int` literal construction in tests.**
  Parser and lexer unit tests build these with bare `i64`s
  (e.g. `Expr::Int(5)`, `TokenKind::Int(25)`). Provide `From<i64> for Roman` and
  rewrite them as `Expr::Int(5.into())` / `TokenKind::Int(Roman::from(25))` (or a
  local `fn r(n: i64) -> Roman` helper). Mechanical, but many sites
  (`parser.rs` ~a dozen, `lexer.rs` `roman_numbers_become_values`).
- **Interpreter/integration/example expected output.** Every test that prints a
  number changes its expected string to the Roman form. Non-exhaustive, from
  `interp.rs` alone: `°V + °III * °II` → `XI`; `°X + °II` → `XII`;
  while-sum `10` → `X`; for-break `3` → `III`; `iunge(°IV, °VI)` → `X`;
  `[°I, °II, °III]` → `[I, II, III]`; index reads → `X`, `XXX`; index write →
  `[I, X, III]`; `longitudo` → `III`; nested reads → `II`, `III`. Plus
  `tests/programs.rs` and every numeric `scribe` in `examples/*.liss`
  (`calc.liss`, `praecisio.liss`, `series.liss`). **Watch two specials:** a
  computed `0` now prints `nullus` (not `0`), and any 4000+ result prints
  vinculum.
- **`arithmetic_overflow_errors` flips meaning.** `°MMMCMXCIX` to the 6th power no
  longer overflows; repurpose the test to assert it now **succeeds** and prints
  the correct (large, vinculum) numeral — a good end-to-end exercise of unbounded
  tiers + display (§7).
- **Float tests are untouched.** The `mnr`/`nr` precision divergence
  (`°I:I/III` → `1.3333334` / `1.3333333333333333`) and every fractional-literal
  test stay exactly as-is; `roman.rs`'s `parse_fraction_literal`
  (`°V:I/II` → `RomanNumber::Float(5.5)`) still holds, now against
  `RomanNumber::Int(Roman)` for its integer sibling tests.

Documentation:

- Amend `2026-07-19-lingua-serpentis-rust-interpreter-design.md` **in place**
  where it says Roman literals become `i64` / the `ni` type is `i64`-backed, to
  point here. Do not rewrite it.
- Update the top-level `README` if it shows numeric output or claims `ni` is an
  `i64`.

## 7. Testing

`roman.rs` unit tests (the engine — the load-bearing new coverage):

- **Round-trip:** `parse_roman("°N").display()` reproduces the numeral for a
  spread of values (subtractive `IV`, `IX`, `XL`, `XC`, `CD`, `CM`; `nullus`;
  the multiplicative `°VII.M`).
- **Carry/borrow:** `IV + I = V`, `XLIX + I = L`, `MMMCMXCIX + I = MV̄`… (tier
  rollover), and their inverses for subtract.
- **Sign:** `III - V = -II`; `neg(neg(x)) = x`; `x - x = nullus` (non-negative
  zero); `-nullus = nullus`.
- **Mul/div parity with `i64`:** a table of `(a, b)` (including negatives and a
  zero) asserts `add/sub/mul` and `div_rem` match the corresponding `i64`
  results (quotient truncates toward zero, remainder follows the dividend) —
  pins §3.4's "results identical" guarantee.
- **Unbounded:** a product that exceeds `i64::MAX` computes and displays the
  right vinculum numeral (no panic, no overflow) — pins §3.6.
- **Ordering:** `Ord` agrees with numeric order across signs and magnitudes.
- **Canonical/equality:** two independently built representations of the same
  number are structurally equal; distinct numbers are not.

Interpreter / integration:

- All existing interp tests, with Roman-updated expected strings (§6), stay
  green.
- The repurposed unbounded-arithmetic test (former overflow test).
- A `nullus`-prints-`nullus` case and a negative-prints-`-…` case.
- List index bound errors still fire (index 0, index > length, negative index).

Whole suite: the current **161** tests stay green after string updates, plus the
new `roman.rs` engine tests.

## 8. Design Notes

**Why tiers (a `Vec`), not more named fields.** The user's question — "extend the
fields to represent more numbers?" — has a better answer than adding
`v̄, x̄, …`: the numeral system is a repeating `×5, ×2` ladder, so a `Vec` of
`(units, five)` tiers captures *all* of it, is unbounded, and collapses the six
special-case carry rules into **one** uniform rule applied per tier. Adding fields
would only raise the ceiling and multiply the special cases.

**Why the counts are still "honest".** A tier holds `units ≤ 4`, `five ≤ 1` —
these are not the number, they are how many of each symbol, exactly as a Roman
would tally on an abacus. The value is the collection of symbols; no single field
holds the magnitude as a binary integer. That is the line that separates this
from "an `i64` that prints Roman" (the rejected cheap option).

**Why keep the variant name `Int` (payload `Roman`).** `ni` is still `Type::Int`;
the runtime value of an `Int` being a `Roman` reads consistently and keeps the
diff minimal (the payload-blind checker arms, the exhaustive matches). Renaming
`Value::Int`/`Expr::Int`/`TokenKind::Int` to `Roman` was considered; it is more
self-documenting but touches every arm for a naming-only gain. Keep `Int`; the
payload type name carries the clarity where it matters (construction, arithmetic).

**Why `M` for 1000, not a uniform `Ī`.** A fully uniform ladder would print 1000
as `Ī` and 3000 as `Ī̄Ī̄Ī̄` — regular in code, alien to read. Keeping `M` (so
`MMM` = 3000) costs one special case in the renderer only; the representation
stays uniform. The uniform `Ī` remains a switch we could flip (§9).

**Why parsing goes through the engine.** Building each literal with `add`/`mul`
makes the parser a free, continuous test of the arithmetic engine, and keeps even
literal construction free of a hidden `i64` value path. The `from_i64` fallback
exists only so bring-up is never blocked; it is confined to literals, which fit
`i64` by construction.

**Performance.** `Value` is cloned freely by the interpreter (`get`, argument
passing), so each clone now heap-allocates the tier `Vec`. For a teaching
interpreter on human-sized numbers this is negligible; a `SmallVec`-style inline
representation (first few tiers on the stack) is an obvious later optimization
(§9) and does not change semantics.

**Alternatives rejected.** (a) *Keep `i64`, print Roman only* — the "int in a
costume" the user explicitly rejected. (b) *A distinct Roman type beside a plain
`i64` `ni`* — reintroduces an Arabic integer with no way to write it, the exact
irritant. (c) *A Roman string as the value* — parses on every operation and
degenerates into a tier structure anyway. (d) *Naive repeated-addition multiply* —
authentic but `O(value)`; the `O(log)` doubling ladder is still purely additive
and keeps big numbers usable.

## 9. v2 Candidates

Not rejected, just not now:

- **Roman-native fractions** for `mnr`/`nr` on the duodecimal `uncia` system, so
  the whole language prints Roman rather than integers-Roman / fractions-decimal.
  The natural sequel; a second engine.
- **Casts / conversions** between `ni` and the float types.
- **`SmallVec` tiers** to avoid heap allocation on clone for common magnitudes.
- **Alternate display notations** already designed-for: the round-tripping dot
  form (`VII.M`) — output that re-parses as input — and an ASCII bar marker
  (`[V]`), as fallbacks if vinculum renders poorly anywhere; and the uniform `Ī`
  option for 1000.
- **Splitting `roman.rs`** into a `roman/` module if the engine outgrows one file
  (§4.1).
