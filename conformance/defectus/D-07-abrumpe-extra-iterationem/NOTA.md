# D-07 · `abrumpe` outside a loop is a silent no-op

`abrumpe` sits directly in `initium`'s body, with no enclosing loop. Nothing
rejects it and nothing acts on it: execution continues to the next statement, so
both `scribe` calls run. `continua` behaves identically.

- **Observed:** stdout `I\nII\n`, stderr empty, exit 0.
- **Expected:** an error naming `abrumpe` outside a loop.
- **Cause:** `checker.rs:449` — `StmtKind::Break | StmtKind::Continue => Ok(())`,
  with no loop-depth tracking anywhere in the checker; `interp.rs:225` produces
  `Flow::Break`, and the function-body loop at `interp.rs:166` matches only
  `Flow::Return`, so the `Break` is discarded.

This fixture pins the WRONG behaviour so the suite stays green. When D-07 is
fixed this test will fail — that failure is the fix's acceptance criterion.
Move the fixture to `errores/` with the checker's diagnostic in `nuntii` at that
point.
