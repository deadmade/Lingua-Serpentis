/// A Roman-native integer: a sign plus a little-endian bag of decimal digits.
/// Canonical invariant: every digit is `0..=9`, there are no trailing
/// all-zero digits, and the empty `tiers` vector is the unique zero (never
/// negative). Structural equality is therefore numeric equality.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Roman {
    negative: bool,
    tiers: Vec<u8>,
}

/// Drop trailing zero digits so the canonical form is unique.
fn trim(mut tiers: Vec<u8>) -> Vec<u8> {
    while matches!(tiers.last(), Some(0)) {
        tiers.pop();
    }
    tiers
}

/// Build a canonical `Roman` from a sign and carry-normalized digits; forces
/// zero to be non-negative.
fn make(negative: bool, tiers: Vec<u8>) -> Roman {
    let tiers = trim(tiers);
    Roman { negative: negative && !tiers.is_empty(), tiers }
}

/// Compare two magnitudes (digits only, sign ignored). Canonical form has no
/// trailing zero digits, so the longer vector is strictly larger.
fn mag_cmp(a: &[u8], b: &[u8]) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    if a.len() != b.len() { return a.len().cmp(&b.len()); }
    for k in (0..a.len()).rev() {
        let c = a[k].cmp(&b[k]);
        if c != Ordering::Equal { return c; }
    }
    Ordering::Equal
}

/// Magnitude add in base-10 digit space (per-digit carrying).
fn mag_add(a: &[u8], b: &[u8]) -> Vec<u8> {
    let n = a.len().max(b.len());
    let mut out = Vec::with_capacity(n + 1);
    let mut carry = 0u8;
    for k in 0..n {
        let da = a.get(k).copied().unwrap_or(0);
        let db = b.get(k).copied().unwrap_or(0);
        let s = da + db + carry; // <= 9 + 9 + 1 = 19
        out.push(s % 10);
        carry = s / 10;
    }
    if carry > 0 { out.push(carry); }
    trim(out)
}

/// Magnitude subtract in base-10 digit space; requires magnitude a >= b.
fn mag_sub(a: &[u8], b: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(a.len());
    let mut borrow = 0i16;
    for k in 0..a.len() {
        let da = a[k] as i16;
        let db = b.get(k).map_or(0, |&d| d as i16);
        let mut d = da - db - borrow;
        if d < 0 { d += 10; borrow = 1; } else { borrow = 0; }
        out.push(d as u8);
    }
    trim(out) // borrow ends at 0 because a >= b
}

/// Halve a magnitude, returning (quotient, remainder ∈ {0,1}) — long division
/// by two from the most-significant digit down.
fn mag_halve(a: &[u8]) -> (Vec<u8>, u8) {
    let mut out = vec![0u8; a.len()];
    let mut rem = 0u8;
    for k in (0..a.len()).rev() {
        let cur = rem * 10 + a[k]; // <= 19
        out[k] = cur / 2;
        rem = cur % 2;
    }
    (trim(out), rem)
}

