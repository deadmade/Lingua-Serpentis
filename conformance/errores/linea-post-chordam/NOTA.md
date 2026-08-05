# Line numbers survive a multi-line string

D-03 is fixed. The string literal on source lines 2–3 contains one newline. The
lexer never counted it, so every token after the closing quote carried a line
number one too small — and the drift is one per embedded newline, with no
bound. The undefined variable is on source line 5; the diagnostic used to say
`linea 4`. It now says `linea 5`.

The literal itself is still attributed to the line it *opens* on, so a
diagnostic about the string points at the string rather than at its last line.
`lexer.rs::a_string_literal_is_attributed_to_the_line_it_opens_on` pins that
half, and this fixture pins the other.

Warnings were attributed the same way, so `Monitum in ..., linea N` drifted
identically and is fixed by the same change.
