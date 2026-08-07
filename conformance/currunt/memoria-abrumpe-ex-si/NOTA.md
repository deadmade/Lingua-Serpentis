# `abrumpe` and `continua` out of a nested `si`, past two owned scopes

Spec §7.3.5 and §7.3.6. Both jumps leave a `si` arm *and* the loop body, and
both must free **from the current block up to and including the loop body** —
and **not** the `dum` header.

The header owns `marca`, a `voc`, which is what makes the rule observable:

- If `abrumpe` also freed `header.declares`, the teardown §7.3.5 emits **once,
  after the exit block** would free `marca` a second time and `abolere`'s
  cleared magic would report `defectus internus: memoria violata est`, exit 1.
- If the post-exit teardown were omitted instead, `marca` leaks and §7.5 turns
  this into `memoria non liberata est`, exit 1.
- If `continua` freed nothing, the four `nota`s allocated on the iterations that
  reach it leak the same way.

The two jumps also differ in target, and the trace pins it: `continua` must
reach the **step** clause (`i = i + °I`), not the condition, or `i` never
advances past II and the loop does not terminate. `abrumpe` must reach the exit
block, where the header teardown lives.

Trace, one line per iteration — `nota` is allocated and freed on every one of
the six, `marca` exactly once:

| `i` | what happens | `acc` |
|---|---|---|
| 0 | appends | `n\|` |
| I | appends | `n\|n\|` |
| II | `continua` — the body's `nota` is freed by the jump, the step still runs | `n\|n\|` |
| III | appends | `n\|n\|n\|` |
| IV | appends (`IV maior_est IV` is false) | `n\|n\|n\|n\|` |
| V | `abrumpe` — frees `nota`, leaves `marca` to the exit block | — |
