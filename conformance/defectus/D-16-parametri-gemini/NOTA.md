# D-16 · Duplicate parameter names are accepted; the last binding wins

`munus f capit (ni a, voc a)` declares the same parameter name twice with two
different types. Nothing rejects it. Both the checker and the interpreter store
parameters in a `HashMap` keyed by name, so the second `a` overwrites the first:
the body's `redde a` sees the `voc`, and the `ni` argument is unreachable.

- **Observed:** stdout `secunda\n`, stderr empty, exit 0.
- **Expected:** an error naming the duplicated parameter `a`.
- **Cause:** three last-wins sites. Each line cited is the statement that must
  change, not the enclosing function:
  `parser.rs:305` — `params.push((ty, pname))`, no uniqueness check;
  `checker.rs:39` — the `insert` inside `declare`, reached once per parameter
  from `checker.rs:404`;
  `interp.rs:154` — `frame.insert(pname.clone(), v)`, inside the loop opened at
  `interp.rs:153`.

This fixture pins the WRONG behaviour so the suite stays green. When D-16 is
fixed this test will fail — that failure is the fix's acceptance criterion.
Move the fixture to `errores/` with the duplicate-parameter diagnostic in
`nuntii` at that point.
