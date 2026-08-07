# Compiler Backend — Implementation Plan

**Goal:** `liss build programma/` emits a standalone static ELF. The interpreter is deleted.

**Spec:** `docs/superpowers/specs/2026-08-07-compiler-backend-design.md` — authoritative for every design decision. Where this plan and the spec disagree, the spec governs and the disagreement is a bug in this plan.

**Register:** `docs/defectus/2026-08-03-index.md` — D-04 is superseded, D-06 relocates, D-19 closes.

> **For agentic workers:** REQUIRED SUB-SKILL: `superpowers:subagent-driven-development` or `superpowers:executing-plans`. Follow the house step convention: every code task opens with **Step 1: Write the failing tests**, **Step 2: Run them and verify they fail** with the expected failure text stated, then implement, verify, commit.

---

## Context

The language has a mature front end, an 86-fixture corpus and a defect register that is 22-of-25 fixed. What it does not have is an artefact. Every program needs the source tree and a Rust toolchain before it prints a word.

This plan builds the compiler and, at the end, deletes the interpreter. The deletion is the point rather than a side effect: two backends means writing every Latin diagnostic, the mod-256 exit rule, 1-based indexing and the grain arithmetic twice and keeping them in step forever. Spec §3.1 has the full argument.

**The interpreter is scaffolding until Task 19.** It stays green throughout, and it does real work while it is here: it is the reference to diff against while codegen is half-built, it is the source of the golden display vectors (Task 4), and it is what makes the ported fixtures provably *today's* behaviour rather than the new backend's opinion. Task 19's precondition is that every fixture has been shown to produce byte-identical output under both backends.

**Order note:** the spec's §9.2 probe cannot run first. It needs a musl staticlib, which needs musl std, which needs the toolchain change. Task 1 is therefore the toolchain and Task 2 is the probe.

---

## Global Constraints

- **Build:** `nix develop -c cargo test --workspace`. Every task leaves it green.
- **The interpreter stays green until Task 19.** Any task whose checkpoint is "N fixtures pass compiled" *also* requires the existing suite to be unchanged and passing. A task that makes the interpreter suite fail has failed.
- **Every user-facing string is Latin.** No Arabic digits outside quoted spans. New coinages are checked against `latinitas::is_latin` before landing. The **one** exception is the `liss` usage string, whose English subcommands require an explicit, commented carve-out in `tests/support/diag.rs` — scoped to the usage text, never a general weakening.
- **Diagnostics are asserted verbatim, never `contains()`.**
- **`RECORDED_RULES` (`tests/support/diag.rs`) may only shrink.** Never weaken a rule to get green.
- **Runtime diagnostics are inline byte-string literals at their call site.** Never a shared constant table — `catalogus.rs` reads literals near markers and would go blind. Spec §10.3.
- **Every async-signal-safe message constant is named `NUNTIUS_*`.** That prefix is a catalogue marker.
- **Zero runtime dependencies, with one narrowed exception:** `liss-abi` may depend on `libc`, and nothing else may. `liss-runtime`, `liss-ir` and `lingua-serpentis` stay at zero.
- **`Signature::call_conv` always comes from `module.isa().default_call_conv()`.** Never a literal. This is what makes the Windows port a recompile.
- **Commit style:** conventional commits, Latin-flavoured scopes (`feat(codegen)`, `refactor(roman)`, `test(conformance)`, `docs(defectus)`). Subject is a lowercase prose claim, no trailing period. Trailer `Co-Authored-By: Claude Opus 5 (1M context) <noreply@anthropic.com>`.
- **Commit safety:** `git add <path>` for untracked paths, then `git commit --only <paths>`. Never `git add -A`.

### Two cross-cutting effects on every task

- `catalogus__catalogus.snap` is keyed by `file:line`, so **any** line shift in a scanned crate re-blesses it. Read the diff line by line; `insta accept` will happily absorb a regression.
- `tests/support/mutate.rs::seed_corpus` seeds the fuzzer from every `.liss` under `examples/` and `conformance/`. Adding or editing a fixture `.liss` shifts the whole mutation stream. Blessing a `nuntii` file does not.

---

## Task 1: The toolchain

`flake.nix` provides plain nixpkgs rustc with std for gnu and wasm only, and there is no `rustup`. Nothing downstream can be built until musl std exists.

