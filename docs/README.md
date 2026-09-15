# Documentation

Everything written about Lingua Serpentis that is not source lives here, in five places:
`internals/` is evergreen reference for the code as it stands, `via.md` is the road to 1.0,
`superpowers/specs/` is one design document per milestone, `superpowers/plans/` is the
implementation plan each of those designs became, and `defectus/` is the defect register. There is
**no consolidated language reference** — the surface language is described across three specs and
their amendments, and this file is the map to them. The reference is milestone 10 of `via.md`.

Read it before reading anything else here, because roughly half of the spec directory has been
amended or superseded by a later document and the front matter alone does not tell you the order.

## Which document answers which question

| Question | Document |
|---|---|
| What is the language, what does it look like | `superpowers/specs/2026-07-19-lingua-serpentis-rust-interpreter-design.md` §5, amended — see below |
| What is a program, where does it start, what is a package | `superpowers/specs/2026-08-01-programs-and-packages-design.md` §4 |
| How do `ni`, `mnr` and `nr` behave and print | the three numeric-type specs — see below |
| How does the compiler work, today | `internals/codegen.md`, and spec `2026-08-07-compiler-backend-design.md` |
| What does a compiled program call, and what owns what | `internals/abi.md` |
| Why is the front end shaped like this; what was tried and broke | `internals/rationale.md` |
| How is any of this tested | `internals/testing.md` |
| Why may I not write a block comment there | `superpowers/specs/2026-08-09-test-harness-libraries-design.md` §6 |
| What shape does a diagnostic have, and why | `superpowers/specs/2026-08-09-caret-diagnostics-design.md` §3 |
| What may a `dum` header's first clause be | `superpowers/specs/2026-09-15-dum-header-clause-design.md` §3 |
| How must a Roman numeral literal be written | `superpowers/specs/2026-09-15-roman-numeral-form-design.md` §3 |
| Is this known to be wrong | `defectus/2026-08-03-index.md` |
| What is left before 1.0, and what 1.0 freezes | `via.md` |

**An unqualified "spec §N" anywhere in this repository — in a comment, a doc, a commit message —
means `superpowers/specs/2026-08-07-compiler-backend-design.md`.** It is the canonical current spec.
Every other spec is cited by filename.

---

## `internals/` — the code as it stands

Four documents, written by extracting the design commentary that the Rust source carried as `//!`
and `///` prose, so that the source could stop carrying it. They are reference documents, organised
by subsystem, and they navigate by file and symbol name rather than line number. They track the code
rather than any particular milestone, which is why they are undated.

**`internals/codegen.md` — code generation.** `crates/codegen`: the crate layout (`lib.rs`),
the Cranelift code generator (`emissio.rs`), the target and embedded-runtime layer (`target.rs`),
the in-process linker (`nexus.rs`) and the musl cross-build that fills them (`build.rs`). Twelve
sections: the IR in and the object out, types and the shared tables, the ownership and free
discipline (the longest section, and the part to read before touching anything), lists as the only
recursive type, the two silent rules, control flow, constants and operators, the linker and TLS,
the target layer, the build script, and codegen's own diagnostics.

**`internals/abi.md` — the ABI, the runtime, and the IR.** The three crates below the checker:
`abi`, the C ABI a compiled program calls; `runtime`, the arithmetic and formatting shared
between compiler and compiled program; and `ir`, the typed IR the checker emits. It states the
one-machine-word value representation (spec §5.1), the kind codes (spec §5.3), the consuming calling
convention (spec §5.4), the ownership and null-or-live invariant (spec §7), the magic word, the heap
guard, the output path and signal-handler buffer, the stack guard in `custos.rs` and why not
`.init_array`, and the Latin sentences the crate can say. The `# Safety` clauses on individual
`unsafe` functions are deliberately **not** reproduced there — they stay in the source; the document
is the contract all 62 of them instantiate.

**`internals/rationale.md` — the front end, and the counterfactual record.** Part 1 is the reference
for `crates/lingua-serpentis`: the pipeline and where it stops, program loading, the lexer, the
parser, the checker, the Latinity lint, error rendering, the `liss` driver, and the diagnostic-site
convention. Part 2 is the workspace-wide record of what was tried and what it broke — every "this
used to be Z", every "without this line, *measured failure*" — from all six crates, filed by file
and symbol. Where a defect has a number, Part 2 keeps the one-line statement of what would break and
points at the register.

