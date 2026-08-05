D-05 is fixed. Recursive descent re-entered itself once per nesting level with
nothing counting, so a deeply nested source aborted the process: SIGABRT, exit
134, outside the error model entirely and not something an embedder can
survive. It used to be fixture-shaped only in theory — the abort killed the
test process, and `catch_unwind` does not catch it — so this case could not
exist until the defect was fixed. That it exists at all is the fix.

The register cites one recursion site, `primary`'s LParen arm. There are
three, and they abort independently. Measured on this debug build before the
fix: nested parentheses clean at 1000 and aborting at 2000, nested `si` blocks
the same, stacked unary `-` clean at 2000 and aborting at 4000. A counter in
the expression ladder alone would have left the other two live for the
mutation fuzzer to find.

100 parentheses, comfortably past the limit and far short of any depth
that used to abort.
