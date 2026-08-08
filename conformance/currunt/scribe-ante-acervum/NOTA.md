**This is the fixture that pins the handler-side flush** (spec §6, D-08), and
it could not exist while the interpreter owned the expectation.

The program prints two lines and then recurses to death. Both lines are in
`emissio`, the diagnostic is in `nuntii`, and the status is 1.

## Why the printed lines are the whole point

The runtime buffers output — it writes through its own buffer rather than making
a `write(2)` per `scribe`, which is what keeps a printing loop from being
syscall-bound. So at the moment the guard page is touched, "ante acervum" and
"adhuc ante" exist only in that buffer. Nothing in the ordinary exit path is
going to run: the thread is inside a `SIGSEGV` handler, on an alternate stack,
with the faulting frame unrecoverable.

If the handler wrote its sentence and called `_exit` without flushing first, the
two lines would be **lost** — and the fixture would be `emissio` empty with the
same `nuntii` and the same status, which is to say it would still look like a
perfectly ordinary failing program. That is exactly D-08's shape: a runtime
error discarding everything the program had provably already printed. The defect
was fixed in the interpreter; this is the compiled path's copy of the same
obligation, and the handler is the only place that can honour it.

Order matters too, and is asserted by the two streams being separate files: the
output must be flushed **before** the diagnostic, or a merged view would show
them the wrong way round.

## Why it could not exist before

The interpreter capped at 64 frames and answered `vocatio nimis alte implicata`
through the ordinary error path, where flushing is not in question — there is no
signal, no alternate stack and no buffer at risk. A fixture written then would
have blessed the interpreter's answer and would have had to be re-blessed here,
and the corpus is allowed exactly one such re-blessing this milestone
(`errores/acervus-exhaustus`). So this case waited for the backend that makes it
meaningful.

## One honest exception to a `currunt` rule

The milestone's final check is that every `currunt` and `monita` fixture exits
with the live-allocation count at zero. **This case cannot satisfy that**, and
not because it leaks: spec §7.5 has the runtime check the counter on the
*success* path, and this program has no success path. It is killed on the guard
page and the handler exits 1 without reaching the check. The plan places the
case in `currunt` deliberately — what it demonstrates is output surviving, not
an error being reported — and the leak-counter rule should be read as covering
the `currunt` cases that exit 0.
