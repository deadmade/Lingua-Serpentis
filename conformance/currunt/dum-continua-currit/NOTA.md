Ports `interp.rs`'s `continue_skips_iteration`: `continua` inside a `dum`
loop skips the rest of that iteration's body but the loop's own update clause
still runs.
