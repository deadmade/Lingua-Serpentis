# D-01 · No all-paths-return analysis; the type system is unsound

`nomen` is declared `reddit voc` and its only `redde` sits inside `si (falsus)`,
so no `redde` ever runs. The call still succeeds: the interpreter fabricates an
`Int` zero for the missing return, and `scribe` prints it as `nullus`. A
checker-approved program has produced a value of the wrong type.

- **Observed:** stdout `nullus\n`, stderr empty, exit 0.
- **Expected:** a checker error naming the missing return in `nomen`.
- **Cause:** `interp.rs:168` ends every call body with
  `Ok(Value::Int(Roman::zero()))` regardless of the declared return type;
  `checker.rs:412` only type-checks the `redde` statements actually written and
  never asks whether one is reachable on every path.

This fixture pins the WRONG behaviour so the suite stays green. When D-01 is
fixed this test will fail — that failure is the fix's acceptance criterion.
Move the fixture to `errores/` with the checker's diagnostic in `nuntii` at that
point. Any fix must special-case `initium`, whose fall-through the packages
design licenses deliberately and which `interp.rs::falling_off_initium_exits_zero`
pins as correct.
