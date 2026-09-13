# The road to 1.0

Lingua Serpentis is `0.1.0`. 1.0 is the last feature release: a closed product you can write the
same kinds of programs in that you would write in Python, Rust, or Odin using only what ships, and
then the language stops. After 1.0 the work is dependency updates, bug fixes, and the existing test
ratchets — not new syntax, not new bibliotheca packages, not a second OS.

This file is the map. It is undated on purpose, like `internals/`: when a milestone ships, it is
marked done in place and its spec is linked. It does not replace those specs. Keyword choices,
ADT form, bibliotheca package names, and the shape of `try`/`catch` are decided there, not here.
Names used below that the language does not already have are **placeholders**.

An unqualified "spec §N" still means `superpowers/specs/2026-08-07-compiler-backend-design.md`.

---

## What 1.0 is

A working product. CLI tools, files, JSON, an HTTP client and a small HTTP server, with inference,
a REPL, exceptions, threads, async, and debug info in the box. Batteries live in the bibliotheca,
implemented in Rust, shipped with `liss`. There is no package manager and no way for a program to
link an extra crate.

Then the version is `1.0.0` and the surface freezes.

## Current position

Linux AOT compiler, `x86_64` and `aarch64`. Explicit types, Roman numerals, lists, packages, Latin
diagnostics with carets. `liss build` emits a static musl ELF.

Not yet: maps, first-class functions, stdin, files, `>=`/`<=`, `sit`, a REPL, `try`/`catch`,
threads, the two named optimisations, a consolidated language reference. Three deferred language
decisions (`D-13`, `D-20`, `D-26`) and two recorded implementation defects (`D-28`, `D-29`) sit in
`defectus/2026-08-03-index.md`.

## What 1.0 is not (and therefore not later either)

There is no 1.1 to grow into. Anything omitted here is omitted for good, unless a 2.0 is ever
declared.

- Windows, macOS
- A package manager, user-facing Rust FFI, crates.io for `latinitas`
- Short-circuiting `et`/`vel` — today both sides run; spec §3.5. Changing that is a language
  change this road does not take
- Nested packages, visibility keywords, manifests
- **D-30** (caret vs wide characters) — accepted. `unicode-width` is a dependency the snippet
  renderer does not earn

---

## Milestones

Build in this order. A later milestone may start its spec while an earlier one is in review, but it
does not merge first. Breaking work is all in `0.x`. Milestones 2–9 each need a dated spec in
`superpowers/specs/` before they are built; this file only names the bar.

### 1. Close the implementation holes

No language change. No new spec: the closes are already written in the register.

- **D-28** — `runtime`'s panics and English strings ship in every compiled binary. Give `Roman` a
  `display_ad` into a `Sink`; hold the shared `runtime` modules to `abi`'s rule; test that the four
  English strings are absent from the image.
- **D-29** — the layout proof compares source against source and never sees the blob. The close in
  the register stands: the proof has to see the bytes that ship.

### 2. The last breaking language decisions

Needs a spec per row. Do them before anything additive so 1.0 does not freeze a hole.

| ID | Decision the spec must take |
|----|-----------------------------|
| **D-13** | Canonical Roman form, legal subtractive pairs, repetition limits. `°IIX` is accepted today. |
| **D-20** | A magnitude cap and a diagnostic. `x = x * x` in a loop is why `terminates` is still allowlisted. |
| **D-26** | Terminator in a `dum` init clause. Recommended: reject it. The compiled reading (return / bind to the enclosing loop) is the other candidate; do not freeze by accident. |

### 3. Small holes

Needs a spec. Additive. Original v1 deferred these; they stay cheap.

- `maior_aequalis` / `minor_aequalis` — the 2026-07-19 spec's guess; the milestone spec confirms
- A for-each over `lista` — keyword is the spec's
- Fraction-literal gaps: `mnr x = °V`, a sub-1 literal (`°:N/D` or `°nullus:N/D`)
- Explicit `mnr`/`nr` conversion. Still no implicit mix

### 4. Compiler performance

Needs a spec. No language change. Named in compiler-backend spec §2 and §13 so they would have
names when scheduled.

- ***Subscriptio mutuata*** first — `exemplum-series` is quadratic; this is the cheaper of the two
  (an `Index` whose base is a `Load` of a slot)
