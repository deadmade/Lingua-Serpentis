# D-16 · Duplicate parameter names are accepted; the last binding wins

`munus f capit (ni a, voc a)` declares the same parameter name twice with two
different types. Nothing rejects it. Both the checker and the interpreter store
parameters in a `HashMap` keyed by name, so the second `a` overwrites the first:
the body's `redde a` sees the `voc`, and the `ni` argument is unreachable.

- **Observed:** stdout `secunda\n`, stderr empty, exit 0.
- **Expected:** an error naming the duplicated parameter `a`.
- **Cause:** `parser.rs:305` pushes each `(ty, pname)` with no uniqueness check;
  `checker.rs:404` declares them with `declare` (`checker.rs:38`, a
  `HashMap::insert`); `interp.rs:153` binds them with `frame.insert`. Every one
  of the three is last-wins.

This fixture pins the WRONG behaviour so the suite stays green. When D-16 is
fixed this test will fail — that failure is the fix's acceptance criterion.
Move the fixture to `errores/` with the duplicate-parameter diagnostic in
`nuntii` at that point.
