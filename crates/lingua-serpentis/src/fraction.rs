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
