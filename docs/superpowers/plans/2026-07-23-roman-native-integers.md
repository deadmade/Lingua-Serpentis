# Roman-Native Integers (`ni`) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the `i64` backing of the `ni` integer type with a Roman-native value that computes and prints as a numeral, so the language is Roman end-to-end.

**Architecture:** Build a self-contained `Roman` type in `roman.rs` first (a sign plus a little-endian `Vec` of decimal `(units, five)` tiers, arithmetic by carry/borrow), fully unit-tested in isolation (Tasks 1–4). Then flip the runtime value over to it (Task 5), then push it up into the literal/AST layer so parsing builds numerals through the engine (Task 6), then update integration tests, examples, and docs (Task 7). Each task keeps the whole crate compiling and green.

**Tech Stack:** Rust (2021 edition), no third-party dependencies, hand-written lexer/parser/checker/interpreter. Spec: `docs/superpowers/specs/2026-07-23-roman-native-integers-design.md`.

## Global Constraints

- **No third-party crates.** The interpreter and `latinitas` depend on nothing but `std`. Do not add dependencies.
- **Build/test command (this machine).** `cargo` is not on `PATH` and MSVC `link.exe` needs importing first. Run every `cargo` command through this wrapper (from the repo root):

```powershell
cmd /c '"C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat" && set PATH=%USERPROFILE%\.cargo\bin;%PATH% && cargo test -p lingua-serpentis'
```

  Task steps below write the bare `cargo test -p lingua-serpentis <filter>`; wrap it as above when actually running. A bare `cargo test` also covers both crates (`default-members` was removed).
- **Manifest hygiene.** Something in this environment rewrites `edition = "2021"` → `"2024"` and `resolver = "2"` → `"3"` in the three `Cargo.toml` files as an *uncommitted* change. Before every `git add`, stage only the files the task names — never `git add -A`. Check `git status` and leave the manifests unstaged.
- **Latin error messages.** No new user-facing error strings are expected (§5 of the spec). If one becomes necessary, validate each content word with `latinitas::is_latin` before committing.
- **Branch.** Work stays on `rust-rewrite` (unmerged, per project convention). Commit there; do not merge to `master`.
- **`Roman` is `pub`** in the `roman` module (consistent with the existing `pub` `RomanNumber`), so `value.rs`, `token.rs`, `ast.rs`, and `interp.rs` can name `crate::roman::Roman`.

---

## File Structure

- `crates/lingua-serpentis/src/roman.rs` — **grows into the engine.** Adds the `Roman` type, `Tier`, canonical-form helpers, constructors, conversions, arithmetic, comparison, and display, plus their unit tests, alongside the existing `parse_roman`/`convert_*`/`RomanNumber`. If it becomes hard to hold in view later, split into a `roman/` module (out of scope for this plan).
- `crates/lingua-serpentis/src/value.rs` — `Value::Int` payload becomes `Roman`; `display` delegates.
- `crates/lingua-serpentis/src/interp.rs` — arithmetic/compare/neg/index/`longitudo`/default-return use `Roman`; the `i64`↔`Roman` boundary lives here (list indices, lengths).
- `crates/lingua-serpentis/src/token.rs`, `ast.rs`, `lexer.rs` — literal payload becomes `Roman` (Task 6).
- `crates/lingua-serpentis/src/parser.rs` — **no source change** (its `peek().cloned()` already owns the payload); only its unit-test literals change (Task 6).
- `crates/lingua-serpentis/src/checker.rs` — **no change** (its `Expr::Int(_)` arm is payload-blind).
- `crates/lingua-serpentis/tests/programs.rs`, `examples/series.liss` — expected output becomes Roman (Task 7).

---

## Task 1: The `Roman` value type — construction & conversion

**Files:**
- Modify: `crates/lingua-serpentis/src/roman.rs` (prepend the new type above the existing `convert_single`; keep everything already there untouched)
- Test: `crates/lingua-serpentis/src/roman.rs` (its `#[cfg(test)] mod tests`)

**Interfaces:**
- Produces: `pub struct Roman` (private fields `negative: bool`, `tiers: Vec<Tier>`); `Roman::zero()`, `Roman::from_u64(u64)`, `Roman::from_i64(i64)`, `Roman::from_usize(usize)`, `Roman::is_zero(&self) -> bool`, `Roman::to_i64(&self) -> Option<i64>`, `Roman::to_usize(&self) -> Option<usize>`, `Roman::to_f64(&self) -> f64`; `impl From<i64> for Roman`. Private helpers `Tier`, `Tier::digit`, `Tier::from_digit`, `trim`, `make` (all used by later tasks).

- [ ] **Step 1: Write the failing tests**

Add to `roman.rs`'s `mod tests`:

```rust
    fn r(n: i64) -> Roman { Roman::from_i64(n) }

    #[test]
    fn roman_value_roundtrips_through_i64() {
        for n in [0i64, 1, 4, 9, 25, 40, 91, 3999, 4000, 30532, -7, -3999, i64::MAX] {
            assert_eq!(r(n).to_i64(), Some(n), "n={n}");
        }
    }

    #[test]
    fn zero_is_canonical_and_never_negative() {
        assert!(Roman::zero().is_zero());
        assert_eq!(Roman::zero(), r(0));
        assert_eq!(r(-0), Roman::zero());
    }

    #[test]
    fn to_usize_rejects_negative_accepts_len() {
        assert_eq!(r(-1).to_usize(), None);
        assert_eq!(Roman::from_usize(5).to_usize(), Some(5));
    }

    #[test]
    fn to_f64_matches() {
        assert_eq!(r(30532).to_f64(), 30532.0);
        assert_eq!(r(-7).to_f64(), -7.0);
    }

    #[test]
    fn magnitude_beyond_i64_has_no_i64() {
        assert_eq!(Roman::from_u64(u64::MAX).to_i64(), None);
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p lingua-serpentis roman::tests`
Expected: FAIL — `cannot find type Roman` / `Roman::zero` not found.

