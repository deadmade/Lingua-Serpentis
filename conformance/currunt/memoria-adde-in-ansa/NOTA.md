`adde` in a loop, which is the shape `liss_list_append`'s design exists for.

`quadrata = adde(quadrata, i * i)` is a `StmtKind::Assign` whose right-hand side
consumes the slot's own value, so it exercises §7.3.3's evaluate → free old →
store in the one order that works: the `Load` copies the list, `liss_list_append`
consumes that copy and returns a new one, and only then is the slot's previous
pointer freed. Freeing first would hand `adde` a pointer whose magic was already
cleared, which is `defectus internus: memoria violata est: verificatio` rather than a wrong
answer.

**What this case cannot see.** `liss_list_append` *steals* the old list's element
words — a header one longer, one `memcpy`, and only the old header freed — which
is what keeps a loop of `adde` linear rather than quadratic. A copying
implementation would print exactly the same six lines and exit 0 just the same.
The steal is measured where it is visible, in `liss-abi`'s
`append_in_a_loop_stays_balanced_and_copies_no_element`, which reads the live
count after each of a hundred appends and finds `1 + i`; this case's job is only
to prove that codegen reaches that entry point and adds no bookkeeping of its
own.

The last two lines append a `ni lista` to a `ni lista lista`, so the word being
stolen is itself a list header — the element kind is 10, not 2, and nothing in
the append path looks at it.