**Files:**
- Modify: `flake.nix`, `flake.lock`
- Modify: `Cargo.toml` (root), `crates/lingua-serpentis/Cargo.toml`, `crates/latinitas/Cargo.toml`

**Interfaces:**
- Produces: a dev shell where `rustc --print target-libdir --target=x86_64-unknown-linux-musl` and the aarch64 equivalent both resolve, and `wild` is on `PATH` for the probe.

- [ ] **Step 1: Write the failing check**

Add `crates/lingua-serpentis/tests/instrumenta.rs`:

```rust
//! The toolchain the compiler backend needs. These are environment assertions,
//! not unit tests: they fail loudly in a shell that cannot build the runtime.
use std::process::Command;

fn target_libdir(triple: &str) -> Option<String> {
    let out = Command::new("rustc")
        .args(["--print", "target-libdir", "--target", triple])
        .output().ok()?;
    out.status.success().then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
}

#[test]
fn musl_std_exists_for_both_targets() {
    for t in ["x86_64-unknown-linux-musl", "aarch64-unknown-linux-musl"] {
        let dir = target_libdir(t).unwrap_or_else(|| panic!("rustc knows no target-libdir for {t}"));
        assert!(std::path::Path::new(&dir).join("libstd.rlib").exists()
             || std::fs::read_dir(&dir).map(|mut d| d.any(|e| e.map(|e|
                    e.file_name().to_string_lossy().starts_with("libstd-")).unwrap_or(false)))
                .unwrap_or(false),
            "no libstd in {dir} for {t}");
    }
}

#[test]
fn the_self_contained_crt_objects_are_present() {
    for t in ["x86_64-unknown-linux-musl", "aarch64-unknown-linux-musl"] {
        let dir = target_libdir(t).unwrap();
        let sc = std::path::Path::new(&dir).join("self-contained");
        for o in ["crt1.o", "crti.o", "crtn.o"] {
            assert!(sc.join(o).exists(), "missing {o} in {}", sc.display());
        }
    }
}
```

- [ ] **Step 2: Run them and verify they fail**

Run: `nix develop -c cargo test -p lingua-serpentis --test instrumenta`
Expected: FAIL — `no libstd in … for x86_64-unknown-linux-musl` (rustc knows the triple but the sysroot has no std).

- [ ] **Step 3: Rewrite `flake.nix`**

Add `rust-overlay` as an input; build one toolchain carrying both musl targets; keep the existing tools. Restore the bump comment that was lost:

```nix
inputs = {
  # nixos-unstable; bump with:
  #   nix flake update
  nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  rust-overlay = {
    url = "github:oxalica/rust-overlay";
    inputs.nixpkgs.follows = "nixpkgs";
  };
};
```

The toolchain must request `targets = [ "x86_64-unknown-linux-musl" "aarch64-unknown-linux-musl" ]` and the `rust-src` component. Add `wild` (already at 0.9.0 in the pin) and `qemu` (Task 18) to the shell.

- [ ] **Step 4: Raise the MSRV**

Introduce `[workspace.package]` in the root `Cargo.toml` with `edition = "2024"`, `rust-version = "1.94"`, `version = "0.1.0"`, and `[workspace.dependencies]`. Both member crates inherit with `edition.workspace = true` etc. 1.94 is libwild's floor.

- [ ] **Step 5: Verify**

Run: `nix develop -c cargo test --workspace`
Expected: PASS, including both new tests.

- [ ] **Step 6: Commit**

`feat(instrumenta): the dev shell can build for musl`

---

## Task 2: The probe

Everything rests on one unverified assumption: that a Rust program with no `cc` and no `rustc` can link a musl staticlib into a running static ELF. **This task is throwaway code whose only job is to answer that.** If it fails, the milestone's shape changes, and with no interpreter to fall back to there is no cheaper moment to find out.

It also answers two questions the design left open.

**Files:**
- Create: `probatio/` at the repo root — a scratch cargo project, **not** a workspace member. Deleted in Step 6.

- [ ] **Step 1: Build a musl staticlib and inspect it**

A crate with `crate-type = ["staticlib"]`, `panic = "abort"`, one `#[unsafe(no_mangle)] extern "C" fn main` that writes a string and returns 3.

Run `nm` and `ar t` over the archive. **Record in the report whether `libc.a`'s members are already bundled** — this is spec Open Question 2 and decides 2–4 MB per target.

- [ ] **Step 2: Confirm `std::rt::init` does not run**

Build a second staticlib whose `main` recurses infinitely. Run it.

