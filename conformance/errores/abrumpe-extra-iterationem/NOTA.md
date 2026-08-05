D-07 is fixed. `abrumpe` and `continua` outside a loop used to be accepted
silently: the checker's arm was `StmtKind::Break | StmtKind::Continue => Ok(())`
with no loop tracking anywhere, and the interpreter's function-body loop matches
only `Flow::Return` — so the `Flow::Break` was dropped on the floor and
execution simply carried on. This program printed `I` then `II` and exited 0.

The checker now carries a loop depth, incremented around `indem` and `dum`
bodies only. A `dum` header (`init`, `cond`, `step`) is not the body, and a
function body resets the depth, so neither can smuggle a loose `abrumpe` in.

`continua` gets its own wording rather than sharing this one — two conditions,
two sentences, so the catalogue can tell them apart.
