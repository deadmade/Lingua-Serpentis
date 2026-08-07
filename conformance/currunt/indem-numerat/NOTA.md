Ports `interp.rs`'s `while_loop_counts`: an `indem` loop (`while`) runs its
body once per iteration and the counter/accumulator update inside the body is
visible to the next iteration's condition test.