- [ ] **Step 3: Write the implementation**

Prepend to `roman.rs` (above `convert_single`):

```rust
/// A Roman-native integer: a sign plus a little-endian bag of decimal tiers.
/// Canonical invariant: every tier has `units <= 4` and `five <= 1`, there are
/// no trailing all-zero tiers, and the empty `tiers` vector is the unique zero
/// (never negative). Structural equality is therefore numeric equality.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Roman {
    negative: bool,
    tiers: Vec<Tier>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Tier {
    units: u8, // 0..=4
    five: u8,  // 0..=1
}

impl Tier {
    fn digit(self) -> u8 { self.five * 5 + self.units } // 0..=9
    fn from_digit(d: u8) -> Tier { Tier { units: d % 5, five: d / 5 } } // d in 0..=9
}

/// Drop trailing all-zero tiers so the canonical form is unique.
fn trim(mut tiers: Vec<Tier>) -> Vec<Tier> {
    while matches!(tiers.last(), Some(t) if t.digit() == 0) {
        tiers.pop();
    }
    tiers
}

/// Build a canonical `Roman` from a sign and carry-normalized tiers; forces zero
/// to be non-negative.
fn make(negative: bool, tiers: Vec<Tier>) -> Roman {
    let tiers = trim(tiers);
    Roman { negative: negative && !tiers.is_empty(), tiers }
}

impl Roman {
    pub fn zero() -> Roman {
        Roman { negative: false, tiers: Vec::new() }
    }

    pub fn from_u64(mut n: u64) -> Roman {
        let mut tiers = Vec::new();
        while n > 0 {
            tiers.push(Tier::from_digit((n % 10) as u8));
            n /= 10;
        }
        Roman { negative: false, tiers } // no trailing zeros by construction
    }

    pub fn from_i64(n: i64) -> Roman {
        let mut val = Roman::from_u64(n.unsigned_abs());
        val.negative = n < 0 && !val.tiers.is_empty();
        val
    }

    pub fn from_usize(n: usize) -> Roman {
        Roman::from_u64(n as u64)
    }

    pub fn is_zero(&self) -> bool {
        self.tiers.is_empty()
    }

    pub fn to_i64(&self) -> Option<i64> {
        let mut acc: i128 = 0;
        for t in self.tiers.iter().rev() {
            acc = acc * 10 + t.digit() as i128;
            if acc > i64::MAX as i128 + 1 { return None; } // allow magnitude of i64::MIN
        }
        if self.negative { acc = -acc; }
        i64::try_from(acc).ok()
    }

    pub fn to_usize(&self) -> Option<usize> {
        if self.negative { return None; }
        let mut acc: u128 = 0;
        for t in self.tiers.iter().rev() {
            acc = acc.checked_mul(10)?.checked_add(t.digit() as u128)?;
        }
        usize::try_from(acc).ok()
    }

    pub fn to_f64(&self) -> f64 {
        let mut acc = 0.0f64;
        for (k, t) in self.tiers.iter().enumerate() {
            acc += t.digit() as f64 * 10f64.powi(k as i32);
        }
        if self.negative { -acc } else { acc }
    }
}

impl From<i64> for Roman {
    fn from(n: i64) -> Roman { Roman::from_i64(n) }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p lingua-serpentis roman::tests`
Expected: PASS (all five new tests, plus the existing `roman.rs` tests still green).

- [ ] **Step 5: Commit**

```bash
git add crates/lingua-serpentis/src/roman.rs
git commit -m "feat(roman): Roman value type with constructors and conversions"
```

---

## Task 2: Vinculum display

**Files:**
- Modify: `crates/lingua-serpentis/src/roman.rs`
- Test: `crates/lingua-serpentis/src/roman.rs` (`mod tests`)

**Interfaces:**
- Consumes: `Roman`, `Tier::digit`, `is_zero` (Task 1).
- Produces: `Roman::display(&self) -> String`; private `glyph_one`, `glyph_five`, `render_digit`, `with_bars`.

- [ ] **Step 1: Write the failing tests**

```rust
    #[test]
    fn display_basic_and_subtractive() {
        assert_eq!(r(1).display(), "I");
        assert_eq!(r(4).display(), "IV");
        assert_eq!(r(9).display(), "IX");
        assert_eq!(r(40).display(), "XL");
        assert_eq!(r(90).display(), "XC");
        assert_eq!(r(91).display(), "XCI");
        assert_eq!(r(2024).display(), "MMXXIV");
        assert_eq!(r(3999).display(), "MMMCMXCIX");
        // 30532 = 30000 + 500 + 30 + 2 → X̄X̄X̄ D XXX II
        assert_eq!(r(30532).display(), "X\u{0305}X\u{0305}X\u{0305}DXXXII");
    }

    #[test]
    fn display_zero_and_negative() {
        assert_eq!(Roman::zero().display(), "nullus");
        assert_eq!(r(-25).display(), "-XXV");
    }

    #[test]
    fn display_vinculum_thousands() {
        assert_eq!(r(4000).display(), "MV\u{0305}");  // M V̄  (5000 - 1000)
        assert_eq!(r(5000).display(), "V\u{0305}");   // V̄
        assert_eq!(r(10000).display(), "X\u{0305}");  // X̄
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p lingua-serpentis roman::tests::display`
Expected: FAIL — no method `display` on `Roman`.

- [ ] **Step 3: Write the implementation**

Add to `roman.rs` (near the `Roman` impl):

