//! `scriptio` — the `scribe` formatter: the printable rendering of a language
//! value. Free functions rather than a `Display`/`ToString`-style method:
//! each one is monomorphic (renders exactly one value shape) and writes into
//! a caller-supplied `Sink` rather than allocating and returning a `String`,
//! which is what the runtime's static output buffer (spec §6) needs.
//!
//! Ported from `Value::display`'s match arms (the interpreter's `value.rs`,
//! spec §5.5) — the same seven one-line bodies, unchanged, just monomorphised
//! into separate functions. The list case is deliberately absent: it is
//! `ekind`-driven, has no data here to drive it, and lands in
//! `liss_abi::liss_print_list` (Task 8).

use crate::fraction;
use crate::roman::Roman;

/// A byte-oriented output destination. `push` may be called any number of
/// times; every call's bytes are appended in order.
pub trait Sink {
    fn push(&mut self, bytes: &[u8]);
}

impl Sink for String {
    /// Every byte `scriptio` ever pushes comes from a Roman numeral, a Latin
    /// fraction word, or a `&str`/`char` that was already valid UTF-8 — so
    /// this cannot fail in practice. It is written as a checked conversion,
    /// not an unchecked one, because `Sink` is a public trait and nothing
    /// stops a future implementor from feeding it something else.
    fn push(&mut self, bytes: &[u8]) {
        self.push_str(std::str::from_utf8(bytes).expect("scriptio only ever writes valid utf-8"));
    }
}

/// Render a `ni` value: `Roman::display`, unchanged.
pub fn scribe_int(r: &Roman, out: &mut impl Sink) {
    out.push(r.display().as_bytes());
}

/// Render a fractional value: a signed grain count over the given
/// denominator (12 for `mnr`, 288 for `nr`) — `fraction::display`, unchanged.
pub fn scribe_frac(count: &Roman, grain: u64, out: &mut impl Sink) {
    out.push(fraction::display(count, &Roman::from_u64(grain)).as_bytes());
}

/// Render `filum` text: identity.
pub fn scribe_text(s: &str, out: &mut impl Sink) {
    out.push(s.as_bytes());
}

/// Render a `littera`: its one character, UTF-8 encoded.
pub fn scribe_char(c: char, out: &mut impl Sink) {
    let mut buf = [0u8; 4];
    out.push(c.encode_utf8(&mut buf).as_bytes());
}

/// Render a boolean: `verus` / `falsus`.
pub fn scribe_bool(b: bool, out: &mut impl Sink) {
    out.push(if b { b"verus" } else { b"falsus" });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rendered(f: impl FnOnce(&mut String)) -> String {
        let mut s = String::new();
        f(&mut s);
        s
    }

    #[test]
    fn scribe_int_matches_roman_display() {
        let r = Roman::from_i64(-25);
        assert_eq!(rendered(|s| scribe_int(&r, s)), r.display());
    }

    #[test]
    fn scribe_frac_matches_fraction_display_both_grains() {
        let c = Roman::from_i64(16);
        assert_eq!(
            rendered(|s| scribe_frac(&c, 12, s)),
            fraction::display(&c, &fraction::uncia_grain())
        );
        assert_eq!(
            rendered(|s| scribe_frac(&c, 288, s)),
            fraction::display(&c, &fraction::scripulum_grain())
        );
    }

    #[test]
    fn scribe_text_is_identity() {
        assert_eq!(rendered(|s| scribe_text("Lingua Serpentis", s)), "Lingua Serpentis");
    }

    #[test]
    fn scribe_char_encodes_utf8() {
        assert_eq!(rendered(|s| scribe_char('\u{1F40D}', s)), "\u{1F40D}");
    }

    #[test]
    fn scribe_bool_both_literals() {
        assert_eq!(rendered(|s| scribe_bool(true, s)), "verus");
        assert_eq!(rendered(|s| scribe_bool(false, s)), "falsus");
    }
}
