//! The kind codes (spec §5.3) — the one type descriptor that crosses the ABI.

pub const KIND_BOOL: u32 = 0;
pub const KIND_CHAR: u32 = 1;
pub const KIND_INT: u32 = 2;
/// `mnr` — a count of unciae, 1/12 of the `as`.
pub const KIND_FRAC_UNCIA: u32 = 3;
/// `nr` — a count of scripula, 1/288.
pub const KIND_FRAC_SCRIPULUM: u32 = 4;
pub const KIND_TEXT: u32 = 5;

/// One level of `lista` nesting.
pub const GRADUS: u32 = 8;

/// The scalar type underneath any nesting.
#[inline]
pub const fn basis(kind: u32) -> u32 {
    kind & (GRADUS - 1)
}

/// How many `lista` levels deep — 0 for a scalar.
#[inline]
pub const fn profunditas(kind: u32) -> u32 {
    kind >> 3
}

/// The grain denominator a fractional kind counts in: XII for `mnr`,
/// CCLXXXVIII for `nr`. `None` for every other kind.
#[inline]
pub const fn granum(kind: u32) -> Option<u64> {
    match kind {
        KIND_FRAC_UNCIA => Some(12),
        KIND_FRAC_SCRIPULUM => Some(288),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nesting_is_closed_form() {
        let ni_lista = KIND_INT | (1 << 3);
        let ni_lista_lista = KIND_INT | (2 << 3);
        assert_eq!(ni_lista, 10);
        assert_eq!(ni_lista_lista, 18);
        assert_eq!(ni_lista_lista - GRADUS, ni_lista);
        assert_eq!(basis(ni_lista_lista), KIND_INT);
        assert_eq!(profunditas(ni_lista_lista), 2);
        assert_eq!(profunditas(KIND_TEXT), 0);
    }

    #[test]
    fn every_base_fits_under_one_level() {
        for k in [
            KIND_BOOL,
            KIND_CHAR,
            KIND_INT,
            KIND_FRAC_UNCIA,
            KIND_FRAC_SCRIPULUM,
            KIND_TEXT,
        ] {
            assert!(k < GRADUS, "{k} collides with the depth field");
            assert_eq!(basis(k), k);
        }
    }

    #[test]
    fn only_the_two_fractions_have_a_grain() {
        assert_eq!(granum(KIND_FRAC_UNCIA), Some(12));
        assert_eq!(granum(KIND_FRAC_SCRIPULUM), Some(288));
        for k in [KIND_BOOL, KIND_CHAR, KIND_INT, KIND_TEXT] {
            assert_eq!(granum(k), None);
        }
    }
}
