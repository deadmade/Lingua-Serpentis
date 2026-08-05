D-16 is fixed. `munus f capit (ni a, voc a)` used to be accepted, and the last
binding won at all three layers — the parser pushed both, the checker and the
interpreter each `insert`ed into a map. So `f(°I, "secunda")` printed `secunda`,
exit 0: the body saw the `voc`, and the `ni` argument could not be named at all.

The register lists three sites that "must change together". Only the parser
needed to: once it rejects the declaration, neither of the other two can ever
see a duplicate, and guarding them as well would have manufactured two more
unreachable diagnostics — the very thing D-24 and D-25 are about.

The program opens with a comment so the error does not land on line 1, per the
suite's rule for `errores/` cases.
