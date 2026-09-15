Ports `interp.rs`'s `unbounded_arithmetic_no_longer_overflows`: `°MMMCMXCIX`
(3999) to the sixth power is ~4.09e21, past `i64::MAX` (~9.22e18). The
original unit test only asserted `execute(..).is_ok()`; `Roman` is
arbitrary-precision so this both runs and renders without overflow or panic.

Twenty-two digits is inside the XXXVIII-digit cap D-20 set on a stored number; the fixture name is
history, and the program runs exactly as it did.
