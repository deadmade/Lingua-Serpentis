# Lingua Serpentis — Test Harness Libraries Design

- **Date:** 2026-08-09
- **Status:** Proposed
- **Author:** Manuel (with Claude)
- **Relationship:** Settles open question §11.1 of
  `2026-08-03-conformance-and-robustness-suite-design.md` — *"whether the grep needs
  to become a real parse"* — in the affirmative. Amends nothing and supersedes
  nothing. Adds no language features, changes no compiler behaviour, and touches
  no `[dependencies]` table: every change here is test machinery.

## 1. Vision

Three pieces of this repository's test machinery reimplement solved problems, and
one of them charges the *source* for the privilege.

`crates/lingua-serpentis/tests/catalogus.rs` is the diagnostic ratchet: it
enumerates every user-facing Latin sentence the source can emit and records
whether a fixture provokes it. It finds those sentences by scanning source
**bytes** for one of eleven marker strings, then taking the first string literal
within a 300-byte forward window. It reads text where it means to read code, and
because text has no structure, it cannot tell a statement from a sentence
*about* a statement. So it carries four guards, five `UNPARSED` rows, and two
rules imposed on how Rust may be written in this repository:

> **Never write a `/* … */` block comment under `crates/*/src`.** The scanner
> suppresses a marker found in prose by looking for `//` earlier on the line, and
> assumes no block comment exists. One would invent phantom diagnostic sites.
>
> — `CLAUDE.md`

> Read `docs/internals/testing.md` before changing this: the 300-byte window is
> why a comment may never sit between a marker and its literal.
>
> — `catalogus.rs`

Those are not incidental. The comment guard exists because eight phantom rows
appeared the day `NUNTIUS_` and `Defectus::novus(` joined the marker list —
markers plain enough to occur in English, so every note explaining them became a
site, one of them quoting `Error: ` out of a sentence explaining why `Error: ` is
*not* a message. `docs/internals/testing.md` records that as *"a fixtured
fraction that fell because the file documented itself."*

**A token stream contains no comments.** Parsing with `syn` does not reimplement
guards 1–3 more carefully; it deletes them, and the source rules with them. That
is the whole argument, and it is why this is worth a dependency.

The other two are ordinary duplication, found alongside it and fixed in passing:
two scratch-directory implementations that cannot share code because they live in
different crates, and one `try_wait`/`sleep` poll loop written three times.

## 2. Goals and Non-Goals

### Goals

- **Derive the site list from the AST**, so guards 1–3 become structurally
  impossible rather than heuristically handled (§4, §5).
- **Lift the block-comment prohibition** from `crates/lingua-serpentis/src` and
  the five other crates the ratchet scans (§6).
- **Preserve the catalogue's coverage exactly.** The snapshot header reads
  `123 sites · 70 fixtured · 0 ambiguous · 48 uncovered · 5 unparsed`. Every
  difference must be enumerated in §8 with its cause; measured, there are five,
  all of them scanner artifacts being dropped, and **`70 fixtured · 0 ambiguous`
  does not move.**
- **One scratch directory and one wait loop** instead of two and three (§7).

### Non-Goals

- **Not a coverage change.** No fixture is added, removed or re-blessed. A site
  that was `UNCOVERED` before is `UNCOVERED` after.
- **Not a `liss-abi` change.** That crate's own source-reading test keeps its
  line-oriented scanner, and keeps the assumption that goes with it (§6).
- **Not a runtime dependency.** See §3.

## 3. Dependencies

| crate | where | why |
|---|---|---|
| `syn` (features `full`, `visit`) | `lingua-serpentis` **dev** | parse `crates/*/src` into an AST for the ratchet |
| `quote` | `lingua-serpentis` **dev** | `ToTokens`, to read a site's own token stream for its literal |
| `proc-macro2` (feature `span-locations`) | `lingua-serpentis` **dev** | `Span::start().line`; off by default outside a proc macro |
| `tempfile` | `lingua-serpentis` **dev**, `liss-codegen` **dev** | one scratch-directory implementation instead of two |

**None of these is a runtime dependency, and the distinction is the whole
argument for their admissibility.** Spec §11's rule — *"zero runtime
dependencies"* — governs what a compiled `liss` binary and a compiled user
program link. `lingua-serpentis` already dev-depends on `insta`,
`libtest-mimic` and `proptest`; `liss-codegen` on `object` and
`lingua-serpentis`; `liss-runtime` on `proptest`; `liss-abi` on `libc`. These
four join that list. No `[dependencies]` table in this repository is touched.

**Three of the four are already in the compiler's normal dependency graph, and
were before this document existed.** `libwild` — the in-process linker, tabled in
spec §11 — depends on `derive_more`, `prost`, `strum` and `zerocopy`, each of
which has a proc-macro half, and those pull `syn` v2.0.119, `quote` and
`proc-macro2` into `liss-codegen` and therefore into `lingua-serpentis`:

