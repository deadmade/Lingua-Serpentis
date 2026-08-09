# The ABI, the runtime, and the IR

This document is the reference for the three crates that sit below the checker: `abi`, the C
ABI a compiled Lingua Serpentis program calls; `runtime`, the arithmetic and formatting shared
between the compiler and compiled programs; and `ir`, the typed intermediate representation the
checker emits and codegen consumes. It states the value representation, the kind codes, the
consuming calling convention, the ownership invariants, the heap guard, the output path, the stack
guard, and the shape of the IR — together with the reasons each of them is what it is, and the
measurements and defects that settled the ones that were argued. The `# Safety` clauses on
individual `unsafe` functions are **not** reproduced here; they stay in the source. What is here is
the contract those 62 clauses all instantiate.

---

## 1. Overview: three crates and why they are three

Everything above `abi` — the lexer, the parser, the checker, the typed IR — deals in Rust
values; everything below it is machine words that codegen emits. `abi` is **the
memory-safety-critical layer, and the only crate in the workspace containing `unsafe`**. The rules
those words obey are stated once, in `abi`, and are the contract codegen emits calls against.

**`runtime` is compiled twice.** Once for the host, inside the compiler, because the typed IR
needs `Roman` for integer constants; once for musl, embedded in `abi`'s staticlib. `abi`
is compiled once, only for musl, and is never linked into the compiler — which is why the two
crates cannot be one (spec §11). `runtime` is dependency-free by construction: nothing in it
may depend on any crate, in either the host or the musl build. That is what makes compiling it
twice, for two different targets, straightforward.

**`abi` is an ordinary member of the one workspace**, though `codegen`'s `build.rs` invokes cargo
on it *while the outer build is already running*. It was its own workspace until 2026-08-09, on the
reasoning that a member would contend for the outer `.cargo-lock` and deadlock. What actually keeps
the two builds apart is the `--target-dir` that build script passes: cargo's exclusive lock lives in
the target directory, so pointing the inner build at `$OUT_DIR/aedificium` is the whole of the fix.
The exclusion predated that build script by eight commits — it was precautionary, and the
precaution outlived the danger.

Two things were paid for the second workspace, and both are recovered by dropping it.

`cargo test` now reaches `abi.rs`, `custos.rs` and `emissio.rs`; there is no second command to
forget. And the shipped runtime no longer carries the **absolute path of the machine that built
it**. `runtime` is a path dependency of `abi`; across a workspace boundary cargo named it
absolutely, so every `#[track_caller]` location baked into the musl runtime — and therefore into
every compiled program — read `/home/<user>/<checkout>/crates/runtime/src/roman.rs`. Inside one
workspace the same names are relative. That is four strings out of the artefact, next to the four
English ones **D-28** records; it fixes none of D-28, which is about the messages, not the paths.

The cost is one trap, and it is silent. **A workspace member's `[profile.release]` is ignored** —
cargo honours profiles at the workspace root only, and warns rather than failing. The root's
release profile belongs to the compiler, so `panic = "abort"` for the runtime is named by
`build.rs` on the inner build instead, as `--config profile.release.panic="abort"`. Dropped, it
costs 28 landing pads in `abi`'s and `runtime`'s own code inside a runtime whose panic never
unwinds, and **every test in the repository still passes** — which is why
`codegen/tests/pondus.rs` exists and asserts both properties on the embedded bytes.

**`abi` is the only crate permitted `libc`** — the one dependency the whole workspace has
(spec §11): `write`, `_exit`, `malloc`, `free`, and, from Task 9, `sigaltstack`/`sigaction`, none
of which std exposes. `runtime`, `ir` and `lingua-serpentis` stay at zero.

Two build details follow from the same place. The crate is `crate-type = ["rlib", "staticlib"]`:
`staticlib` is what the musl cross-build embeds, because every `#[unsafe(no_mangle)]` symbol is
garbage-collected out of an rlib; `rlib` is what the host test binary links against. And the
runtime's `main` is behind the **`ingressus` feature**, off by default and required to stay off for
every host build: this crate is *also* built as an `rlib` for its own tests, every test binary
already defines `main`, and an ungated one here is a `duplicate symbol: main` at link time —
measured, not feared. `codegen/build.rs` turns the feature on for the two musl staticlibs it
embeds, and nothing else does.

The embedded runtime is always built `--release` regardless of the compiler's own profile, so "debug
build" never becomes a property of *which* prebuilt musl archive shipped. `panic = "abort"`, because
`liss_panic` never unwinds (spec §6): no landing pads and no `_Unwind_Resume` from this crate's own
code. That does **not** remove the need to prelink `libunwind.a` — the precompiled musl `libstd`
is built with unwinding and references `_Unwind_*` regardless — and cargo ignores the setting for
the `test` profile, so `cargo test --release` is unaffected. Both flags are arguments to the inner
cargo in `codegen/build.rs`, for the reason given above; neither is in `abi/Cargo.toml`.

---

## 2. One machine word per value (spec §5.1)

| type | word |
|---|---|
| `veritas` | `i8`, 0 or 1 |
| `lit` | `i32`, a Unicode scalar value |
| `ni` | `*mut LissInt` |
| `mnr`, `nr` | `*mut LissInt`, a count of a grain the *type* fixes |
| `voc` | `*mut LissText` |
| `T lista` | `*mut LissList` |

Where a word is passed generically — a list element, `liss_list_get`'s result — it is a `u64`,
and the two scalar types are **zero-extended** into it. There are no `liss_bool_*` or `liss_char_*`
entry points: both are immediates that codegen operates on directly, and neither is owned.

### The three heap layouts (lib.rs)

All three begin with `LissHdr { magic: u32, kind: u32 }` and put their payload at the **same
offset**:

- **`LissInt`** — a `ni`, or an `mnr`/`nr` grain count. `len`, `neg`, then the payload at `ONUS`,
  which is `Roman`'s own representation: `len` little-endian digits, each `0..=9`, canonical (no
  trailing zeros, and the empty payload the unique zero).
- **`LissText`** — a `voc`. `len`, `_rsv`, then `len` bytes of UTF-8 at `ONUS`. `_rsv` is
  **reserved and load-bearing**: it is what puts `bytes` at `ONUS`, the same offset as `LissInt`'s
  digits and `LissList`'s items, and must not be removed.
- **`LissList`** — a `T lista`. `len`, `ekind`, then `len` machine words at `ONUS`, each an
  element of kind `ekind`. `h.kind` is `ekind + GRADUS`.

`ekind` is the **element** kind, so the runtime can free, copy, compare and print recursively
without a type environment.

### `ONUS` and `VERBUM` (dispositio.rs)

`ONUS` is 24: the offset of the payload in **all three** layouts, so codegen carries one constant
rather than three — element `i` of a list is at `ONUS + 8*i`. `VERBUM` is 8: one element word,
whatever the element's kind, the two narrow scalars zero-extended into it, which is what makes the
payload an array rather than a packed record codegen would have to compute offsets into.

**Both are defined in `runtime::dispositio`, not in `abi`.** `abi` owns the three
`#[repr(C)]` headers and *was* the sole definition of the offset — but codegen is now a **writer**
of the layout as well as a caller of the ABI: `liss_list_new` deliberately has no per-element setter
and leaves codegen to store each element word itself. `abi` is cross-compiled for musl and
never linked into the compiler (spec §11), so `codegen` cannot import from it; the alternative
was a second `24` in `emissio.rs` held to this one by a comment, which is precisely the arrangement
Task 12 removed for `KIND_INT`. `abi` re-exports it, so `abi::ONUS` still names it.

### The `const _` block: the proof, not the promise (lib.rs)

The re-export alone would be a promise. The `const _` assertion block in `lib.rs` is what makes it a
proof: it asserts `size_of` of all three headers equals `ONUS`, `align_of` of all three is 8,
`size_of::<LissHdr>()` is 8, and `VERBUM == size_of::<u64>()`. The musl cross-build of `abi` is
a prerequisite of every compiled program (`codegen/build.rs`, which also re-runs on
`runtime/src`), so a constant that drifted from the layout is a **build failure** rather than a
wrong answer.

`VERBUM` needs its own line and did not have one. `ONUS` is held by the three `size_of`s; the
element **stride** is held by nothing at all, because the reader never spells it —
`series::elementa` returns a `*mut u64` and every access is `.add(i)`, which is a hard-wired 8. So a
`VERBUM` of 16 would have codegen store element `i` at `ONUS + 16*i` while `abi` reads it at
`ONUS + 8*i`: `quantum` would over-allocate (harmless), every element past the first would read as a
zero word, and `liberare_verbum` and `copiare_verbum` both *skip* a zero word. Silent wrong answers
and silent leaks, and nothing would fail to compile. Now something does. The assertion is written
`size_of::<u64>()` and not a literal 8, because the element array *is* a `[u64]` and `elementa`'s
`.add(i)` scales by exactly this.

