D-26 is fixed. A `dum` init clause used to be read with `statement()`, so any
statement parsed there, a `si` included. Its `redde °I` returned from `initium`
out of the loop's header: no output, no diagnostic, exit 1.

A `dum` init clause is now a declaration or an assignment, and nothing else
parses. The program opens with a comment so the error does not land on line 1.