```console
$ cargo tree -e normal -i syn@2.0.119
syn v2.0.119
├── derive_more-impl v2.1.1 (proc-macro)
│   └── derive_more v2.1.1
│       └── libwild v0.10.0
│           └── liss-codegen v0.1.0
│               └── lingua-serpentis v0.1.0
├── prost-derive v0.14.4 (proc-macro)   …
├── strum_macros v0.28.0 (proc-macro)   …
└── zerocopy-derive v0.8.55 (proc-macro) …
```

So naming them in `[dev-dependencies]` adds **no crate that was not already
compiled on every build**; cargo unifies the dev-dep on the version the lockfile
already pins. `tempfile` is the only genuinely new crate here, and it is
dev-only in both crates that name it.

The check is therefore not "these names are absent from the graph" — three of
them are legitimately present. It is two claims, both mechanical:

```console
$ cargo tree -e normal | grep -i tempfile          # empty: the one new crate is dev-only
$ git diff --stat -- '*/Cargo.toml'                # only [dev-dependencies] hunks
```

`proc-macro2`'s `span-locations` deserves its own line, because it is the one
non-obvious entry. `Span::start()` returns `LineColumn { line: 0, column: 0 }`
unless that feature is on — outside a real proc-macro invocation `proc-macro2`
runs in fallback mode and only tracks locations when asked. Without it every site
in the catalogue would report line 0 and the snapshot would collapse to a single
row per file.

## 4. The marker-to-AST mapping

The eleven markers become eleven predicates over `syn` nodes. The right-hand
column is not decoration: **the span must be taken from the named sub-node, not
from the whole node.** `Expr::MethodCall::span()` starts at the *receiver*,
which for a multi-line chain is an earlier line than the marker `.err(` sat on;
`node.method.span()` is where the scanner was looking.

| Marker | AST node | Span from |
|---|---|---|
| `LinguaError {` | `Expr::Struct`, path ends `LinguaError` | `path` |
| `Warning {` | `Expr::Struct`, path ends `Warning` | `path` |
| `ProgramError::structural(` | `Expr::Call`, callee path ends `ProgramError::structural` | `func` |
| `.err(` | `Expr::MethodCall`, method `err` | `method` |
| `Err("` | `Expr::Call`, callee path `Err`, argument list **opening** with a string literal token | `func` |
| `.ok_or("` | `Expr::MethodCall`, method `ok_or`, argument list opening with a string literal token | `method` |
| `ok_or_else(\|\| format!(` | `Expr::MethodCall`, method `ok_or_else`, closure body an `Expr::Macro` with path `format` | `method` |
| `eprintln!(` | `Expr::Macro` **and `Stmt::Macro`**, macro path `eprintln` | `mac.path` |
| `liss_panic(` | `Expr::Call`, callee path ends `liss_panic` | `func` |
| `Defectus::novus(` | `Expr::Call`, callee path ends `Defectus::novus` | `func` |
| `NUNTIUS_` | `Item::Const` / `Item::Static`, ident begins `NUNTIUS_` | `ident` |

Two rows in that table are written the way they are because the first attempt
was wrong, and the differential test of §8 said so:

- **`eprintln!` needs both node kinds.** A macro invocation in statement
  position — `eprintln!("…");`, which is how all twelve of `cli.rs`'s are
  written — is a `Stmt::Macro`, not an `Expr::Macro`. Visiting only expressions
  found none of them.
- **`Err("` is about the first *token*, not the first *argument*.** The marker
  means the quote follows the paren directly;
  `Err("numerus Romanus vacuus".into())` satisfies it, and its argument is a
  method call rather than a literal.

**Why each marker exists is unchanged by how it is found**, and
`docs/internals/testing.md` remains the place that explains it:
`ProgramError::in_file` is deliberately absent because it re-wraps rather than
raises; the three `String`-shaped markers exist because `roman.rs` is the one
module whose errors are bare `String`s; the last three are the back end's and
were deferred to Task 19 so that no pattern was non-unique while two
implementations of the same three messages stood.

**Finding the literal.** The scanner's 300-byte window becomes *the first
string, byte-string or C-string literal in the site's own token stream*,
descending into groups. `liss-abi` writes `b"Error: "` and
`c"acervus vocationum exhaustus est"` rather than plain `&str`, because the crate
is `no_std`-shaped and async-signal-safe; all three are `syn::Lit` variants and
all three are read. The window, the `char_boundary` growth that kept it from
splitting `°`, and `opening_quote`'s special case for the char literal `'"'` all
go — they were consequences of scanning bytes.

