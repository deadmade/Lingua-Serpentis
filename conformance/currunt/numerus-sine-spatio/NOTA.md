# `°X/°II` divides — whitespace is no longer load-bearing

D-14 is fixed. `/` is part of a numeral's body because a fraction literal is
written `°V:I/II`, and the `°` scan is greedy — so `°X/°II` used to swallow the
division sign into the first numeral, produce the lexeme `X/`, and fail on it,
while the identical `°X / °II` printed `V` and exited 0. Nothing told the user
that the space mattered. The same trap held for `.`, the multiplying separator
in `°XXX.M`: `°V.summa` reported `numerus Romanus vacuus`.

The lexer now gives back any trailing `.`, `:` or `/`. That is sound rather
than merely convenient: `parse_roman` splits on those separators and every leaf
goes to `convert_single`, which rejects the empty string — so a lexeme ending in
a separator can never parse, and the separator provably belongs to the next
token.

Retreating one *character* at a time, which D-14's register entry implies,
would have been wrong: `°I:I` would fall back to `°I` and leave a stray
`Ident("I")`, silently killing `errores/fractio-sine-denominatore`. Both that
case and `°` alone (`numerus Romanus vacuus`) are pinned by unit tests in
`lexer.rs` alongside this fixture.

This case now runs: stdout `V`, empty stderr, exit 0.
