Ports `interp.rs`'s `list_literal_prints`: a non-empty list literal renders as
`[elem, elem, ...]` and an empty list renders as `[]`. Unlike the original
`run()`-based unit test (which bypasses the checker), the full pipeline's
checker cannot infer an empty list literal's element type from no context —
`scribe([])` alone is `typus elementorum seriei vacuae ignotus` — so the empty
case here is written `ni lista xs = []; scribe(xs);`, where the `let`
annotation supplies the type. The rendered behaviour pinned (`[]`) is
identical.