---

## 3. Kind codes (spec §5.3, genus.rs)

```text
base: 0 Bool  1 Char  2 Int  3 Frac(Uncia)  4 Frac(Scripulum)  5 Text
kind = base | (depth << 3)
```

`ni lista` is 10; `ni lista lista` is 18; the element kind of kind `k` is `k - GRADUS`, where
`GRADUS` is 8. Closed form, unbounded nesting, no interning table.

This is a descriptor **on the container**, written at every construction site as a compile-time
immediate. Codegen only ever writes it and never dispatches on it, so it is categorically not the
tagged value the ABI forbids. The runtime is the only reader, and it reads kind codes at run time in
exactly one place: walking a list.

`basis(kind)` is `kind & (GRADUS - 1)`, the scalar type underneath any nesting. `profunditas(kind)`
is `kind >> 3`, how many `lista` levels deep — 0 for a scalar. `granum(kind)` is the grain
denominator a fractional kind counts in: XII for `mnr`, CCLXXXVIII for `nr`, and `None` for every
other kind.

**These live in `runtime::genus`, not in `abi`, for the same reason `ONUS` does.** They
are written by codegen and read by the runtime, and those two never share a compilation. Until Task
12 the numbers existed twice — `KIND_INT` in `abi` and a copied `GENUS_NI = 2` in
`codegen` — held together by a comment. `runtime` is already a dependency of *both*, and
is dependency-free in both the host and the musl build, so it is the one place the two ends of the
contract can genuinely share a definition, and it needs no new row in spec §11's table. `abi`
re-exports them, so `abi::KIND_INT` still names the same constant.

`genus.rs`'s own tests pin the two worked examples from spec §5.3 and their inverse, that every
base fits below `GRADUS` (or the `|` would carry into the depth field), and that only the two
fractions have a grain.

---

## 4. The consuming convention (spec §5.4)

**Every operation takes ownership of its owned arguments and returns exactly one new owned value.**

Two families are exempt, and both are lifecycle rather than operations:

- **`liss_*_copy` borrows.** It is what `Load` calls to read a slot without emptying it; a consuming
  copy would leave the slot dangling.
- **`liss_list_put` borrows the list it mutates.** It consumes its index and its value, and frees
  the element it displaces. It is the **sole borrowing operation** in the ABI, and the list staying
  in its slot is what makes `IndexAssign` a mutation rather than a rebuild.

Everything else consumes. That is what makes a statement's expression tree leave exactly one live
temporary — its root — so codegen never maintains a set of pending temporaries.

There is no `liss_enter`/`liss_leave`. With the recursion cap gone (spec §3.3) calls carry no
bookkeeping whatsoever; stack exhaustion is caught by the signal handler of §8 instead.

---

## 5. Ownership, and the null-or-live invariant (spec §7)

The `# Safety` clause on each `unsafe` entry point states this contract for that entry point. In
general it says:

1. **An owned pointer argument is live and the caller's to give away.** After the call the caller no
   longer owns it — the consuming convention above.
2. **A borrowed pointer argument (`*const`, the `copy` family) is live and stays the caller's.**
3. **Every returned pointer is fresh and owned**, and the caller must eventually free it exactly
   once.
4. **A slot is null or live, never dangling.** Codegen defines every slot to zero on entry; a free
   of a slot may therefore legitimately see null.
5. **A single thread.** Compiled programs are single-threaded, and nothing in a compiled program can
   create a second writer. The output buffer's ordering rules and the allocation counter's `Relaxed`
   atomics both rest on this.
6. **Kind codes are written by codegen and read by the runtime.** A word the runtime is asked to
   free, copy, compare or print carries no tag of its own; the caller-supplied `kind`, or the
   container's `ekind`, is the only thing that says how to read it. A kind code naming no type is an
   internal defect, not a language-level error.

### Null is a no-op on the free path — and that is a contract

**`memoria::abolere`, and with it `liss_int_free`, `liss_text_free` and `liss_list_free`, treat a
null pointer as a no-op, exactly as C's `free` does.** This is a contract of the ABI rather than
defensive padding.

The reason is §7.3's teardown rule meeting how the checker builds `Block::declares`: `pop_scope`
yields every slot bound *anywhere* in the block, wherever the `Let` sits. So in

```text
munus initium reddit ni {
  si (verus) { redde nullus; }
  voc s = "salve";
  redde nullus;
}
```

`body.declares` holds `s`, and the `redde` inside the `si` frees a slot whose `Let` never ran.
Cranelift requires a `Variable` be defined on every path reaching a use, so codegen defines every
slot to zero on entry — and that free sees null. Refusing it would turn a legal program that
prints nothing and exits 0 into exit 1 with a corruption diagnostic, and the robustness harness
would report an invariant violation.

The alternative is codegen proving per-slot, per-path initialisation, which is precisely the
bookkeeping §5.4 exists to abolish. One branch in `abolere` keeps §7.3.4's "one uniform walk over
`Block::declares`, with no special case" literally true.

**The read guard is the opposite.** `memoria::verifica` treats null as fatal: nothing in a compiled
program can read a value it does not have.

### Raising never unwinds

`liss_panic` never unwinds: it flushes, writes one Latin sentence to stderr and `_exit`s (spec §6).
Everything live leaks, exactly as the interpreter dropped everything on its error path — which is
why the leak assertion of §7.5 runs only on the *success* path, and why unwinding for `redde` /
`abrumpe` / `continua` is a purely syntactic problem with no landing pads. `liss_int_div` and
`liss_int_mod` therefore free neither operand when they raise `divisio per nullum`, and
`index_extra_fines` allocates a `LissInt` for the length it will never free.

---

## 6. The magic word, and what it can and cannot catch

`LISS_MAGIC` is `0x4C49_5353` — `'LISS'` — stamped into every heap object's header and checked
on **every** free (spec §5.2). It turns a double free or a use-after-free into a deterministic
Latin exit rather than musl heap corruption, and it covers the live-allocation counter's one blind
spot: an extra free plus a missing free nets to zero, and only the magic can tell that apart from a
balanced program.

**`memoria::verifica`** is the read guard: null, or a magic that is not `LISS_MAGIC`, calls
`defectus::memoria_violata`. Three per-type refinements sit on top of it, because the magic alone
cannot say *which* type a live object is — a `LissText` handed to an integer entry point has a
perfectly good magic and a payload that is not digits:

| checker | file | additionally requires |
|---|---|---|
| `verifica_numerum` | numeri.rs | `profunditas(kind) == 0` and `basis(kind)` in {`KIND_INT`, `KIND_FRAC_UNCIA`, `KIND_FRAC_SCRIPULUM`} |
| `verifica_textum` | textus.rs | `kind == KIND_TEXT` |
| `verifica_seriem` | series.rs | `profunditas(kind) > 0` and `ekind + GRADUS == kind` |

`verifica_seriem` uses `checked_add` for that last comparison, because `ekind` is a field of an
object the magic vouches for but whose contents it does not: an `ekind` within eight of `u64::MAX`
overflows, which is a debug panic across an `extern "C"` boundary and therefore an abort — the
same class as the two defects this crate already fixed.

**What the magic does not vouch for is any other field.** `liss_int_copy` and `liss_text_copy` both
range-check `len` before using it, because the magic says "a Lingua Serpentis object was here", not
"its length field is sane". Unchecked, a corrupt `len` near `usize::MAX` wraps to a small allocation
carrying a huge length, and the next read walks off it.

### `abolere`, and why the clear had to become volatile (D-27, memoria.rs)

**`memoria::abolere`** is the only way a header ever reaches the allocator: check the magic, clear
it, free. Clearing it before the free is what makes an immediate double free — or a read after one
— deterministic rather than probabilistic: the block reads as not-a-`LISS`-object from the moment
it is handed back, instead of only once `malloc` has given it out again and something has
overwritten the first four bytes.

