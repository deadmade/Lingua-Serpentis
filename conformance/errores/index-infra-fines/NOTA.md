Ports the `bad_lo` half of `interp.rs`'s `index_out_of_range_errors`: index
`nullus` (0) is below the 1-based floor and raises the same `index extra
fines` diagnostic as an index past the end — see `errores/index-extra-fines`
for the high end, ported from the same unit test's `bad_hi` half.
