# Lingua Serpentis — working notes

A Latin programming language and its ahead-of-time compiler. `README.md` covers what it is and how
to build it; this file covers what is easy to get wrong.

## Build and test

Work inside `nix develop`. The build cross-compiles `liss-abi` for both musl targets at compiler
build time and fails without both musl `std`s; `crates/liss-codegen/build.rs` names the missing
target and the flake when that happens.

- `cargo test` — the workspace.
- `cargo test --manifest-path crates/liss-abi/Cargo.toml` — the second workspace, which the first
  command does not reach.

`crates/liss-abi` is `exclude`d from the root workspace deliberately: `liss-codegen/build.rs`
invokes cargo on it *during* an outer build, and a member would contend for the outer `.cargo-lock`.
Do not "fix" this by adding it to `members`.

## Invariants

- **There is no interpreter.** The tree-walker was deleted in `677c4b3`; `parse`, `check` and
  `interp::execute` no longer exist. The entry points are `parse_file` and `check_program`. The
  compiler is the only backend.
- **Zero runtime dependencies is a rule** (spec §11), not a coincidence. `liss-runtime`, `liss-ir`
  and `lingua-serpentis` have none. Only `liss-abi` may use `libc`; `liss-codegen` may use the crates
  its `Cargo.toml` already names and no others. Adding a dependency is a change to the spec.
- **Diagnostics are Latin, and ratcheted.** `tests/catalogus.rs` scans `crates/*/src` with a
  **non-recursive** `read_dir`, so anything under `src/bin/` is invisible to it. That is why the
  `liss` driver lives in `cli.rs` and `src/bin/liss.rs` is a shim that says nothing. New
  user-facing sentences belong in module files.
- **The source does not explain itself; `docs/internals/` does.** The comments were a design
  document typed as `//!`, and they are now four reference documents. What stays in the source is
  the `# Safety` clauses, the fixture and test pointers, and a bare `D-NN`. Write the reasoning for
  a new decision in `docs/internals/`, not above the line it defends.
- **Comments move the catalogue snapshot.** `catalogus__catalogus.snap` records `file:line` for
  123 diagnostic sites, so deleting or adding a comment line above any of them fails
  `tests/catalogus.rs`. Re-bless it, then check the header still reads
  `123 sites · 70 fixtured · 0 ambiguous · 48 uncovered · 5 unparsed` and that only the `:line`
  field moved — a changed count means a site was gained, lost or reclassified.
- **Never write a `/* … */` block comment under `crates/*/src`.** The scanner suppresses a marker
  found in prose by looking for `//` earlier on the line, and assumes no block comment exists. One
  would invent phantom diagnostic sites. `liss-abi/tests/abi.rs` makes the same assumption about
  the `c"…"` literals it greps, and skips whole-line comments only.

## The conformance corpus

`conformance/` is 118 fixtures in four groups: `currunt/` (programs that run), `errores/` (programs
that must be rejected), `monita/` (programs that warn), `defectus/` (one deferred defect). Each
fixture directory holds:

- `programma/` — the program, or `VIA` — a file naming a program elsewhere in the repo;
- `emissio` — expected stdout, `nuntii` — expected stderr, `exitus` — expected exit code (absent
  means 0). An empty stream has no file at all.
- `NOTA.md` — why the case exists, where one was written.

`cargo test --test conformance` runs them (`harness = false`, one named trial per fixture).
`LISS_BLESS=1` rewrites the expectation files from actual output — read the diff before committing
it; blessing a regression is the one way this suite fails silently.

## Conventions

- Reference for the code as it stands: `docs/internals/` — `codegen.md`, `abi.md`, `rationale.md`,
  `testing.md`. Undated on purpose; they track the code and are edited in place. `docs/README.md`
  indexes everything and resolves the specs' amends/supersedes chain into reading order.
- Design specs: `docs/superpowers/specs/YYYY-MM-DD-<topic>-design.md`. Plans:
  `docs/superpowers/plans/`. Defects: `docs/defectus/2026-08-03-index.md`.
- Commit subjects are `type(scope): a sentence in the repo's voice` — read `git log` before writing
  one. Scopes are the milestone or area (`codegen`, `interp`, `conformance`, `spec`), not the crate
  name.
