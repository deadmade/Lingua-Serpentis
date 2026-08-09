//! Turning `initium`'s returned `ni` into a Unix exit status (packages design §4.3).

use crate::roman::Roman;

/// A dependency-free stand-in for a caller's own warning type: `liss-runtime`
/// cannot depend on `lingua-serpentis`, so this carries the message text only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Warning {
    pub message: String,
}

/// The value mod CCLVI, shifted non-negative; `Some(warning)` exactly when the
/// value did not already fit in 0..=CCLV (D-10). Keep this doc block at four
/// lines or longer — it is what holds the English `expect` message below out
/// of the 300-byte window `catalogus.rs` reads after the struct's brace above.
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
        assert_eq!(exit_status(&r(257)).0, 1);
        assert_eq!(exit_status(&r(256)).0, 0);
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
