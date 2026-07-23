# Roman-Native Fractions (`mnr` / `nr`) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the IEEE `f32`/`f64` backing of `mnr`/`nr` with a Roman-native duodecimal fixed-point value, so fractions compute and print in classical Latin fraction names instead of decimal.

**Architecture:** Build a self-contained `fraction.rs` first — a fractional value is a signed `Roman` grain-count (`mnr` counts unciae = 1/12, `nr` counts scripula = 1/288), so arithmetic is the existing `Roman` engine plus one rescaling `div_rem`, and display is a duodecimal decomposer. Tasks 1–2 build and unit-test that module in isolation. Task 3 then flips the whole literal→runtime pipeline over to it — and its integration tests and example move with it, since the payload types thread through ast/token/lexer/parser/checker/value/interp and cannot compile half-flipped. Task 4 amends the superseded docs.

**Tech Stack:** Rust (2021 edition), no third-party dependencies, hand-written lexer/parser/checker/interpreter. Reuses the `Roman` integer engine from `roman.rs`. Spec: `docs/superpowers/specs/2026-07-23-roman-native-fractions-design.md`.

## Global Constraints

- **No third-party crates.** The interpreter and `latinitas` depend on nothing but `std`. Do not add dependencies.
- **Build/test command (this machine).** `cargo` is not on `PATH` and MSVC `link.exe` needs importing first. Run every `cargo` command through this wrapper (from the repo root):

```powershell
cmd /c '"C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat" && "%USERPROFILE%\.cargo\bin\cargo.exe" test -p lingua-serpentis'
```

  Invoke `cargo.exe` by absolute path and let `vcvars64.bat` supply the linker PATH. Do **not** chain `set PATH=%USERPROFILE%\.cargo\bin;%PATH%` after vcvars — cmd expands `%PATH%` once, against the *pre-vcvars* environment, which wipes the freshly-added linker path (`link.exe not found`). Append `<filter>` for a focused run.
- **Manifest hygiene.** Something in this environment rewrites `edition = "2021"` → `"2024"` and `resolver = "2"` → `"3"` in the three `Cargo.toml` files as an *uncommitted* change. Before every `git add`, stage only the files the task names — never `git add -A`. Check `git status` and leave the manifests unstaged.
- **Latin error messages.** No new user-facing error strings are needed — fraction divide-by-zero reuses the existing `divisio per nullum`. The fraction *names* (`uncia`, `semuncia`, …) are display data in `fraction.rs`, not identifiers, so they are not `latinitas`-linted; they are genuine Latin regardless. If any new error string does become necessary, validate each content word with `latinitas::is_latin` before committing.
- **Branch.** Work stays on `rust-rewrite` (unmerged, per project convention). Commit there; do not merge to `master`.
- **Truncation, not rounding.** Every off-grid result truncates toward zero via the existing `Roman::div_rem` — no rounding, no tie-break. This is the same convention `ni` division already uses.
- **`fraction` is `pub mod`** (like `roman`), so `checker.rs`, `value.rs`, and `interp.rs` can name `crate::fraction::…` and `crate::fraction::{uncia_grain, scripulum_grain}`.

---

## File Structure

- `crates/lingua-serpentis/src/fraction.rs` — **new module.** The fixed-point layer over `Roman`: grain constructors, `from_rational`/`mul`/`div`, the duodecimal `display`, and the fraction-name tables, plus their unit tests. One-way dependency on `roman.rs`; nothing else in Tasks 1–2.
- `crates/lingua-serpentis/src/lib.rs` — add `pub mod fraction;` (Task 1).
- `crates/lingua-serpentis/src/ast.rs` — `Expr::FloatLit` payload becomes `{ numer, den }` (exact rational); `Expr::Float`/`Double` payloads become `Roman` counts (Task 3).
- `crates/lingua-serpentis/src/token.rs` — `TokenKind::Float(f64)` becomes `Frac { numer, den }`; `From<RomanNumber>` arm (Task 3).
- `crates/lingua-serpentis/src/roman.rs` — `RomanNumber::Float(f64)` becomes `Frac { numer, den }`; `parse_roman`'s fraction branch builds the rational instead of an `f64` (Task 3). `Roman::to_f64` becomes unused but is retained (pub, harmless).
- `crates/lingua-serpentis/src/lexer.rs` — **no source change** (the `°…` branch already pushes `value.into()`); only its `roman_numbers_become_values` test changes (Task 3).
- `crates/lingua-serpentis/src/parser.rs` — one arm in `primary()` destructures `Frac` into `FloatLit` (Task 3); no test changes (no test asserts on `FloatLit`).
- `crates/lingua-serpentis/src/checker.rs` — `resolve_floatlit` narrows the rational to a grain count; the `FloatLit(_)` pattern becomes `FloatLit { .. }`. Type rules are otherwise payload-blind and unchanged (Task 3).
- `crates/lingua-serpentis/src/value.rs` — `Value::Float`/`Double` payloads become `Roman`; `display` delegates to `fraction::display` (Task 3).
- `crates/lingua-serpentis/src/interp.rs` — float/double eval, unary neg, `eval_binary`, and `cmp` arms compute on grain counts via `fraction::` (Task 3).
- `crates/lingua-serpentis/tests/programs.rs`, `examples/praecisio.liss` — fractional output becomes Latin names (Task 3).
- `docs/superpowers/specs/2026-07-22-nr-double-distinct-type-design.md`, `docs/superpowers/specs/2026-07-23-roman-native-integers-design.md` — amended as superseded (Task 4).

