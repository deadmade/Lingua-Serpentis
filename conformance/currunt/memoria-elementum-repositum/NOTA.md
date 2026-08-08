`xs[i] = v` over elements that own memory — the one statement in the language
that mutates through a slot instead of rebinding it.

`liss_list_put` is one of the ABI's two borrowing entry points: the list stays in
its slot, and the element it displaces is freed inside the call. Both halves are
load-bearing and both fail quietly if got wrong:

- **If codegen emitted a `Load` instead of a bare `use_var`**, the write would
  land on a private copy, the copy would leak, and `scribe(nomina)` would print
  the *unmodified* list. `[primum, secundum, tertium]` on line one is the
  assertion that this did not happen, and the leaked copy would make the exit
  status 1 as well.
- **If codegen freed the list after the call**, the slot would hold a cleared
  header and the next statement to read it would be
  `defectus internus: memoria violata est: verificatio`.
- **If nothing freed the displaced element**, `memoria non liberata est`.

The second block displaces a whole `voc lista` rather than a `voc`, so the free
of the displaced element is itself recursive. The third is `nomina[i] = nomina[i]
+ "!"` in a loop, which reads and writes the same slot in one statement: the
index expression is evaluated, then the value expression — whose own `nomina[i]`
takes a deep copy of the list and a copy of the element out of it — and only then
is the slot observed for the write.