Expected: exit **139** (128+11) with **empty stderr**. If instead the English `has overflowed its stack` appears, `rt::init` *is* running and spec §8.1 is wrong — stop and escalate before anything else is built.

- [ ] **Step 3: Prelink and link in-process**

`ld -r` the staticlib with `libc.a` (if separate) into one relocatable object. Then, from a Rust program using `libwild` with an in-memory `FileSystem`, link `crt1.o crti.o probatio.o crtn.o` into an executable image and write it out.

Run it. Expected: prints, exits 3, and `file` reports `ELF 64-bit LSB executable, statically linked`.

- [ ] **Step 4: Confirm the signal handler reaches a guard-page fault**

Add `sigaltstack` on a static array plus a `SIGSEGV`/`SIGBUS` handler that writes a fixed byte string and `_exit(1)`s. Recurse to death.

Expected: the chosen message on stderr, exit 1, no signal death. **Also print the stack bounds derived by each of spec §8.2's three layers against `/proc/self/maps`**, and record which actually works for the main thread on musl — the spec says this is implementation-defined and must be measured.

- [ ] **Step 5: Time a link**

Link the image 20 times and report the mean. This is the number spec §10.2's estimate rests on, and Task 17's mitigation decision depends on it.

- [ ] **Step 6: Delete `probatio/` and commit only the findings**

Nothing from this task ships. Write the five answers into the task report and, if any contradicts the spec, amend the spec in the same commit.

`docs(spec): the probe settles libc bundling, stack bounds and link cost`

---

## Task 3: `Roman`'s digits become bytes

`LissInt`'s payload must be `Roman`'s representation, so `Tier { units: u8, five: u8 }` becomes a packed `u8` digit in `0..=9`. Pure refactor: **zero behaviour change**, pinned by 25 existing tests.

**Files:**
- Modify: `crates/lingua-serpentis/src/roman.rs`

**Interfaces:**
- Produces: `Roman { negative: bool, tiers: Vec<u8> }`, canonical (no trailing zeros, empty vec the unique zero), plus `pub fn digits(&self) -> &[u8]` and `pub fn from_digits(negative: bool, digits: &[u8]) -> Roman`.

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn digits_are_little_endian_and_canonical() {
    let r = Roman::from_u64(1024);
    assert_eq!(r.digits(), &[4, 2, 0, 1]);
    assert!(Roman::zero().digits().is_empty());
}

#[test]
fn from_digits_round_trips_through_display() {
    for n in [0u64, 1, 9, 10, 4999, 1_000_000] {
        let r = Roman::from_u64(n);
        let back = Roman::from_digits(false, r.digits());
        assert_eq!(r.display(), back.display());
    }
}

#[test]
fn from_digits_strips_trailing_zeros_to_stay_canonical() {
    assert_eq!(Roman::from_digits(false, &[1, 0, 0]).digits(), &[1]);
    assert!(Roman::from_digits(true, &[0, 0]).is_zero());
}
```

- [ ] **Step 2: Run them and verify they fail**

Run: `cargo test -p lingua-serpentis --lib roman::tests::digits`
Expected: FAIL — `no method named 'digits' found for struct 'Roman'`.

- [ ] **Step 3: Implement**

Every access already goes through `Tier::digit()` / `Tier::from_digit()`, so `mag_add`, `mag_sub`, `mag_halve`, `mag_mul`, `mag_divmod` and `display` change mechanically. `from_digits` must strip trailing zeros, and a zero magnitude must clear `negative` — otherwise structural `PartialEq` stops being numeric equality and `is_zero` breaks.

- [ ] **Step 4: Verify nothing moved**

Run: `nix develop -c cargo test --workspace`
Expected: PASS. `cargo insta` must report **no** snapshot change beyond line shifts in `catalogus`; re-bless only line numbers, and read the diff.

- [ ] **Step 5: Commit**

`refactor(roman): a tier is a digit, not a pair of counts`

---

## Task 4: Capture what the interpreter knows

**This task must happen while the interpreter exists.** It captures today's behaviour as data, at the one moment both implementations can be compared. Spec §10.4.

**Files:**
- Create: `conformance/currunt/` — roughly 20 new fixtures
- Create: `crates/lingua-serpentis/examples/scriptio-aurea.rs` (deleted in the final task)
- Create: `crates/lingua-serpentis/tests/data/scriptio-aurea.txt` (moved to `liss-runtime` in Task 5)

- [ ] **Step 1: Generate the golden display vectors**

**An example, not a test.** This repo retired the "assertions inside `#[ignore]`d tests" carve-out, and a one-shot data generator is not a test in any case. Write `crates/lingua-serpentis/examples/scriptio-aurea.rs`, which walks a fixed list of values through `Value::display()` and prints `(kind, payload, expected)` triples to stdout.

