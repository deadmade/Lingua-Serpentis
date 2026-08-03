# D-03 · Line numbers are wrong after a multi-line string

The string literal on source lines 2–3 contains one newline. The lexer never
counts it, so every token after the closing quote carries a line number one too
small. The undefined variable is on source line 5; the diagnostic says `linea 4`.

- **Observed:** stderr `Error in 'principale.liss', linea 4: variabilis ignota: 'ignota'`, exit 1.
- **Expected:** the same message with `linea 5`.
- **Cause:** `lexer.rs:46` — the string-scanning loop
  `while i < chars.len() && chars[i] != '"' { i += 1; }` advances over embedded
  newlines without touching `line`. Warnings are attributed the same way, so
  `Monitum in ..., linea N` drifts too.

This fixture pins the WRONG behaviour so the suite stays green. When D-03 is
fixed this test will fail — that failure is the fix's acceptance criterion.
Move the fixture to `errores/` with `linea 5` in `nuntii` at that point.
