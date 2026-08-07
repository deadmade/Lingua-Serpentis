Ports `interp.rs`'s `unbounded_arithmetic_no_longer_overflows`: `°MMMCMXCIX`
(3999) to the sixth power is ~4.09e21, past `i64::MAX` (~9.22e18). The
original unit test only asserted `execute(..).is_ok()`; `Roman` is
arbitrary-precision so this both runs and renders without overflow or panic.