A plain `(*h).magic = 0` **does not do that in a release build, and did not** (D-27). `free` ends
the object's lifetime, so LLVM's dead-store elimination removes any store that immediately precedes
it — correctly, by the language rules, since nothing may legally observe the result. Nothing
*legal*: this store exists precisely for the illegal reader. The shipped `liss_text_free`
disassembled to a null test, a magic test, a kind test, `call free` and the counter decrement, with
**no write to the magic anywhere**, and the same held of `liss_int_free`, `liss_list_free`'s element
loop and `abolere` itself. It was uniform, not one inlining accident.

`write_volatile` is the fix and the whole of it: a volatile access is an observable side effect that
no optimiser may remove, and it costs one `movl` that was always meant to be there.

This matters out of proportion to its size. Spec §5.2 makes the magic word the *entire* answer to
the live counter's blind spot. Codegen's null-store rule (`emissio.rs`'s `libera`) makes a redundant
free of a *slot* benign; a redundant free of a **temporary** has no such protection, and this is
what catches it.

---

## 7. Memory (memoria.rs)

Every heap object in a compiled program passes through exactly one `allocare`/`liberare` pair, which
maintain a live count. `main` checks it on the success path; a nonzero count is `memoria non
liberata est` and exit **1** — see `defectus::liss_exit`.

**The counter is always on, in every build**: one relaxed atomic against a `malloc`. Making it
debug-only would have meant "debug build" becoming a property of *which prebuilt musl archive is
embedded*, doubling the artefact matrix; and an environment-variable gate would make behaviour
depend on the environment. If this counter ever fires the compiler is broken, and a cheap loud
invariant is the right shape for that.

`VIVAE` is `Relaxed` throughout: compiled programs are single-threaded, so it synchronises nothing
— it is a counter, not a lock. `liss_live_count` exposes it so the runtime's `main` and the ABI's
own tests can read it without reaching into the module.

- **`allocare(bytes)`** — `bytes` of uninitialised, 8-aligned storage, counted. `malloc` is
  16-aligned on both targets, which covers every layout here (all three are `align_of` 8). A null
  return calls `defectus::memoria_exhausta`.
- **`allocare_nulla(bytes)`** — as `allocare`, but `calloc`. Used for `LissList`, whose items
  codegen fills with direct stores after the header comes back: zeroing means a list caught
  part-built — because an element expression raised between two stores — holds null words rather
  than whatever `malloc` returned. Nothing frees such a list (a raise never unwinds, spec §6), but
  "deterministic" beats "whatever was on the heap" for a structure the signal handler of §8 may yet
  walk past.
- **`liberare(p)`** — return a block to the allocator, counted. Callers holding a `LissHdr` use
  `abolere`, which checks the magic first.

**What leans on the zeroing, and what deliberately does not.** The zeroing is *a net for the failing
path and not part of the construction protocol*. Codegen emits the constructor and then one store
per element with no call in between, and both `liss_list_copy` and `liss_list_append` write every
word of what they allocate, so **no correct path ever reads a zero it did not put there**. The one
path that does is an element expression raising between two stores. Separately, `liberare_verbum`
and `copiare_verbum` skip a zero word — which costs one branch instead of a call, and covers the
two immediate kinds, for which a zero word is a *value* (`falsus`, `U+0000`) rather than an absent
one.

---

## 8. Strings and series

### `voc` (textus.rs)

**No interning, no reference counting, no small-string optimisation**: a `voc` is one allocation,
copied on `Load` and freed on scope exit like every other owned value. The language has pure value
semantics — the interpreter cloned on every variable read — so there is no user-observable
sharing to preserve.

`liss_text_new` validates the byte blob as UTF-8 once, on the way in, so every later reader —
`textum_of`, `scribe_text`, `liss_text_cmp` — is looking at something the type system of the
source language already promised. `liss_text_concat` is the only arithmetic `voc` has.
`liss_text_cmp` is byte order over the UTF-8 payload, which is exactly `str`'s `Ord` and so exactly
what `"apple" minor_est "banana"` has always meant.

### `T lista` (series.rs)

Lists are the only recursive type, and the only place a kind code is read at run time. They are
**1-based**, and every element operation goes through `ekind`: freeing, copying, comparing and
printing a list are one walk each, dispatched on the container's descriptor rather than on a tag per
element.

Construction is `liss_list_new` plus **direct stores**: codegen allocates a header of the final
length and writes each element word at `ONUS + VERBUM*i` itself. There is no per-element setter,
because there is nothing for one to do that a store does not. `liss_list_new` computes the header's
own `kind` as `ekind + GRADUS` itself, so codegen passes one immediate rather than two that could
disagree. `quantum` refuses an overflowing length rather than wrapping into a short allocation.

The four element-word helpers are the whole of the recursion: `liberare_verbum`, `copiare_verbum`,
`aequum_verbum` and (in scriptio.rs) `scribe_verbum`. `aequae_series` is the borrowing form of deep
equality, so the recursion does not consume what the public entry point still has to free.

- **`liss_list_get`** — `xs[i]`, 1-based, bounds-checked, returning a **deep copy**, consuming
  both `p` and `index`. The copy keeps the no-aliasing invariant: after the call the element is in
  the list and the result is a separate value the caller owns. Held by
  `conformance/currunt/lista-index-lectio` and, more sharply, by `codegen/tests/series.rs`'s
  `series_indicem_per_exemplar_legit`, which reads the emitted call sequence — a missing copy is
  visible there and invisible in a program that prints the right answer. `interp.rs:182` did the
  same thing with `items[i - 1].clone()`.
- **`liss_list_put`** — `xs[i] = v`, freeing the element it displaces; the ABI's sole borrowing
  operation.
- **`liss_list_append`** — `adde(xs, v)`, a new list one longer. **It steals the element words**:
  a header one longer, one `memcpy` of the existing words, the new word stored, and only the *old
  header* freed — zero element copies, because the caller already handed over a private copy of
  every one of them. That is what keeps `adde` in a loop linear in words rather than in deep copies.
  Freeing the old elements here is the one mistake this entry point exists to make impossible.
- **`liss_list_eq`** — deep equality, consuming both. Lists have `par_est` and `non_par_est` and
  no ordering, so there is no three-way form to return: `Value`'s `PartialEq` recursed but its `Ord`
  never covered `List`.
- **`liss_list_len`** — the element count as a `ni`, consuming the list.

**`intra_fines`** turns a 1-based subscript into a zero-based offset. A subscript that does not fit
`usize` at all — a hundred-digit `ni`, which this language can spell — is out of range for every
list, so `to_usize` returning `None` and a subscript of zero take the same branch. It is written
`then`, not `then_some`: `then_some` evaluates its argument eagerly, so the zero subscript of
`errores/index-infra-fines` computed `0usize - 1` before the guard could reject it. That is an
overflow panic in debug and a wrap to `usize::MAX` in release — and a panic crossing an `extern
"C"` boundary aborts, which is `SIGABRT` and exit 134, exactly the shape D-04 recorded as outside
the error model entirely.

Out of range raises `index extra fines: {} (longitudo {})` with **both numbers as Roman numerals**.

---

## 9. Numbers and Roman fractions

### `ni`, `mnr`, `nr` at the ABI (numeri.rs)

All three are a `*mut LissInt`. `mnr` counts unciae (1/12 of the `as`) and `nr` counts scripula
(1/288); **the grain is a property of the *type*, so it never travels in the value.** That is why
the three share `add`, `sub`, `neg` and `cmp` outright and differ only in `mul`, `div` and
formatting.

**`ni` is boxed unconditionally.** A small-integer optimisation would be a tagged value, which the
ABI rules out, and unconditional boxing makes the ownership rules uniform — every `ni` is exactly
one allocation to free, with no branch. The consequence, that `ni i = i + °I` allocates twice per
iteration, is stated in the Non-Goals rather than hidden.

Two `memcpy`s bound the type at the ABI edge:

- **`roman_of`** reads a `LissInt` back into a `Roman` — one `memcpy`, plus the digit check.
  `Roman::from_digits` returns `Option`, and that is the whole point of this boundary: a `LissInt`
  arriving with a digit above 9 is corrupt; left unchecked it panics inside `render_digit`'s
  `unreachable!` or silently wraps the `u8` addition in `mag_add` in a release build. Corrupt input
  must produce a Latin diagnostic, never a panic and never silent wrapping.
- **`int_novus`** boxes a `Roman` as a `LissInt` of a given kind. The zero-copy alternative, a
  header pointing at the `Vec`'s own buffer, costs a second allocation per integer; for numbers
  almost always under twenty digits, a twenty-byte copy beats a `malloc`. The `neg` field is
  `u64::from(*r < Roman::zero())`: `Roman` exposes no sign accessor to `int_novus` and does not need
  one, because its `Ord` orders every negative below zero and forces zero non-negative, so this is
  the canonical sign by construction.

**`liss_int_copy` is a raw copy of header plus payload rather than a `Roman` round-trip**: it
preserves the kind, allocates once, and does not need the digits to be valid — a corrupt integer
is caught wherever it is *used*, by `roman_of`.

**`genus`** reads the kind stamped on a `LissInt` so arithmetic can carry it to its result. Nothing
in the runtime dispatches on a scalar's own kind — a list reads its `ekind`, never its elements'
headers — but propagating it keeps every header honest rather than leaving `mnr` values labelled
`ni`, and costs one load.

**`liss_int_cmp` and `liss_text_cmp` are three-way**, returning `-1`, `0` or `1`, rather than four
entry points (`par_est`, `non_par_est`, `maior_est`, `minor_est`): codegen compares the result
against an immediate, which is one instruction, and the runtime carries one implementation of the
ordering.

**`liss_frac_mul` and `liss_frac_div`** carry the grain as an argument, XII or CCLXXXVIII.
`exige_granum` refuses anything else: `fraction::mul` divides by the grain and `expect`s it nonzero,
so a zero arriving from a miscompiled call site would be a Rust panic crossing an `extern "C"`
boundary. Checking turns it into a Latin exit, which is the rule everywhere else in the crate.

### `Roman` (roman.rs)

A Roman-native integer: a sign plus a little-endian bag of decimal digits. **Canonical invariant:**
every digit is `0..=9`, there are no trailing all-zero digits, and the empty `tiers` vector is the
unique zero (never negative). Structural equality is therefore numeric equality.

The magnitude engine is base-10 digit space throughout: `mag_add` carries per digit; `mag_sub`
borrows and requires magnitude `a >= b`; `mag_halve` is long division by two from the
most-significant digit down; `mag_mul` is the **doubling ladder (Russian-peasant)**, only `mag_add`
and `mag_halve`, **never a host multiply of the value**; `mag_divmod` is doubling long division
built on `mag_add`/`mag_sub`/`mag_cmp`. `mag_cmp` can compare lengths first because canonical form
has no trailing zero digits, so the longer vector is strictly larger.

**`Roman::from_digits` is the checked boundary** for digit vectors arriving from outside the type's
own invariant, such as a `LissInt` payload crossing the ABI by `memcpy`. **`Roman::negative`** is
its exact inverse, added so a `Roman` can be read back **whole**: `digits()` alone left the round
trip lossy, and `liss_int_new(kind, neg, digiti, len)` wants both halves. That closed a gap, not a
live bug — no negative `Const` can be produced today, the lexer building every literal through
`from_u64` or `zero` and `-°V` lowering to `ExprKind::Unary` with no constant folding.

**D-21 — `div_rem` returns `Option`.** The zero guard used to be a `debug_assert!` inside
`mag_divmod`, which is compiled out of a release build; with an empty divisor `mag_cmp` never
reports Greater, so the doubling loop runs forever, doubling `pow` until memory runs out. Every
caller happened to guard, so it was unreachable — but `div_rem` is `pub`, and a latent
release-only hang behind a caller's diligence is not a guarantee. Returning `Option` moves the
obligation into the type. Deliberately not an `assert!`: an unreachable panic is still a panic, and
`no panic` is a live rule the fuzzer enforces.

**D-25 — three guards examined, one kept.** `parse_roman`'s missing-`°` guard is an API-only
guard: the lexer, its only caller in the workspace, builds `literal` from a slice that starts at the
`°` it just matched, so the prefix is there by construction and the arm cannot fire through the
language. It is kept because `parse_roman` is `pub`, the catalogue records it UNCOVERED — the
honest label — and `parse_roman_rejects_a_literal_without_its_degree_sign` covers it, which is
what makes keeping it honest rather than dead. The other two are gone. `"fractio sine numeratore"`
was not merely unreached but **impossible**: `splitn` always yields at least one item, for every
input including `""`. `"denominator est nullus"` was unreachable because `convert_int_roman` cannot
return zero — every numeral it accepts is a sum of products of `convert_single` values, and
`convert_single` is at least 1 for every string it accepts. Rather than rest on that argument,
`convert_single_is_never_zero_or_negative` enumerates every string the function accepts up to length
6 — **137,256 of them** — and checks.

Rendering (`display`) is subtractive per tier, with the vinculum for thousands: `glyph_one` gives I,
X, C, M, X̄, C̄, M̄, X̿, … and `glyph_five` gives V, L, D, V̄, L̄, D̄, V̿, … , a bar
being ×1000 (`U+0305 COMBINING OVERLINE`). Zero renders `nullus`; a negative gains a single leading
`-`.

### Fractions (fraction.rs)

A value is a signed `Roman` count of a fixed grain unit, the grain chosen by the type. Arithmetic is
the integer engine plus one rescaling `div_rem`; **off-grid results truncate toward zero, exactly
like `ni` division** (the 2026-07-23 fractions design spec).

- `from_rational(numer, den, g)` narrows an exact rational onto the grain grid:
  `count = trunc(numer * g / den)`. The caller guarantees `den` is non-zero — the parser rejects a
  zero denominator, and no numeral this language accepts evaluates to zero.
- `mul(c1, c2, g)` — `(c1/g)*(c2/g)` back in grain units is `trunc(c1*c2 / g)`.
- `div(c1, c2, g)` — `(c1/g)/(c2/g)` back in grain units is `trunc(c1*g / c2)`. `None` when the
  divisor count is zero, so the caller can raise `divisio per nullum`.

`display` renders a grain count as classical Latin fraction names: the whole part as a Roman
numeral, the fractional part as its twelfth name plus a greedy sub-uncia stack, joined with
`" et "`. Zero is `nullus`; a zero whole part is omitted; a negative value gets a single leading
`-`.

| k/12 | name | | scripula | name | as |
|---|---|---|---|---|---|
| 1 | `uncia` | | 12 | `semuncia` | 1/24 |
| 2 | `sextans` | | 6 | `sicilicus` | 1/48 |
| 3 | `quadrans` | | 4 | `sextula` | 1/72 |
| 4 | `triens` | | 2 | `dimidia sextula` | 1/144 |
| 5 | `quincunx` | | 1 | `scripulum` | 1/288 |
| 6 | `semis` | | | | |
| 7 | `septunx` | | | | |
| 8 | `bes` | | | | |
| 9 | `dodrans` | | | | |
| 10 | `dextans` | | | | |
| 11 | `deunx` | | | | |

The sub-uncia stack is greedy and uses each unit at most once, which is exact and repeat-free for
every remainder 0..=23. Worked examples the tests pin: `°I et triens` is 16 unciae; 1/7 as `nr` is
`trunc(288/7) = 41` scripula and reads `uncia et semuncia et sextula et scripulum`; the maximal
23-scripula stack is `semuncia et sicilicus et sextula et scripulum`. Every `mnr` value is a whole
number of unciae, so it never emits a sub-uncia name — at most `NUMERAL et TWELFTH`.

---

## 10. Output, streams, and the signal-handler buffer

### The formatter (runtime/scriptio.rs)

Free functions rather than a `Display`/`ToString`-style method: each one is monomorphic (renders
exactly one value shape) and *delivers* its bytes to a caller-supplied `Sink` rather than returning
a `String`, which is what the runtime's static output buffer (spec §6) needs. Ported from
`Value::display`'s match arms (the interpreter's `value.rs`, spec §5.5) — the same seven one-line
bodies, unchanged, just monomorphised into separate functions. The list case is deliberately absent:
it is `ekind`-driven, has no data here to drive it, and lands in `abi::liss_print_list`.