Cover: zero, negatives, single and multi-tier, the vinculum above M, every twelfth name, the maximal sub-uncia stack (`uncia et semuncia et sextula et scripulum`), whole-plus-fraction, empty list, nested list, list of fractions, both bool literals, a multi-byte char.

```sh
cargo run -p lingua-serpentis --example scriptio-aurea \
  > crates/lingua-serpentis/tests/data/scriptio-aurea.txt
```

Run once; commit the output. It is data from here on, and Task 5 moves it into `liss-runtime` alongside the code it pins. The example itself is deleted with the interpreter in the final task.

- [ ] **Step 2: Port the interpreter's semantic tests to fixtures**

The ~20 tests in `interp.rs`'s legacy block that test *language* behaviour rather than the interpreter: arithmetic and precedence, variables and assignment, boolean rendering, string concatenation, `indem` counting, `dum` with `abrumpe` and with `continua`, call and return, text comparison, char round-trip, unbounded arithmetic, list literal, 1-based index read, out-of-range read at both ends, index write, out-of-range write, `longitudo`, `adde`, list equality, nested list read.

Each becomes a three-line `munus initium reddit ni { … }` program under `currunt/` or `errores/`. Bless with `LISS_BLESS=1` **and read every blessed file** — these expectations are the contract the compiler will be held to.

- [ ] **Step 3: Verify under the interpreter**

Run: `nix develop -c cargo test --workspace`
Expected: PASS. The new fixtures pass *under the interpreter*, which is what establishes them as today's behaviour.

Note `seed_corpus` shifts (new `.liss` files) — expect `robustness` output to change and confirm no rule newly breaks.

- [ ] **Step 4: Commit**

`test(conformance): the interpreter's semantics become fixtures while it lives`

---

## Task 5: `liss-runtime`

**Files:**
- Create: `crates/liss-runtime/` — `roman.rs`, `fraction.rs`, `scriptio.rs`, `exitus.rs`, `lib.rs`
- Modify: `crates/lingua-serpentis/src/lib.rs` (re-export), delete the moved modules

**Interfaces:**
- Produces: `liss_runtime::{Roman, fraction, scriptio, exit_status}`, `BinOp`, `UnOp`, and `#[cfg(test)] mod speculum` (the test-only reference formatter, spec §10.4).
- `scriptio` is sink-based: `pub trait Sink { fn push(&mut self, bytes: &[u8]); }` and `scribe_int`, `scribe_frac`, `scribe_text`, `scribe_char`, `scribe_bool`.

- [ ] **Step 1: Write the failing tests**

The golden-vector test in the new crate, reading `scriptio-aurea.txt` and asserting each row against the `scriptio` functions. Plus a property test comparing `scriptio` against `speculum`'s `display`.

- [ ] **Step 2: Run and verify they fail**

Run: `cargo test -p liss-runtime`
Expected: FAIL — the crate does not exist.

- [ ] **Step 3: Move, do not rewrite**

`git mv` `roman.rs` and `fraction.rs` with their tests. `value.rs` becomes `#[cfg(test)] mod speculum`, renamed so nobody mistakes it for production. `BinOp`/`UnOp` move down from `ast.rs` (they carry no AST data). `exit_status` moves from `interp.rs`. `lingua-serpentis` re-exports so no call site changes.

- [ ] **Step 4: Verify**

Run: `nix develop -c cargo test --workspace`
Expected: PASS. `git diff --stat` must show moves and re-exports only — any behaviour change here is a bug.

- [ ] **Step 5: Commit**

`refactor(runtime): arithmetic and formatting leave the interpreter`

---

## Task 6: `liss-ir`

**Files:**
- Create: `crates/liss-ir/src/lib.rs`

**Interfaces:**
- Produces: the types in spec §4.1 verbatim — `Grain`, `Ty`, `Const`, `Expr { ty, kind }`, `ExprKind`, `Stmt { line, kind }`, `StmtKind`, `Block { stmts, declares }`, `Func`, `Programma`, plus `SlotId`/`FuncId`.
- Depends on `liss-runtime` only, for `Roman`.

