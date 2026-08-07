Ports `interp.rs`'s `index_read_is_one_based`: index `°I` reads the first
element and index `°III` reads the third (last) element of a three-element
list — lists are 1-based, not 0-based.
