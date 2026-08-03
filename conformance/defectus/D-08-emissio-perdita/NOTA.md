# D-08 · A runtime error discards everything already printed

`scribe("ante")` runs before the division, but stdout is empty: `scribe`
accumulates into `Interp::out` and `lib.rs`'s `Err` arm never returns it.

- **Observed:** stdout empty, stderr the division error, exit 1.
- **Expected:** stdout `ante\n`, then the error.
- **Cause:** `interp.rs` accumulates output in `Interp::out`; `run_loaded`
  (`lib.rs:44`) discards it on the error path.

This fixture pins the WRONG behaviour so the suite stays green. When D-08 is
fixed this test will fail — that failure is the fix's acceptance criterion.
Move the fixture to `errores/` with `emissio` containing `ante` at that point.
