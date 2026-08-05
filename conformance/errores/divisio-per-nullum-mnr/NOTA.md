Not a duplicate of `errores/divisio-per-nullum`. `interp.rs`'s `BinOp::Div`
has three arms that say `divisio per nullum` — Int (`:317`), Float/`mnr`
(`:321`) and Double/`nr` (`:325`). The plain fixture provokes the Int arm
only; this one provokes the `mnr` arm, where the zero divisor is detected by
`fraction::div` returning `None` rather than by a `b.is_zero()` guard.

The three arms are worded identically, so the `catalogus` snapshot reports all
three `ambiguous` and cannot tell them apart by text. That is why this case
has to exist as a program rather than as a line in the catalogue.
