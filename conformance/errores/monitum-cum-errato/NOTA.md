D-09 is fixed. The Latinity warnings were computed and then dropped by
`run_loaded`'s `?` on both the checker and the interpreter path, so a program
with two problems only ever reported one. Removing the division from this
program produced the `Monitum`; leaving it in silenced it.

Warnings now travel on the failure and are rendered before it — chronological,
and matching the success path's ordering. Linting also moved ahead of the
checker, so a program that fails to type-check still reports its style
problems; a lint that only appears once the program compiles is one nobody
reads.

The one case where warnings genuinely cannot survive is a lex, parse or
structural failure: there is no parsed program to lint yet.
