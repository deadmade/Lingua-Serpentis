**`et` evaluates both operands, always** (spec §3.5). The `et` half of the pair;
`errores/vel-utrumque-latus` carries the full argument for why this shape is the
only observable one and why the corpus had no coverage of the rule until now.

The left operand is `falsus` — the value a short-circuiting `et` would stop at,
returning `falsus` without touching the right side. Evaluating both raises
`divisio per nullum` instead, which is what is recorded here.

Both halves are needed. A codegen that short-circuited only one of the two
operators would be caught by exactly one of these fixtures, and the two are
emitted by separate arms.
