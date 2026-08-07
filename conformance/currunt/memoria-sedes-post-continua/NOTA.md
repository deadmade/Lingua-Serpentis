# A `Let` the `continua` skips, in a loop that runs again

**This fixture is a defect the spec's §7.3 did not cover, found while
implementing it.** It is the one shape where "free `block.declares` on scope
exit" frees the *same pointer twice* without any rule being broken.

`s` is declared in the `indem` body, after the `continua`. `Block::declares`
lists every slot bound anywhere in the block, wherever the `Let` sits, so the
`continua` on iteration II frees `s` — and that is correct and necessary, and
spec §5.4's "freeing a null pointer is a no-op" is what makes it harmless on the
*first* such iteration. What it does not cover is the iteration after a `Let`
that **did** run:

| `i` | path | `s` before the teardown | the teardown |
|---|---|---|---|
| I | falls through | null (entry) | frees `"salve"`, slot left **dangling** |
| II | `continua` | the freed pointer from I | frees it **again** |

Deterministically `defectus internus: memoria violata est`, exit 1 —
`abolere` clears the magic before handing the block back, precisely so that a
double free is caught rather than merely probable.

The fix is not a special case, it is the missing half of the invariant the rest
of §7 already relies on: **a slot holds a live owned value or null, at every
moment.** Codegen zero-initialises owned slots at entry for exactly that reason;
a free that leaves the pointer behind breaks it. So `Corpus::libera` now stores
null into each slot it frees, and freeing null is already a no-op. It costs one
`iconst 0` per owned slot per teardown, all of it in the SSA `Variable` layer,
and nothing at machine level on the paths that do not need it.

`facta` counts the iterations that reached the `Let`, so the expected output
also distinguishes a `continua` that jumped to the wrong place: **III**, not IV
(iteration II is skipped) and not a hang.