- [ ] **Step 1: Write the failing test**

A unit test constructing by hand a two-function `Programma` — an `initium` calling a one-parameter function that returns `n * °II` — and asserting its shape. Plus `fn symbol(ns: Option<&str>, name: &str) -> String` with its cases:

```rust
#[test]
fn symbols_are_injective_over_unicode_identifiers() {
    assert_eq!(symbol(None, "initium"), "liss_0__7_initium");
    assert_eq!(symbol(Some("mathematica"), "duplum"), "liss_11_mathematica_6_duplum");
    // the escape must keep distinct names distinct
    assert_ne!(symbol(None, "aê"), symbol(None, "a_u00ea_"));
    // every mangled name has a digit after `liss_`; every runtime entry has a letter
    assert!(symbol(None, "x")[5..6].chars().all(|c| c.is_ascii_digit()));
}
```

- [ ] **Step 2: Run and verify it fails**

Run: `cargo test -p liss-ir`
Expected: FAIL — the crate does not exist.

- [ ] **Step 3: Implement, then Step 4: verify, Step 5: commit**

`feat(ir): a typed intermediate representation the checker can emit`

---

## Task 7: The checker emits IR

The highest-risk edit in the plan. `check_program`'s public signature does not change; `check_program_typed` sits underneath it and the checker **always** builds the IR (spec §3.4).

**Files:**
- Modify: `crates/lingua-serpentis/src/checker.rs`
- Create: `crates/lingua-serpentis/tests/ir.rs`

**Interfaces:**
- Produces: `checker::check_program_typed(&mut ParsedProgram) -> Result<liss_ir::Programma, ProgramError>`.

- [ ] **Step 1: Write the failing tests**

`tests/ir.rs`, snapshotting the IR for six programs that between them cover every statement form: `Let`/`Assign`/`Print`/`Return`; `If` with and without `aut`; `While`; `For` with `abrumpe` and `continua`; a two-package program with a qualified call; and one exercising `ListLit`, `Index`, `IndexAssign`, `longitudo`, `adde`.

Assert specifically:
- every `Expr` carries the `Ty` the checker inferred;
- `Block::declares` lists exactly the slots that block introduces;
- slots are never reused, so a shadowing inner `ni x` gets a fresh id;
- `aut si` arrives already nested (the parser did it);
- `For` is **not** desugared into `While`;
- root `initium` falling off the end gained a `Return(Const::Int(zero))`.

- [ ] **Step 2: Run and verify they fail**

Run: `cargo test -p lingua-serpentis --test ir`
Expected: FAIL — `cannot find function 'check_program_typed'`.

- [ ] **Step 3: Implement**

Internal signatures grow a second component on the `Ok` arm only — `check_expr` and `resolve_expr` return `(ty, ir::Expr)`. Every error path is untouched. Add `slots`, `slot_types` and `scope_slots` beside the existing scope stack.

`resolve_floatlit` has a mirror, `resolve_floatlit_ir(&mut ir::Expr, Grain)`, with the identical three-arm shape, **placed immediately adjacent in the source** so the two cannot drift.

- [ ] **Step 4: Verify nothing moved**

Run: `nix develop -c cargo test --workspace`
Expected: PASS, **byte for byte**. This is the strongest checkpoint in the plan: conformance, cli, robustness and generative must be untouched, proving no diagnostic changed text, order or line. A re-blessed `catalogus` snapshot beyond pure line shifts is a **failure, not a fix**.

- [ ] **Step 5: Commit**

`feat(checker): checking now also lowers to typed IR`

---

## Task 8: `liss-abi`, host-only

Every `#[unsafe(no_mangle)]` symbol, built and tested for the host before musl enters the picture.

**Files:**
- Create: `crates/liss-abi/` — its own workspace (`[workspace]` table in its manifest; `exclude` in the root)

**Interfaces:**
- Produces: the entry points of spec §5.4 — lifecycle, integer and fraction arithmetic, text, lists, output, `liss_panic`. Layouts per §5.2, payload at offset 24, `magic` checked on every free.
- The output buffer is a `static mut [u8; 8192]` with an `AtomicUsize` length, bytes written *before* the release-store (§6).

- [ ] **Step 1: Write the failing tests**

