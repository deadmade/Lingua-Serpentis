`index seriei 'ni' esse debet` has two emission sites in the checker, worded
identically: `checker.rs:195` guards the *expression* form `xs[i]`, and
`checker.rs:437` guards the `IndexAssign` statement form `xs[i] = v`. They are
separate code paths with separate `resolve_expr` calls.

`errores/index-seriei-non-ni` provokes `:195`. This case provokes `:437`. The
`catalogus` snapshot reports both `ambiguous`, because their text is the same;
only these two programs distinguish them.