**"Delivers", not "does not allocate", and the difference is a defect.** `scribe_text`,
`scribe_char` and `scribe_bool` genuinely allocate nothing. `scribe_int` and `scribe_frac` do: both
delegate to `Roman::display` / `fraction::display`, which **return a `String`**, so every `scribe`
of a `ni` or a fraction takes Rust's global allocator — outside §7.5's counted pair, and with
`panic = "abort"` an allocation failure there reaches `handle_alloc_error` and dies in English
rather than saying `memoria exhausta est`. The module's doc used to claim the opposite. See **D-28**
in `docs/defectus/2026-08-03-index.md`; the fix is a `display_ad(&mut impl Sink)` on `Roman`, which
is a change to `roman.rs`, not to `scriptio.rs`.

`Sink for String` uses a checked UTF-8 conversion, not an unchecked one, because `Sink` is a public
trait and nothing stops a future implementor from feeding it something else — though every byte
`scriptio` ever pushes comes from a Roman numeral, a Latin fraction word, or a `&str`/`char` that
was already valid UTF-8.

**`speculum.rs`** is a second, independent implementation of the same formatter,
`#[cfg(test)]`-only, kept purely as the randomised oracle for `scriptio`'s tests. It is
`lingua-serpentis::value`'s `Value` moved across unchanged but renamed, so nobody mistakes it for
production. **This is a pre-approved, deliberate duplication, not a defect** — spec §10.4 ruled
on it explicitly. It exists because it is the *only* randomised oracle for the formatters: golden
vectors cover only cases someone thought to write down. It earns its place because it is small,
test-only, and the alternative is having no randomised oracle at all.

