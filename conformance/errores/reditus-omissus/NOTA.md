D-01 is fixed, and it was the reason the type system was unsound.

Nothing asked whether a declared return was reachable on every path, and
`interp::call` ended every body with `Int(zero)` regardless of the declared
type. So this program — a `reddit voc` function whose only `redde` sits behind
`si (falsus)` — printed `nullus` and exited 0. A checker-approved call to a
`voc` function handed back a number.

The analysis is deliberately conservative: `redde` returns, an `si` returns
when both arms do and an `aut` is present, and nothing else counts. In
particular a loop never counts, because it may run zero times. `indem (verus)`
is not special-cased even though it diverges; Rust and Java only treat such a
loop as diverging when it has no reachable break, and the checker tracks loop
depth but not break targets. Over-rejecting costs a user one extra `redde`.

Only the **root** `initium` is exempt. The packages design §4.3 licenses its
fall-through and `interp.rs::falling_off_initium_exits_zero` pins it; a package
function that merely shares the name is ordinary, and a test pins that too.

The program opens with a comment so the error does not land on line 1, per the
suite's rule for `errores/` cases.

Knock-on worth knowing: this fix makes four interpreter diagnostics
unreachable again — `subscriptio invalida`, `condicio non est veritas`,
`operatio binaria invalida` and `comparatio invalida`. D-01's own note observed
that the defect had *promoted* them from unreachable to reachable; fixing it
demotes them. They read UNCOVERED in the catalogue, which is now the honest
label rather than a gap.