---

## Task 1: The `fraction.rs` engine — arithmetic

**Files:**
- Create: `crates/lingua-serpentis/src/fraction.rs`
- Modify: `crates/lingua-serpentis/src/lib.rs` (add the module declaration)
- Test: `crates/lingua-serpentis/src/fraction.rs` (`#[cfg(test)] mod tests`)

**Interfaces:**
- Consumes: `Roman`, `Roman::from_u64`, `mul`, `div_rem`, `is_zero`, `to_i64` (existing `roman.rs`).
- Produces: `pub fn uncia_grain() -> Roman`, `pub fn scripulum_grain() -> Roman`, `pub fn from_rational(numer: &Roman, den: &Roman, g: &Roman) -> Roman`, `pub fn mul(c1: &Roman, c2: &Roman, g: &Roman) -> Roman`, `pub fn div(c1: &Roman, c2: &Roman, g: &Roman) -> Option<Roman>`.

- [ ] **Step 1: Declare the module**

In `crates/lingua-serpentis/src/lib.rs`, add after `pub mod roman;` (line 2):

```rust
pub mod fraction;
```

- [ ] **Step 2: Write the failing tests**

Create `crates/lingua-serpentis/src/fraction.rs` with only this test module for now (the `impl` follows in Step 4):

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::roman::Roman;

    fn r(n: i64) -> Roman { Roman::from_i64(n) }

    #[test]
    fn from_rational_matches_reference_both_grains() {
        // (numer, den) pairs: exact, off-grid, and negative.
        let cases = [(4i64, 3i64), (11, 2), (5, 4), (25, 24), (9, 8), (1, 7), (-4, 3), (7, -3)];
        for (numer, den) in cases {
            for g in [12i64, 288] {
                let got = from_rational(&r(numer), &r(den), &r(g)).to_i64().unwrap();
                // Rust integer division truncates toward zero — the reference.
                let want = ((numer as i128 * g as i128) / den as i128) as i64;
                assert_eq!(got, want, "from_rational {numer}/{den} g={g}");
            }
        }
    }

    #[test]
    fn mul_matches_reference() {
        let cases = [(16i64, 16i64), (6, 6), (5, 5), (360, 2), (-5, 5)];
        for (c1, c2) in cases {
            for g in [12i64, 288] {
                let got = mul(&r(c1), &r(c2), &r(g)).to_i64().unwrap();
                let want = ((c1 as i128 * c2 as i128) / g as i128) as i64;
                assert_eq!(got, want, "mul {c1}*{c2} g={g}");
            }
        }
    }

    #[test]
    fn div_matches_reference_and_guards_zero() {
        let cases = [(16i64, 4i64), (66, 6), (360, 360), (-16, 6)];
        for (c1, c2) in cases {
            for g in [12i64, 288] {
                let got = div(&r(c1), &r(c2), &r(g)).unwrap().to_i64().unwrap();
                let want = ((c1 as i128 * g as i128) / c2 as i128) as i64;
                assert_eq!(got, want, "div {c1}/{c2} g={g}");
            }
        }
        assert!(div(&r(5), &r(0), &r(12)).is_none(), "zero divisor must be guarded");
    }
}
```

- [ ] **Step 3: Run tests to verify they fail**

Run: `cargo test -p lingua-serpentis fraction::tests`
Expected: FAIL — `cannot find function from_rational` / `mul` / `div` in this scope.

- [ ] **Step 4: Write the implementation**

Prepend to `fraction.rs` (above the test module):

```rust
//! Roman-native fractions: a value is a signed `Roman` count of a fixed grain
//! unit, the grain chosen by the type. `mnr` counts unciae (1/12 of the `as`);
//! `nr` counts scripula (1/288, the finest classical unit). Arithmetic is the
//! integer engine plus one rescaling `div_rem`; off-grid results truncate toward
//! zero, exactly like `ni` division. See the 2026-07-23 fractions design spec.

use crate::roman::Roman;

/// Grain denominator for `mnr`: an `as` is 12 unciae.
pub fn uncia_grain() -> Roman { Roman::from_u64(12) }

/// Grain denominator for `nr`: an `as` is 288 scripula.
pub fn scripulum_grain() -> Roman { Roman::from_u64(288) }

/// Narrow an exact rational `numer/den` onto the grain grid:
/// `count = trunc(numer * g / den)`, truncating toward zero. The caller
/// guarantees `den` is non-zero (the parser rejects a zero denominator).
pub fn from_rational(numer: &Roman, den: &Roman, g: &Roman) -> Roman {
    numer.mul(g).div_rem(den).0
}