**`UNPARSED` survives, and is still a rule.** A site whose literal is not its own
— `liss_panic(nuntii::DIVISIO_PER_NULLUM.as_ptr(), …)` — still reports
`UNPARSED`, and spec §10.3 still requires the runtime to write its sentences
inline at the call site. Parsing does not make an absent literal present.

## 5. Which guards survive

One of four.

**Guard 1, "a declaration is not a raise" — gone, structurally.** It tested
whether the text before a marker ended in `->` or `fn`, so that
`-> LinguaError {` (a return type, followed by the function body's brace) and
`pub unsafe extern "C" fn liss_panic(` (a definition) were not counted. In an
AST a return type is a `Type` and never an `Expr::Struct`, and a definition is an
`ItemFn` and never an `Expr::Call`. There is nothing to guard against.

**Guard 2, "prose about a diagnostic is not a diagnostic" — gone, structurally.**
It looked left from the marker to the start of its line for `//`, and assumed no
block comment existed anywhere in a scanned tree. Comments are not tokens. This
is the guard whose disappearance lifts the `CLAUDE.md` rule.

**Guard 3, "a `NUNTIUS_*` constant is a site where it is declared, and nowhere
else" — gone, structurally.** It required the preceding text to end in `const` or
`static`. A declaration is an `Item::Const` or `Item::Static`; a use is an
`Expr::Path`. Only the first is visited.

**Guard 4, "a pattern that is nothing but holes identifies nothing" — kept,
verbatim.** If removing every `{}` leaves an empty string the site is dropped:
`eprintln!("{}", e)` prints a diagnostic built elsewhere, and matching it against
observed messages would match every one of them. This is semantic, not
syntactic — a parser has no opinion on it — and it is the same reasoning that
keeps `ProgramError::in_file` out of the marker list.

**`production_source` — replaced, and tightened.** It truncated each file at the
first `"\n#[cfg(test)]\n"`, which was exact only because every file in this
workspace happens to have at most one such module and it always runs to EOF. The
replacement checks the `#[cfg(test)]` attribute on the item itself, which holds
regardless of how many such modules a file grows or where they sit.

**The three meta-tests are kept**, though two of them can no longer fail:

- `byte_and_c_string_literals_are_extracted_like_any_other` still earns its
  place — the three `syn::Lit` arms are exactly what it guards, and a future
  edit that dropped `Lit::CStr` would silently stop reading every message the
  runtime can say.
- `a_diagnostic_entry_points_own_definition_is_not_a_site` and
  `a_marker_in_a_comment_is_prose_and_a_marker_in_code_is_not` now hold by
  construction. They are kept as statements of intent at negligible cost; a test
  that cannot fail is cheap, and the day someone reaches for a token-level
  shortcut it becomes able to again.

## 6. What is out of scope, and why

`crates/liss-abi/tests/abi.rs`'s `sententiae` reads that crate's own `src/` line
by line, skipping whole-line `//` comments, and collects every `c"…"` literal —
a second, smaller source scanner with the same block-comment assumption:

```rust
// Whole-line comments only, and no block comments exist: docs/internals/testing.md.
if linea.trim_start().starts_with("//") {
```

It is **left alone deliberately.** `liss-abi` is its own workspace, `exclude`d
from the root because `liss-codegen/build.rs` invokes cargo on it during an
outer build; converting it would mean a second, separate set of three
dev-dependencies for a thirty-line scanner, in the one crate whose build graph
the project works hardest to keep simple.

The consequence is stated rather than hidden: **`CLAUDE.md`'s block-comment
prohibition narrows to `crates/liss-abi/src` rather than disappearing.** Five of
the six scanned crates are freed; the sixth keeps the rule and now carries the
only reason for it.

## 7. The two smaller items

**One scratch directory.** `tests/support/proc.rs` names its directory
`{prefix}-{pid}-{counter}` and `remove_dir_all`s the path before creating it,
because the name is only as unique as the process ID.
`crates/liss-codegen/tests/support/mod.rs`'s `exsequere` writes
`liss-{pid}-{nomen}` and removes it on the way out — **on the success path
only**, so four `expect`s between the two lines leak the directory whenever a
codegen test fails. That is the one defect in this section; the rest is
duplication. `tempfile` fixes both, and is the only way the two crates share one
implementation, since neither can import the other's test support.

`proc.rs`'s `TempDir` stays as a newtype rather than becoming a re-export: its
`write(rel, content)` helper is this harness's own and roughly forty call sites
across `cli.rs`, `robustness.rs` and `liss_cli.rs` spell it.

**One wait loop.** The same `try_wait` / `sleep` / deadline loop appears in
`proc.rs::run_binary`, `proc.rs::run_closing_stdout_early` and
`support/mod.rs::curre`. The first two are byte-identical; the third differs in
poll interval and in returning `Result<_, String>` rather than an `Outcome`.
`wait_or_kill(&mut Child, timeout, pausa) -> io::Result<Option<ExitStatus>>`
serves all three, with the interval a **parameter** — `docs/internals/testing.md`
documents `curre`'s `PAUSA` = 2 ms by name, and `proc.rs` polls at 5 ms, so
unifying the interval would silently change a documented fact for no gain. No
dependency: three copies of ten lines do not justify one.

## 8. Differences from the scanner

Measured. **Five rows, all in the same direction: the scanner reports them and
the parser does not. Nothing the parser finds is new, and no `fixtured` site
moves.** The catalogue goes from 123 sites to 118, `5 unparsed` to `2`, and
`48 uncovered` to `46`; `70 fixtured · 0 ambiguous` is unchanged, which is the
number that matters.

All five are scanner artifacts of the same kind — a marker that names a *type*
matching a place the type is written rather than constructed:

| Row | Status | What the scanner matched |
|---|---|---|
| `lingua-serpentis/src/error.rs:2` | `UNPARSED` | `pub struct LinguaError {` — the type's **definition** |
| `lingua-serpentis/src/error.rs:11` | `UNPARSED` | `pub struct Warning {` — likewise |
| `liss-runtime/src/exitus.rs:8` | `UNPARSED` | `pub struct Warning {` — likewise |
| `lingua-serpentis/src/error.rs:17` | `UNCOVERED` | `impl std::fmt::Display for Warning {`, whose pattern was then lifted from the `write!` three lines down |
| `lingua-serpentis/src/error.rs:83` | `UNCOVERED` | `impl std::fmt::Display for LinguaError {`, likewise from line 85 |

Guard 1 excluded a marker preceded by `->` or `fn`; it never considered `struct`
or `impl … for`. In an AST a definition is an `ItemStruct` and an impl header is
an `ItemImpl`, and neither is an `Expr::Struct`, so all five stop being sites
without anything being written to exclude them.

**The two `UNCOVERED` rows are not a coverage loss, and could never have become
one.** They carry the *envelope* — `Monitum in '{}', linea {}: {}` and
`Error in linea {}: {}` — and `observed_messages` reduces every observed line to
what follows its first `": "`, which strips exactly that envelope before the
comparison runs. An envelope pattern therefore has nothing to match against and
is `UNCOVERED` by construction, not for want of a fixture.

**Why `write!` was not added as a twelfth marker.** It would have kept those two
rows, and it is the honest way to find them — they are real sentences a user
reads. It was rejected because it finds *ten* such sites (nine in `error.rs`,
one in `liss-codegen/src/lib.rs`), and by the paragraph above every one of them
would be permanently `UNCOVERED`. That trades two accidental rows for ten
deliberate ones that the ratchet can never discharge, which is noise in the one
report whose value is that its `UNCOVERED` count means something. If the
envelope is ever to be ratcheted, the thing to change is `observed_messages`,
not the marker list.

**One extractor bug this comparison caught**, and the reason the differential
test earned its place: `Err("numerus Romanus vacuus".into())` in
`liss-runtime/src/roman.rs:191` is `fixtured`, and a first predicate that
required the first *argument* to be a string literal missed it, because the
argument is a method call. The marker `Err("` means the quote follows the paren
directly, whatever is done to the literal afterwards, so the predicate asks
whether the argument list **opens** with a string literal token.

## 9. Success criteria

- `cargo test` and `cargo test --manifest-path crates/liss-abi/Cargo.toml` both
  pass. Baseline counts, measured per target before any change: `conformance`
  118, `programs` 31, `liss_cli` 29, `cli` 23 — 25 after §7's two new tests, the
  only count this work moves — `ir` 23, `catalogus` 5, `robustness` 5,
  `generative` 4, `instrumenta` 4; and in the second workspace `abi` 21,
  `custos` 6, `emissio` 1.
- The catalogue header reads
  `118 sites · 70 fixtured · 0 ambiguous · 46 uncovered · 2 unparsed` — five
  fewer rows than before, exactly the five §8 enumerates, with `fixtured` and
  `ambiguous` unmoved.
- `conformance/` has no modifications. This work blesses no expectation file.
- `cargo tree -e normal` does not name `tempfile`, the one crate this work adds
  to the tree at all; `syn`, `quote` and `proc-macro2` are already there under
  `libwild` and stay where they are (§3). No `[dependencies]` table is touched.
- A `/* … */` block comment containing the text `NUNTIUS_`, written above a
  diagnostic site in `crates/lingua-serpentis/src/checker.rs`, changes no site.
  Under the scanner it invented one. This is the criterion the whole design is
  for.
