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

pub(crate) fn convert_single(s: &str) -> Result<i64, String> {
    if s.is_empty() {
        return Err("numerus Romanus vacuus".into());
    }
    fn val(c: char) -> Option<i64> {
        match c {
            'I' => Some(1), 'V' => Some(5), 'X' => Some(10), 'L' => Some(50),
            'C' => Some(100), 'D' => Some(500), 'M' => Some(1000), _ => None,
        }
    }
    let chars: Vec<char> = s.chars().collect();
    let mut total = 0i64;
    for i in 0..chars.len() {
        let cur = val(chars[i]).ok_or_else(|| format!("numerus Romanus invalidus: '{}'", s))?;
        let next = chars.get(i + 1).and_then(|c| val(*c)).unwrap_or(0);
        if cur < next { total -= cur; } else { total += cur; }
    }
    Ok(total)
}

pub(crate) fn convert_int_roman(s: &str) -> Result<i64, String> {
    let nums: Vec<i64> = s
        .split('.')
        .map(convert_single)
        .collect::<Result<_, _>>()?;
    let mut total = 0i64;
    let mut i = 0;
    while i + 1 < nums.len() {
        total += nums[i] * nums[i + 1];
        i += 2;
    }
    if nums.len() % 2 == 1 {
        total += nums[nums.len() - 1];
    }
    Ok(total)
}

#[derive(Debug, Clone, PartialEq)]
pub enum RomanNumber {
    Int(i64),
    Float(f64),
}

pub fn parse_roman(literal: &str) -> Result<RomanNumber, String> {
    if literal == "nullus" {
        return Ok(RomanNumber::Int(0));
    }
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
            let num_str = fp.next().ok_or("fractio sine numeratore")?;
            let den_str = fp.next().ok_or("fractio sine denominatore")?;
            let num = convert_int_roman(num_str)?;
            let den = convert_int_roman(den_str)?;
            // defensive: guards a future where a denominator could evaluate to 0
            if den == 0 {
                return Err("denominator est nullus".into());
            }
            Ok(RomanNumber::Float(int_val as f64 + num as f64 / den as f64))
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
            s.push_str(&render_digit(self.tiers[k].digit(), k));
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
    /// dividend — matching Rust's `i64` `/` and `%`. Caller must ensure
    /// `!other.is_zero()`.
    pub fn div_rem(&self, other: &Roman) -> (Roman, Roman) {
        let (q, rem) = mag_divmod(&self.tiers, &other.tiers);
        (make(self.negative != other.negative, q), make(self.negative, rem))
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

    #[test]
    fn single_basic_and_subtractive() {
        assert_eq!(convert_single("XXV").unwrap(), 25);
        assert_eq!(convert_single("VII").unwrap(), 7);
        assert_eq!(convert_single("IV").unwrap(), 4);
        assert_eq!(convert_single("IX").unwrap(), 9);
        assert_eq!(convert_single("M").unwrap(), 1000);
    }

    #[test]
    fn single_rejects_garbage() {
        assert!(convert_single("XQ").is_err());
    }

    #[test]
    fn int_multiplicative_pairs() {
        // XXX.M.V.C.VII = 30*1000 + 5*100 + 7 = 30507
        assert_eq!(convert_int_roman("XXX.M.V.C.VII").unwrap(), 30507);
        // VII.M = 7*1000 = 7000  (even number of chunks: pure product)
        assert_eq!(convert_int_roman("VII.M").unwrap(), 7000);
        // single chunk, no dots
        assert_eq!(convert_int_roman("XXV").unwrap(), 25);
    }

    #[test]
    fn parse_integer_literal() {
        assert_eq!(parse_roman("°XXV").unwrap(), RomanNumber::Int(25));
        assert_eq!(parse_roman("°XXX.M.V.C.VII").unwrap(), RomanNumber::Int(30507));
    }

    #[test]
    fn parse_nullus_is_zero() {
        assert_eq!(parse_roman("nullus").unwrap(), RomanNumber::Int(0));
    }

    #[test]
    fn parse_fraction_literal() {
        assert_eq!(parse_roman("°V:I/II").unwrap(), RomanNumber::Float(5.5));
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
}
