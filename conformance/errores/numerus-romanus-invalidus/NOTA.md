The program used to be `ni x = °X/;`, which reached `roman.rs`'s "invalid
numeral" arm only because the lexer swallowed the trailing `/` into the
numeral. Fixing D-14 gives that separator back, so `°X/;` now lexes as `°X`
`/` `;` and fails in the parser with `expressio expectata` — a different
diagnostic, belonging to a different fixture.

`°I:I/II/III` reaches the same arm on its own merits: the fraction part splits
on the first `/` only, so the denominator is the literal `II/III`, which is
not a numeral. Without this rewrite `roman.rs`'s `numerus Romanus invalidus`
would have quietly dropped to UNCOVERED in the catalogue.