Hand-written ABI exercises: build a nested list through `liss_list_new` and direct stores, read an element, append, free, and assert the live count returns to **zero** after each. Division by zero raises through `liss_panic`. `liss_list_get` at 0 and at `len+1` both raise `index extra fines` with Roman numerals. The golden vectors drive `liss_print_*`.

- [ ] **Step 2: Run and verify they fail** — crate does not exist.

- [ ] **Step 3: Implement**

Diagnostics are **inline byte literals** at their call site, never a shared table (Global Constraints).

- [ ] **Step 4: Verify, Step 5: Commit**

`feat(abi): a C ABI over Roman integers, text and lists`

---

## Task 9: `custos` — surviving stack exhaustion

Spec §8 in full. The highest-risk item in the design, and five of its details fail *silently* if omitted: the static alternate stack, both `SIGSEGV` and `SIGBUS`, `sysconf(_SC_PAGESIZE)` rather than 4096, the 1 MiB guard band, and `_exit` rather than `exit`.

**Files:**
- Create: `crates/liss-abi/src/custos.rs` (`#[cfg(unix)]`)

- [ ] **Step 1: Write the failing test**

Fork a child that recurses infinitely; assert exit **1**, stderr exactly `Error: acervus vocationum exhaustus est\n`, and — the part that pins the handler-side flush — that output printed *before* the recursion still reaches stdout.

Also assert the derived stack bounds against `/proc/self/maps`, recording which of the three layers won (Task 2 measured this; this test keeps it true).

- [ ] **Step 2: Run and verify it fails**

Expected: the child dies of signal 11 with empty stderr.

- [ ] **Step 3: Implement**

Bounds computed once at init, never in the handler. The handler may use only `write(2)`, `_exit(2)`, atomic loads and integer comparison. **It must not read `ucontext_t`** — `si_addr` against precomputed bounds is architecture-independent and needs no hand-rolled musl struct offsets.

Also set `SIGPIPE` to `SIG_IGN` here and treat `EPIPE` as exit 0 (spec §6, D-06 relocating into the runtime).

- [ ] **Step 4: Verify, Step 5: Commit**

`feat(abi): the guard page becomes a Latin diagnostic`

---

## Task 10: Cross-build and embed

**Files:**
- Create: `crates/liss-codegen/build.rs`, `crates/liss-codegen/src/target.rs`

- [ ] **Step 1: Write the failing test**

Parse the embedded blobs with `object` and assert `liss_panic`, `main` and the CRT symbols are present for both targets.

> **Carried from Task 9 — do not lose this.** `liss_custos_institue` currently has
> no caller. The runtime's `main` **must call `liss_custos_institue()` first, before
> `liss_initium`**, or stack exhaustion is signal death again and D-04 quietly
> reopens. Every Task 9 test still passes without it, because those tests install
> the handler explicitly — so nothing else will catch the omission. Assert the call
> exists, not merely that the symbol does.
>
> **`a_main_in_this_crate_must_install_the_guard` is not that assertion.** It is a
> substring co-occurrence check — "this file contains `fn main(`, therefore it must
> also contain `liss_custos_institue(` somewhere". A green result does **not** mean
> the call is inside `main`, that it precedes `liss_initium`, or that it is a call
> at all rather than a comment mentioning the name. Treat it as catching total
> omission only, and add a real check here.
>
> Task 9 also established that an `.init_array` constructor is the wrong mechanism
> *on a host build* — constructors run before `std::rt::init`, so std installs its
> own `SIGSEGV` handler over ours immediately afterwards, leaving the guard inert
> rather than merely untested. But the musl target bypasses `lang_start` entirely,
> so there is no std handler to lose the race to, and a constructor **would** work
> there. That is this task's call to make.

- [ ] **Step 2–3: Implement**

`build.rs` invokes cargo on `liss-abi` per target into an `OUT_DIR`-local `--target-dir`, with `RUSTFLAGS`, `RUSTC_WRAPPER`, `CARGO_BUILD_TARGET` and the make jobserver variables **removed** — inherited nix flags break a cross-build. Prelink with `ld -r` per spec §9.2. Honour `LISS_RUNTIME_DIR` as an escape hatch, and when a musl target is missing, fail with a message naming the target and the flake.

- [ ] **Step 4: Verify, Step 5: Commit**

`feat(codegen): the musl runtime ships inside the compiler`

---

## Task 11: Codegen — the walking skeleton

**Files:**
- Create: `crates/liss-codegen/src/lib.rs`, `emissio.rs`, `nexus.rs`