### The buffer (abi/scriptio.rs)

**The buffer is a `static mut [u8; 8192]` with a separate `AtomicUsize` length.** Not a `Vec`: no
allocator involvement, no `realloc` window, no lock. The signal handler walks this buffer from a
signal context to flush pending output before it reports stack exhaustion, and a `Vec` could be
mid-`realloc` when the fault arrives — which would lose D-08 for exactly the case §8 exists to
handle.

**`scribe` writes the bytes first, then stores the new length with `Release`.** That ordering is the
whole guarantee: any length the handler observes names bytes that are already fully written, so
stack-overflow output is never torn. **The load side is `Relaxed`** because there is no second
writer — a compiled program is single-threaded — and the handler runs on the same thread, so the
release-store is what it needs, not a matching acquire in this file.

**Writing decides nothing about the exit status.** `scribe_ad` reports a dead descriptor to its
caller and never ends the process itself, because the right answer differs between the success path
and the failing ones and only the caller knows which it is on.

**`effunde`** writes pending output to fd 1 and empties the buffer, reporting whether it got out;
`false` means the descriptor is gone. **It does not decide the exit status, and that is the whole
design.** A flush with nothing pending returns at `n == 0` without reaching a `write`, so it never
sees `EPIPE`; while the write chose the status, the *same* fault therefore ended differently
depending on how much output the program happened to have buffered — exit 0 with a silent stderr
for a program that had printed something, exit 1 with its diagnostic for one that had not. Nothing
stated that rule because nobody designed it.

A closed stdout now means one thing — the bytes did not get out — and each caller says what that
is worth:

- the **success** path (`liss_flush`, and `liss_exit` through it) treats it as an ordinary end of
  output and exits 0: D-06, and `prog | head`;
