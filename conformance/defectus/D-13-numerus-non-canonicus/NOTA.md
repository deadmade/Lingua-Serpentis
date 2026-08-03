# D-13 · Roman numeral form is never validated

`°IIX`, `°VV` and `°IM` are not Roman numerals. The parser accepts all three by
summing and subtracting letter values with no check that the form is canonical:
`IIX` = 10, `VV` = 10, `IM` = 999. The program runs and prints their sum.

- **Observed:** stdout `MXIX\n` (10 + 10 + 999 = 1019), stderr empty, exit 0.
- **Expected:** `numerus Romanus invalidus` for each of the three literals.
- **Cause:** `roman.rs:204` — `convert_single`'s loop applies the subtractive
  rule to each letter positionally (`if next_bigger { sub } else { add }`) and
  accepts anything built from `I V X L C D M`. It never constrains repetition
  counts, never restricts which subtractive pairs are legal, and never
  re-renders the result to compare it with the input.

This fixture pins the WRONG behaviour so the suite stays green. When D-13 is
fixed this test will fail — that failure is the fix's acceptance criterion.
Move the fixture to `errores/` with the rejection message in `nuntii` at that
point. Note that form validation is a language change, explicitly out of scope
for the conformance suite design (§2, Non-Goals).
