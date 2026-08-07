# A `dum` header holding a `voc`, with `redde` as the loop's only exit

The narrowest statement of spec §7.3.5's exception, and the reason
`StmtKind::For` carries its header as a **`Block`** rather than a bare `Stmt`.

`dum (verus; …)` has no false edge a program can take: the only way out of this
loop is the `redde` inside it. §7.3.5 says the header teardown is emitted once
**after the exit block** — and this program never reaches the exit block. So the
header's `pars` can only be freed by §7.3.4's walk over *every enclosing
`Block`*, which reaches it exactly because the checker put the `dum` variable in
a `Block::declares` like any other.

**The header variable is a `voc` deliberately.** With the natural
`dum (ni j = °I; …)` the same leak exists and the same counter sees it — but the
temptation to write the header as a bare statement, and the whole of the spec's
argument for the `Block`, is about a slot the teardown can *miss*. A `voc` is
the shape that makes the failure a single, obvious `LissText`:
`memoria non liberata est`, exit 1 (spec §7.5).

`redde "numquam"` after the loop is unreachable and the checker still demands
it — a loop never guarantees a return (`checker.rs`'s `always_returns`) — so it
also pins that the code emitted into the exit block, header teardown included,
is well-formed even where nothing runs it.