- the **failing** paths (`defectus::liss_panic`, and `custos`'s signal handler) ignore it, write
  their sentence to descriptor 2 and exit 1.

`judge` is what settles that and rules out the middle: exit 0 carrying a diagnostic is a `clean
success` violation, exit 1 carrying none is `diagnostic present`. Exit 1 *with* the sentence is the
only failing outcome that satisfies the invariant, so the flush must not be able to pre-empt it.

**Two scenarios measure it**, `fistula-abyssus` and `fistula-violatio` in `abi/tests/custos.rs`
(`a_closed_pipe_never_decides_the_exit_status`): the same two faults as `abyssus` and `violatio`,
with stdout already closed, and **deliberately with different amounts of pending output, because
that is the variable that used to decide the exit status**. `abyssus` leaves 13 bytes in the buffer,
so the flush reached a `write` and met `EPIPE`; `violatio` leaves none, so the flush short-circuited
and never saw it. Under the old rule that split them — `fistula-abyssus` exited 0 with an empty
stderr, `fistula-violatio` exited 1 with its sentence. Both now exit 1 and say what happened:
`Error: acervus vocationum exhaustus est`, and
`Error: defectus internus: memoria violata est: signum`.

The length is cleared only after the bytes are out, so a handler that fires between the two writes
the same bytes twice rather than losing them; and it cannot fire *inside* the `write`, because a
`SIGSEGV` arrives from an instruction this thread executes and there are none between here and the
syscall.

**`liss_scribe_erratum`** writes to stderr, unbuffered. Diagnostics and warnings are not program
output and must not queue behind it: D-09 requires a warning to reach stderr even when the program
then fails, and D-10 requires the truncated-exit-status note to reach it before the process ends.
**A failure here is ignored, in both directions.** This is `mone`'s entry point too, and a warning
that cannot be delivered is no reason to fail a program that otherwise succeeded; while the failing
paths that also call it reach `_exit(1)` by their own control flow whether or not the sentence
arrived — which is what stops a program whose error channel closed from claiming success.

**`scribe_ad`** loops `write(2)` until everything is out, retrying `EINTR`. Every failure is the
same outcome, `EPIPE` or not. The alternative was the old non-`EPIPE` arm's `_exit(1)`, which
produced exit 1 with an empty stderr — `judge`'s `diagnostic present` violation, raised from
inside the code whose job is to keep the program's failures well-formed. Saying more about a stream
that may itself be broken is not worth a second failed write. A `write` returning 0 for a non-empty
buffer would spin forever on a stale `EINTR`, and nothing useful can be done with a descriptor that
does it, so it is reported as a dead descriptor.

**Stream ordering is not observable and must not be designed for.** The fixture format stores stdout
and stderr in separate files, so nothing in the suite can see how the two interleave. What is owed
is only: everything `scribe` produced before a failure still reaches stdout (D-08), warnings still
reach stderr (D-09), and the truncated-exit note reaches stderr (D-10). All three hold with direct,
unordered writes to the two descriptors.

**`scribe_verbum`** renders one machine word of kind `kind`, without a trailing newline and without
consuming it — the whole of spec §5.5's list case: `ekind` is the only thing that says how to
read an element, and the closed-form kind code makes the recursion one subtraction deep per level.
`scribe_seriem` produces `[a, b, c]`, `Value::display`'s list arm driven by `ekind` instead of a tag
on each element. The six `liss_print_*` entry points each append exactly one newline, as
`StmtKind::Print` has always done; the two immediate kinds are not owned, so nothing is freed.

### Exit status (runtime/exitus.rs)

`exit_status(&Roman)` turns `initium`'s returned `ni` into a Unix exit status: **the value mod
CCLVI, shifted non-negative** (a negative remainder gains one modulus). Moved from `interp.rs`
unchanged in behaviour.

The truncation is deliberate and specified (packages design §4.3), so it stays. What was missing is
any sign that it happened — `redde °CCC` exited 44 in silence, and `redde °CCLVI` exited 0,
which reads as success. The second return value is that sign (**D-10**): `Some(warning)` exactly
when the returned value did not already fit in 0..=CCLV, carrying the message

```text
reditus 'initium' extra fines status exitus est; ad XLIV redactus
```

`Warning` here is a minimal, dependency-free stand-in for a caller's own warning type, carrying only
the message text, because `runtime` cannot depend on `lingua-serpentis` or anything else; a
caller with a richer warning type (file, line) wraps it.

---

## 11. The guard (custos.rs), and why not `.init_array`

Removing the recursion cap means deep recursion faults on the guard page, and the robustness
invariant does not permit signal death — `judge` maps a signalled process straight to a `no abort`
violation, and **D-04 recorded exactly this** (`SIGABRT`, exit 134) as "outside the error model
entirely". So the fault is caught in `custos` and reported as an ordinary exit-1 Latin diagnostic.

**`std::rt::init` never runs** (spec §8.1). The entry point is `#[unsafe(no_mangle)] extern "C" fn
main`, so `lang_start` is bypassed, std installs no `SIGSEGV` handler, and there is no English `has
overflowed its stack` to suppress and nothing to coexist with. The disposition is ours outright.
**Invariant:** should the entry point ever become a Rust `fn main`, `liss_custos_institue` must run
*after* `std::rt::init` so that it overrides std's handler rather than being overridden by it. The
test target is precisely that situation and installs from `main`.

**And the override is measured, not assumed.** `main` in `tests/custos.rs` is a Rust `fn main`, so
`std::rt::init` *has* run and std *has* installed a `SIGSEGV` handler and an alternate stack of its
own; `liss_custos_institue` is called afterwards and must override both. The `abyssus` scenario is
what proves it does, because before this task that child died with std's English `has overflowed its
stack` and `SIGABRT`. It now exits 1 with `Error: acervus vocationum exhaustus est`, and
`deep_recursion_is_a_latin_exit_and_never_a_signal` holds it there.

### Getting the bounds right

The main thread's stack **grows on demand**, so every interface that reports the stack as it
currently *exists* reports a small fraction of what it may become, while the guard page sits at the
bottom of the *potential* stack, megabytes below. Measured on musl, x86_64, `RLIMIT_STACK` 16 MiB:

| method | reports | |
|---|---|---|
| `pthread_getattr_np` + `pthread_attr_getstack` | 124 KiB | wrong |
| `getrlimit` against a local in `main` | 16 MiB, top ~32 KiB low | wrong |
| `/proc/self/maps` `[stack]` | 152 KiB | wrong |
| **`[stack]` high address − `RLIMIT_STACK`** | **16 MiB** | **right** |

The first and third describe only the pages already *touched*. The second gets the size right and
the top wrong, because `argv`, the environment and the auxv sit above `main`'s frame. So **`fines`**
takes the high address of the `[stack]` mapping and subtracts `RLIMIT_STACK`, falling back to the
known-approximate second method only where `/proc` is not mounted, and treating `RLIM_INFINITY` as
bounds-unknown. `fines` is pure — it installs nothing and stores nothing — so tests can compare
it against `/proc/self/maps` without arming a handler.

`culmen_mappae` reads `/proc/self/maps` with `open`/`read` over a fixed buffer rather than through
`std::fs`: this crate is `core` and `libc` throughout, and a `/proc` read that allocates would be
one more thing to reason about on a path that runs before the program does. The scan is its own
function (`culmen_ex_fd`) so that no `?` inside it can return past the `close`; every such path is
provably unreachable — the slices are bounded by the read's own return value — but a descriptor
whose release depends on that argument is the wrong shape to leave in a runtime. A line longer than
the 256-byte buffer is truncated and then simply fails to end with `[stack]`, which is correct,
because the long lines are file mappings and the anonymous ones are short. In `culmen_lineae`, the
**first** `-` separates the two addresses: hex digits contain none, and the `rw-p` that does comes
after a space.

### The constants

- **`MENSURA_ALTERNA` — 64 KiB**, the alternate stack. Deliberately not `SIGSTKSZ`: musl's is a
  small compile-time constant, and on aarch64 with SVE the true minimum can exceed it. 64 KiB is
  unconditionally enough for a handler that allocates nothing.
- **`LIMES` — 1 MiB**, the guard band below the low bound, not one page. Linux's `stack_guard_gap`
  defaults to 256 pages, so a fault caused by stack growth can land that far below the mapping.
- **`PAGINA_MINIMA` — 4096**, the page size to assume if `sysconf` cannot say — never the answer,
  only a floor. aarch64 kernels commonly ship 16 KiB and 64 KiB pages, which is why `pagina` asks
  rather than assuming; but `_SC_PAGESIZE` cannot fail on Linux, and the 1 MiB band swallows the
  difference either way.

`ALTERNUS` is `static`, not allocated: by the time the handler runs there is no stack left to
allocate one on. It is all-zero, so it costs `.bss` and not image size, and 16-byte aligned, because
the kernel hands the handler this buffer as a stack and the ABI wants the frame aligned. `FUNDUS`
and `PAGINA` are computed once at initialisation and **never in the handler**; a `FUNDUS` of zero
means bounds-unknown, which the handler reports as exhaustion — in a program whose only unbounded
recursion is user-written that is overwhelmingly the right guess, and a wrong guess still satisfies
the invariant.

### Installation

**`pone_manum`** installs the alternate stack and the two dispositions, returning `false` if the
kernel refused any of the three. All three are checked, and all three fail the same way — an
unchecked call beside a checked one reads as an oversight rather than a decision, and the decision
here is that there is nothing to do: a handler with no stack of its own would fault on entry, and a
kernel that refuses a `static` buffer of a size it already accepts, or a signal number that has
existed since v7, is not a kernel a Latin sentence will help.

`SA_ONSTACK` because there is no stack left; `SA_SIGINFO` because `si_addr` is the whole
discriminator; **both signals**, because on aarch64 a stack fault may arrive as either. Both signals
go in the mask, not an empty one: without `SA_NODEFER` the kernel blocks the signal being delivered,
but only that one — so a `SIGBUS` could re-enter the handler on the alternate stack it is already
using. One line closes it, and `_exit(1)` means neither is ever unblocked again.

**`SIGPIPE`** is set here too, and for the same reason: `std::rt::init` never ran, so the runtime
does not inherit std's `SIG_IGN` and the default disposition would kill the child on `prog | head`.
`scriptio::scribe_ad` already treats the `EPIPE` that then surfaces as an ordinary end of output and
exits 0 (spec §6); without the disposition change that code is unreachable. This is **D-06**
relocating from the CLI into the runtime, unchanged in behaviour.

### The handler

It may use only `write(2)`, `_exit(2)`, atomic loads on already-initialised statics, and integer
comparison. No allocation, no formatting, no `std::io` — it locks. **It never reads
`ucontext_t`:** recovering the faulting stack pointer from `uc_mcontext` would mean hand-rolled musl
struct offsets with no header to check them against, and `si_addr` against precomputed bounds is
architecture-independent — which is why this module needs no per-target code beyond the two signal
numbers.

It first calls `effunde`, for the one case the flush-at-exit design could not have covered:
everything `scribe` produced before the fault still reaches stdout (**D-08**). Its result is
deliberately dropped — a closed descriptor 1 must not decide this program's status. Then it writes
the prefix, the sentence and a newline to descriptor 2 and calls **`_exit`, never `exit`**: `exit`
runs `atexit` handlers on a corrupted stack.

The discriminator can afford to be generous, and its failure mode is the point: **both branches are
well-formed exit-1 Latin diagnostics**, so a misclassification yields a wrong-but-well-formed
message, never an invariant violation. A Lingua Serpentis program has no pointers, no FFI and no
`unsafe`, so the only ways compiled code can fault are stack exhaustion and a compiler bug.

### Why the installer is called from `main`, and not from `.init_array`

**The runtime's `main` must call `liss_custos_institue` before `liss_initium`, and nothing in
`abi` can make it.** `tests/custos.rs` carries the enforcement instead:
`a_main_in_this_crate_must_install_the_guard` fails the moment an entry point appears in `src/`
without the call.

An `.init_array` constructor would remove the need to remember. On a **host** build it is rejected
outright: `std::rt::init` runs *after* the constructors and installs std's own `SIGSEGV` handler
over ours, so the mechanism is not merely untested there — it is inert there, which is the worst
way to ship something. On the **musl** target it would in fact work — musl's `__libc_start_main`
runs `.init_array` before `main`, and since `std::rt::init` never runs there is no std handler to
lose the race to. It is still not what `ingressus` does:

- **The host build and the musl build must be the same program.** A constructor is inert on the host
  and live on musl. Shipping a mechanism that is *provably wrong* in the configuration `cargo test`
  exercises means the thing tested is not the thing shipped.
- **`liss_custos_institue` must run once**, and a constructor is an invisible second entry: the next
  reader who adds the obvious explicit call gets two `sigaltstack`s and two `sigaction`s with
  nothing to say so.
- **A call is checkable and a constructor is barely so.** Task 10's test reads the prelinked runtime
  object and asserts that `main`'s relocations name `liss_custos_institue` *at a lower offset than*
  `liss_initium` — i.e. that the installer is called, from `main`, first. Against `.init_array`
  the strongest available statement is "some pointer is in the section", which is where the Task 9
  tripwire already was.
- **A constructor would have to survive `--gc-sections`.** The program link runs it (it is worth 0.9
  MB on the image), and an `.init_array` entry is reachable only from the linker's own root set, not
  from `main` — so the guard's liveness would depend on the linker keeping a section nothing
  references. Linkers do treat `.init_array` as a root, and libwild presumably does too, but
  "presumably" is a poor foundation for the one mechanism standing between deep recursion and signal
  death. A call from `main` is live because `main` is live.

So: one call, first line, and a test that reads the shipped object rather than the source it came
from.

### The entry point (ingressus.rs)

Compiled code provides `liss_initium` — the lowering of the user's `munus initium reddit ni`,
returning one owned `ni`. It is undefined in the runtime object on purpose: it is the one symbol the
program link supplies, and Task 10's test asserts it is still undefined in the embedded blob.
`abi` provides `main`, which owes four things:

1. Call `liss_custos_institue` — first line, before anything can recurse. It also sets `SIGPIPE`
   to `SIG_IGN` (D-06, spec §6), so that is not repeated.
2. Call `liss_initium`, read the returned `ni` with `roman_of`, then **free it** — the leak
   counter is checked two lines later and a returned value nobody freed is a leak like any other.
   Reading before freeing matters: `roman_of` copies the digits out, so the pointer is dead by the
   time `exit_status` sees the value; freeing before the conversion would be reading freed memory.
3. Convert with `runtime::exit_status` and, if there is a truncation note, write it to stderr
   *before* exiting, as `Monitum: …` — the same sentence and the same prefix the CLI wrote, so
   `conformance/monita/status-exitus-redactus` reads identically. Unbuffered, so it cannot queue
   behind program output that a closed descriptor 1 would then lose (D-10).
4. Hand the status to `defectus::liss_exit`, which flushes stdout, checks the live-allocation count,
   and exits — **1**, not 70, for a leak.

The order matters in one place only and it is the one the fixture pins: the note reaches descriptor
2 before the process ends. Stream *interleaving* is not observable (spec §6), so nothing else is
ordered by design.

Exit **1** for a leak specifically, and not a distinctive code such as 70, because `judge` treats
any status outside {0, 1, 101} as "this came from `initium`, therefore it is the program's own
choice and legitimate" — a leak reported as 70 would be silently accepted by the one harness most
meant to catch it. The diagnostic text carries the distinction; the exit code carries the invariant.

`custos` owns its own `main` in `tests/custos.rs` (`harness = false`) for a related reason: every
bound spec §8.2 derives describes the *main* thread's stack, and `libtest` spawns a thread per test
even at `--test-threads=1`; a recursion on a worker faults in an ordinary `mmap` megabytes from the
band and takes the handler's other branch. Owning `main` is the only way to put the scenario on the
thread the runtime runs on.

---

## 12. The typed IR (`ir`)

Types only: no producer (the checker) and no consumer (codegen) live in this crate. See spec §4.
**Every `Expr` node carries its own `Ty` so codegen never consults an environment and never infers
— that is the entire purpose of the crate.**

`Grain` is `Uncia` (1/12, `mnr`) or `Scripulum` (1/288, `nr`). `Type::Float`/`Double` collapse into
`Ty::Frac(Grain)` here — every operation on them is identical modulo one constant (12 or 288) —
but the *checker* keeps them distinct in its own `Type`, because its diagnostics say `mnr` and `nr`.

`Ty` has six variants and no more: `Bool`, `Char`, `Int`, `Frac(Grain)`, `Text`, `List(Box<Ty>)`.
`Const` has one variant per `Ty` that can be spelled as a literal; `List` has no `Const` arm because
list literals are always built from `ExprKind::ListLit`, never a single constant. A `Const::Frac`
carries the grain it was resolved to plus its count of that grain — `°I.VI mnr` is six unciae,
`Roman::from_i64(6)`.

`SlotId(u32)` indexes one local in a function's flat `slots: Vec<Ty>` (§4.2). **Slots are allocated
per function and never reused**: leaving a block pops the name binding, not the number, so shadowing
needs no special case. Parameters are slots `0..params`.

`ExprKind::Binary` carries `operand_ty` **separately from the node's own `ty`**, because comparisons
(`Eq`, `Ne`, `Gt`, `Lt`) yield `Bool` from operands that are not `Bool`, and codegen needs to know
which `cmp` to call (§4.1, property 2). `Len` (`longitudo`) and `Append` (`adde`) are desugared at
lowering time (§4.4); neither is a function and neither has a symbol.

`Stmt` carries `line` and nothing finer — exactly the interpreter's granularity (`exec` passes
`s.line` into every `eval`), so it is exactly the attribution the corpus is blessed against.

`StmtKind::Assign` rebinds an existing slot, and ownership is **evaluate → free old → store**
(§7.3.3): `x = x + °I` must read `x` before the old value dies. `IndexAssign` frees the displaced
element inside `liss_list_put` (§7.3). `ExprStmt`'s root is freed if nothing consumed it (§7.3.1).
`If` carries no separate "else if" arm, because `aut si` is already flattened by the parser into a
one-statement else block before lowering (§4.4). `While` (`indem`) is not desugared: it is
trivially a CFG.

### `StmtKind::For` — why a `dum` header is a `Block`

`dum` is a C-style init/cond/step loop, and it is **not desugared**: `continua` must still run the
step, so codegen builds the CFG directly rather than lowering to a statement tree (§4.4).

`header` is the init clause and, decisively, **the scope it opens**: the `dum` variable is declared
there and appears in `header.declares`. It is a `Block` rather than a bare `Stmt` so that **every
owned slot in a function lives in some `Block::declares`**, with no exception codegen has to
remember. A header slot outside that invariant leaks on `redde`, which leaves the function without
ever reaching the exit block where §7.3.5 emits the header teardown — and §7.3.4's "every
enclosing block" would then find nothing to free.

The header's own teardown is still emitted once, after the exit block, rather than on the loop's
back edge: `abrumpe` and `continua` do *not* free `header.declares` (§7.3.5, §7.3.6). `redde`
does, along with every other enclosing block.

`Return` evaluates, then frees every slot of every enclosing block of the function, innermost first;
the returned temporary itself is never among them (§7.3.4). **"Every enclosing block" is exhaustive
and needs no supplement**: the parameter frame is the body `Block` and a `dum` header is a `Block`,
so walking the block stack reaches every owned slot exactly once.

### `Block` and `Func`

A `Block` is its statements plus the slots it introduces, freed in reverse declaration order on
scope exit (§7.3.2). `declares` comes from the checker's scope stack, **not from codegen
re-deriving scopes** — it is the ownership machinery (§7.3), and re-deriving it would be the same
mistake as re-deriving types. Every scope the checker opens is one of these: a `si`/`aut` arm, a
loop body, a `dum` header, and a function body, which is the parameter frame. That is the invariant
codegen leans on: **an owned slot that is in no `Block::declares` is a slot nothing frees.**

`Func::file` is `None` in single-file mode, where diagnostics read `Error in linea N:` rather than
`Error in 'x.liss', linea N:`. It lowers to a zero-length file name at the ABI (§4.1, property 4).

`Func::params` exists **for the signature and the ABI** — how many incoming values there are and
which slots to store them into on entry. It is **not a free list.** The parameter frame *is* the
body's scope, so the parameter slots are already the first entries of `body.declares`, and the
ordinary scope-exit teardown (§7.3.2) and `Return`'s walk of every enclosing block (§7.3.4) each
free them exactly once. Freeing `0..params` again off this field, on the strength of §7.3.4's
"including the parameter frame", **double-frees every heap parameter on every `redde`**.

### Symbol mangling

```text
liss_<n>_<ns>_<m>_<name>
```

where `n` and `m` are the byte lengths of the *escaped* components (§4.3). `escape` maps any
character outside `[A-Za-z0-9_]` to `_uXXXX_`, that character's Unicode scalar value in lowercase
hex — **one escape per `char`, not per UTF-8 byte**. Identifiers are Unicode-alphabetic
(`lexer.rs` uses `is_alphabetic()`), so a passthrough is not an option: non-ASCII identifiers are
legal in this language.

Length-prefixing the escaped form is what keeps the mapping injective — without it, `symbol(None,
"aê")` and `symbol(None, "a_u00ea_")` would collide, since the first's escape is a literal prefix
of the second's spelling. And the character immediately after `liss_` is always a digit (the first
digit of `n`), which is the collision proof against runtime entry points such as `liss_int_add` and
`liss_panic`, whose next character is always a letter.

A worked example, from `codegen/tests/vocationes.rs`: `mathēmatica` escapes to
`math_u113_matica` — sixteen characters — and `quadrātum` to `quadr_u101_tum`, fourteen, so the
package function comes out of the object as `liss_16_math_u113_matica_14_quadr_u101_tum`. **A
per-*byte* escape would have written `_uc4__u93_` for the one `ē` and the length prefixes would
still have been self-consistent**, so nothing but a written-out expectation catches it. That is why
`nomina_non_ascii_per_fasciculos_transeunt` and `omne_munus_symbolum_fasciculi_sui_gerit` spell
their expected symbols as literals rather than as `ir::symbol(..)` calls: a mangling that
changed would still agree with itself, and what these assert is that the *linker* sees what §4.3
wrote down. The one to read closely there is `liss_5_alpha_7_initium` — the prefixes count the
**escaped** components, and `alpha` is five characters and `initium` seven.

---

## 13. Latin sentences this crate can say

Every runtime diagnostic is written as an **inline literal at its `liss_panic` call site**, never as
a reference to a shared constant table (spec §10.3). `catalogus.rs` extracts the first string
literal within 300 bytes of a marker; a site that forwards a constant carries no literal of its own
and reads **`UNPARSED`**, hiding the message from the ratchet. Where two call sites would raise the
same sentence they share one `-> !` helper in `defectus`, which keeps the literal unique in the
workspace and so uniquely attributable — the same reason `interp.rs` collapsed its three `divisio
per nullum` arms into one closure (D-24).

The message pointers are **C string literals** (`c"…"`). That is not decoration: it means the
sentence appears exactly once at the call site, with no separate byte length to keep in sync and no
`\0` escape for `catalogus` to mangle, while still being a plain `*const c_char` for codegen to hand
over. `tests/abi.rs` reads every `c"…"` literal out of this crate's source as a user-visible Latin
sentence — which is why a path (`VIA_MAPPARUM = b"/proc/self/maps\0"`) and two prefixes
(`PRAEFIXUM_ERRORIS = b"Error: "`, `MONITUM_PRAEFIXUM = b"Monitum: "`) are deliberately **not**
`c"…"` literals. A path is not a sentence, and a prefix is a fragment.

`PRAEFIXUM_ERRORIS` is also deliberately **not** named `NUNTIUS_*`, **and it was.** Spec §10.3's
rule is that every async-signal-safe *message* constant is named `NUNTIUS_*` and that `NUNTIUS_` is
a catalogue marker. This is a prefix, not a message: under the old name the ratchet made it a site
and read `Error: ` as its diagnostic, adding a row that is not a sentence anyone can be shown.
Renaming it keeps the rule exact in both directions — every `NUNTIUS_*` is a message, and every
message is a `NUNTIUS_*` — instead of carving an exception into the scanner for the one constant
that breaks it.

### The catalogue

| sentence | raised from | when |
|---|---|---|
| `divisio per nullum` | `defectus::divisio_per_nullum` | `/` by zero, for `ni`, `mnr` and `nr` alike |
| `modulus per nullum` | `defectus::modulus_per_nullum` | `%` by zero |
| `index extra fines: {} (longitudo {})` | `defectus::index_extra_fines` | a subscript outside `I..=longitudo`, from either `xs[i]` or `xs[i] = v` |
| `memoria non liberata est` | `defectus::liss_exit` | the live-allocation count is nonzero at exit |
| `defectus internus: memoria violata est: verificatio` | `defectus::memoria_violata` | a `verifica` rejected a value |
| `defectus internus: memoria violata est: signum` | `custos`'s `NUNTIUS_MEMORIA` | the kernel reported a fault away from the stack |
| `acervus vocationum exhaustus est` | `custos`'s `NUNTIUS_ACERVUS` | the fault is within the stack band, or the bounds are unknown |
| `memoria exhausta est` | `defectus::memoria_exhausta` | `malloc` returned null, or a length computation overflowed `usize` |

`modulus per nullum` is deliberately *not* folded into `divisio_per_nullum` with an interpolated
operator: it is a different diagnostic, and one pattern with a hole would cost the catalogue the
ability to tell the two apart.

`index extra fines` renders **both numbers as Roman numerals**. The length interpolated as a bare
`usize` was **D-12** — the one place in the language a user could see an Arabic number. An empty
list reads `longitudo nullus`.

**The two `memoria violata est` sentences share a stem and differ in the tail, and must.** They are
the same fact about the same program. They cannot share a literal: `memoria_violata` reaches its
sentence through `liss_panic`, which the signal handler must not call, and §10.3 wants the literal
inline at the site the catalogue attributes it to. Both **are** catalogue sites — the handler's
through the `NUNTIUS_` marker, `defectus`'s through `liss_panic(` — and an earlier note in
`custos` claiming otherwise was simply wrong. Written identically they were a pair that could only
ever degrade: `catalogus.rs` grants `fixtured` only to a pattern unique in the workspace, so the
first provocation of either would mark both `ambiguous`, permanently. The tails say how the fault
was *found*, which is the one thing a reader of the sentence could not otherwise recover:
`verificatio` is a `verifica` rejecting a malformed value; `signum` is a hardware fault the kernel
reported.

`acervus vocationum exhaustus est` keeps the `vocatio` root so that a reader who knew the
interpreter's `vocatio nimis alte implicata` recognises its successor. It is structural — no file,
no line — because a signal handler has a return address and no line table (spec §8.4).

`memoria exhausta est` is new vocabulary, and the one sentence in this crate the spec does not
already name. It is needed because the alternative to reporting it is ignoring a null from `malloc`,
and it is **permanently UNCOVERED by design**: no fixture can exhaust a 4 MiB-floor address space on
purpose.

`defectus internus: ` in either form is the compiler admitting to a bug. `parse_diagnostic`
classifies these as `Diagnostic::Structural`, and `judge` refuses them outright; the fuzzers exist
to find them.

### `liss_panic` (defectus.rs)

`fmt` is the sentence; up to two `{}` holes are filled from `arg0` and `arg1`, rendered as Roman
numerals — which is what lets `index extra fines: {} (longitudo {})` be one literal at one site
rather than a message assembled somewhere the diagnostic catalogue cannot see.

`file` is the source path and `line` the statement's line, both supplied by codegen. **Null and
empty both mean "no file"**: spec §4.1 lowers `Func::file: None` to a zero-length name, and a
codegen that emits a null pointer instead means the same thing, so accepting both costs one
comparison and removes a way for the two halves of the contract to disagree. `line` of 0 likewise
means "no line". Codegen supplies them because a signal handler has no line table and this design
emits no debug information (spec §8.4), so there is no ambient location for the runtime to consult.
The four shapes are the **first two levels** of the diagnostic grammar
(`superpowers/specs/2026-08-09-caret-diagnostics-design.md` §3), which `ProgramError`'s `Display`
writes in full:

```text
erratum: index extra fines: X (longitudo II)
  in 'principale.liss', linea III

erratum: divisio per nullum
  in linea III

erratum: ...
  in 'principale.liss'

erratum: memoria non liberata est
```

**The third level — the source line under a caret — is never written here, and that is by design
rather than by omission.** Each level of the grammar is independent of the one below precisely so
this can stop after two: a running program has no source text, and getting it one would mean
widening the ABI, which §5.4 pins at `(file, line)`.

The line number is a **Roman numeral**, through the same `Roman::display()` the `{}`-hole loop two
lines below already calls. `decimales` — the function whose own doc comment called it *"the one
number this crate formats without going through `Roman`"* — is gone, and with it the last Arabic
numeral anything in this language prints. `judge`'s `no Arabic digits` rule covers the location row
because of it.

**The risk in changing any of this is the build, not the diff.** `codegen/build.rs`
cross-compiles this crate into two prebuilt musl archives during the outer build; a stale archive
leaves the two workspaces disagreeing while *both* test commands pass. Force a rebuild and confirm
a runtime-origin fixture actually moved.

**Arity is validated before a single byte is written.** A hole with no argument used to render as a
literal `{}`, which produces a *plausible* exit-1 Latin diagnostic — braces are not digits,
English or Rust type names, so `judge` accepts it and a codegen bug that forgot an argument goes
unnoticed. Refusing up front turns it into the internal defect it is, and refusing before any output
means the sentence is never half-written.

**Termination.** Reading an argument checks its magic and its digits, and a corrupt one calls
`memoria_violata`, which re-enters `liss_panic` — with both arguments null. That inner call
converts nothing, so the recursion is **bounded at depth two by construction** rather than by a
flag.

**The argument-free path allocates nothing.** It is written as direct `write(2)` calls over borrowed
byte slices and one stack buffer for the line number, rather than as one assembled `String`. That is
not tidiness: `memoria_exhausta` is raised from `allocare` at the moment `malloc` returned null, and
a `String` here would then reach `handle_alloc_error`, which **aborts** — `SIGABRT` and exit 134,
D-04's shape, produced inside the one function whose whole job is to prevent it. Only the
two-argument path allocates, in `Roman::display`, and it is reached only from `index extra fines`
where the heap is healthy by construction.

Several writes rather than one is harmless: stderr is unbuffered, writes append in order, and stream
*interleaving* is not observable (spec §6).

`decimales` renders `n` in decimal into a caller-supplied buffer — **the one number this crate
formats without going through `Roman`**, because `linea 3` is the one place spec §8's diagnostic
grammar admits an Arabic numeral.

`liss_panic` uses `effunde` and not `liss_flush`: a closed descriptor 1 must not end the program
from inside a raise. `liss_flush` is the *success* path's flush and exits 0 on one, which from
inside a raise would be `judge`'s `clean success` violation — and, because the flush returns early
on an empty buffer, would fire only for programs that happened to have printed something.
