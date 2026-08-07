//! The kind codes (spec §5.3) — the one type descriptor that crosses the ABI.
//!
//! # Why these live here and not in `liss-abi`
//!
//! They are written by **codegen** and read by the **runtime**, and those two
//! never share a compilation: `liss-abi` is cross-compiled for musl and is
//! never linked into the compiler (spec §11), so `liss-codegen` cannot import
//! from it. Until now the numbers existed twice — `KIND_INT` in `liss-abi` and
//! a copied `GENUS_NI = 2` in `liss-codegen` — held together by a comment.
//!
//! `liss-runtime` is already a dependency of *both*, and is dependency-free in
//! both the host and the musl build, so it is the one place the two ends of the
//! contract can genuinely share a definition. `liss-abi` re-exports these at its
//! crate root, so `liss_abi::KIND_INT` still names this constant.
//!
//! # The encoding
//!
//! ```text
//! base: 0 Bool  1 Char  2 Int  3 Frac(Uncia)  4 Frac(Scripulum)  5 Text
//! kind = base | (depth << 3)
//! ```
//!
//! `ni lista` is 10; `ni lista lista` is 18; the element kind of kind `k` is
//! `k - GRADUS`. Closed form, unbounded nesting, no interning table.
//!
//! This is a descriptor **on the container**, written at every construction site
//! as a compile-time immediate. Codegen only ever writes it and never dispatches
//! on it, so it is categorically not the tagged value the ABI forbids.

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

    /// The two worked examples from spec §5.3, and the inverse.
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

    /// Every base fits below `GRADUS`, or the `|` would carry into the depth.
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
