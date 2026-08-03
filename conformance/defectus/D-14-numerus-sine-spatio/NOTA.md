# D-14 · `°X/°II` lexes as one numeral; whitespace is load-bearing

`/` is part of a numeral's body because a fraction literal is written
`°V:I/II`. So in `°X/°II` the lexer swallows the division sign into the first
numeral, produces the lexeme `X/`, and fails on it. Written `°X / °II` the same
program prints `V` and exits 0. Nothing tells the user that the space mattered.

The same trap exists for `.`: `°V.summa` reports `numerus Romanus vacuus`
(`roman.rs:193`), because `.` is also numeral body — it is the multiplying
separator in `°XXX.M`.

- **Observed:** stderr `Error in 'principale.liss', linea 2: numerus Romanus invalidus: 'X/'`, exit 1.
- **Expected:** either `°X/°II` lexes as division and prints `V`, or the
  diagnostic explains that `/` was read as part of the numeral.
- **Cause:** `lexer.rs:85` — `is_roman_body` accepts `'.' | ':' | '/'`, and the
  `°` arm at `lexer.rs:34` consumes greedily to the first non-body character
  with no backtracking.

This fixture pins the WRONG behaviour so the suite stays green. When D-14 is
fixed this test will fail — that failure is the fix's acceptance criterion.
Move the fixture to `currunt/` with `emissio` of `V` (or to `errores/` with the
improved diagnostic) at that point.