- ***Elisio ad ultimum usum*** second — needs dataflow; ownership stays syntactic

Tagged small-integers stay out: the ABI is one machine word, every `ni` is an allocation, and
changing that is not a performance pass.

### 5. Functional layer

Needs a spec. Additive language. Assignment, `si`/`dum`/`indem`, and explicit types stay. This is
not a rewrite into Haskell.

- First-class `munus` values
- Closures, under the existing consuming / last-use ownership story
- Product and sum types
- Pattern matching
- An `optio` / `exitus` (placeholder) as a sum type values can carry. `try`/`catch` is milestone 6;
  that spec decides whether it is sugar over the sum or a second path

Maps (`tabula`, placeholder) are a built-in parametric type like `lista`, in this milestone or
immediately after it, **before** the bibliotheca. JSON and HTTP need them.

### 6. Inference and exceptions

Needs a spec. Additive, after the functional layer so there is something to infer and something to
throw.

- **`sit`** — local type inference. Declarations may omit the type when the right-hand side
  determines it. Function headers stay explicit until that spec says otherwise
- **`try`/`catch`** — language-level exceptions. Latin keywords are the spec's. Coexistence with
  the sum type from milestone 5 is a design question, not a roadmap one

The REPL is the same want; it is **built in milestone 9**, after the bibliotheca and after
threads / async / DWARF, so the prompt speaks the finished language.

### 7. Bibliotheca

Needs a spec. The bulk of 1.0. No package manager: `affer io;` (placeholder name) resolves to a
package the compiler ships. Resolution, the set of names, and the Rust implementation (in `abi` /
`runtime`) are the spec.

**Closed 1.0 set** — if it is not in this list, it is not in the language:

| Package (placeholders) | What it is for |
|---|---|
| operations on `lista` and `voc` | remove, concat, slice, contains, split, join — the lists design deferred these |
| operations on `tabula` | get / set / keys, enough for JSON objects |
| `io` | stdin, files, args, env. `scribe` stays the language primitive for stdout |
| path | join, split, exists |
| `tempus` | clocks, timestamps, formatting. Compiler diagnostics still carry no Arabic; program output prints what the package specifies |
| `json` | encode / decode to `tabula` / `lista` / scalars |
| `mathematica` | the rest of math, and a random source |
| `hash` | digest a `voc` / bytes |
| `rete` | sockets |
| `http` | client, and a small server |
| TLS | in 1.0 with `http`, or the server is a toy you cannot add HTTPS to later |

New `abi` dependencies (an HTTP stack, a TLS stack, a JSON parser) are explicit spec amendments
when this milestone is designed. They are expected. Zero runtime dependencies remains the rule
until that amendment.

Still out of the bibliotheca, forever: SQL, images, compression as a user-facing package,
subprocess, regex. Threads are a language/runtime feature (milestone 8), not a package.

### 8. Threads, async, debug info

Needs a spec.

- **DWARF** — line tables in the image. Compiler-backend spec §2 left this out, which is why stack
  exhaustion has no line number (§8.3). This is the close of that
- **Threads** — a memory model, under the existing ownership rules. The spec is the work; this file
  does not invent the keyword
- **Async** — after threads, or instead of a second concurrency story if the spec collapses them.
  HTTP's server is allowed to use this; it must not wait on a 1.1

### 9. REPL

Needs a spec. `liss` with no file argument opens a session. Compile-and-run, one backend, the whole
language including the bibliotheca.

### 10. The reference and the freeze

Last, because the language has to stop moving. No feature spec — the reference is a document, the
freeze is a version number.

- A **consolidated language reference**. `docs/README.md` already says nobody has written one. It
  is the reading-order collapse of the base spec §5, packages §4, the three numeric specs, lists,
  plus milestones 2–9. Internals stay internals; this is the user-facing document
- Version `1.0.0`
- This file stops being a road and becomes the product bar. New features are out of scope

---

## Compatibility

`0.x` may break. `1.x` may not, except to fix a defect that made a program mean the wrong thing.

From `1.0.0`, a well-typed program keeps compiling and keeps its observational behaviour: stdout,
exit status, accept versus reject. Diagnostic wording remains ratcheted by `catalogus` and `nuntii`
and may change only to fix a wrong sentence.

Additive 1.x work is not a thing this project is doing. The freeze is the point.
