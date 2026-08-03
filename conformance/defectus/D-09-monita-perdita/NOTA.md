# D-09 · Any error discards all Latinity warnings

`counter` is not Latin, so `lint_program` produces
`Monitum in 'principale.liss', linea 2: nomen 'counter' fortasse non Latinum est`.
The same program then divides by zero. `nuntii` holds the error and nothing
else: the warning was computed and thrown away.

Delete the two lines that force the division by zero and the warning appears —
that is the control this fixture is arguing against.

- **Observed:** stdout empty, stderr only
  `Error in 'principale.liss', linea 4: divisio per nullum`, exit 1.
- **Expected:** the `Monitum` line first, then the error.
- **Cause:** `lib.rs:43` computes `warnings`, but `run_loaded` returns them only
  on the `Ok` arm at `lib.rs:45`; the `?` on `lib.rs:44` drops them, as does the
  `?` on `lib.rs:42` for a checker error.

This fixture pins the WRONG behaviour so the suite stays green. When D-09 is
fixed this test will fail — that failure is the fix's acceptance criterion.
Move the fixture to `errores/` with the `Monitum` line preceding the error in
`nuntii` at that point.
