`lista` is the language's only recursive type, and this is the case that makes
a deep copy and a deep free disagree if they can.

Three things happen here that no `lista-*` case reaches:

1. **Depth 3.** `ni lista lista lista` is kind `2 | (3 << 3)` = 26, and every
   `_free`/`_copy`/`_eq`/print walk recurses on the *child's own header* rather
   than on `k - GRADUS` arithmetic. A walk that subtracted its way down would
   still be right here; a walk that stopped at one level would not.
2. **A deep copy and a deep free of the same tree.** `scribe(tabula)` is a
   `Load`, which is the language's only unconditional copy site: the whole
   nested structure is copied, printed, and freed by the printer, while the slot
   still owns the original for scope exit to free. If the copy is shallow, the
   free is a double free of the shared children; if the free is shallow, the
   children leak. Either way this case exits nonzero — the first as
   `defectus internus: memoria violata est: verificatio`, the second as
   `memoria non liberata est`.
3. **An intermediate that nothing binds.** `tabula[°I][°II]` builds a copy of
   the outer list, a copy of the inner one, and a `voc`; two of those three are
   temporaries with no slot, and only the statement-root rule frees them.

The name is `memoria-*` rather than `lista-*` deliberately: the printed output
is a weak assertion — the live-allocation count reaching zero is the real one,
and it is spelled in the exit status.
