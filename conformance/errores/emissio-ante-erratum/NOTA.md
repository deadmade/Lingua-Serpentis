D-08 is fixed. `scribe` output accumulates in `Interp::out`, and
`execute_program` returned it only on the Ok arm — so `run_loaded`'s `?`
discarded it and `render` emitted an empty stdout for any error. This program
printed `ante`, then divided by zero, and the `ante` was thrown away.

Output the program provably produced is not the error's to discard. A failure
now carries its partial stdout, and `render` writes it to the real stdout.

The absence of an `emissio` file used to be the pinned defect; its presence,
containing `ante`, is now the pinned fix.
