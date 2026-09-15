# Numerus Romanus non canonicus

D-13 is fixed. `°IIX`, `°VV` and `°IM` are not Roman numerals, and the parser
used to accept all three by summing and subtracting letter values positionally:
`IIX` read as 10, `VV` as 10, `IM` as 999. This program printed `MXIX` and
exited 0.

Every chunk of a literal, split at `.`, `:` and `/`, must now be written the
way its own value prints: classical form, 1..=3999. `convert_single` compares
each chunk with the display of the value it computed and rejects it with
`numerus Romanus non canonicus`, quoting the chunk.

The lexer stops at its first error, so only the first bad literal is reported:
`'IIX'` is named, and `°VV` and `°IM` are not.
