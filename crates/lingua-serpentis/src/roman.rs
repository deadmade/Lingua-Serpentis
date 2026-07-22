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
#[allow(dead_code)] // used by arithmetic in later tasks
fn trim(mut tiers: Vec<Tier>) -> Vec<Tier> {
    while matches!(tiers.last(), Some(t) if t.digit() == 0) {
        tiers.pop();
    }
    tiers
}

/// Build a canonical `Roman` from a sign and carry-normalized tiers; forces zero
/// to be non-negative.
#[allow(dead_code)] // used by arithmetic in later tasks
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
}
