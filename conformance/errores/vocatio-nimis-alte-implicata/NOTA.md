D-04 is fixed. `Interp::call` recursed through `exec` and `eval` back into
itself with no depth counter and no growable stack, so unbounded recursion
killed the process with SIGABRT and exit 134. Like D-05 this could not be a
fixture before the fix: the abort takes the test process with it.

Measured before the fix: a self-recursive function was clean at 1000 levels
and aborted at 2000 — a per-level cost close to the parser's rather than the
lighter one a first estimate assumed, which is why the limit is what it is and not
higher. `fac(°XX)` still runs, and so does any recursion inside the limit.

`cli.rs::unbounded_recursion_is_a_clean_latin_error` covered this as an
`#[ignore]`d, aspirational test. It is now an ordinary passing one.