**`internals/testing.md` — testing.** The one cargo invocation and the second it replaced,
the fixture corpus, the diagnostic ratchet (`catalogus.rs`), the robustness invariant, the
generated-program property (`generative.rs`), the CLI suites, and the several tests that read Rust
source as text and therefore constrain how Rust source may be written. It documents the machinery,
not the coverage: the corpus and the `catalogus` snapshot remain the authority on what is covered.

---

## `via.md` — the road to 1.0

One undated file. 1.0 is the last feature release: a closed product, then dependency updates and
fixes, not new syntax and not a second OS. The file sequences ten milestones, names the never-list
(Windows, macOS, a package manager, user-facing Rust FFI, short-circuiting `et`/`vel`, nested
packages, D-30), and states the compatibility promise. It is not a spec. Each build milestone still
gets a dated document in `superpowers/specs/` before it is built; when one ships, `via.md` is marked
done in place and the spec is linked. After 1.0 the file stops being a road and becomes the product
bar.

The consolidated language reference does not exist yet. It is milestone 10.

---

## `superpowers/specs/` — one design per milestone

Fifteen documents, dated, each the design agreed before a milestone was implemented. Front matter
carries **Date**, **Status**, **Author**, and — where one exists — a **Relationship**, **Amends** or
**Supersedes** line naming the document it changes.

| Spec | Date | Status | |
|---|---|---|---|
| `2026-07-19-latin-identifier-checker-design.md` | 2026-07-19 | Approved (design) | The warning-only Latinity checker and the `latinitas` crate, backed by Whitaker's WORDS |
| `2026-07-19-lingua-serpentis-rust-interpreter-design.md` | 2026-07-19 | Approved (design) | The base language: keywords, types, Roman literals, operators, control flow, functions. Amended six times |
| `2026-07-21-haskell-style-signatures-design.md` | 2026-07-21 | **Superseded** | The `munus summare :: ni -> ni -> ni` signature line. Replaced before it reached production |
| `2026-07-22-capit-reddit-function-headers-design.md` | 2026-07-22 | Approved (design) | The function header as it is today — `munus summare capit (ni a, ni b) reddit ni` |
| `2026-07-22-nr-double-distinct-type-design.md` | 2026-07-22 | Approved (design) | Makes `nr` a type distinct from `mnr`. Its `f32`/`f64` backing is superseded; the distinction stands |
| `2026-07-22-v2-lists-design.md` | 2026-07-22 | Approved (brainstorming complete; plan not yet written) | `lista`: the first parametric type, `[...]` literals, 1-based indexing, `longitudo`/`adde`, value semantics |
| `2026-07-23-roman-native-integers-design.md` | 2026-07-23 | Design approved in brainstorming; written spec pending user review | `ni` stops being an `i64` wearing Roman input syntax: the numeral becomes the runtime value. Amended once, for literal form |
| `2026-07-23-roman-native-fractions-design.md` | 2026-07-23 | Design approved in brainstorming; written spec pending user review | `mnr`/`nr` become duodecimal fixed point — `mnr` counts unciae (1/12), `nr` scripula (1/288) — and print in Latin fraction names |
| `2026-08-01-programs-and-packages-design.md` | 2026-08-01 | Approved — implemented | Program = folder, package = subfolder, entry at `munus initium reddit ni`, `affer` imports, declarations-only top level |
| `2026-08-03-conformance-and-robustness-suite-design.md` | 2026-08-03 | Proposed | The conformance corpus, the robustness invariant, the diagnostic catalogue ratchet, and the defect register it opened |
| `2026-08-07-compiler-backend-design.md` | 2026-08-07 | Proposed | **The canonical current spec.** The ahead-of-time compiler, the typed IR, the C ABI, ownership, stack exhaustion, linking — and the removal of the interpreter |
| `2026-08-09-test-harness-libraries-design.md` | 2026-08-09 | Proposed | The diagnostic ratchet parses rather than scans, which lifts the block-comment rule from five crates; `tempfile` and one shared wait loop in the test support |
| `2026-08-09-caret-diagnostics-design.md` | 2026-08-09 | Proposed | Spans through the front end, and a three-level diagnostic grammar with a caret under the offending expression. `Error` becomes `erratum`; the line number becomes a Roman numeral everywhere |
| `2026-09-15-dum-header-clause-design.md` | 2026-09-15 | Proposed | A `dum` init clause is a declaration or an assignment and nothing else parses; closes D-26 and the `munus`-in-header hole |
| `2026-09-15-roman-numeral-form-design.md` | 2026-09-15 | Proposed | Every `.`, `:` and `/` chunk of a Roman literal is written the way its value prints, 1..=3999, or it does not lex; closes D-13 |

