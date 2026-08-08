# Assignment in a loop: `s = s + s` three times over

Spec §7.3.3 — **`Assign` is evaluate → free old → store** — inside a loop body,
with an owned (`voc`) left-hand side that is also its own right-hand operand.

Straight-line `s = s + s` was already pinned by Task 12. What a loop adds is
*repetition*: the slot is written three times, and the free of the old value
happens three times against a pointer the previous iteration stored. Getting the
order wrong is a use-after-free that `liss_text_concat`'s magic check turns into
`defectus internus: memoria violata est: verificatio`, exit 1; freeing nothing at all is
three leaked `LissText`s and `memoria non liberata est`, exit 1 (spec §7.5).
This program exits 0 with the live-allocation count at zero.

The `indem` also pins §7.3.2 across a back edge: `i` and `s` belong to the
*function body* block, not the loop body, so the loop must **not** tear them
down on each iteration.