```rust
const ONES3: [&str; 3] = ["X", "C", "M"];
const FIVES3: [&str; 3] = ["V", "L", "D"];

fn with_bars(base: &str, bars: usize) -> String {
    let mut s = base.to_string();
    for _ in 0..bars { s.push('\u{0305}'); } // U+0305 COMBINING OVERLINE
    s
}

/// The "one" glyph for a tier: I, X, C, M, X̄, C̄, M̄, X̿, … (bar = ×1000).
fn glyph_one(tier: usize) -> String {
    if tier == 0 { return "I".to_string(); }
    with_bars(ONES3[(tier - 1) % 3], (tier - 1) / 3)
}

/// The "five" glyph for a tier: V, L, D, V̄, L̄, D̄, V̿, …
fn glyph_five(tier: usize) -> String {
    with_bars(FIVES3[tier % 3], tier / 3)
}

/// Render one tier's digit (0..=9) with the classic subtractive rules, using
/// this tier's one/five glyphs plus the next tier's one glyph for the 9 case.
fn render_digit(d: u8, tier: usize) -> String {
    let one = glyph_one(tier);
    let five = glyph_five(tier);
    match d {
        0 => String::new(),
        1..=3 => one.repeat(d as usize),
        4 => format!("{one}{five}"),
        5 => five,
        6..=8 => format!("{five}{}", one.repeat((d - 5) as usize)),
        9 => format!("{one}{}", glyph_one(tier + 1)),
        _ => unreachable!("tier digit out of range"),
    }
}

impl Roman {
    pub fn display(&self) -> String {
        if self.is_zero() { return "nullus".to_string(); }
        let mut s = String::new();
        if self.negative { s.push('-'); }
        for k in (0..self.tiers.len()).rev() {
            s.push_str(&render_digit(self.tiers[k].digit(), k));
        }
        s
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p lingua-serpentis roman::tests::display`
Expected: PASS (three new tests).

- [ ] **Step 5: Commit**

```bash
git add crates/lingua-serpentis/src/roman.rs
git commit -m "feat(roman): vinculum display for numerals"
```

---

## Task 3: Addition, subtraction, negation, ordering

**Files:**
- Modify: `crates/lingua-serpentis/src/roman.rs`
- Test: `crates/lingua-serpentis/src/roman.rs` (`mod tests`)

**Interfaces:**
- Consumes: `Roman`, `Tier`, `make`, `trim` (Task 1).
- Produces: `Roman::neg(&self) -> Roman`, `Roman::add(&self, &Roman) -> Roman`, `Roman::sub(&self, &Roman) -> Roman`; `impl Ord`/`PartialOrd for Roman`; private magnitude helpers `mag_cmp(&[Tier], &[Tier]) -> Ordering`, `mag_add(&[Tier], &[Tier]) -> Vec<Tier>`, `mag_sub(&[Tier], &[Tier]) -> Vec<Tier>` (used by Task 4).

- [ ] **Step 1: Write the failing tests**

```rust
    #[test]
    fn add_sub_match_i64() {
        let cases = [(0, 0), (2, 3), (4, 1), (49, 1), (3999, 1), (30507, 25),
                     (1234, 8766), (-3, 5), (3, -5), (-7, -8), (5, -5)];
        for (a, b) in cases {
            assert_eq!(r(a).add(&r(b)).to_i64(), Some(a + b), "{a}+{b}");
            assert_eq!(r(a).sub(&r(b)).to_i64(), Some(a - b), "{a}-{b}");
        }
    }

    #[test]
    fn carry_and_borrow_rollover() {
        assert_eq!(r(3999).add(&r(1)).display(), "MV\u{0305}"); // 4000
        assert_eq!(r(4000).sub(&r(1)).display(), "MMMCMXCIX");  // 3999
        assert_eq!(r(50).sub(&r(1)).display(), "XLIX");         // 49
    }

    #[test]
    fn neg_and_zero() {
        assert_eq!(r(7).neg().to_i64(), Some(-7));
        assert_eq!(r(-7).neg().to_i64(), Some(7));
        assert!(r(5).sub(&r(5)).is_zero());
        assert_eq!(Roman::zero().neg(), Roman::zero());
    }

    #[test]
    fn ordering_across_signs() {
        assert!(r(-1) < r(0));
        assert!(r(0) < r(1));
        assert!(r(-5) < r(-3));
        assert!(r(3999) < r(4000));
        assert!(r(100) > r(99));
        assert_eq!(r(42).cmp(&r(42)), std::cmp::Ordering::Equal);
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p lingua-serpentis roman::tests`
Expected: FAIL — no method `add`/`sub`/`neg`, `Roman` is not `Ord`.

- [ ] **Step 3: Write the implementation**

Add to `roman.rs`:

