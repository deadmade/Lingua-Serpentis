**The one expectation in the compiler-backend milestone that changed**, and it
was scheduled from the start (spec §10.1). This case was
`errores/vocatio-nimis-alte-implicata`; it is renamed, and its `nuntii` now
records what a compiled program does.

## What the sentence used to be, and why

D-04 was that unbounded recursion killed the process: `Interp::call` recursed
through `exec` and `eval` back into itself with no depth counter and no growable
stack, so a runaway function aborted with SIGABRT and exit 134. Like D-05 it
could not be a fixture before the fix, because the abort took the test process
with it.

The fix was a counter. `interp.rs`'s `VOCATIONES_MAXIMAE = 64` refused the 65th
frame and raised `vocatio nimis alte implicata` **at the call's own line**, so
this fixture's `nuntii` read
`Error in 'principale.liss', linea 2: vocatio nimis alte implicata`.

Sixty-four was never a language decision. It was measured against a 2 MiB
spawned-thread stack — a tree-walker spends a lot of Rust frames per Lingua
Serpentis frame — and set at half the level that overflowed. Spec §3.3 says so
outright and removes it.

## What it is now

Compiled code recurses on the real stack until it reaches the guard page. §8's
handler catches the `SIGSEGV`, writes `acervus vocationum exhaustus est` from an
alternate stack with async-signal-safe primitives, and exits 1.

Two differences follow from that, and both are visible above:

- **No line number.** The handler runs on a faulting thread with no idea which
  call was the last one (spec §8.4). `Error: ` with no `in 'file', linea N` is
  the structural form, and it is the honest one.
- **No file.** Same reason.

The two sentences describe different events. `vocatio nimis alte implicata`
means "the interpreter refused to go deeper"; `acervus vocationum exhaustus est`
means "the machine ran out of stack". Only the second is a fact about the
language, which is why it is the one that survived.

## What guards the change

`currunt/recursio-profunda` is the other half: 5,000 levels, which the old
ceiling forbade and which must **not** produce this diagnostic. A guard that
fired early would pass this case and fail that one.

`currunt/scribe-ante-acervum` pins the third property — that output written
before the exhaustion still reaches stdout, because the handler flushes (spec
§6, D-08).

On aarch64 the same sentence is pinned by `codegen/tests/aemulatio.rs`'s
`acervus_exhaustus_sub_aemulatore`, under `qemu-aarch64` and outside the corpus.
