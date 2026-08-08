**The root `initium` may fall off its end, and then the program exits 0.**

Every other function in the language must return on every path — that is D-01,
and the checker raises `munus '{}' non in omni via reddit` for a violation. The
root `initium` is the single exemption, licensed by the packages design §4.3,
and a package function that merely happens to be named `initium` does not get it.

## Why this fixture exists

`interp.rs::falling_off_initium_exits_zero` was the only end-to-end assertion of
the exemption, and it went with the interpreter. What was left is
`checker.rs`'s side — that such a program *type-checks* — and the IR snapshot's
— that lowering appends a `Return(0)`. Neither observes the exit status, and the
exit status is the whole of what §4.3 licenses.

Task 19's re-homing table originally certified this row to
`currunt/sententia-post-redditum`. That was wrong: that program has an explicit
`redde` and pins something else entirely — that a statement *after* `redde` is
legal. Its own `NOTA.md` says so. The gap was found in review and this is it
closed.

## What it pins

Three things at once, and only the last is new: the program runs, prints, and
**exits 0 with an empty stderr** — no `non in omni via reddit`, no warning about
a missing return, and none of the truncated-status note (D-10) that a nonzero
fall-through value would produce.

Two frames deep would test nothing extra: the exemption is about the *root*
namespace's `initium` and nothing else, so the minimal program is the honest one.