/// Magnitude multiply via the doubling ladder (Russian-peasant): only `mag_add`
/// (doubling) and `mag_halve`. Never a host multiply of the value.
fn mag_mul(a: &[u8], b: &[u8]) -> Vec<u8> {
    let mut result: Vec<u8> = Vec::new();
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
fn mag_divmod(n: &[u8], d: &[u8]) -> (Vec<u8>, Vec<u8>) {
    // The `d != 0` precondition is now enforced by `Roman::div_rem`, this
    // function's only caller, which returns None rather than calling in. It
    // was a `debug_assert!` here, compiled out of release, where an empty `d`
    // meant `mag_cmp` never reported Greater and the doubling loop below ran
    // until memory ran out. See D-21.
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
    pub fn zero() -> Roman {
        Roman { negative: false, tiers: Vec::new() }
    }

    pub fn from_u64(mut n: u64) -> Roman {
        let mut tiers = Vec::new();
        while n > 0 {
            tiers.push((n % 10) as u8);
            n /= 10;
        }
        Roman { negative: false, tiers } // no trailing zeros by construction
    }

    /// This `Roman`'s digits, little-endian, with no trailing zeros (empty
    /// for zero).
    pub fn digits(&self) -> &[u8] {
        &self.tiers
    }

    /// Whether this `Roman` is negative. Never true for zero — [`make`]
    /// clears the sign when the magnitude is empty, which is what makes
    /// structural equality numeric equality.
    ///
    /// The exact inverse of [`Roman::from_digits`]'s first argument, and it
    /// exists for the same reason [`Roman::digits`] does: `liss_int_new(kind,
    /// neg, digiti, len)` takes the sign and the digits as two separate
    /// arguments, so codegen must be able to read back both halves of what
    /// `from_digits` consumes. `digits()` alone would silently compile every
    /// negative literal as its magnitude.
    pub fn negative(&self) -> bool {
        self.negative
    }

    /// Build a canonical `Roman` from a sign and little-endian digits,
    /// stripping trailing zeros and clearing the sign if the magnitude is
    /// zero. `None` if any digit is out of range (each must be `0..=9`) —
    /// this is the checked boundary for digit vectors arriving from outside
    /// the type's own invariant, such as a `LissInt` payload crossing the
    /// ABI by `memcpy`.
    pub fn from_digits(negative: bool, digits: &[u8]) -> Option<Roman> {
        if digits.iter().any(|&d| d > 9) {
            return None;
        }
        Some(make(negative, digits.to_vec()))
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
            acc = acc * 10 + *t as i128;
            if acc > i64::MAX as i128 + 1 { return None; } // allow magnitude of i64::MIN
        }
        if self.negative { acc = -acc; }
        i64::try_from(acc).ok()
    }

    pub fn to_usize(&self) -> Option<usize> {
        if self.negative { return None; }
        let mut acc: u128 = 0;
        for t in self.tiers.iter().rev() {
            acc = acc.checked_mul(10)?.checked_add(*t as u128)?;
        }
        usize::try_from(acc).ok()
    }

    pub fn to_f64(&self) -> f64 {
        let mut acc = 0.0f64;
        for (k, t) in self.tiers.iter().enumerate() {
            acc += *t as f64 * 10f64.powi(k as i32);
        }
        if self.negative { -acc } else { acc }
    }
}

impl From<i64> for Roman {
    fn from(n: i64) -> Roman { Roman::from_i64(n) }
}

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

#[derive(Debug, Clone, PartialEq)]
pub enum RomanNumber {
    Int(Roman),
    Frac { numer: Roman, den: Roman },
}

