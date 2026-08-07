Ports `interp.rs`'s `index_write_out_of_range_errors`: writing through an
out-of-range index on the assignment form `xs[i] = v` raises `index extra
fines`, the same diagnostic and bound check as the reading form.