**Interfaces:**
- Produces: `compila(&Programma, &Target) -> Result<Vec<u8>, Defectus>` and the `Nexus` trait with `WildNexus`.

- [ ] **Step 1: Write the failing test** — `currunt/exemplum-salve` compiles, links, runs, exits 0, stdout matches its `emissio`.

> **Check this before writing anything else: does libwild relax TLS?** The
> prelinked runtime carries general-dynamic TLS relocations — 36 `R_X86_64_TLSGD`,
> 16 `TLSLD` and 28 `DTPOFF32` on x86_64; 53 `R_AARCH64_TLSDESC_CALL` and 53
> `TLSDESC_LD64_LO12` on aarch64. A **static, non-dynamic** executable requires the
> linker to perform GD→LE and TLSDESC→LE relaxation. If libwild does not, the link
> fails or produces a binary that faults on first TLS access, and no amount of
> codegen work will help. Measured in Task 10; `rust-lld` handles it, libwild is
> untested against it.
>
> Ignore `_GLOBAL_OFFSET_TABLE_` appearing undefined — it has **zero relocations
> naming it**, so it is an unreferenced symbol-table entry, not a link input.
> `--gc-sections` is worth ~0.92 MB on the final image and should be on.

- [ ] **Step 2–3: Implement**

ISA with `is_pic=false`, `enable_probestack=true`, `probestack_strategy=inline`, `unwind_info(false)`. `MemoriaFS` implements `libwild::FileSystem` in memory. **`args.on_warning` must be installed** to swallow libwild's English, and `libwild::error::Error` is never forwarded — link failure is one structural Latin diagnostic.

- [ ] **Step 4: Verify, Step 5: Commit**

`feat(codegen): a program that prints becomes a static binary`

---

## Task 12: Scalars

Straight-line code for all six scalar types: constants, load/store, every binop and unop, comparisons, div and mod panics.

**`et` and `vel` evaluate both operands.** They do not short-circuit today (`interp.rs:131-135`) and the corpus is blessed against that. Emitting a branch here is the easiest silent divergence in the project — spec §3.5.

- [ ] Checkpoint: `exemplum-praecisio` and `numerus-sine-spatio` pass compiled; the interpreter suite is untouched.

`feat(codegen): every scalar type and operator`

---

## Task 13: Control flow and ownership

If / While / For / Break / Continue / Return, scope frees, the `Variable` and sealing discipline (spec §7.4).

- [ ] **Step 1:** the six `memoria-*` fixtures — nested list build/read/free, `adde` in a loop, assignment in a loop, `redde` from inside `si` inside `dum` inside `indem`, `abrumpe`/`continua` from a nested `si`, overwriting an owned element.
- [ ] Checkpoint: those six pass with the live count at zero, plus `exemplum-calc` and `exemplum-computatrum`.

`feat(codegen): control flow that frees what it leaves behind`

---

## Task 14: Calls, packages, deep recursion

Mangled symbols, cross-package calls, and no frame bookkeeping at all — `liss_enter`/`liss_leave` do not exist.

- [ ] Checkpoint: a multi-package program runs; `currunt/recursio-profunda` (5,000 levels) succeeds; the renamed `errores/acervus-exhaustus` produces the Latin diagnostic and exit 1.
- [ ] Rename `conformance/errores/vocatio-nimis-alte-implicata` → `errores/acervus-exhaustus`, rewrite `nuntii` and `NOTA.md`. **This is the only fixture whose expectation changes.**
- [ ] Update `cli.rs`: `unbounded_recursion_is_a_clean_latin_error` asserts the new message; `recursion_inside_the_limit_still_runs` is renamed and *raised* to a 10,000-level recursion. 22 of 23 tests untouched.

`feat(codegen): calls across packages, and a stack that ends in Latin`

---

## Task 15: Lists

The last and most intricate part of codegen. Lists are the only recursive type, the only place a kind code is read at runtime, and the only place a deep copy and a deep free can disagree.

- [ ] `ListLit`, `Index` read and write, `longitudo`, `adde`, arbitrary nesting (`ni lista lista`), deep copy, deep equality, recursive printing.
- [ ] `liss_list_append` **steals** the element words from the list it consumes — allocate a header one longer, `memcpy`, store, free only the old header. Zero element copies, because the caller already handed over a private copy. This is what keeps `adde` in a loop linear.
- [ ] Checkpoint: `exemplum-series` and every list fixture pass compiled, live count zero; the interpreter suite untouched.