/// Multiply two grain counts: `(c1/g)*(c2/g)` back in grain units is
/// `trunc(c1*c2 / g)`.
pub fn mul(c1: &Roman, c2: &Roman, g: &Roman) -> Roman {
    c1.mul(c2).div_rem(g).0
}

/// Divide two grain counts: `(c1/g)/(c2/g)` back in grain units is
/// `trunc(c1*g / c2)`. Returns `None` when the divisor count is zero, so the
/// caller can raise `divisio per nullum`.
pub fn div(c1: &Roman, c2: &Roman, g: &Roman) -> Option<Roman> {
    if c2.is_zero() { return None; }
    Some(c1.mul(g).div_rem(c2).0)
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p lingua-serpentis fraction::tests`
Expected: PASS (three tests). The rest of the crate still compiles (the new module is only referenced by its own tests).

- [ ] **Step 6: Commit**

```bash
git add crates/lingua-serpentis/src/fraction.rs crates/lingua-serpentis/src/lib.rs
git commit -m "feat(fraction): duodecimal fixed-point arithmetic over the Roman engine"
```

---

## Task 2: The `fraction.rs` engine — duodecimal display

**Files:**
- Modify: `crates/lingua-serpentis/src/fraction.rs`
- Test: `crates/lingua-serpentis/src/fraction.rs` (`mod tests`)

**Interfaces:**
- Consumes: `Roman`, `is_zero`, `neg`, `div_rem`, `to_i64`, `Roman::display`, `PartialOrd`/`Ord`, `Roman::zero` (existing); `uncia_grain`, `scripulum_grain` (Task 1).
- Produces: `pub fn display(count: &Roman, g: &Roman) -> String`; private `const TWELFTHS: [&str; 12]`, `const SUBUNCIA: [(i64, &str); 5]`.

- [ ] **Step 1: Write the failing tests**

Add to `fraction.rs`'s `mod tests`:

```rust
    #[test]
    fn display_zero_and_whole() {
        assert_eq!(display(&r(0), &uncia_grain()), "nullus");
        assert_eq!(display(&r(24), &uncia_grain()), "II");      // 24 unciae = 2 as
        assert_eq!(display(&r(576), &scripulum_grain()), "II"); // 576 scripula = 2 as
    }

    #[test]
    fn display_every_twelfth_mnr() {
        let names = ["nullus", "uncia", "sextans", "quadrans", "triens", "quincunx",
                     "semis", "septunx", "bes", "dodrans", "dextans", "deunx", "I"];
        for (k, want) in names.iter().enumerate() {
            assert_eq!(display(&r(k as i64), &uncia_grain()), *want, "k={k} unciae");
        }
    }

    #[test]
    fn display_whole_plus_twelfth() {
        assert_eq!(display(&r(16), &uncia_grain()), "I et triens"); // 12 + 4 unciae
        assert_eq!(display(&r(66), &uncia_grain()), "V et semis");  // 60 + 6 unciae
    }

    #[test]
    fn display_sub_uncia_stacks_nr() {
        assert_eq!(display(&r(12), &scripulum_grain()), "semuncia");          // 1/24
        assert_eq!(display(&r(6), &scripulum_grain()), "sicilicus");          // 1/48
        assert_eq!(display(&r(36), &scripulum_grain()), "uncia et semuncia"); // 1/8
        assert_eq!(display(&r(300), &scripulum_grain()), "I et semuncia");    // 1 + 1/24
        // 1/7 as nr = trunc(288/7) = 41 scripula = 24 + 12 + 4 + 1
        assert_eq!(display(&r(41), &scripulum_grain()),
            "uncia et semuncia et sextula et scripulum");
        // maximal 23-scripula stack = 12 + 6 + 4 + 1
        assert_eq!(display(&r(23), &scripulum_grain()),
            "semuncia et sicilicus et sextula et scripulum");
    }

    #[test]
    fn display_mnr_never_stacks_sub_uncia() {
        // Every mnr value is a whole number of unciae, so it never emits a
        // sub-uncia name — at most "NUMERAL et TWELFTH".
        for k in 0..=200i64 {
            let s = display(&r(k), &uncia_grain());
            for sub in ["semuncia", "sicilicus", "sextula", "dimidia sextula", "scripulum"] {
                assert!(!s.contains(sub), "mnr {k} unexpectedly stacked {sub}: {s}");
            }
        }
    }

    #[test]
    fn display_negative_prefixes_minus() {
        assert_eq!(display(&r(-16), &uncia_grain()), "-I et triens");
        assert_eq!(display(&r(-12), &scripulum_grain()), "-semuncia");
    }

    #[test]
    fn display_handles_unbounded_whole_part() {
        // The count is a Roman, so a whole part beyond i64 still decomposes and
        // renders with a vinculum — no panic, no overflow.
        let big = Roman::from_u64(u64::MAX);
        let s = display(&big, &scripulum_grain());
        assert!(s.contains('\u{0305}'), "huge whole part uses a vinculum: {s}");
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p lingua-serpentis fraction::tests::display`
Expected: FAIL — `cannot find function display` in this scope.

- [ ] **Step 3: Write the implementation**

Add to `fraction.rs` (below the arithmetic functions, above the test module):

```rust
/// Fraction of the `as` for k/12, indexed by k (0..=11); index 0 is unused.
const TWELFTHS: [&str; 12] = [
    "", "uncia", "sextans", "quadrans", "triens", "quincunx",
    "semis", "septunx", "bes", "dodrans", "dextans", "deunx",
];

/// Sub-uncia units in scripula, largest first; greedy uses each at most once
/// (exact and repeat-free for every remainder 0..=23).
const SUBUNCIA: [(i64, &str); 5] = [
    (12, "semuncia"),        // 1/24 as
    (6, "sicilicus"),        // 1/48
    (4, "sextula"),          // 1/72
    (2, "dimidia sextula"),  // 1/144
    (1, "scripulum"),        // 1/288
];

/// Render a grain count as classical Latin fraction names: the whole part as a
/// Roman numeral, the fractional part as its twelfth name plus a greedy
/// sub-uncia stack, joined with " et ". Zero is `nullus`; a zero whole part is
/// omitted; a negative value gets a single leading `-`.
pub fn display(count: &Roman, g: &Roman) -> String {
    if count.is_zero() {
        return "nullus".to_string();
    }
    let negative = *count < Roman::zero();
    let magnitude = if negative { count.neg() } else { count.clone() };

    // whole `as` units and the leftover in grain units (0 <= rem < g).
    let (whole, rem) = magnitude.div_rem(g);
    let scr_per_grain = 288 / g.to_i64().unwrap();               // 24 for mnr, 1 for nr
    let mut scripula = rem.to_i64().unwrap() * scr_per_grain;    // 0..=287

    let mut parts: Vec<String> = Vec::new();
    if !whole.is_zero() {
        parts.push(whole.display());
    }
    let twelfths = (scripula / 24) as usize;                    // 0..=11
    scripula %= 24;                                             // 0..=23 sub-uncia
    if twelfths > 0 {
        parts.push(TWELFTHS[twelfths].to_string());
    }
    for (val, name) in SUBUNCIA {
        if scripula >= val {
            scripula -= val;
            parts.push(name.to_string());
        }
    }

    let body = parts.join(" et ");
    if negative { format!("-{body}") } else { body }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p lingua-serpentis fraction`
Expected: PASS (all Task 1 + Task 2 fraction tests).

- [ ] **Step 5: Commit**

```bash
git add crates/lingua-serpentis/src/fraction.rs
git commit -m "feat(fraction): duodecimal display in classical Latin names"
```

---

## Task 3: Flip the pipeline to Roman-native fractions (with tests & example)

The engine is done and tested. Now thread it through the pipeline, update the tests whose output changes, and rewrite the `praecisio` example — all in one task, because the payload types change together across ast/token/lexer/parser/checker/value/interp and the crate will not compile until every site is updated. The literal stops being an `f64` and becomes an exact rational (`numer/den`); the checker narrows it to a grain count; the runtime value and arithmetic are grain counts; display prints Latin names.

**Why this is one atomic change (unlike the integer flip).** The integer flip could bridge with `Roman::from_i64` because `i64` is lossless. There is **no** lossless float bridge: converting the old `f64` back to a grain count truncates wrongly (`1/3` as `f64` is `0.333…`, and `0.333… × 288 = 95.999…` truncates to **95**, not the correct **96** scripula). So the literal must go rational→count directly, in the same change that flips the runtime. Do all source edits (Steps 1–8) before compiling.

**Files:**
- Modify: `crates/lingua-serpentis/src/ast.rs`
- Modify: `crates/lingua-serpentis/src/token.rs`
- Modify: `crates/lingua-serpentis/src/roman.rs` (`RomanNumber`, `parse_roman`, and the `parse_fraction_literal` test)
- Modify: `crates/lingua-serpentis/src/parser.rs` (`primary()` one arm)
- Modify: `crates/lingua-serpentis/src/checker.rs` (`resolve_floatlit`, the `FloatLit` pattern)
- Modify: `crates/lingua-serpentis/src/value.rs`
- Modify: `crates/lingua-serpentis/src/interp.rs`
- Modify: `crates/lingua-serpentis/src/lexer.rs` (**test only** — `roman_numbers_become_values`)
- Modify: `crates/lingua-serpentis/tests/programs.rs` (fraction expectations)
- Modify: `examples/praecisio.liss` (rewrite around the grain story)

**Interfaces:**
- Consumes: `fraction::{from_rational, mul, div, display, uncia_grain, scripulum_grain}` (Tasks 1–2); `Roman::{add, sub, neg, mul, cmp, clone}` (existing).
- Produces: `Expr::FloatLit { numer: Roman, den: Roman }`, `Expr::Float(Roman)`, `Expr::Double(Roman)`; `TokenKind::Frac { numer: Roman, den: Roman }`; `RomanNumber::Frac { numer: Roman, den: Roman }`; `Value::Float(Roman)`, `Value::Double(Roman)` (counts).

- [ ] **Step 1: `ast.rs` — literal and resolved float payloads**

Change the three `Expr` variants (`Roman` is already imported at the top):

```rust
    Int(Roman),
    FloatLit { numer: Roman, den: Roman }, // abstract fractional literal (exact rational), pre-elaboration
    Float(Roman),  // resolved mnr: count of unciae (1/12)
    Double(Roman), // resolved nr:  count of scripula (1/288)
```

- [ ] **Step 2: `token.rs` — fractional literal token**

Change the `Float` variant and the `From<RomanNumber>` arm:

```rust
    // literals (values already converted)
    Int(Roman),
    Frac { numer: Roman, den: Roman },
```

```rust
impl From<RomanNumber> for TokenKind {
    fn from(n: RomanNumber) -> Self {
        match n {
            RomanNumber::Int(r) => TokenKind::Int(r),
            RomanNumber::Frac { numer, den } => TokenKind::Frac { numer, den },
        }
    }
}
```

- [ ] **Step 3: `roman.rs` — `RomanNumber` and `parse_roman` carry the rational**

Change `RomanNumber`:

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum RomanNumber {
    Int(Roman),
    Frac { numer: Roman, den: Roman },
}
```

In `parse_roman`, replace the fraction branch (the `Some(frac) => { … RomanNumber::Float(…) }` arm) so it builds the exact rational and drops the `to_f64` computation. The combined numerator folds in the whole part: `numer = int_val*den + num`.

```rust
    match parts.next() {
        None => Ok(RomanNumber::Int(int_val)),
        Some(frac) => {
            let mut fp = frac.splitn(2, '/');
            let num_str = fp.next().ok_or("fractio sine numeratore")?;
            let den_str = fp.next().ok_or("fractio sine denominatore")?;
            let num = convert_int_roman(num_str)?;
            let den = convert_int_roman(den_str)?;
            if den.is_zero() {
                return Err("denominator est nullus".into());
            }
            let numer = int_val.mul(&den).add(&num);
            Ok(RomanNumber::Frac { numer, den })
        }
    }
```

Update the `parse_fraction_literal` test (was `°V:I/II` → `RomanNumber::Float(5.5)`); `°V:I/II` = 5 + 1/2 = 11/2:

```rust
    #[test]
    fn parse_fraction_literal() {
        assert_eq!(
            parse_roman("°V:I/II").unwrap(),
            RomanNumber::Frac { numer: Roman::from_u64(11), den: Roman::from_u64(2) }
        );
    }
```

Leave `parse_rejects_malformed_or_zero_fraction` unchanged — `°V:I` (missing `/denominator`) and `°V:I/nullus` (`nullus` is not a valid Roman body, so `convert_int_roman` errors) both still return `Err`. `Roman::to_f64` is now unused by `parse_roman` but stays (pub, no warning); its `to_f64_matches` test stays green.

- [ ] **Step 4: `parser.rs` — build `FloatLit` from `Frac`**

In `primary()`, replace the float arm (was `Some(TokenKind::Float(f)) => { self.advance(); Ok(Expr::FloatLit(f)) }`):

```rust
            Some(TokenKind::Frac { numer, den }) => { self.advance(); Ok(Expr::FloatLit { numer, den }) }
```

(`self.peek().cloned()` yields an owned `TokenKind`, so `numer`/`den` move out cleanly.)

- [ ] **Step 5: `checker.rs` — narrow the rational to a grain count**

Change the `FloatLit` arm of `check_expr` (it only reads the shape, not the payload):

```rust
            Expr::FloatLit { .. } => Ok(InferTy::FloatLit),
```

Rewrite the free function `resolve_floatlit` at the bottom of the file to produce grain counts instead of `f32`/`f64`:

```rust
/// Rewrite every still-abstract fractional literal reachable through negation
/// and arithmetic into a concrete-precision node, narrowing the exact rational
/// onto the type's grain grid (uncia for mnr, scripulum for nr) by truncation.
/// Only ever called on an expression whose synthesized type was `FloatLit`.
fn resolve_floatlit(e: &mut Expr, ty: Type) {
    match e {
        Expr::FloatLit { numer, den } => {
            let (numer, den) = (numer.clone(), den.clone());
            *e = match ty {
                Type::Float => Expr::Float(
                    crate::fraction::from_rational(&numer, &den, &crate::fraction::uncia_grain())),
                Type::Double => Expr::Double(
                    crate::fraction::from_rational(&numer, &den, &crate::fraction::scripulum_grain())),
                _ => return,
            };
        }
        Expr::Unary { expr, .. } => resolve_floatlit(expr, ty),
        Expr::Binary { left, right, .. } => {
            resolve_floatlit(left, ty.clone());
            resolve_floatlit(right, ty);
        }
        _ => {}
    }
}
```

Every other checker arm is payload-blind (`Expr::Float(_)`/`Expr::Double(_)` still match a `Roman` payload; the `Add`/`Sub`/`Mul`/`Div`/`Eq`/`Gt` type rules read `InferTy`, not values) and needs no change. In particular the `BinOp::Mod` arm already accepts only `(Int, Int)`, so `%` on a fraction is **already** a type error — the spec's "no fraction modulo" decision needs no code change here; the test in Step 10 pins it.

- [ ] **Step 6: `value.rs` — Roman-count payloads and Latin display**

```rust
use crate::roman::Roman;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Int(Roman),
    Float(Roman),
    Double(Roman),
    Text(String),
    Char(char),
    Bool(bool),
    List(Vec<Value>),
}

impl Value {
    pub fn display(&self) -> String {
        match self {
            Value::Int(r) => r.display(),
            Value::Float(c) => crate::fraction::display(c, &crate::fraction::uncia_grain()),
            Value::Double(c) => crate::fraction::display(c, &crate::fraction::scripulum_grain()),
            Value::Text(s) => s.clone(),
            Value::Char(c) => c.to_string(),
            Value::Bool(b) => (if *b { "verus" } else { "falsus" }).to_string(),
            Value::List(items) => {
                let inner: Vec<String> = items.iter().map(|v| v.display()).collect();
                format!("[{}]", inner.join(", "))
            }
        }
    }
}
```

- [ ] **Step 7: `interp.rs` — compute on grain counts**

Add `use crate::fraction;` near the top (beside `use crate::roman::Roman;`). Then:

The three literal-eval arms (was `Expr::FloatLit(f)` / `Expr::Float(f)` / `Expr::Double(f)`):

```rust
            Expr::FloatLit { numer, den } => {
                // defensive: an unelaborated literal (checker skipped) defaults to nr
                Ok(Value::Double(fraction::from_rational(numer, den, &fraction::scripulum_grain())))
            }
            Expr::Float(c) => Ok(Value::Float(c.clone())),
            Expr::Double(c) => Ok(Value::Double(c.clone())),
```

The unary-negate float arms:

```rust
                    (UnOp::Neg, Value::Float(c)) => Ok(Value::Float(c.neg())),
                    (UnOp::Neg, Value::Double(c)) => Ok(Value::Double(c.neg())),
```

In `eval_binary`, the `Float`/`Double` arms of Add/Sub/Mul/Div:

```rust
        BinOp::Add => match (l, r) {
            (Int(a), Int(b)) => Ok(Int(a.add(&b))),
            (Float(a), Float(b)) => Ok(Float(a.add(&b))),
            (Double(a), Double(b)) => Ok(Double(a.add(&b))),
            (Text(a), Text(b)) => Ok(Text(a + &b)),
            _ => Err(bad()),
        },
        BinOp::Sub => match (l, r) {
            (Int(a), Int(b)) => Ok(Int(a.sub(&b))),
            (Float(a), Float(b)) => Ok(Float(a.sub(&b))),
            (Double(a), Double(b)) => Ok(Double(a.sub(&b))),
            _ => Err(bad()),
        },
        BinOp::Mul => match (l, r) {
            (Int(a), Int(b)) => Ok(Int(a.mul(&b))),
            (Float(a), Float(b)) => Ok(Float(fraction::mul(&a, &b, &fraction::uncia_grain()))),
            (Double(a), Double(b)) => Ok(Double(fraction::mul(&a, &b, &fraction::scripulum_grain()))),
            _ => Err(bad()),
        },
        BinOp::Div => match (l, r) {
            (Int(_), Int(ref b)) if b.is_zero() => Err(LinguaError { message: "divisio per nullum".into(), line }),
            (Int(a), Int(b)) => Ok(Int(a.div_rem(&b).0)),
            (Float(a), Float(b)) => match fraction::div(&a, &b, &fraction::uncia_grain()) {
                Some(q) => Ok(Float(q)),
                None => Err(LinguaError { message: "divisio per nullum".into(), line }),
            },
            (Double(a), Double(b)) => match fraction::div(&a, &b, &fraction::scripulum_grain()) {
                Some(q) => Ok(Double(q)),
                None => Err(LinguaError { message: "divisio per nullum".into(), line }),
            },
            _ => Err(bad()),
        },
```

Leave `BinOp::Mod` as-is (only `Int` arms) — fractions have no modulo.

In `cmp`, replace the `Float`/`Double` arms (drop the NaN handling — grain counts are totally ordered):

```rust
        (Int(a), Int(b)) => a.cmp(&b),
        (Float(a), Float(b)) => a.cmp(&b),
        (Double(a), Double(b)) => a.cmp(&b),
        (Text(a), Text(b)) => a.cmp(&b),
```

(The `line` parameter is still used by `cmp`'s final `_ => return Err(… line)` arm; keep the signature.)

- [ ] **Step 8: `lexer.rs` — update the one affected test**

`mod tests`, `roman_numbers_become_values` — the `°V:I/II` line (was `TokenKind::Float(5.5)`):

```rust
        assert_eq!(kinds("°XXV"), vec![TokenKind::Int(Roman::from_u64(25))]);
        assert_eq!(kinds("°V:I/II"), vec![TokenKind::Frac { numer: Roman::from_u64(11), den: Roman::from_u64(2) }]);
        assert_eq!(kinds("nullus"), vec![TokenKind::Int(Roman::zero())]);
```

(The test module already imports `Roman` via `Roman::from_u64`.)

- [ ] **Step 9: Rewrite `examples/praecisio.liss`**

Replace the whole file (the old version demonstrated f32/f64 decimal digits, which no longer exist):

```
// Typi fluitantes distincti: mnr (granum unciae, 1/12) et nr (granum scripuli, 1/288).
// Eadem fractio Romana, granum tamen diversum.

// °I:I/III est unus plus tertia pars (1 + 1/3 = 4/12, in utroque grano exacta).
mnr tertia_brevis = °I:I/III;   // granum unciae
nr tertia_longa = °I:I/III;     // granum scripuli
scribe(tertia_brevis);          // I et triens
scribe(tertia_longa);           // I et triens

// °I:I/XXIV (1 + 1/24) cadit infra granum unciae: mnr id abicit, nr servat.
mnr minor = °I:I/XXIV;          // 1/24 infra unciam -> truncatur
nr maior = °I:I/XXIV;           // 1/24 = semuncia, exacte
scribe(minor);                  // I
scribe(maior);                  // I et semuncia

// Munus quod nr accipit et reddit; litterae fractae typum contextus assumunt.
munus iunge capit (nr x, nr y) reddit nr {
    redde x + y;
}
nr summa = iunge(°I:I/IV, °I:I/IV);   // 1¼ + 1¼ = 2½
scribe(summa);                          // II et semis

// Litterae fractae sine typo declarato ad nr recidunt.
scribe(°I:I/III);                       // I et triens
```

- [ ] **Step 10: Update the fraction integration tests in `tests/programs.rs`**

Update `praecisio_example_runs` (the six `scribe` lines above, in order):

```rust
#[test]
fn praecisio_example_runs() {
    // mnr grains at the uncia (1/12), nr at the scripulum (1/288); an
    // unconstrained fractional literal defaults to nr.
    let src = include_str!("../../../examples/praecisio.liss");
    assert_eq!(
        run(src).unwrap(),
        "I et triens\nI et triens\nI\nI et semuncia\nII et semis\nI et triens\n"
    );
}
```

Replace `mnr_is_single_precision` and `nr_is_double_precision` with three tests pinning the grain behaviour (divergence and the shared exact case):

```rust
#[test]
fn mnr_truncates_below_its_uncia_grain() {
    // 1/24 is below mnr's uncia grid, so it is dropped — only the whole prints.
    assert_eq!(run("mnr x = °I:I/XXIV; scribe(x);").unwrap(), "I\n");
}

#[test]
fn nr_keeps_the_sub_uncia_part() {
    // nr's scripulum grid holds 1/24 exactly: it is a semuncia.
    assert_eq!(run("nr y = °I:I/XXIV; scribe(y);").unwrap(), "I et semuncia\n");
}

#[test]
fn exact_third_prints_same_under_both() {
    // 1/3 = 4/12 is exact on both grids, so mnr and nr agree here.
    assert_eq!(run("mnr x = °I:I/III; scribe(x);").unwrap(), "I et triens\n");
    assert_eq!(run("nr y = °I:I/III; scribe(y);").unwrap(), "I et triens\n");
}
```

Update `representable_fraction_prints_same_under_both` (5 + 1/2 = `V et semis`, exact in both):

```rust
#[test]
fn representable_fraction_prints_same_under_both() {
    assert_eq!(run("mnr x = °V:I/II; scribe(x);").unwrap(), "V et semis\n");
    assert_eq!(run("nr y = °V:I/II; scribe(y);").unwrap(), "V et semis\n");
}
```

Leave `mixing_float_types_is_a_type_error` unchanged (still a type error).

Update `nr_arithmetic_and_comparison_run` (1¼ + 1¼ = 2½ = `II et semis`; 1½ > 1¼ still `verus`):

```rust
#[test]
fn nr_arithmetic_and_comparison_run() {
    assert_eq!(run("nr x = °I:I/IV + °I:I/IV; scribe(x);").unwrap(), "II et semis\n");
    assert_eq!(run("scribe(°I:I/II maior_est °I:I/IV);").unwrap(), "verus\n");
}
```

Rename and update `unconstrained_fraction_prints_at_nr_precision`:

```rust
#[test]
fn unconstrained_fraction_defaults_to_nr() {
    // A bare fractional literal in an untyped sink defaults to nr.
    assert_eq!(run("scribe(°I:I/III);").unwrap(), "I et triens\n");
}
```

Add two new tests — one pinning the runtime divide-by-zero (now an error, not `inf`), one pinning the "no fraction modulo" decision:

```rust
#[test]
fn fraction_divide_by_zero_errors() {
    // (½) / (½ − ½): a runtime zero divisor is an error, not inf.
    assert!(run("nr z = °I:I/II - °I:I/II; scribe(°I:I/II / z);").is_err());
}

#[test]
fn fraction_modulo_is_a_type_error() {
    assert!(run("nr x = °I:I/II % °I:I/IV;").is_err());
}
```

- [ ] **Step 11: Compile and run the whole crate — fully green**

Run: `cargo test -p lingua-serpentis`
Expected: PASS — the whole suite (fraction + roman engine unit tests, interpreter, checker, parser, lexer, and integration) is green, fractions printing Latin names throughout. Notes:
- The `checker` tests are payload-blind (`is_ok`/`is_err`); the `interp` `mod tests` use no fractional literals; the `parser` tests assert no `FloatLit` — all stay green with no edits.
- If the compiler flags any remaining `Value::Float(f32)` / `Expr::Double(f64)` / `TokenKind::Float` construction elsewhere (an overlooked match), convert it to the `Roman`/rational form shown above.
- If `praecisio_example_runs` mismatches, rerun with `-- --nocapture` and confirm the actual six lines equal the expected block.

- [ ] **Step 12: Commit**

```bash
git add crates/lingua-serpentis/src/ast.rs crates/lingua-serpentis/src/token.rs crates/lingua-serpentis/src/roman.rs crates/lingua-serpentis/src/parser.rs crates/lingua-serpentis/src/checker.rs crates/lingua-serpentis/src/value.rs crates/lingua-serpentis/src/interp.rs crates/lingua-serpentis/src/lexer.rs crates/lingua-serpentis/tests/programs.rs examples/praecisio.liss
git commit -m "feat: mnr/nr compute and print as Roman duodecimal fractions"
```

---

## Task 4: Amend the superseded design docs

The code is green; this task only refreshes the two design docs whose "fractions stay IEEE / print decimal" statements are now superseded. No code changes.

**Files:**
- Modify: `docs/superpowers/specs/2026-07-22-nr-double-distinct-type-design.md`
- Modify: `docs/superpowers/specs/2026-07-23-roman-native-integers-design.md`
- Possibly modify: a top-level `README*` (only if it makes a superseded claim — see Step 2).

- [ ] **Step 1: Amend the two prior specs**

At the top of `docs/superpowers/specs/2026-07-22-nr-double-distinct-type-design.md`, immediately after its metadata/header block, add:

```markdown
> **Amended 2026-07-23:** `mnr`/`nr` are no longer IEEE `f32`/`f64`. They are now
> Roman-native duodecimal fixed-point (uncia / scripulum grain) and print in Latin
> fraction names — see `2026-07-23-roman-native-fractions-design.md`. Passages
> below describing binary floating-point precision (e.g. `1.3333334`) and the
> f32/f64 backing are superseded; the `mnr`/`nr` type distinction itself stands.
```

In `docs/superpowers/specs/2026-07-23-roman-native-integers-design.md`, its §2 Non-Goals states that `mnr`/`nr` stay IEEE and print decimals ("a whole second engine and is deferred"). Add, right after that Non-Goal bullet's paragraph:

```markdown
> **Superseded 2026-07-23:** that deferred second engine now exists —
> `2026-07-23-roman-native-fractions-design.md`. Fractions are Roman-native too;
> the "integers Roman, fractions decimal" mixed look described here no longer
> applies.
```

- [ ] **Step 2: Check the README**

Grep the repo's `README*` (if any) for decimal fraction output (e.g. `1.333`) or claims that `mnr`/`nr` are `f32`/`f64`/IEEE. If found, update to the Latin-name output and the duodecimal description; if there is no README or it makes no such claim, make no change (do not create one).

- [ ] **Step 3: Commit**

```bash
git add docs/superpowers/specs/2026-07-22-nr-double-distinct-type-design.md docs/superpowers/specs/2026-07-23-roman-native-integers-design.md
git commit -m "docs: mark mnr/nr IEEE passages superseded by the fraction engine"
```

(Add `README*` to the `git add` line only if Step 2 changed it.)

---

## Notes for the implementer

- **The engine is the foundation.** Tasks 1–2 are pure `fraction.rs` and never touch the rest of the crate; the suite stays green throughout. Do not start Task 3 until the fraction engine tests pass — the parity tests are what guarantee the arithmetic matches the truncating rational reference.
- **Task 3's source flip is atomic.** The payload types thread through the whole pipeline, so the crate will not compile between Steps 1 and 8; write all eight source edits, then compile. There is no lossless float bridge (see the box in Task 3), which is exactly why it can't be split the way the integer flip was. Steps 9–10 then move the example and integration expectations, so the task ends fully green.
- **The grain is chosen at the call site.** `Value::Float`/`Expr::Float` ⇒ `uncia_grain()`; `Value::Double`/`Expr::Double` ⇒ `scripulum_grain()`. The stored `Roman` is a bare count — the grain is never stored, always supplied. Keep the two grains straight; swapping them silently changes every fraction's value.
- **Sub-1 fractions can't be written as literals.** `is_roman_body` (in `lexer.rs`) admits only `I V X L C D M . : /`, so a `°…` literal's integer part is never empty or `nullus`; write `1 + 1/24` as `°I:I/XXIV`. Sub-1 *values* still arise from arithmetic and display correctly (zero whole part omitted). This is a known, documented limitation (spec §2).
- **`Roman::to_f64` is now orphaned** but retained (pub, so no dead-code warning); its unit test stays green. Removing it and its test is a valid but optional cleanup — out of scope here.
- **Don't `git add -A`.** Stage only each task's named files so the `Cargo.toml` edition-drift never rides along (Global Constraints).
```