```rust
/// Compare two magnitudes (tiers only, sign ignored). Canonical form has no
/// trailing zero tiers, so the longer vector is strictly larger.
fn mag_cmp(a: &[Tier], b: &[Tier]) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    if a.len() != b.len() { return a.len().cmp(&b.len()); }
    for k in (0..a.len()).rev() {
        let c = a[k].digit().cmp(&b[k].digit());
        if c != Ordering::Equal { return c; }
    }
    Ordering::Equal
}

/// Magnitude add in base-10 digit space (per-tier carrying).
fn mag_add(a: &[Tier], b: &[Tier]) -> Vec<Tier> {
    let n = a.len().max(b.len());
    let mut out = Vec::with_capacity(n + 1);
    let mut carry = 0u8;
    for k in 0..n {
        let da = a.get(k).map_or(0, |t| t.digit());
        let db = b.get(k).map_or(0, |t| t.digit());
        let s = da + db + carry; // <= 9 + 9 + 1 = 19
        out.push(Tier::from_digit(s % 10));
        carry = s / 10;
    }
    if carry > 0 { out.push(Tier::from_digit(carry)); }
    trim(out)
}

/// Magnitude subtract in base-10 digit space; requires magnitude a >= b.
fn mag_sub(a: &[Tier], b: &[Tier]) -> Vec<Tier> {
    let mut out = Vec::with_capacity(a.len());
    let mut borrow = 0i16;
    for k in 0..a.len() {
        let da = a[k].digit() as i16;
        let db = b.get(k).map_or(0, |t| t.digit() as i16);
        let mut d = da - db - borrow;
        if d < 0 { d += 10; borrow = 1; } else { borrow = 0; }
        out.push(Tier::from_digit(d as u8));
    }
    trim(out) // borrow ends at 0 because a >= b
}

impl Roman {
    pub fn neg(&self) -> Roman {
        make(!self.negative, self.tiers.clone())
    }

    pub fn add(&self, other: &Roman) -> Roman {
        use std::cmp::Ordering;
        if self.negative == other.negative {
            make(self.negative, mag_add(&self.tiers, &other.tiers))
        } else {
            match mag_cmp(&self.tiers, &other.tiers) {
                Ordering::Greater => make(self.negative, mag_sub(&self.tiers, &other.tiers)),
                Ordering::Less => make(other.negative, mag_sub(&other.tiers, &self.tiers)),
                Ordering::Equal => Roman::zero(),
            }
        }
    }

    pub fn sub(&self, other: &Roman) -> Roman {
        self.add(&other.neg())
    }
}

impl PartialOrd for Roman {
    fn partial_cmp(&self, other: &Roman) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Roman {
    fn cmp(&self, other: &Roman) -> std::cmp::Ordering {
        use std::cmp::Ordering;
        match (self.negative, other.negative) {
            (false, true) => Ordering::Greater,
            (true, false) => Ordering::Less,
            (false, false) => mag_cmp(&self.tiers, &other.tiers),
            (true, true) => mag_cmp(&other.tiers, &self.tiers), // larger magnitude is smaller
        }
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p lingua-serpentis roman::tests`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/lingua-serpentis/src/roman.rs
git commit -m "feat(roman): add/sub/neg and total ordering via carry/borrow"
```

---

## Task 4: Multiplication and division

**Files:**
- Modify: `crates/lingua-serpentis/src/roman.rs`
- Test: `crates/lingua-serpentis/src/roman.rs` (`mod tests`)

**Interfaces:**
- Consumes: `mag_add`, `mag_sub`, `mag_cmp`, `make`, `Roman::from_u64` (Tasks 1, 3).
- Produces: `Roman::mul(&self, &Roman) -> Roman`, `Roman::div_rem(&self, &Roman) -> (Roman, Roman)` (caller must guard `!other.is_zero()`); private `mag_halve`, `mag_mul`, `mag_divmod`.

- [ ] **Step 1: Write the failing tests**

```rust
    #[test]
    fn mul_matches_i64() {
        let cases = [(0, 5), (7, 6), (25, 40), (123, 45), (3999, 3999),
                     (-6, 7), (6, -7), (-6, -7), (1000, 1000)];
        for (a, b) in cases {
            assert_eq!(r(a).mul(&r(b)).to_i64(), Some(a * b), "{a}*{b}");
        }
    }

    #[test]
    fn div_rem_matches_i64_truncating() {
        let cases = [(7, 2), (-7, 2), (7, -2), (-7, -2), (30507, 25), (100, 10), (5, 7)];
        for (a, b) in cases {
            let (q, rem) = r(a).div_rem(&r(b));
            assert_eq!(q.to_i64(), Some(a / b), "{a}/{b} quotient");
            assert_eq!(rem.to_i64(), Some(a % b), "{a}%{b} remainder");
        }
    }

    #[test]
    fn multiply_is_unbounded() {
        // 3999^6 ≈ 4.1e21, far beyond i64::MAX; the tier engine must not overflow.
        let base = r(3999);
        let mut acc = r(1);
        for _ in 0..6 { acc = acc.mul(&base); }
        assert!(acc.to_i64().is_none(), "should exceed i64 range");
        assert!(acc > r(0));
        assert!(acc.display().contains('\u{0305}'), "a number this big prints with a vinculum");
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p lingua-serpentis roman::tests`
Expected: FAIL — no method `mul`/`div_rem`.

- [ ] **Step 3: Write the implementation**

Add to `roman.rs`:

```rust
/// Halve a magnitude, returning (quotient, remainder ∈ {0,1}) — long division
/// by two from the most-significant tier down.
fn mag_halve(a: &[Tier]) -> (Vec<Tier>, u8) {
    let mut out = vec![Tier { units: 0, five: 0 }; a.len()];
    let mut rem = 0u8;
    for k in (0..a.len()).rev() {
        let cur = rem * 10 + a[k].digit(); // <= 19
        out[k] = Tier::from_digit(cur / 2);
        rem = cur % 2;
    }
    (trim(out), rem)
}

/// Magnitude multiply via the doubling ladder (Russian-peasant): only `mag_add`
/// (doubling) and `mag_halve`. Never a host multiply of the value.
fn mag_mul(a: &[Tier], b: &[Tier]) -> Vec<Tier> {
    let mut result: Vec<Tier> = Vec::new();
    let mut x = a.to_vec();
    let mut y = b.to_vec();
    while !y.is_empty() {
        let (yh, bit) = mag_halve(&y);
        if bit == 1 { result = mag_add(&result, &x); }
        x = mag_add(&x, &x);
        y = yh;
    }
    result
}

/// Magnitude divide, returning (quotient, remainder). Requires d != 0.
/// Doubling long division, built on `mag_add`/`mag_sub`/`mag_cmp`.
fn mag_divmod(n: &[Tier], d: &[Tier]) -> (Vec<Tier>, Vec<Tier>) {
    use std::cmp::Ordering;
    if mag_cmp(n, d) == Ordering::Less {
        return (Vec::new(), n.to_vec());
    }
    // Largest dd = d * 2^k with dd <= n, tracking pow = 2^k.
    let mut dd = d.to_vec();
    let mut pow = Roman::from_u64(1).tiers;
    loop {
        let next = mag_add(&dd, &dd);
        if mag_cmp(&next, n) != Ordering::Greater {
            dd = next;
            pow = mag_add(&pow, &pow);
        } else {
            break;
        }
    }
    let (q1, r1) = mag_divmod(&mag_sub(n, &dd), d);
    (mag_add(&pow, &q1), r1)
}

impl Roman {
    pub fn mul(&self, other: &Roman) -> Roman {
        make(self.negative != other.negative, mag_mul(&self.tiers, &other.tiers))
    }

    /// Truncating division toward zero; the remainder's sign follows the
    /// dividend — matching Rust's `i64` `/` and `%`. Caller must ensure
    /// `!other.is_zero()`.
    pub fn div_rem(&self, other: &Roman) -> (Roman, Roman) {
        let (q, rem) = mag_divmod(&self.tiers, &other.tiers);
        (make(self.negative != other.negative, q), make(self.negative, rem))
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p lingua-serpentis roman::tests`
Expected: PASS. Also run the whole `roman.rs` suite to confirm nothing regressed: `cargo test -p lingua-serpentis roman`.

- [ ] **Step 5: Commit**

```bash
git add crates/lingua-serpentis/src/roman.rs
git commit -m "feat(roman): unbounded multiply and truncating div_rem"
```

---

## Task 5: Flip the runtime value to `Roman`

The engine is done and tested. Now make the interpreter's integer value a `Roman`. The literal still arrives as `i64` in the AST (Task 6 changes that); convert it to `Roman` at eval time via `Roman::from_i64`. Every place that printed a number now prints Roman, so this task also updates the interpreter's own test expectations and repurposes the overflow test.

**Files:**
- Modify: `crates/lingua-serpentis/src/value.rs`
- Modify: `crates/lingua-serpentis/src/interp.rs` (`eval` `Expr::Int`/`Unary`/`Call`/`Index`; `exec` `IndexAssign`; `call` default return; `eval_binary`; `cmp`)
- Test: `crates/lingua-serpentis/src/interp.rs` (`mod tests`)
- Test: `crates/lingua-serpentis/tests/programs.rs` (numeric expectations — this task changes runtime output to Roman, so the integration tests move with it; Task 6 is behavior-preserving and leaves them green)

**Interfaces:**
- Consumes: `Roman` and its methods (Tasks 1–4).
- Produces: `Value::Int(Roman)`.

- [ ] **Step 1: Update `value.rs`**

Change the variant and its display arm:

```rust
use crate::roman::Roman;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Int(Roman),
    Float(f32),
    Double(f64),
    Text(String),
    Char(char),
    Bool(bool),
    List(Vec<Value>),
}
```

and in `display`:

```rust
            Value::Int(r) => r.display(),
```

- [ ] **Step 2: Update `interp.rs`**

Add `use crate::roman::Roman;` near the top. Then:

Literal eval (`eval`, was `Expr::Int(i) => Ok(Value::Int(*i))`):

```rust
            Expr::Int(i) => Ok(Value::Int(Roman::from_i64(*i))),
```

Unary negate arm:

```rust
                    (UnOp::Neg, Value::Int(r)) => Ok(Value::Int(r.neg())),
```

`longitudo` result:

```rust
                        Some(Value::List(items)) => Ok(Value::Int(Roman::from_usize(items.len()))),
```

Index read (`Expr::Index`), replacing the `Value::Int(idx)` arm:

```rust
                    (Value::List(items), Value::Int(idx)) => {
                        match idx.to_usize() {
                            Some(i) if i >= 1 && i <= items.len() => Ok(items[i - 1].clone()),
                            _ => Err(LinguaError {
                                message: format!("index extra fines: {} (longitudo {})", idx.display(), items.len()),
                                line,
                            }),
                        }
                    }
```

Default "fell off the end" return in `call`:

```rust
            Ok(Value::Int(Roman::zero()))
```

Index write (`exec`, `StmtKind::IndexAssign`) — replace the `let idx = match idx { Value::Int(i) => i, ... }` and the bounds check with:

```rust
                let idx = match idx {
                    Value::Int(r) => r,
                    _ => return Err(LinguaError { message: "index seriei invalidus".into(), line: s.line }),
                };
                for scope in self.scopes.iter_mut().rev() {
                    if let Some(slot) = scope.get_mut(name) {
                        return match slot {
                            Value::List(items) => match idx.to_usize() {
                                Some(i) if i >= 1 && i <= items.len() => {
                                    items[i - 1] = v;
                                    Ok(Flow::Normal)
                                }
                                _ => Err(LinguaError {
                                    message: format!("index extra fines: {} (longitudo {})", idx.display(), items.len()),
                                    line: s.line,
                                }),
                            },
                            _ => Err(LinguaError { message: format!("'{}' non est series", name), line: s.line }),
                        };
                    }
                }
```

`eval_binary` — delete the `overflow` closure and rewrite the five integer arms:

```rust
        BinOp::Add => match (l, r) {
            (Int(a), Int(b)) => Ok(Int(a.add(&b))),
            (Float(a), Float(b)) => Ok(Float(a + b)),
            (Double(a), Double(b)) => Ok(Double(a + b)),
            (Text(a), Text(b)) => Ok(Text(a + &b)),
            _ => Err(bad()),
        },
        BinOp::Sub => match (l, r) {
            (Int(a), Int(b)) => Ok(Int(a.sub(&b))),
            (Float(a), Float(b)) => Ok(Float(a - b)),
            (Double(a), Double(b)) => Ok(Double(a - b)),
            _ => Err(bad()),
        },
        BinOp::Mul => match (l, r) {
            (Int(a), Int(b)) => Ok(Int(a.mul(&b))),
            (Float(a), Float(b)) => Ok(Float(a * b)),
            (Double(a), Double(b)) => Ok(Double(a * b)),
            _ => Err(bad()),
        },
        BinOp::Div => match (l, r) {
            (Int(_), Int(ref b)) if b.is_zero() => Err(LinguaError { message: "divisio per nullum".into(), line }),
            (Int(a), Int(b)) => Ok(Int(a.div_rem(&b).0)),
            (Float(a), Float(b)) => Ok(Float(a / b)),
            (Double(a), Double(b)) => Ok(Double(a / b)),
            _ => Err(bad()),
        },
        BinOp::Mod => match (l, r) {
            (Int(_), Int(ref b)) if b.is_zero() => Err(LinguaError { message: "modulus per nullum".into(), line }),
            (Int(a), Int(b)) => Ok(Int(a.div_rem(&b).1)),
            _ => Err(bad()),
        },
```

Delete the now-unused line `let overflow = || LinguaError { ... };` at the top of `eval_binary`.

`cmp` integer arm:

```rust
        (Int(a), Int(b)) => a.cmp(&b),
```

- [ ] **Step 3: Update the interpreter's own tests**

In `interp.rs`'s `mod tests`, change these expected strings (numbers now print Roman):

| test | old | new |
|---|---|---|
| `arithmetic_and_print` | `"11\n"` | `"XI\n"` |
| `variables` | `"12\n"` | `"XII\n"` |
| `while_loop_counts` | `"10\n"` | `"X\n"` |
| `for_loop_with_break` | `"3\n"` | `"III\n"` |
| `function_call_returns` | `"10\n"` | `"X\n"` |
| `continue_skips_iteration` | `"8\n"` | `"VIII\n"` |
| `list_literal_prints` | `"[1, 2, 3]\n"` | `"[I, II, III]\n"` (the `"[]\n"` case is unchanged) |
| `index_read_is_one_based` | `"10\n"`, `"30\n"` | `"X\n"`, `"XXX\n"` |
| `index_write_mutates_in_place` | `"[1, 10, 3]\n"` | `"[I, X, III]\n"` |
| `longitudo_and_adde_run` | `"3\n"`, `"[1, 2]\n"` | `"III\n"`, `"[I, II]\n"` |
| `nested_list_read` | `"2\n"`, `"3\n"` | `"II\n"`, `"III\n"` |

Replace the `arithmetic_overflow_errors` test entirely (unbounded tiers can no longer overflow):

```rust
    #[test]
    fn unbounded_arithmetic_no_longer_overflows() {
        // 3999^6 overflowed i64 and used to be an error; the Roman engine grows
        // another tier instead, so this now succeeds.
        let src = "ni x = °MMMCMXCIX; ni y = x * x * x * x * x * x; scribe(y);";
        let program = parse(&tokenize(src).unwrap()).unwrap();
        assert!(execute(&program).is_ok());
    }
```

Leave unchanged (no numeric output or already `is_err`): `booleans_print_in_latin`, `string_concatenation`, `text_comparison`, `char_literal_roundtrip`, `index_out_of_range_errors`, `index_write_out_of_range_errors`, `list_equality_runs`.

- [ ] **Step 4: Update the integration-test expectations**

This task changes what the pipeline prints, so `tests/programs.rs` must move with it now (not later). Change these numeric expectations:

| test | old | new |
|---|---|---|
| `full_program_with_function_and_if` | `"30532\n"` | `"X\u{0305}X\u{0305}X\u{0305}DXXXII\n"` |
| `calc_example_still_runs` | `"30532\n"` | `"X\u{0305}X\u{0305}X\u{0305}DXXXII\n"` |
| `non_latin_name_warns_but_program_still_runs` | `"5\n"` | `"V\n"` |
| `latin_program_produces_no_warnings` | `"5\n"` | `"V\n"` |
| `series_example_runs` | `"[1, 20, 30, 40]\n91\n"` | `"[I, XX, XXX, XL]\nXCI\n"` |
| `lists_have_value_semantics` | `"3\n2\n"` | `"III\nII\n"` |

Leave every float/precision test unchanged (`praecisio_example_runs`, `mnr_is_single_precision`, `nr_is_double_precision`, `representable_fraction_prints_same_under_both`, `mixing_float_types_is_a_type_error`, `nr_arithmetic_and_comparison_run`, `unconstrained_fraction_prints_at_nr_precision`) and the non-numeric tests (`type_error_reports_latin_and_line`, `char_and_text_comparison_end_to_end`, `type_errors_still_halt_before_any_linting`).

The three overlined X's are `X\u{0305}` written literally three times (30000); `D` is 500; `XXXII` is 32.

- [ ] **Step 5: Run the whole crate suite**

Run: `cargo test -p lingua-serpentis`
Expected: PASS. (The `checker`, `parser`, and `lexer` suites still pass unchanged — they build literals from source and assert on tokens/AST/types, not printed numbers.) If `full_program_with_function_and_if`/`calc_example_still_runs` mismatch, rerun with `-- --nocapture` and confirm the actual equals `X̄X̄X̄DXXXII`.

- [ ] **Step 6: Commit**

```bash
git add crates/lingua-serpentis/src/value.rs crates/lingua-serpentis/src/interp.rs crates/lingua-serpentis/tests/programs.rs
git commit -m "feat: ni computes and prints as Roman numerals at runtime"
```

---

## Task 6: Push `Roman` into the literal/AST layer

The runtime is Roman; the literal is still an `i64` in the token/AST, converted at eval. Complete the "Roman is the truth" story: parse builds a `Roman` through the engine, so `TokenKind::Int`, `Expr::Int`, and `RomanNumber::Int` all carry `Roman`, and the eval-time `from_i64` disappears. Behavior is identical — this is an internal-purity finish.

**Files:**
- Modify: `crates/lingua-serpentis/src/roman.rs` (`RomanNumber`, `convert_single`, `convert_int_roman`, `parse_roman`, and their tests)
- Modify: `crates/lingua-serpentis/src/token.rs` (`TokenKind::Int` payload, `From<RomanNumber>`)
- Modify: `crates/lingua-serpentis/src/lexer.rs` (`nullus`, and its test)
- Modify: `crates/lingua-serpentis/src/ast.rs` (`Expr::Int` payload)
- Modify: `crates/lingua-serpentis/src/interp.rs` (drop the eval-time `from_i64`)
- Modify: `crates/lingua-serpentis/src/parser.rs` (**tests only** — the `primary()` source already owns the payload via `peek().cloned()`)

**Interfaces:**
- Consumes: `Roman`, `add`, `mul`, `neg`, `to_f64`, `is_zero`, `from_u64` (Tasks 1–4).
- Produces: `RomanNumber::Int(Roman)`, `TokenKind::Int(Roman)`, `Expr::Int(Roman)`.

- [ ] **Step 1: Change the types (crate will not compile yet)**

`roman.rs` — `RomanNumber`:

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum RomanNumber {
    Int(Roman),
    Float(f64),
}
```

`token.rs`:

```rust
use crate::roman::{Roman, RomanNumber};

// … in TokenKind:
    Int(Roman),

// … the From impl:
impl From<RomanNumber> for TokenKind {
    fn from(n: RomanNumber) -> Self {
        match n {
            RomanNumber::Int(r) => TokenKind::Int(r),
            RomanNumber::Float(f) => TokenKind::Float(f),
        }
    }
}
```

`ast.rs` — add `use crate::roman::Roman;` at the top and change the variant:

```rust
    Int(Roman),
```

`lexer.rs` — add `use crate::roman::Roman;` and change the `nullus` keyword:

```rust
        "nullus" => TokenKind::Int(Roman::zero()),
```

`interp.rs` — the literal is now already a `Roman`; clone it instead of converting:

```rust
            Expr::Int(r) => Ok(Value::Int(r.clone())),
```

- [ ] **Step 2: Rebuild `convert_single`/`convert_int_roman`/`parse_roman` on the engine**

In `roman.rs`, replace the three functions so they return `Roman` and build it through `add`/`mul` (the parser now exercises the arithmetic engine on every literal):

```rust
pub(crate) fn convert_single(s: &str) -> Result<Roman, String> {
    if s.is_empty() {
        return Err("numerus Romanus vacuus".into());
    }
    fn sym(c: char) -> Option<Roman> {
        let v = match c {
            'I' => 1, 'V' => 5, 'X' => 10, 'L' => 50,
            'C' => 100, 'D' => 500, 'M' => 1000, _ => return None,
        };
        Some(Roman::from_u64(v))
    }
    let chars: Vec<char> = s.chars().collect();
    let mut total = Roman::zero();
    for i in 0..chars.len() {
        let cur = sym(chars[i]).ok_or_else(|| format!("numerus Romanus invalidus: '{}'", s))?;
        let next_bigger = chars.get(i + 1).and_then(|c| sym(*c)).is_some_and(|nx| cur < nx);
        total = if next_bigger { total.sub(&cur) } else { total.add(&cur) };
    }
    Ok(total)
}

pub(crate) fn convert_int_roman(s: &str) -> Result<Roman, String> {
    let nums: Vec<Roman> = s.split('.').map(convert_single).collect::<Result<_, _>>()?;
    let mut total = Roman::zero();
    let mut i = 0;
    while i + 1 < nums.len() {
        total = total.add(&nums[i].mul(&nums[i + 1]));
        i += 2;
    }
    if nums.len() % 2 == 1 {
        total = total.add(&nums[nums.len() - 1]);
    }
    Ok(total)
}
```

Then update `parse_roman`: the `nullus` early return becomes `Ok(RomanNumber::Int(Roman::zero()))`, the integer case is `Ok(RomanNumber::Int(int_val))`, and the fraction case computes its `f64` from the `Roman` parts:

```rust
    if literal == "nullus" {
        return Ok(RomanNumber::Int(Roman::zero()));
    }
    // … after `let int_val = convert_int_roman(int_str)?;`
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
            Ok(RomanNumber::Float(int_val.to_f64() + num.to_f64() / den.to_f64()))
        }
    }
```

- [ ] **Step 3: Update the affected unit tests**

`roman.rs` `mod tests` — the `convert_*` and `parse_roman` integer assertions now compare `Roman`s:

```rust
    // convert_single / convert_int_roman now return Roman:
    assert_eq!(convert_single("XXV").unwrap(), Roman::from_u64(25));
    assert_eq!(convert_single("VII").unwrap(), Roman::from_u64(7));
    assert_eq!(convert_single("IV").unwrap(), Roman::from_u64(4));
    assert_eq!(convert_single("IX").unwrap(), Roman::from_u64(9));
    assert_eq!(convert_single("M").unwrap(), Roman::from_u64(1000));
    assert_eq!(convert_int_roman("XXX.M.V.C.VII").unwrap(), Roman::from_u64(30507));
    assert_eq!(convert_int_roman("VII.M").unwrap(), Roman::from_u64(7000));
    assert_eq!(convert_int_roman("XXV").unwrap(), Roman::from_u64(25));
    // parse_roman integer literals:
    assert_eq!(parse_roman("°XXV").unwrap(), RomanNumber::Int(Roman::from_u64(25)));
    assert_eq!(parse_roman("°XXX.M.V.C.VII").unwrap(), RomanNumber::Int(Roman::from_u64(30507)));
    assert_eq!(parse_roman("nullus").unwrap(), RomanNumber::Int(Roman::zero()));
```

Leave the fraction tests as-is — `parse_roman("°V:I/II")` is still `RomanNumber::Float(5.5)`, and the malformed/zero-denominator error tests are unchanged.

`lexer.rs` `mod tests`, `roman_numbers_become_values`:

```rust
        assert_eq!(kinds("°XXV"), vec![TokenKind::Int(Roman::from_u64(25))]);
        assert_eq!(kinds("°V:I/II"), vec![TokenKind::Float(5.5)]);
        assert_eq!(kinds("nullus"), vec![TokenKind::Int(Roman::zero())]);
```

Add `use crate::roman::Roman;` inside the `lexer.rs` test module if not already imported at file scope.

`parser.rs` `mod tests` — wrap every `Expr::Int(n)` literal in the `Roman` constructor. Add a helper at the top of the test module and update each site:

```rust
    fn ri(n: i64) -> crate::roman::Roman { crate::roman::Roman::from_i64(n) }
```

Sites to change (each `Expr::Int(N)` → `Expr::Int(ri(N))`): the `let x = 5` assertion, the `x = 1` assign, the `2 + 3 * 4` precedence tree (`2`, `3`, `4`), the `10 / 3 + 1` tree (`10`, `3`, `1`), the list-literal `[1, 2, 3]`, the `ni lista xs = [1]` value, the index read `[…]index 1`, and the index-assign `index 2` / `value 10`. Example transformation:

```rust
    // before:
    assert_eq!(e, Expr::ListLit(vec![Expr::Int(1), Expr::Int(2), Expr::Int(3)]));
    // after:
    assert_eq!(e, Expr::ListLit(vec![Expr::Int(ri(1)), Expr::Int(ri(2)), Expr::Int(ri(3))]));
```

- [ ] **Step 4: Compile and run the whole crate**

Run: `cargo test -p lingua-serpentis`
Expected: PASS. If the compiler flags a remaining `TokenKind::Int(i64)` / `Expr::Int(i64)` construction, convert that literal with `Roman::from_i64`/`Roman::from_u64` as above.

- [ ] **Step 5: Commit**

```bash
git add crates/lingua-serpentis/src/roman.rs crates/lingua-serpentis/src/token.rs crates/lingua-serpentis/src/lexer.rs crates/lingua-serpentis/src/ast.rs crates/lingua-serpentis/src/interp.rs crates/lingua-serpentis/src/parser.rs
git commit -m "feat: numerals carry through the token/AST, parsed via the engine"
```

---

## Task 7: Examples and docs

Task 6 is behavior-preserving, so the tests are already green (the integration expectations moved into Task 5). This task only refreshes the example comments and the older design doc.

**Files:**
- Modify: `examples/series.liss`
- Modify: `docs/superpowers/specs/2026-07-19-lingua-serpentis-rust-interpreter-design.md`

- [ ] **Step 1: Update the example's output comments**

In `examples/series.liss`, update the two trailing comments so the examples stay honest (the code is unchanged):

```
scribe(numeri);   // [I, XX, XXX, XL]
scribe(summa);    // XCI
```

(The other examples need no change: `calc.liss` has no output comment, `praecisio.liss` prints only floats, `salve.liss` prints text.)

- [ ] **Step 2: Amend the original design doc**

At the top of `docs/superpowers/specs/2026-07-19-lingua-serpentis-rust-interpreter-design.md`, immediately after its metadata/header block, add:

```markdown
> **Amended 2026-07-23:** the `ni` type is no longer `i64`-backed. Roman
> numerals are now the runtime representation — see
> `2026-07-23-roman-native-integers-design.md`. Passages below describing Roman
> literals collapsing to `i64` and integers printing in Arabic are superseded.
```

- [ ] **Step 3: Full suite green, then commit**

Run: `cargo test -p lingua-serpentis`
Expected: PASS — the whole suite (engine unit tests + interpreter + integration), numbers printing Roman throughout.

```bash
git add examples/series.liss docs/superpowers/specs/2026-07-19-lingua-serpentis-rust-interpreter-design.md
git commit -m "docs: refresh example output and amend the original design doc"
```

---

## Notes for the implementer

- **The engine is the foundation.** Tasks 1–4 are pure `roman.rs` and never touch the rest of the crate; the suite stays green throughout. Do not start Task 5 until the engine tests pass — the parity-with-`i64` tests (Tasks 3–4) are what guarantee "only output changes, not values."
- **The one boundary.** `Roman` ↔ `usize`/`i64` conversion happens only at the list-`Vec` edge (Task 5: index read/write, `longitudo`). Everywhere else, integers are `Roman` end to end.
- **`ref` in `eval_binary` guards.** The `(Int(_), Int(ref b)) if b.is_zero()` arms borrow `b` so the following arm can still move it; keep the `ref`.
- **Private-field access.** `mag_divmod` reads `Roman::from_u64(1).tiers` — legal because the helper lives in the same module as `Roman`. Keep all magnitude helpers in `roman.rs`.
- **Don't `git add -A`.** Stage only each task's named files so the `Cargo.toml` edition-drift never rides along (Global Constraints).
```