`feat(codegen): lists, nested and freed`

---

## Task 16: The CLI

- [ ] `bin/liss.rs`: `run` / `build`, `--exi`, `--ad`. The bare positional form means `run`; the first argument is a subcommand iff it is exactly `run` or `build`.
- [ ] The temp file is created `O_EXCL` 0700, written, **unlinked**, then exec'd via `/proc/self/fd/<n>` — that unlink is the entire cleanup story and is what covers `SIGKILL`. Fallback path for a missing `/proc`, with parent-side `SIGINT`/`SIGTERM` handlers that unlink and re-raise.
- [ ] Stdio **inherited**, so the child's own runtime handles the closed pipe (D-06). The parent writes its Latinity warnings **before** spawning. A signalled child becomes `Error: programma abrupte cessavit`, exit 1.
- [ ] `write_or_exit` and `finish` move into `lingua_serpentis::cli`, shared by both binaries. `lingua-serpentis` stays byte-identical.
- [ ] The `ENGLISH_WORDS` carve-out for the usage string — commented, scoped to the usage text, never a general weakening.
- [ ] New `tests/liss_cli.rs`: subcommand arity, the `./run` disambiguation, `--exi`, an unwritable output path, `build` on a program that fails to check, and the produced binary's mode bits.

`feat(cli): liss run and liss build`

---

## Task 17: The differential harness

- [ ] `tests/compilata.rs`: every fixture run **both** ways, the two `Rendered` required equal. Scaffolding with a defined lifetime — deleted in Task 19.
- [ ] Measure: `cargo test --test compilata -- --test-threads=1` against the default, and record both numbers in the spec. If it disappoints, build the content-hash cache (spec §10.2) — harness only, never in the shipped compiler, where a stale cache is a correctness hazard.
- [ ] Cap the proptest generator that emits well-typed programs; it now reaches codegen on every case. Record the chosen number.
- [ ] Checkpoint: **all 86 fixtures identical between the two backends**, with only the one intentional re-blessing from Task 14.

`test(compilata): every fixture, both ways, byte for byte`

---

## Task 18: aarch64

- [ ] Checkpoint: a cross-built binary runs under `qemu-aarch64`. Failing that, `object` asserts `EM_AARCH64`, `ET_EXEC` and no `PT_INTERP`.
- [ ] Confirm the page-size and `SIGBUS` handling of Task 9 on a 16 KiB-page target.

`feat(codegen): aarch64 targets`

---

## Task 19: Delete the interpreter

**Precondition, checked and stated in the commit before a single line is removed:** every one of the 86 fixtures produces byte-identical `Rendered` under both backends, on both architectures; `cli.rs` is green; `robustness` and `generative` are green; `RECORDED_RULES` has not grown.

- [ ] Delete `interp.rs`, `Flow`, `parser::parse`, `checker::check`, the five `run*` entry points, and `tests/compilata.rs`.
- [ ] `support::run_fixture` switches to the compiled path. `conformance.rs` inherits compilata's job.
- [ ] Remove the two now-unreachable checker sites **structurally, not with `unreachable!`**: `current_ret` becomes non-optional, `root_namespace` a constructor parameter.
- [ ] `catalogus.rs`: add the three crates, the `liss_panic(`, `Defectus::novus(` and `NUNTIUS_` markers, plus a unit test pinning that `b"…"` literals are extracted.
- [ ] Register: D-04 superseded with the new mechanism and wording, D-06 noted as relocated into the runtime, **D-19 closed**.
- [ ] Re-bless `catalogus` and read every removed line. Expect fewer sites and a higher fixtured fraction.

Expected: `cargo test --workspace` green with no `interp` module; `git grep -c 'interp'` zero.

`refactor(interp): the tree-walker is gone; the compiler is the language`

---

## Final Verification

- [ ] `nix develop -c cargo test --workspace` green.
- [ ] `liss build examples/computatrum --exi /tmp/computatrum`; `file` reports statically linked; running it matches `conformance/currunt/exemplum-computatrum`.
- [ ] The binary runs in a container with no Rust toolchain.
- [ ] Every `currunt` and `monita` fixture exits with the live-allocation count at zero.
- [ ] `catalogus` snapshot has fewer sites and more fixtured than at Task 1.
- [ ] `docs/defectus/2026-08-03-index.md` records D-19 closed and D-04 superseded.
