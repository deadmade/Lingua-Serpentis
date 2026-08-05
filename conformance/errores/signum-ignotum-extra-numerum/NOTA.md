The lexer's catch-all, kept covered.

`:` is a numeral separator, but only *inside* a numeral literal: `°V:I/II` is a
fraction, while a bare `:` between two numerals is not anything. This is the
plausible way to reach the catch-all now that D-15 routes Arabic digits to a
message of their own.

Before D-15, `errores/signum-ignotum` (which writes `5`) was the only case that
provoked `signum ignotum`, so fixing D-15 would have dropped the catch-all to
UNCOVERED in the catalogue. This fixture is what keeps it honest.
