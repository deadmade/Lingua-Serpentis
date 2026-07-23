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
    // `scr_per_grain = 288 / g` below assumes a supported grain; guard it.
    debug_assert!(
        *g == uncia_grain() || *g == scripulum_grain(),
        "fraction grain must be uncia (12) or scripulum (288)"
    );
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
        let cases = [(16i64, 16i64), (6, 6), (5, 5), (360, 2), (-5, 5), (-6, -7)];
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
        let cases = [(16i64, 4i64), (66, 6), (360, 360), (-16, 6), (-16, -6)];
        for (c1, c2) in cases {
            for g in [12i64, 288] {
                let got = div(&r(c1), &r(c2), &r(g)).unwrap().to_i64().unwrap();
                let want = ((c1 as i128 * g as i128) / c2 as i128) as i64;
                assert_eq!(got, want, "div {c1}/{c2} g={g}");
            }
        }
        assert!(div(&r(5), &r(0), &r(12)).is_none(), "zero divisor must be guarded");
    }

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
}
