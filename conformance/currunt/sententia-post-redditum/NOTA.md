# A statement after `redde` is legal, and prints nothing

The checker's all-paths analysis asks whether **any** statement returns, not
whether the last one does — `checker.rs:128-131` says so in as many words:

> `any`, not "the last statement": a `redde` followed by unreachable code
> still returns, and this language has no dead-code lint to disagree with.

So this program type-checks, and the interpreter has always run it to an empty
stdout and exit 0: `exec` returns `Flow::Return` at the first statement and
never reaches the second.

**The compiler had no fixture holding it to that.** Codegen walked every
statement in a block unconditionally, so `redde` filled the Cranelift block and
`scribe` then tried to add an instruction to a filled one — a debug panic
(`you cannot add an instruction to a block already filled`) and, worse, a
release `Err` telling the user their perfectly legal program had triggered
`defectus internus: munus componi non potuit`. `Corpus::blocus` now stops at the
first terminator, which is the same `terminatus()` it already consulted one line
later for the scope teardown.

Nothing in `conformance/` or `examples/` had this shape, so neither Task 12's
checkpoint nor Task 17's whole-corpus differential would have caught it. It is
here because after Task 19 the corpus is the only oracle there is.

`abrumpe` and `continua` fill a block exactly the same way, so Task 13 inherits
the guard rather than the bug.

This case runs: empty stdout, empty stderr, exit 0.
