D-26 is fixed. A `dum` init clause used to be read with `statement()`, so a
`munus` parsed there. The checker checked it and dropped it — nothing could call
it — and the loop ran, printing `nullus` and `I`, exit 0.

A `dum` init clause is now a declaration or an assignment, and nothing else
parses. The program opens with a comment so the error does not land on line 1.