The **Status** lines record the state at the moment each document was written and have not been
maintained since. Every one of the fifteen has an implementation plan in `superpowers/plans/`, so a
status of "pending implementation plan" or "plan not yet written" should be read as history, not as
the current state of the tree.

---

## The amends / supersedes chain

This chain is the project's substitute for a consolidated spec. Resolved into reading order:

### Live, and unamended

- **`2026-07-19-latin-identifier-checker-design.md`** — nothing amends it.
- **`2026-07-22-capit-reddit-function-headers-design.md`** — supersedes the 2026-07-21 signature
  design. Nothing amends it; the header it describes is the one in the language today.
- **`2026-07-22-v2-lists-design.md`** — nothing amends it.

### Superseded outright

- **`2026-07-21-haskell-style-signatures-design.md` is superseded by
  `2026-07-22-capit-reddit-function-headers-design.md`.** The `::` / `->` signature it describes was
  replaced by the `capit` / `reddit` header one day later, before it reached production. It is kept
  as a historical record and should not be read as a description of the language. Its plan,
  `plans/2026-07-21-haskell-style-signatures.md`, carries the same superseded notice.

### The base language spec, and its six amendments

**`2026-07-19-lingua-serpentis-rust-interpreter-design.md`** is still the only document that
describes keywords, types, operators, statements and control flow in one place. Six later documents
change parts of it, and the amendment notices are stacked at the top of the file:

1. **2026-07-21 — function headers.** The code samples in §5.6 and §5.9 were amended for the
   signature syntax of `2026-07-21-haskell-style-signatures-design.md`; prose unchanged. That
   syntax was then superseded, and the samples were re-amended in place to the `capit` / `reddit`
   form — so the samples now read correctly even though the amendment line names the superseded
   spec.
2. **2026-07-22 — `mnr` vs `nr`.** `2026-07-22-nr-double-distinct-type-design.md` amends **§5.2 and
   §6.2**, resolving their open question about whether `mnr` and `nr` collapse to one type. They do
   not.
3. **2026-07-23 — Roman-native integers.** `2026-07-23-roman-native-integers-design.md` amends the
   handling of Roman literals and the `ni` type. Passages describing Roman literals collapsing to
   `i64` and integers printing in Arabic are superseded.
4. **2026-08-01 — the program model.** `2026-08-01-programs-and-packages-design.md` **supersedes
   §5.5's script-style program model**: top-level statements are removed, so §5.5 and the §5.9
   example no longer run as written, and §9's "Imports / modules" v2 candidate is realised as
   Go-style packages (*fasciculi*).
5. **2026-09-15 — the `dum` header.** `2026-09-15-dum-header-clause-design.md` amends **§5.5**:
   the `init` of `dum (init; cond; step)` is a declaration or an assignment, and nothing else
   parses. A terminator there used to return from the function or leave the enclosing loop
   silently; it is now a parse error. Closes D-26.
6. **2026-09-15 — Roman numeral form.** `2026-09-15-roman-numeral-form-design.md` amends **§5.3**
   and supersedes its "exact semantics of `RomanNumeralConverter.py`" and "broaden only if a test
   demands it" clauses: every chunk of a literal split at `.`, `:` and `/` must be the classical
   numeral for its value, 1..=3999, or it does not lex. `°IIX` used to read as 10; it is now
   `numerus Romanus non canonicus: 'IIX'`. The dot form and the fraction shape are unchanged.
   Closes D-13.

### The numeric types, in order

The three numeric specs must be read in sequence, because the second and third each undo a backing
decision made by the one before:

