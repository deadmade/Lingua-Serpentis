**`vel` evaluates both operands, always** (spec §3.5), and this fixture is what
holds it to that.

## Why the corpus needed this

`interp.rs:131-135` evaluated both sides of a binary expression and *then*
dispatched on the operator, so `°V / nullus vel verus` was a runtime error and
not `verus`. Compiled code does the same, deliberately: emitting the branch a
compiler writer's reflexes reach for would be a silent semantic change to every
program that relies on it. Spec §3.5 records this as a decision rather than a
footnote because it is the single easiest divergence to introduce by accident in
the whole project.

For most of the milestone that argument ended "…and the corpus is blessed
against it". **It was not.** Not one of the 124 `.liss` programs under
`conformance/` and `examples/` used `et` or `vel` as an operator — the only
matches for those words were the `et` inside a rendered fraction (`I et triens`)
and Latin prose in comments. The rule rested entirely on two `codegen` unit
tests, and three source comments described those tests as a redundant
double-check of a corpus guard that did not exist. Anyone tidying them away on
that basis would have left the invariant with no guard at all.

## Why it looks like this

**Short-circuiting is only observable through a fault in the right operand.**
`A vel B` evaluates `A` first either way, so a fault on the left cannot tell the
two readings apart; and the language has no side-effecting expression — `scribe`
is a statement — so a right operand that merely *ran* would leave no trace. A
division by zero is the one thing an expression can do that is visible from
outside.

The left operand is `verus`, which is exactly the value a short-circuiting `vel`
would stop at. So:

- **evaluating both** (correct): the division raises, nothing is printed, exit 1
  — what this fixture records;
- **short-circuiting** (wrong): `verus` is printed and the program exits 0.

The two outcomes differ on all three streams, which is why this belongs in the
corpus rather than in a test that checks one of them.

## Why `errores/` and not `currunt/`

Because it fails. The corpus sorts by what a program *does*, and this one raises
a diagnostic and exits 1. Filing it under `currunt/` would be filing it by what
it is *for* rather than by what it is, and every other fixture here would then
be readable two ways.

`errores/et-utrumque-latus` is the same shape for `et`, whose left operand is
`falsus` — the value a short-circuiting `et` would stop at.
