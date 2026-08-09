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
- **Diagnostics are Latin, and ratcheted.** `tests/catalogus.rs` parses `crates/*/src` with `syn`
  and a **non-recursive** `read_dir`, so anything under `src/bin/` is invisible to it. That is why
  the `liss` driver lives in `cli.rs` and `src/bin/liss.rs` is a shim that says nothing. New
  user-facing sentences belong in module files. A message whose literal is not written at the call
  site reads `UNPARSED` and is invisible to the ratchet, so write the sentence inline (spec §10.3).
- **The source does not explain itself; `docs/internals/` does.** The comments were a design
  document typed as `//!`, and they are now four reference documents. What stays in the source is
  the `# Safety` clauses, the fixture and test pointers, and a bare `D-NN`. Write the reasoning for
  a new decision in `docs/internals/`, not above the line it defends.
- **Comments move the catalogue snapshot.** `catalogus__catalogus.snap` records `file:line` for
  118 diagnostic sites, so deleting or adding a comment line above any of them fails
  `tests/catalogus.rs`. Re-bless it, then check the header still reads
  `118 sites · 70 fixtured · 0 ambiguous · 46 uncovered · 2 unparsed` and that only the `:line`
  field moved — a changed count means a site was gained, lost or reclassified.
- **Never write a `/* … */` block comment under `crates/liss-abi/src`.** `liss-abi/tests/abi.rs`'s
  `sententiae` greps that crate's own source for the `c"…"` literals it must enumerate, and skips
  whole-line `//` comments only; a block comment would hide a sentence or invent one. The rule used
  to cover all of `crates/*/src` because `catalogus.rs` scanned text too — it parses now, and a
  token stream has no comments, so the other five crates are free of it
  (`docs/superpowers/specs/2026-08-09-test-harness-libraries-design.md` §6).

## The conformance corpus

`conformance/` is 118 fixtures in four groups: `currunt/` (programs that run), `errores/` (programs
that must be rejected), `monita/` (programs that warn), `defectus/` (one deferred defect). Each
fixture directory holds:

- `programma/` — the program, or `VIA` — a file naming a program elsewhere in the repo;
- `emissio` — expected stdout, `nuntii` — expected stderr, `exitus` — expected exit code (absent
  means 0). An empty stream has no file at all.
- `NOTA.md` — why the case exists, where one was written.

`cargo test --test conformance` runs them (`harness = false`, one named trial per fixture).
`LISS_BLESS=1` rewrites the expectation files from actual output — blessing a regression is the one
way this suite fails silently. **`tests/nuntii.rs` is the net**: it snapshots all 99 diagnostics in
the corpus with the framing stripped, so a blessing that changes a *sentence* or a *line* moves it
and a blessing that changes only the shape does not. Never bless without re-running it.

- **A diagnostic now echoes the source line it points at**, so `nuntii` files contain program text.
  `conformance/currunt/exemplum-praecisio` reaches `examples/praecisio.liss` through its `VIA` file
  and warns twice, which means **editing that example moves a conformance expectation.**

## Conventions

- Reference for the code as it stands: `docs/internals/` — `codegen.md`, `abi.md`, `rationale.md`,
  `testing.md`. Undated on purpose; they track the code and are edited in place. `docs/README.md`
  indexes everything and resolves the specs' amends/supersedes chain into reading order.
- Design specs: `docs/superpowers/specs/YYYY-MM-DD-<topic>-design.md`. Plans:
  `docs/superpowers/plans/`. Defects: `docs/defectus/2026-08-03-index.md`.
- Commit subjects are `type(scope): a sentence in the repo's voice` — read `git log` before writing
  one. Scopes are the milestone or area (`codegen`, `interp`, `conformance`, `spec`), not the crate
  name.