1. **`2026-07-22-nr-double-distinct-type-design.md`** makes `nr` distinct from `mnr`, backed by
   `f64` against `f32`.
2. **`2026-07-23-roman-native-integers-design.md`** makes `ni` a Roman value computed on by carry
   and borrow. It explicitly leaves `mnr`/`nr` as IEEE floats (§2 Non-Goals) and defers the
   fractional engine as "a whole second engine" (§9) — and that non-goal is itself marked
   **Superseded 2026-07-23** in place, because the second engine arrived the same day. **Amended
   2026-09-15** by `2026-09-15-roman-numeral-form-design.md`: §3.5's "input notation is untouched"
   and §4.2's "parsing keeps its shape" give way to a per-chunk form rule, and §7's round-trip claim
   for the dot form `°VII.M` is corrected — it never held.
3. **`2026-07-23-roman-native-fractions-design.md`** amends
   `2026-07-22-nr-double-distinct-type-design.md`: the `f32`/`f64` backing and decimal printing are
   superseded by duodecimal fixed point in Latin fraction names. **The `mnr`/`nr` type distinction
   itself stands.** The integer engine (`roman.rs`, the `Roman` type) is reused, not changed.

### Testing and the compiler

- **`2026-08-03-conformance-and-robustness-suite-design.md`** replaces the ad-hoc §6 "Testing
  Strategy" sections of every feature design above with one standing, program-level suite. Two parts
  of it have since been overtaken:
  - its §2 behaviour freeze is marked **Superseded 2026-08-05, for the behaviour freeze only** — the
    freeze was scoped to building the suite, and the fixes were then executed by
    `plans/2026-08-05-defect-register-remediation.md`. The parts of §2 that are *not* about the
    freeze — no benchmark suite, no speculative language features — still stand.
  - its **§3.2** (in-process by default) is superseded by the compiler-backend design.
- **`2026-08-07-compiler-backend-design.md`** adds the ahead-of-time compiler, **removes the
  interpreter**, supersedes §3.2 of the conformance design, and closes deferred defect D-19. It is
  the spec an unqualified `§N` refers to. **Amended 2026-08-09** by the caret-diagnostics design,
  for §5.4 and §10 only: the diagnostic grammar it fixes at one line becomes three levels, and the
  `(file, line)` pair every fallible runtime entry point takes renders its line as a Roman numeral.
  Everything else in it stands, with one correction: **§9.1's diagram calls `abi` its "own
  workspace", and since 2026-08-09 it is not.** The crate is an ordinary member of the one
  workspace; the deadlock the split was meant to prevent is prevented by the `--target-dir` its
  cross-build passes. Only the parenthetical is wrong — the reason `abi` is split from `runtime`,
  which is the point §9.1 exists to make, is untouched. `internals/abi.md` has the current account.
- **`2026-08-09-test-harness-libraries-design.md`** settles the conformance design's open question
  §11.1 — *"whether the grep needs to become a real parse"* — in the affirmative: the diagnostic
  ratchet reads a `syn` AST rather than source bytes. It amends nothing and supersedes nothing. It
  changes only test machinery, but it is the reason `CLAUDE.md`'s block-comment prohibition applies
  to `crates/abi/src` alone and no longer to `crates/*/src`.
- **`2026-08-09-caret-diagnostics-design.md`** amends the compiler-backend spec's §5.4 and §10 and
  supersedes the four-shape `Display` table in `internals/rationale.md`. It adds no dependency:
  §11's table is untouched. Read its §3 before changing anything that writes to stderr — every
  diagnostic in the language, compiler and compiled image alike, is a prefix of the one grammar it
  fixes there.

### There is no consolidated language reference

Nobody has written one, and this index will not pretend otherwise. To answer "what is the language",
read, in this order:

- **`2026-07-19-lingua-serpentis-rust-interpreter-design.md` §5** — keywords, types, Roman literals,
  operators, statements, functions, printing, comments — with the six amendments above applied;
- **`2026-08-01-programs-and-packages-design.md` §4** — what a program is, what a package is, where
  execution starts, what the top level may contain, `affer` and qualified calls;
- **the three numeric-type specs**, in the sequence given above, for how `ni`, `mnr` and `nr`
  actually behave and print;
