# Lingua Serpentis

A programming language written in Latin, and an ahead-of-time compiler that turns it into a
standalone static Linux binary.

Keywords are Latin words (`munus`, `si`, `dum`, `redde`, `scribe`), numbers are written as Roman
numerals (`°XXV`), and every diagnostic the compiler emits is a Latin sentence. `liss build` produces
an ELF that runs on any Linux of its architecture with nothing else installed — no interpreter, no
shared libraries, no toolchain. The compiler itself needs no `cc` and no system linker: it generates
machine code with Cranelift and links in process with [wild](https://github.com/davidlattimore/wild).

## Example

```c
// examples/salve.liss
munus initium reddit ni {
    scribe("Salve, munde!");
    redde nullus;
}
```

```console
$ liss run examples/salve.liss
Salve, munde!
```

`examples/` also has lists (`series.liss`), Roman fractions in two precisions (`praecisio.liss`),
and a multi-file program with packages (`computatrum/`).

## Building the compiler

The build cross-compiles the language runtime for both musl targets, so
`x86_64-unknown-linux-musl` and `aarch64-unknown-linux-musl` standard libraries must be present.
The flake provides them, along with the `qemu` the aarch64 tests run under:

```console
$ nix develop
$ cargo build --release
```

That writes `target/release/liss`. Without nix, install both targets yourself
(`rustup target add x86_64-unknown-linux-musl aarch64-unknown-linux-musl`); the build script says
which target is missing if one is.

## Using it

```console
liss run <via.liss | directorium>
liss build <via.liss | directorium> [--exi <via>] [--ad <machina>]
liss <via.liss | directorium>
```

`run` compiles the program and executes it immediately. `build` writes the binary instead: to
`--exi` if given, otherwise to the `.liss` file's stem — or the program folder's name — in the
current directory. `--ad` selects the architecture, `x86_64` (default) or `aarch64`, and
cross-compiles.

A bare `.liss` file is a one-file program. A directory is a program of several: its own `.liss`
files, plus one level of subfolders, each of which is a package addressed by its folder name
(`affer mathematica;`). Nothing may nest deeper. Either way, execution starts at the `munus initium`
of the root. The first argument is a subcommand only if it is exactly `run` or `build`, so a folder
named `run` must be given as `./run`.

## Status

Linux only, `x86_64` and `aarch64`. 1.0 is the last feature release — after it, dependency updates
and fixes, not new syntax and not a second OS. The sequenced bar, including the two named
optimisations and what is omitted for good, lives in `docs/via.md`.

## Layout

| Path | |
|---|---|
| `crates/lingua-serpentis` | Lexer, parser, checker, and the `liss` command |
| `crates/ir` | The typed IR the checker emits |
| `crates/codegen` | Cranelift code generation, and the linker |
| `crates/runtime` | Roman numerals; compiled for both host and target |
| `crates/abi` | The musl-side runtime — cross-built for musl by `crates/codegen/build.rs` |
| `crates/latinitas` | Latin morphology, backed by Whitaker's WORDS |
| `conformance/` | 123 fixtures: programs that run, that are rejected, that warn |
| `examples/` | Sample programs |
| `docs/` | Design specs, implementation plans, defect register, the road to 1.0 |

## Tests

```console
$ cargo nextest run
```

One workspace, one lock file, one command. `cargo test` runs the same tests, one binary at a time.

## Documentation

`docs/README.md` is the index: which document answers which question, and how the design specs amend
and supersede one another. Start there.

- `docs/internals/` — reference for the code as it stands: `codegen.md` (the back end), `abi.md`
  (the ABI, the runtime, the IR), `rationale.md` (the front end, and what was tried and broke),
  `testing.md` (the corpus and the ratchets).
- `docs/via.md` — the road to 1.0, and what 1.0 freezes.
- `docs/superpowers/specs/` — design specs, one per milestone. The most recent,
  `2026-08-07-compiler-backend-design.md`, describes the compiler as it stands; an unqualified
  "spec §N" means that file.
- `docs/superpowers/plans/` — the implementation plans those specs became.
- `docs/defectus/2026-08-03-index.md` — the defect register: what was found, what was fixed, what is
  deferred, and why.
