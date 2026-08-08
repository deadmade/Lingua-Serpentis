**Five thousand levels of recursion, which the language could not do
yesterday.**

This case exists to prove a negative: that §8's guard does **not** fire
spuriously. `errores/acervus-exhaustus` asserts that a runaway recursion is
caught; on its own, that assertion is satisfied just as well by a guard that
fires at level three. This is the other side, and the two must be read together.

Five thousand is chosen against both bounds. It is two orders of magnitude past
`interp.rs`'s old `VOCATIONES_MAXIMAE = 64` — so this program was simply
impossible before spec §3.3 removed the cap — and past the ~2,000 the
interpreter managed even before the cap existed. It is also comfortably inside
the 40,000–80,000 levels a default 8 MiB stack allows, so it is not a test of
how close to the edge the compiler can be driven.

`numera` returns a value rather than falling off the end, so the recursion is
load-bearing: the printed `V̅` is 5,000, and it is only correct if all five
thousand frames returned and their results were summed on the way back out. A
guard that unwound early, or a return path that lost frames, prints something
else rather than nothing.

`cli.rs::deep_recursion_runs_because_there_is_no_limit` makes the same point at
10,000 levels, outside the corpus, where a subprocess test can afford it.