pub fn parse_roman(literal: &str) -> Result<RomanNumber, String> {
    if literal == "nullus" {
        return Ok(RomanNumber::Int(Roman::zero()));
    }
    // An API-only guard. The lexer, this function's only caller in the
    // workspace, builds `literal` from a slice that starts at the '°' it just
    // matched, so the prefix is there by construction and this arm cannot fire
    // through the language. It is kept because `parse_roman` is `pub` and a
    // direct caller has no such guarantee; the catalogue records it UNCOVERED,
    // which is the honest label. See D-25.
    let body = literal
        .strip_prefix('°')
        .ok_or_else(|| format!("numerus sine '°': '{}'", literal))?;

    let mut parts = body.splitn(2, ':');
    let int_str = parts.next().unwrap();
    let int_val = convert_int_roman(int_str)?;

    match parts.next() {
        None => Ok(RomanNumber::Int(int_val)),
        Some(frac) => {
            let mut fp = frac.splitn(2, '/');
            // `splitn` always yields at least one item, for every input
            // including "" — so the numerator is always present, exactly as
            // for `int_str` above. The arm that used to raise "fractio sine
            // numeratore" here was not merely unreached but impossible, and is
            // gone (D-25).
            let num_str = fp.next().unwrap();
            let den_str = fp.next().ok_or("fractio sine denominatore")?;
            let num = convert_int_roman(num_str)?;
            let den = convert_int_roman(den_str)?;
            // No zero check: `convert_int_roman` cannot return zero. Every
            // numeral it accepts is a sum of products of `convert_single`
            // values, and `convert_single` is at least 1 for every string it
            // accepts — pinned exhaustively by `convert_single_is_never_zero`
            // below. An empty or malformed denominator dies earlier, in
            // `convert_single`. The "denominator est nullus" guard that stood
            // here was unreachable (D-25).
            let numer = int_val.mul(&den).add(&num);
            Ok(RomanNumber::Frac { numer, den })
        }
    }
}

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
            s.push_str(&render_digit(self.tiers[k], k));
        }
        s
    }

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

    pub fn mul(&self, other: &Roman) -> Roman {
        make(self.negative != other.negative, mag_mul(&self.tiers, &other.tiers))
    }

    /// Truncating division toward zero; the remainder's sign follows the
    /// dividend — matching Rust's `i64` `/` and `%`. `None` when `other` is
    /// zero.
    ///
    /// Defectus D-21. The zero guard used to be a `debug_assert!` inside
    /// `mag_divmod`, which is compiled out of a release build — and with an
    /// empty divisor `mag_cmp` never reports Greater, so the doubling loop
    /// runs forever, doubling `pow` until memory runs out. Every caller
    /// happened to guard, so it was unreachable; but `div_rem` is `pub`, and
    /// a latent release-only hang behind a caller's diligence is not a
    /// guarantee. Returning `Option` moves the obligation into the type.
    ///
    /// Deliberately not an `assert!`: an unreachable panic is still a panic,
    /// and `no panic` is a live rule the fuzzer enforces.
    pub fn div_rem(&self, other: &Roman) -> Option<(Roman, Roman)> {
        if other.is_zero() {
            return None;
        }
        let (q, rem) = mag_divmod(&self.tiers, &other.tiers);
        Some((make(self.negative != other.negative, q), make(self.negative, rem)))
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

#[cfg(test)]
mod tests {
    use super::*;

    fn r(n: i64) -> Roman { Roman::from_i64(n) }

    #[test]
    fn roman_value_roundtrips_through_i64() {
        for n in [0i64, 1, 4, 9, 25, 40, 91, 3999, 4000, 30532, -7, -3999, i64::MAX, i64::MIN] {
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

    #[test]
    fn single_basic_and_subtractive() {
        assert_eq!(convert_single("XXV").unwrap(), Roman::from_u64(25));
        assert_eq!(convert_single("VII").unwrap(), Roman::from_u64(7));
        assert_eq!(convert_single("IV").unwrap(), Roman::from_u64(4));
        assert_eq!(convert_single("IX").unwrap(), Roman::from_u64(9));
        assert_eq!(convert_single("M").unwrap(), Roman::from_u64(1000));
    }

    #[test]
    fn single_rejects_garbage() {
        assert!(convert_single("XQ").is_err());
    }

    #[test]
    fn int_multiplicative_pairs() {
        // XXX.M.V.C.VII = 30*1000 + 5*100 + 7 = 30507
        assert_eq!(convert_int_roman("XXX.M.V.C.VII").unwrap(), Roman::from_u64(30507));
        // VII.M = 7*1000 = 7000  (even number of chunks: pure product)
        assert_eq!(convert_int_roman("VII.M").unwrap(), Roman::from_u64(7000));
        // single chunk, no dots
        assert_eq!(convert_int_roman("XXV").unwrap(), Roman::from_u64(25));
    }

    #[test]
    fn parse_integer_literal() {
        assert_eq!(parse_roman("°XXV").unwrap(), RomanNumber::Int(Roman::from_u64(25)));
        assert_eq!(parse_roman("°XXX.M.V.C.VII").unwrap(), RomanNumber::Int(Roman::from_u64(30507)));
    }

    #[test]
    fn parse_nullus_is_zero() {
        assert_eq!(parse_roman("nullus").unwrap(), RomanNumber::Int(Roman::zero()));
    }

    #[test]
    fn parse_fraction_literal() {
        assert_eq!(
            parse_roman("°V:I/II").unwrap(),
            RomanNumber::Frac { numer: Roman::from_u64(11), den: Roman::from_u64(2) }
        );
    }

    #[test]
    fn parse_rejects_missing_degree() {
        assert!(parse_roman("XXV").is_err());
    }

    #[test]
    fn parse_rejects_malformed_or_zero_fraction() {
        assert!(parse_roman("°V:I").is_err());        // missing '/denominator'
        assert!(parse_roman("°V:I/nullus").is_err()); // zero/invalid denominator rejected
    }

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
            let (q, rem) = r(a).div_rem(&r(b)).expect("nonzero divisor");
            assert_eq!(q.to_i64(), Some(a / b), "{a}/{b} quotient");
            assert_eq!(rem.to_i64(), Some(a % b), "{a}%{b} remainder");
        }
    }

    #[test]
    fn convert_single_is_never_zero_or_negative() {
        // The evidence for deleting `parse_roman`'s "denominator est nullus"
        // guard (D-25). That guard could only fire if `convert_int_roman`
        // returned zero, and `convert_int_roman` is a sum of products of
        // `convert_single` values — so it is enough that `convert_single` is
        // always at least 1.
        //
        // The reason it holds: the final symbol is always added (there is no
        // following symbol to be bigger than it), and a symbol is only ever
        // subtracted when a strictly larger one follows. Rather than rest on
        // that argument, enumerate every string the function accepts up to
        // length 6 — 137_256 of them — and check.
        const SYMS: [char; 7] = ['I', 'V', 'X', 'L', 'C', 'D', 'M'];
        let zero = Roman::zero();
        let mut words: Vec<String> = SYMS.iter().map(|c| c.to_string()).collect();
        let mut checked = 0usize;
        for _ in 1..6 {
            let mut next = Vec::with_capacity(words.len() * SYMS.len());
            for w in &words {
                let v = convert_single(w).expect("every string over the symbols parses");
                assert!(v > zero, "convert_single({w:?}) = {} is not >= 1", v.display());
                checked += 1;
                for c in SYMS {
                    next.push(format!("{w}{c}"));
                }
            }
            words = next;
        }
        for w in &words {
            let v = convert_single(w).expect("every string over the symbols parses");
            assert!(v > zero, "convert_single({w:?}) = {} is not >= 1", v.display());
            checked += 1;
        }
        assert_eq!(checked, 137_256, "every string of length 1..=6 over the seven symbols");
    }

    #[test]
    fn parse_roman_rejects_a_literal_without_its_degree_sign() {
        // `parse_roman` is `pub`, so this guard is reachable from outside the
        // crate even though the lexer can never trip it. Covering it here is
        // what makes keeping it honest rather than dead (D-25).
        assert_eq!(
            parse_roman("V"),
            Err("numerus sine '°': 'V'".to_string())
        );
    }

    #[test]
    fn digits_are_little_endian_and_canonical() {
        let r = Roman::from_u64(1024);
        assert_eq!(r.digits(), &[4, 2, 0, 1]);
        assert!(Roman::zero().digits().is_empty());
    }

    #[test]
    fn from_digits_round_trips_through_display() {
        for n in [0u64, 1, 9, 10, 4999, 1_000_000] {
            let r = Roman::from_u64(n);
            let back = Roman::from_digits(false, r.digits()).expect("digits from from_u64 are in range");
            assert_eq!(r.display(), back.display());
        }
    }

    #[test]
    fn from_digits_strips_trailing_zeros_to_stay_canonical() {
        assert_eq!(Roman::from_digits(false, &[1, 0, 0]).unwrap().digits(), &[1]);
        assert!(Roman::from_digits(true, &[0, 0]).unwrap().is_zero());
    }

    #[test]
    fn from_digits_rejects_a_digit_out_of_range() {
        // The checked boundary: a digit beyond 9 must not become a `Roman`.
        // Left unchecked, it panics inside `display` (`render_digit`'s
        // `unreachable!`) or overflows the `u8` addition in `mag_add`,
        // silently wrapping in release — see the review that added this.
        assert_eq!(Roman::from_digits(false, &[12]), None);
        assert_eq!(Roman::from_digits(false, &[200]), None);
        assert_eq!(Roman::from_digits(false, &[9]), Some(Roman::from_u64(9)));
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
}