- **`2026-07-22-v2-lists-design.md`** for `lista`.

The `conformance/` corpus is the operational answer to the same question: 118 fixtures with blessed
`emissio`, `nuntii` and `exitus` files, which are recorded facts about the language as shipped.

### The interpreter was deleted

The tree-walker is gone — commit `677c4b3`, *"refactor(interp): the tree-walker is gone; the
compiler is the language"*, executing §3.1 of the compiler-backend design. `parse`, `check` and
`interp::execute` no longer exist; the entry points are `parse_file` and `check_program`, and the
compiler is the only backend.

**`2026-07-19-lingua-serpentis-rust-interpreter-design.md` is therefore partly obsolete as an
implementation description** — its §4 architecture, its §6 Rust implementation and every reference
to an interpreter walking a tree describe code that is not there — **while remaining the base
language spec.** Read its §5 for the language; do not read the rest as a description of the program.

---

## `superpowers/plans/` — the implementation plans

Sixteen plans, one per spec plus `2026-08-05-defect-register-remediation.md`, which has no spec of
its own — it works from the conformance design (§5, §7, §8.1) and the defect register. Each plan
opens with a Goal, an Architecture paragraph, a Tech Stack line and, usually, a pointer to its spec,
and is written as numbered tasks with checkbox steps. The house convention is that every code task
opens with **Step 1: Write the failing tests** and **Step 2: Run them and verify they fail**, with
the expected failure text stated. Where a plan and its spec disagree, the spec governs.
`plans/2026-07-21-haskell-style-signatures.md` is superseded, alongside its design.

## `defectus/2026-08-03-index.md` — the defect register

Opened 2026-08-03 by the conformance and robustness suite, and authoritative for severity, repro,
observed behaviour, expected behaviour and the `file:line` cause of each entry. It holds **D-01
through D-30** — as of 2026-09-15, **27 fixed, 2 deferred, 1 open** — plus three appendices:
**Appendix A**, the five corrections to the conformance design's §8.1 table; **Appendix B**, the
triage log for candidates that arrived without an ID; and **Appendix C**, ten corrections to the
register's own entries found while fixing them, three of which would have produced a broken fix if
followed literally. **D-30 sits after Appendix C**, not with the numbered run. D-28 and D-29 came
from the compiler-backend milestone's whole-branch review, were recorded rather than fixed, and are fixed by `via.md`'s milestone 1. Each fixed defect's `conformance/defectus/` fixture has moved to
`errores/`.

## Naming

Specs and plans are dated: `YYYY-MM-DD-<topic>-design.md` under `superpowers/specs/`, and
`YYYY-MM-DD-<topic>.md` — the same topic, no `-design` suffix — under `superpowers/plans/`. The date
is the date the document was written, and it never changes; a document that is overtaken gets an
amendment notice or a superseded banner, not a new date, which is what makes the chain above
followable.

`internals/` is deliberately **undated**. Those four documents are evergreen reference that tracks
the code, not a record of a decision made on a day. When the code changes, they are edited in place.

`via.md` is undated for the same reason. It is the closed 1.0 product, sequenced as ten milestones.
When a milestone ships, it is marked done in place and its spec is linked; the file is not renamed
and does not get a date. After 1.0 it stops being a road and becomes the product bar. It is not a
spec: each build milestone still gets a dated document in `superpowers/specs/` before it is built.

## The back-end crates were renamed

`liss-abi`, `liss-codegen`, `liss-ir` and `liss-runtime` are now `abi`, `codegen`, `ir` and
`runtime`, with their directories renamed to match. The prefix bought nothing: everything here
depends on everything else by path, nothing is published, and `latinitas` had never carried one.

`internals/` and `defectus/` were rewritten, because both navigate by path and a stale path in
either is simply wrong. **`superpowers/specs/` and `superpowers/plans/` were not.** Every one of
them still says `liss-codegen`, and should — they are dated records of what was designed on a day,
and the rule above is that a dated document is never edited except by amendment notice. Read the
old name as the crate that now sits at the corresponding path.

What did **not** change: the `liss` binary, the `.liss` extension, and the `liss_*` C symbols the
ABI exports, which spec §5.4 fixes as the contract between generated code and the linked archive.
