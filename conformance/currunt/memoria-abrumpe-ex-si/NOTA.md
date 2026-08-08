# `abrumpe` and `continua` out of a nested `si`, past two owned scopes

Spec §7.3.5 and §7.3.6. Both jumps leave a `si` arm *and* the loop body, and
both must free **from the current block up to and including the loop body** —
and **not** the `dum` header, whose teardown §7.3.5 emits once, after the exit
block.

The header owns `marca`, a `voc`. Three distinct mistakes are observable here,
and it is worth being exact about which, because the obvious one is not:

- **The post-exit teardown dropped** — `marca` is never freed, the live count is
  one at exit, and §7.5 gives `memoria non liberata est`, exit 1.
- **`continua` freeing nothing** — the four `nota`s allocated on the iterations
  that reach it leak the same way.
- **The jumps also freeing `header.declares`** — this fails on the **`continua`**
  and not on the `abrumpe`, which is the part worth writing down. Codegen leaves
  every slot it frees **null** (a slot holds a live owned value or null, at every
  moment), so a redundant free is a no-op and an `abrumpe` that over-freed the
  header would be entirely harmless: the loop is over, and the exit block's
  teardown would free null. `continua` is different — it returns to a body that
  still reads `marca` at `acc = acc + nota + marca`, and reading a null slot is
  fatal by design (`verifica`'s null leg, `defectus internus: memoria violata
  est: verificatio`, exit 1). **A version of this fixture with `abrumpe` alone would pass over
  a codegen that gets §7.3.5 wrong.** The `continua` is load-bearing.

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
