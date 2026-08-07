# Twenty iterations that each allocate and free a `voc`

Spec §7.3.2 across a back edge, repeated enough times that a *drift* shows
rather than only an outright omission.

Each iteration allocates two `LissText`s — `pars` by its `Let`, and the
concatenation `s + pars` — and frees two: the old `s`, inside §7.3.3's
evaluate → free old → store, and `pars`, by the loop body's scope exit. Twenty
iterations, forty allocations, forty frees, and the count is zero at exit.

The count is what makes the number of iterations worth anything. One iteration
proves the frees exist; twenty prove they are **balanced** — a teardown that ran
once for the whole loop instead of once per iteration would leak nineteen
`LissText`s rather than one, and a `pars` freed on the back edge *and* again at
the exit block would fail on the second iteration rather than only on some
unlucky path.

The `dum` header here owns nothing a `voc` counter can see (`ni i` is boxed, and
freed once by the teardown after the exit block), which is the counterpart to
`memoria-caput-dum-redditum`: the same header rule with the loop leaving by its
own false edge instead of by `redde`.
