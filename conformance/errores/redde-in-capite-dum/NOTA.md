D-26 is fixed. A `dum` init clause used to be read with `statement()`, so any
statement parsed there. This program returned from `initium` out of the loop's
header: no output, no diagnostic, exit 0.

A `dum` init clause is now a declaration or an assignment, and nothing else
parses. The program opens with a comment so the error does not land on line 1.
