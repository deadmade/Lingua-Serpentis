//! Turning `initium`'s returned `ni` into a Unix exit status. Moved from
//! `interp.rs` unchanged in behaviour; see the packages design §4.3 for why
//! truncation is specified rather than a bug.

use crate::roman::Roman;

/// A minimal, dependency-free stand-in for a caller's own warning type.
/// `liss-runtime` cannot depend on `lingua-serpentis` (or anything else — see
/// the crate doc), so this carries only the message text; a caller with a
/// richer warning type (file, line) wraps it. `lingua-serpentis::interp`
/// does exactly that at its one call site.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Warning {
    pub message: String,
}

/// Unix keeps an exit status's low 8 bits: the value mod CCLVI, shifted
/// non-negative (a negative remainder gains one modulus).
///
/// The truncation is deliberate and specified (packages design §4.3), so it
/// stays. What was missing is any sign that it happened — `redde °CCC` exited
/// 44 in silence, and `redde °CCLVI` exited 0, which reads as success. The
/// second return value is that sign (D-10): `Some(warning)` exactly when the
/// returned value did not already fit in 0..=CCLV.
pub fn exit_status(r: &Roman) -> (u8, Option<Warning>) {
    let rem = r.div_rem(&Roman::from_u64(256)).expect("CCLVI is not zero").1;
    let x = rem.to_i64().expect("|rem| < 256 fits i64");
    let code = ((x + 256) % 256) as u8;
    let truncated = *r != Roman::from_u64(code as u64);
    let warning = truncated.then(|| Warning {
        message: format!(
            "reditus 'initium' extra fines status exitus est; ad {} redactus",
            Roman::from_u64(code as u64).display()
        ),
    });
    (code, warning)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(n: i64) -> Roman {
        Roman::from_i64(n)
    }

    #[test]
    fn exit_status_is_the_return_mod_256() {
        assert_eq!(exit_status(&r(257)).0, 1); // 257 mod 256
        assert_eq!(exit_status(&r(256)).0, 0); // 256 mod 256
        assert_eq!(exit_status(&r(5)).0, 5);
    }

    #[test]
    fn negative_return_wraps_like_unix() {
        assert_eq!(exit_status(&r(-1)).0, 255);
    }

    #[test]
    fn truncation_warns_only_when_it_happens() {
        let (code, w) = exit_status(&r(300));
        assert_eq!(code, 44);
        assert_eq!(
            w.unwrap().message,
            "reditus 'initium' extra fines status exitus est; ad XLIV redactus"
        );
        assert!(exit_status(&r(5)).1.is_none());
    }
}
