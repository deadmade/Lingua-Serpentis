# `redde` from inside `si` inside `dum` inside `indem`

Spec §7.3.4 — **`redde` evaluates, then frees every slot of every enclosing
block, innermost first** — with five nested `Block::declares` between the
`redde` and the function's signature:

| depth | block | owns |
|---|---|---|
| 5 | the `si` arm | — |
| 4 | the `dum` body | `intus`, a `voc` |
| 3 | the **`dum` header** | `nota`, a `voc` |
| 2 | the `indem` body | — |
| 1 | the function body, which *is* the parameter frame | `k`, and `semen`, a `voc` **parameter** |

Three separate rules are load-bearing here and each has its own failure:

- **The `dum` header is an ordinary enclosing scope for `redde`** (§7.3.5's
  exception). §7.3.5 defers the header teardown to *after the exit block*, which
  is sound for `abrumpe` and `continua` because both reach it — and wrong for
  `redde`, which leaves the function without ever reaching it. `nota` is a
  `voc` precisely so the leak is visible: with the natural
  `dum (ni j = °I; …)` the header slot is boxed but the counter would still
  see it, whereas an unowned header would hide the bug entirely. A header
  outside the walk is one leaked `LissText` and `memoria non liberata est`,
  exit 1.
- **The parameter frame must not be swept a second time** off `Func::params`
  (§7.3.4). `semen` is slot 0 *and* the first entry of the body block's
  `declares`, so a codegen that also frees `0..params` double-frees it and
  `abolere`'s cleared magic reports `defectus internus: memoria violata est: verificatio`.
- **The returned value is never among the freed.** `semen + nota + intus`
  consumes three copies and yields a fresh `LissText`; the three slots die
  immediately afterwards.

The `indem` is there so the `redde` crosses a loop that is *not* the one whose
header owns anything — the walk is over blocks, not over loops.
