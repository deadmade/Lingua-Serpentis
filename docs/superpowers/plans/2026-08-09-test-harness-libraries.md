# Test Harness Libraries Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace three hand-rolled pieces of test machinery — the diagnostic ratchet's byte scanner, two temp-directory implementations, and three copies of a child-wait loop — with libraries or a single shared helper, so the source rules those hand-rolled pieces impose can be retired.

**Architecture:** Three independent changes behind one design doc. The ratchet (`tests/catalogus.rs`) stops scanning source bytes for eleven marker strings and instead walks a `syn` AST, which makes three of its four guards structurally impossible rather than heuristically handled. `tempfile` replaces two divergent temp-dir implementations that cannot share code because they live in different crates. One `wait_or_kill` helper replaces three copies of a `try_wait`/`sleep` poll loop, with no new dependency.

**Tech Stack:** Rust 2024, `syn` + `quote` + `proc-macro2` (span-locations), `tempfile`, `insta` (already present).

## Context

Why this is being done, and what it fixes.

`crates/lingua-serpentis/tests/catalogus.rs` is the Latin-diagnostic ratchet: it finds every user-facing sentence the source can emit and records whether a fixture provokes it. It finds them by scanning source **bytes** for one of eleven marker strings, then hunting the first string literal within a 300-byte forward window. Because it reads text rather than code, it cannot tell a comment from a statement, so it carries four guards and a set of rules imposed **on the source itself**:

- `CLAUDE.md`: *"Never write a `/* … */` block comment under `crates/*/src`. The scanner suppresses a marker found in prose by looking for `//` earlier on the line, and assumes no block comment exists. One would invent phantom diagnostic sites."*
- `docs/internals/testing.md`: a comment may never sit between a marker and its literal (the 300-byte window); a `NUNTIUS_*` use must be distinguished from its declaration by looking left for `const`/`static`; `fn liss_panic(` must be excluded by testing whether the preceding text ends in `fn`.

`docs/superpowers/specs/2026-08-03-conformance-and-robustness-suite-design.md:418` named this as open question #1: *"whether the grep needs to become a real parse. Start with the grep; if `UNPARSED` entries accumulate, revisit."* They accumulated — five `UNPARSED` rows, two special-case guards, and eight phantom rows the comment guard exists to suppress.

**A token stream contains no comments.** Parsing with `syn` does not reimplement guards 1–3, it deletes them, and with them the project-wide prohibition on block comments in `crates/*/src`. Guard 4 (a pattern that is nothing but `{}` holes identifies nothing) is semantic and stays.

The two smaller items are ordinary duplication found alongside it: `tests/support/proc.rs:146` and `crates/liss-codegen/tests/support/mod.rs:194` each implement a scratch directory, differently, and only the first has a drop guard — the second leaks its directory whenever one of its four `.expect(...)` calls panics. And the same `try_wait` + `sleep` poll loop is written three times.

**Intended outcome:** the ratchet's site list is derived from the AST; `CLAUDE.md` loses the block-comment prohibition for `lingua-serpentis`; ~40 lines of temp-dir and ~30 lines of wait-loop code are deleted; no new *runtime* dependency is added anywhere.

---

## Global Constraints

Copied verbatim from `CLAUDE.md` and spec §11. Every task's requirements implicitly include this section.

- **Zero runtime dependencies is a rule** (spec §11). `liss-runtime`, `liss-ir` and `lingua-serpentis` have none. Only `liss-abi` may use `libc`; `liss-codegen` may use the crates its `Cargo.toml` already names and no others. **Every dependency this plan adds is a `[dev-dependencies]` entry.** No `[dependencies]` table is touched. Verify with `cargo tree -e normal`.
- **The source does not explain itself; `docs/internals/` does.** What stays in the source is `# Safety` clauses, fixture and test pointers, and a bare `D-NN`. Write the reasoning for a new decision in `docs/internals/`, not above the line it defends.
- **Comments move the catalogue snapshot.** `catalogus__catalogus.snap` records `file:line` for 123 sites. Re-bless it, then check the header still reads `123 sites · 70 fixtured · 0 ambiguous · 48 uncovered · 5 unparsed` and that only the `:line` field moved.
- Two test commands, both required: `cargo test` and `cargo test --manifest-path crates/liss-abi/Cargo.toml`. The first does not reach the second workspace.
- Work inside `nix develop`.
- Commit subjects are `type(scope): a sentence in the repo's voice` — read `git log` before writing one. Scopes are the area (`conformance`, `spec`, `testing`), not the crate name.
- Design specs: `docs/superpowers/specs/YYYY-MM-DD-<topic>-design.md`. Plans: `docs/superpowers/plans/`.

---

### Task 0: Green baseline — DONE, and the premise was wrong

**This task was written expecting a red tree, and the tree is green.** Recorded here rather than deleted, because the reasoning that predicted red is a trap worth naming.

The prediction was that `comment-strip`'s ~7,200 deletions had moved every diagnostic site's line and left `catalogus__catalogus.snap` stale, on the evidence that the snapshot records `variabilis ignota: '{}'` at `checker.rs:176` while `grep` finds that string at `:177`. That comparison is invalid: the extractor records the line of the **marker**, not of the literal. The marker here is `LinguaError {`, on line 176, one line above the `message:` field `grep` matched. The snapshot was right all along.

Measured baseline, both commands, before any change:

- `cargo test` — every target passes. `conformance` 118, `catalogus` 5, `cli` 23, `liss_cli` 24, `robustness` 29, `generative` 4.
- `cargo test --manifest-path crates/liss-abi/Cargo.toml` — passes: `abi` 21, `custos` 6, `emissio` 1.
- Snapshot header: `123 sites · 70 fixtured · 0 ambiguous · 48 uncovered · 5 unparsed`.

Those are the numbers every later task is judged against. Nothing to re-bless; no commit from this task.

**Files:**
- Modify: `crates/lingua-serpentis/tests/snapshots/catalogus__catalogus.snap`

- [ ] **Step 1: Confirm the baseline is red, and that it is red only for the expected reason**

```bash
cargo test --test catalogus 2>&1 | tail -40
```

Expected: `catalogus` FAILS with an `insta` diff whose changes are confined to `:line` fields. The three meta-tests (`byte_and_c_string_literals_are_extracted_like_any_other`, `a_diagnostic_entry_points_own_definition_is_not_a_site`, `only_a_nuntius_declaration_is_a_site_not_its_uses`, `a_marker_in_a_comment_is_prose_and_a_marker_in_code_is_not`) must PASS. If any meta-test fails, stop — the comment strip broke the extractor itself and that is a separate problem from this plan.

- [ ] **Step 2: Confirm the rest of the workspace is green**

```bash
cargo test 2>&1 | grep -E "^(test result|error)" 
cargo test --manifest-path crates/liss-abi/Cargo.toml 2>&1 | grep -E "^(test result|error)"
```

Expected: every target passes except `catalogus`. Note any other failure and stop.

- [ ] **Step 3: Re-bless the snapshot**

```bash
INSTA_UPDATE=always cargo test --test catalogus
```

- [ ] **Step 4: Verify only line numbers moved**

```bash
git diff --stat crates/lingua-serpentis/tests/snapshots/catalogus__catalogus.snap
head -6 crates/lingua-serpentis/tests/snapshots/catalogus__catalogus.snap
git diff crates/lingua-serpentis/tests/snapshots/catalogus__catalogus.snap | grep -E '^[+-]' | grep -vE '^[+-]{3}' | sed -E 's/:[0-9]+ / /' | sort | uniq -c | awk '$1 % 2 != 0'
```

Expected: the header line still reads exactly `123 sites · 70 fixtured · 0 ambiguous · 48 uncovered · 5 unparsed`. The last command strips the `:line` field and pairs each removed row with its added twin — it must print **nothing**. Any row it prints is a site that was gained, lost or reclassified, which means the strip changed behaviour and must be investigated before continuing.

- [ ] **Step 5: Run everything once more, then commit**

```bash
cargo test && cargo test --manifest-path crates/liss-abi/Cargo.toml
git add -A
git commit -m "test(catalogus): the snapshot follows the lines the strip moved"
```

---

### Task 1: The design document

SDD: the decision is recorded before the code changes. This is also where the dependency question is settled in writing, because `CLAUDE.md` says adding a dependency is a change to the spec.

**Files:**
- Create: `docs/superpowers/specs/2026-08-09-test-harness-libraries-design.md`
- Modify: `docs/README.md` (the index — the specs list and the "Which document answers which question" table)

- [ ] **Step 1: Write the design document**

Follow the structure of `docs/superpowers/specs/2026-08-03-conformance-and-robustness-suite-design.md`. Required sections:

1. **Motivation** — the four guards and the source rules they impose, quoted from `docs/internals/testing.md` and `CLAUDE.md`; the citation of open question #1 in the 2026-08-03 spec; the two temp-dir implementations and the leak in `liss-codegen`'s.
2. **Dependencies** — a table in the shape of spec §11's:

   | crate | where | why |
   |---|---|---|
   | `syn` (features `full`, `visit`) | `lingua-serpentis` **dev** | parse `crates/*/src` into an AST for the ratchet |
   | `quote` | `lingua-serpentis` **dev** | `ToTokens`, to scan a site's own token stream for its literal |
   | `proc-macro2` (feature `span-locations`) | `lingua-serpentis` **dev** | `Span::start().line`, which is off by default outside a proc macro |
   | `tempfile` | `lingua-serpentis` **dev**, `liss-codegen` **dev** | one scratch-directory implementation instead of two |

   State plainly that spec §11's rule is about **third-party runtime dependencies**, that `lingua-serpentis` already dev-depends on `insta`, `libtest-mimic` and `proptest` and `liss-codegen` on `object`, and that these four join that list and no `[dependencies]` table is touched. Name `cargo tree -e normal` as the check.
3. **The marker-to-AST mapping** — the eleven-row table from Task 4 Step 3, verbatim.
4. **Which guards survive** — guards 1 (`-> LinguaError {` / `fn liss_panic(`), 2 (comments) and 3 (`NUNTIUS_` uses) become structurally impossible; guard 4 (all-holes patterns) is semantic and is reimplemented unchanged. `UNPARSED` survives and is still a rule: spec §10.3 requires the runtime to write its sentences inline at the call site, and a site whose literal lives elsewhere still reports `UNPARSED`.
5. **What is out of scope, and why** — `crates/liss-abi/tests/abi.rs:157-185` (`sententiae`) greps its own `src/` line by line for `c"…"` literals and skips whole-line `//` only, carrying the same block-comment assumption. It is in the **second workspace**, which is `exclude`d from the root and tested by a separate cargo invocation, so converting it would mean a second, separate set of dev-dependencies for a 30-line scanner. Left alone deliberately; `CLAUDE.md`'s block-comment prohibition therefore **narrows to `crates/liss-abi/src` rather than disappearing**.
6. **Success criteria** — the snapshot header still reads `123 sites · …`, or every difference is enumerated here with its cause; `cargo tree -e normal` is unchanged; both test commands pass.

- [ ] **Step 2: Index it**

Add a row to `docs/README.md`'s "Which document answers which question" table and an entry in its specs list. Match the surrounding voice; the file resolves the amends/supersedes chain into reading order, so state that this document amends nothing and supersedes nothing.

- [ ] **Step 3: Copy this plan into the repo**

```bash
cp /home/deadmade/.claude/plans/please-propose-a-plan-zesty-hamster.md \
   docs/superpowers/plans/2026-08-09-test-harness-libraries.md
```

- [ ] **Step 4: Commit**

```bash
git add docs/
git commit -m "docs(spec): the ratchet reads bytes where it should read code"
```

---

### Task 2: One wait-with-timeout helper

The smallest item, done first because nothing depends on it and it changes no behaviour. The same loop appears at `tests/support/proc.rs:50-58`, `tests/support/proc.rs:95-105` and `tests/support/mod.rs:39-53`. The first two are byte-identical; the third differs in poll interval (`PAUSA` = 2 ms against 5 ms) and in returning `Result<_, String>` rather than an `Outcome`. **Both intervals are preserved** — `docs/internals/testing.md:87` documents the 2 ms one by name — so the interval is a parameter, not a constant.

**Files:**
- Modify: `crates/lingua-serpentis/tests/support/proc.rs` (add `wait_or_kill` and `PAUSA`; rewrite the two loops)
- Modify: `crates/lingua-serpentis/tests/support/mod.rs:39-53` (rewrite `curre`'s loop)
- Test: `crates/lingua-serpentis/tests/cli.rs` (two new tests)

**Interfaces:**
- Produces: `support::proc::wait_or_kill(&mut Child, Duration, Duration) -> std::io::Result<Option<ExitStatus>>`. `Ok(Some(status))` is an exit; `Ok(None)` means the deadline passed and the child was killed and reaped; `Err` is a `try_wait` failure, which each caller reports in its own idiom.

- [ ] **Step 1: Write the failing tests**

In `crates/lingua-serpentis/tests/cli.rs`, at the end of the file:

```rust
#[test]
fn wait_or_kill_returns_the_status_of_a_child_that_finishes() {
    let mut child = std::process::Command::new("true").spawn().expect("spawn true");
    let status = support::proc::wait_or_kill(
        &mut child,
        Duration::from_secs(10),
        Duration::from_millis(1),
    )
    .expect("try_wait");
    assert!(status.expect("the child exited before the deadline").success());
}

#[test]
fn wait_or_kill_reaps_a_child_that_outlives_its_deadline() {
    let mut child = std::process::Command::new("sleep").arg("30").spawn().expect("spawn sleep");
    let status = support::proc::wait_or_kill(
        &mut child,
        Duration::from_millis(50),
        Duration::from_millis(1),
    )
    .expect("try_wait");
    assert!(status.is_none(), "the deadline must be reported as `None`");
    // A second `wait` would block forever on a child that was not reaped.
    assert!(child.try_wait().expect("try_wait").is_some(), "the child was not reaped");
}
```

`cli.rs` already has `use std::time::Duration;` — confirm before adding a duplicate import.

- [ ] **Step 2: Run them and watch them fail**

```bash
cargo test --test cli wait_or_kill
```

Expected: FAIL, `cannot find function 'wait_or_kill' in module 'support::proc'`.

- [ ] **Step 3: Write the helper**

In `crates/lingua-serpentis/tests/support/proc.rs`, above `run_binary`:

```rust
/// This module's poll interval; `support`'s own `PAUSA` is a different, documented 2 ms.
const PAUSA: Duration = Duration::from_millis(5);

/// Wait for `child`, polling every `pausa`. At `timeout` the child is killed and reaped and
/// `Ok(None)` is returned; each caller renders that in its own idiom.
pub fn wait_or_kill(
    child: &mut std::process::Child,
    timeout: Duration,
    pausa: Duration,
) -> std::io::Result<Option<std::process::ExitStatus>> {
    let deadline = Instant::now() + timeout;
    loop {
        match child.try_wait()? {
            Some(s) => return Ok(Some(s)),
            None if Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                return Ok(None);
            }
            None => std::thread::sleep(pausa),
        }
    }
}
```

- [ ] **Step 4: Run the new tests and watch them pass**

```bash
cargo test --test cli wait_or_kill
```

Expected: PASS, 2 tests.

- [ ] **Step 5: Rewrite the three loops**

In `proc.rs`, in **both** `run_binary` and `run_closing_stdout_early`, replace the `let deadline = …; let status = loop { … };` block with:

```rust
    let status = match wait_or_kill(&mut child, timeout, PAUSA).expect("try_wait") {
        Some(s) => s,
        None => return Outcome::TimedOut,
    };
```

`run_closing_stdout_early`'s `TimedOut` arm returns the same unit variant, so nothing is lost. Remove the now-unused `Instant` import only if nothing else in the file uses it.

In `support/mod.rs`, in `curre`, replace the `let terminus = …; let status = loop { … };` block with:

```rust
    let status = match proc::wait_or_kill(&mut child, MORA, PAUSA)
        .map_err(|e| format!("wait on `liss run {}`: {e}", target.display()))?
    {
        Some(s) => s,
        None => {
            return Err(format!(
                "`liss run {}` was still running after {MORA:?}",
                target.display()
            ));
        }
    };
```

Both message strings are byte-for-byte what `curre` produced before. That matters: `support/diag.rs` and the conformance runner read these.

- [ ] **Step 6: Run every target that uses either module**

```bash
cargo test --test cli --test robustness --test conformance --test liss_cli --test generative
```

Expected: all pass, with the same counts as Task 0 plus the two new `cli.rs` tests.

- [ ] **Step 7: Commit**

```bash
git add crates/lingua-serpentis/tests/
git commit -m "test(support): one wait loop where three had drifted apart"
```

---

### Task 3: `tempfile` for both scratch directories

`proc.rs`'s `TempDir` is kept as a newtype so that the ~40 existing `d.write("fons.liss", …)` call sites across `cli.rs`, `robustness.rs` and `liss_cli.rs` do not change. What goes is the manual `Drop`, the `COUNTER`, and the pre-emptive `remove_dir_all` that exists because the name is only as unique as the PID.

**Files:**
- Modify: `crates/lingua-serpentis/Cargo.toml` (`[dev-dependencies]`)
- Modify: `crates/lingua-serpentis/tests/support/proc.rs:122-152`
- Modify: `crates/liss-codegen/Cargo.toml` (`[dev-dependencies]`)
- Modify: `crates/liss-codegen/tests/support/mod.rs:193-217` (`exsequere`)

**Interfaces:**
- Consumes: nothing from Task 2.
- Produces: `support::proc::temp_dir(prefix: &str) -> TempDir` and `TempDir::path(&self) -> &Path` / `TempDir::write(&self, rel: &str, content: &str) -> PathBuf`, all with the signatures they have today.

- [ ] **Step 1: Add the dev-dependency to both crates**

`crates/lingua-serpentis/Cargo.toml`, under `[dev-dependencies]`:

```toml
# One scratch-directory implementation, shared with `liss-codegen`'s test support, which
# cannot import this module: 2026-08-09-test-harness-libraries-design.md.
tempfile = "3"
```

`crates/liss-codegen/Cargo.toml`, under `[dev-dependencies]`, the same two-line comment and entry.

- [ ] **Step 2: Rewrite `proc.rs`'s `TempDir`**

Replace everything from `/// A directory removed on drop.` through the end of `temp_dir` (`proc.rs:121-152`) with:

```rust
/// A scratch directory removed on drop. A newtype rather than a re-export, because `write`
/// is this harness's own and forty call sites spell it.
pub struct TempDir(tempfile::TempDir);

impl TempDir {
    pub fn path(&self) -> &Path {
        self.0.path()
    }
    /// `rel` is `/`-separated and relative to the temp root; parents are created.
    pub fn write(&self, rel: &str, content: &str) -> PathBuf {
        let p = self.path().join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        let mut f = std::fs::File::create(&p).unwrap();
        f.write_all(content.as_bytes()).unwrap();
        p
    }
}

pub fn temp_dir(prefix: &str) -> TempDir {
    TempDir(
        tempfile::Builder::new()
            .prefix(prefix)
            .tempdir()
            .expect("a scratch directory"),
    )
}
```

- [ ] **Step 3: Run the three targets that use it**

```bash
cargo test --test cli --test robustness --test liss_cli
```

Expected: PASS, unchanged counts. `liss_cli.rs` is the one to watch — `run_leaves_nothing_behind_when_the_parent_is_killed` (`liss_cli.rs:293`) points the **child's** `TMPDIR` at a harness-owned directory and asserts it ends empty. The harness's own scratch directories go to the real `TMPDIR`, not the child's, so this must still pass; if it does not, the newtype is creating directories somewhere the test can see.

- [ ] **Step 4: Fix `liss-codegen`'s leak**

In `crates/liss-codegen/tests/support/mod.rs`, in `exsequere`, replace

```rust
    let dir = std::env::temp_dir().join(format!("liss-{}-{nomen}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a scratch directory");
    let via = dir.join("imago");
```

with

```rust
    // A guard, not a bare path: the four `expect`s below would leak the directory on a panic.
    let dir = tempfile::Builder::new().prefix("liss-").tempdir().expect("a scratch directory");
    let via = dir.path().join("imago");
```

and delete the trailing `let _ = std::fs::remove_dir_all(&dir);` — `dir` now removes itself when it drops at the end of the function. Leave the `ETXTBSY` retry loop exactly as it is: that race is `fd` inheritance across `fork`, not a path collision, and unique directories do not touch it.

- [ ] **Step 5: Run the codegen suites**

```bash
cargo test -p liss-codegen
```

Expected: PASS — `salve`, `scalares`, `regimen`, `series`, `vocationes`, `aemulatio`, all unchanged.

- [ ] **Step 6: Confirm no runtime dependency moved**

```bash
cargo tree -e normal -p lingua-serpentis | grep -i tempfile
cargo tree -e normal -p liss-codegen | grep -i tempfile
```

Expected: **no output** from either. `tempfile` must appear only under `-e dev`.

- [ ] **Step 7: Commit**

```bash
git add crates/lingua-serpentis/ crates/liss-codegen/
git commit -m "test(support): one scratch directory, and it survives a failing expect"
```

---

### Task 4: An AST extractor beside the scanner

The core change, done in two tasks so the switchover is a decision rather than a leap. This task adds `extract_sites_ast` **next to** the existing scanner and a differential test that compares them. Nothing switches over yet, so the snapshot cannot move.

**Files:**
- Modify: `crates/lingua-serpentis/Cargo.toml` (`[dev-dependencies]`)
- Modify: `crates/lingua-serpentis/tests/catalogus.rs` (add the extractor and one differential test)

**Interfaces:**
- Produces: `fn extract_sites_ast() -> Vec<Site>`, returning the same `Site { file, line, pattern }` values, sorted and deduped, as the existing `extract_sites`.

- [ ] **Step 1: Add the dev-dependencies**

`crates/lingua-serpentis/Cargo.toml`, under `[dev-dependencies]`:

```toml
# The diagnostic ratchet parses `crates/*/src` rather than scanning it, which is what lets
# the source carry block comments: 2026-08-09-test-harness-libraries-design.md.
# `span-locations` is off by default and is the only way to get a line number outside a
# proc macro; `quote` is for `ToTokens`, to read a site's own tokens.
syn = { version = "2", features = ["full", "visit"] }
quote = "1"
proc-macro2 = { version = "1", features = ["span-locations"] }
```

- [ ] **Step 2: Write the failing differential test**

At the end of `crates/lingua-serpentis/tests/catalogus.rs`:

```rust
/// The two extractors must agree, or every difference must be written into
/// `docs/superpowers/specs/2026-08-09-test-harness-libraries-design.md` before the scanner goes.
#[test]
fn the_ast_extractor_agrees_with_the_scanner() {
    let scanned = extract_sites();
    let parsed = extract_sites_ast();
    let solum_scanned: Vec<_> = scanned.iter().filter(|s| !parsed.contains(s)).collect();
    let solum_parsed: Vec<_> = parsed.iter().filter(|s| !scanned.contains(s)).collect();
    assert!(
        solum_scanned.is_empty() && solum_parsed.is_empty(),
        "scanner only: {solum_scanned:#?}\nparser only: {solum_parsed:#?}",
    );
}
```

- [ ] **Step 3: Run it and watch it fail**

```bash
cargo test --test catalogus the_ast_extractor_agrees
```

Expected: FAIL, `cannot find function 'extract_sites_ast'`.

- [ ] **Step 4: Write the extractor**

Add to `catalogus.rs`. The eleven markers become eleven AST predicates:

| Old marker | AST node | Span taken from |
|---|---|---|
| `LinguaError {` | `Expr::Struct`, path's last segment `LinguaError` | `node.path` |
| `Warning {` | `Expr::Struct`, last segment `Warning` | `node.path` |
| `ProgramError::structural(` | `Expr::Call`, callee path ends `ProgramError :: structural` | `node.func` |
| `.err(` | `Expr::MethodCall`, method `err` | `node.method` |
| `Err("` | `Expr::Call`, callee path `Err`, first argument a string literal | `node.func` |
| `.ok_or("` | `Expr::MethodCall`, method `ok_or`, first argument a string literal | `node.method` |
| `ok_or_else(\|\| format!(` | `Expr::MethodCall`, method `ok_or_else`, closure body an `Expr::Macro` with path `format` | `node.method` |
| `eprintln!(` | `Expr::Macro`, macro path `eprintln` | `node.mac.path` |
| `liss_panic(` | `Expr::Call`, callee path ends `liss_panic` | `node.func` |
| `Defectus::novus(` | `Expr::Call`, callee path ends `Defectus::novus` | `node.func` |
| `NUNTIUS_` | `Item::Const` or `Item::Static` whose ident starts with `NUNTIUS_` | `ident` |

**Take the span from the named column, not from the whole node.** `Expr::MethodCall::span()` starts at the *receiver*, which for a multi-line chain is an earlier line than the old marker's; `node.method.span()` is where `.err(` was.

```rust
use proc_macro2::{TokenStream, TokenTree};
use quote::ToTokens;
use syn::spanned::Spanned;

/// The first string, byte-string or C-string literal in `tokens`, descending into groups.
/// A site with none of its own is `UNPARSED`, exactly as the scanner reported it — spec §10.3
/// requires the runtime to write its sentences inline at the call site.
fn primum_literale(tokens: TokenStream) -> Option<String> {
    for tt in tokens {
        match tt {
            TokenTree::Literal(l) => match syn::Lit::new(l) {
                syn::Lit::Str(s) => return Some(s.value()),
                syn::Lit::ByteStr(s) => {
                    return Some(String::from_utf8_lossy(&s.value()).into_owned());
                }
                syn::Lit::CStr(s) => {
                    return Some(s.value().to_string_lossy().into_owned());
                }
                _ => {}
            },
            TokenTree::Group(g) => {
                if let Some(s) = primum_literale(g.stream()) {
                    return Some(s);
                }
            }
            _ => {}
        }
    }
    None
}

/// Does `path` end in `nomen`, which may itself be `A::b`?
fn semita_desinit(path: &syn::Path, nomen: &str) -> bool {
    let wanted: Vec<&str> = nomen.split("::").collect();
    let got: Vec<String> = path.segments.iter().map(|s| s.ident.to_string()).collect();
    got.len() >= wanted.len()
        && got[got.len() - wanted.len()..].iter().zip(&wanted).all(|(a, b)| a == b)
}

fn primum_argumentum_literale(args: &syn::punctuated::Punctuated<syn::Expr, syn::Token![,]>) -> bool {
    matches!(args.first(), Some(syn::Expr::Lit(l)) if matches!(l.lit, syn::Lit::Str(_)))
}

struct Lector {
    file: String,
    sites: Vec<Site>,
}

impl Lector {
    /// Record a site at `span`'s line, carrying the first literal in `tokens`.
    fn nota(&mut self, span: proc_macro2::Span, tokens: TokenStream) {
        let pattern = primum_literale(tokens).unwrap_or_else(|| "UNPARSED".to_string());
        // Guard 4, the one that survives: a pattern that is nothing but holes identifies
        // nothing — `eprintln!("{}", e)` prints a diagnostic built elsewhere.
        if pattern != "UNPARSED" && pattern.replace("{}", "").trim().is_empty() {
            return;
        }
        self.sites.push(Site { file: self.file.clone(), line: span.start().line, pattern });
    }
}

impl<'ast> syn::visit::Visit<'ast> for Lector {
    fn visit_item(&mut self, i: &'ast syn::Item) {
        // A Latin sentence written inside a unit test is not a diagnostic. This replaces
        // `production_source`'s truncation at the first `#[cfg(test)]`.
        let attrs: &[syn::Attribute] = match i {
            syn::Item::Mod(m) => &m.attrs,
            syn::Item::Fn(f) => &f.attrs,
            _ => &[],
        };
        if attrs.iter().any(|a| a.path().is_ident("cfg") && a.to_token_stream().to_string().contains("test")) {
            return;
        }
        syn::visit::visit_item(self, i);
    }

    fn visit_item_const(&mut self, i: &'ast syn::ItemConst) {
        if i.ident.to_string().starts_with("NUNTIUS_") {
            self.nota(i.ident.span(), i.expr.to_token_stream());
        }
        syn::visit::visit_item_const(self, i);
    }

    fn visit_item_static(&mut self, i: &'ast syn::ItemStatic) {
        if i.ident.to_string().starts_with("NUNTIUS_") {
            self.nota(i.ident.span(), i.expr.to_token_stream());
        }
        syn::visit::visit_item_static(self, i);
    }

    fn visit_expr(&mut self, e: &'ast syn::Expr) {
        match e {
            syn::Expr::Struct(s) if semita_desinit(&s.path, "LinguaError") || semita_desinit(&s.path, "Warning") => {
                self.nota(s.path.span(), s.fields.to_token_stream());
            }
            syn::Expr::Call(c) => {
                if let syn::Expr::Path(p) = &*c.func {
                    let s = &p.path;
                    let hit = semita_desinit(s, "ProgramError::structural")
                        || semita_desinit(s, "liss_panic")
                        || semita_desinit(s, "Defectus::novus")
                        || (semita_desinit(s, "Err") && primum_argumentum_literale(&c.args));
                    if hit {
                        self.nota(c.func.span(), c.args.to_token_stream());
                    }
                }
            }
            syn::Expr::MethodCall(m) => {
                let nomen = m.method.to_string();
                let hit = nomen == "err"
                    || (nomen == "ok_or" && primum_argumentum_literale(&m.args))
                    || (nomen == "ok_or_else"
                        && matches!(m.args.first(), Some(syn::Expr::Closure(c))
                            if matches!(&*c.body, syn::Expr::Macro(mac) if mac.mac.path.is_ident("format"))));
                if hit {
                    self.nota(m.method.span(), m.args.to_token_stream());
                }
            }
            syn::Expr::Macro(m) if m.mac.path.is_ident("eprintln") => {
                self.nota(m.mac.path.span(), m.mac.tokens.clone());
            }
            _ => {}
        }
        syn::visit::visit_expr(self, e);
    }
}

/// The same six crates, the same non-recursive `read_dir`, the same sorted `.rs` files —
/// parsed rather than scanned.
fn extract_sites_ast() -> Vec<Site> {
    let root = support::repo_root();
    let mut files: Vec<_> =
        ["lingua-serpentis", "latinitas", "liss-runtime", "liss-ir", "liss-codegen", "liss-abi"]
            .iter()
            .flat_map(|c| {
                std::fs::read_dir(root.join("crates").join(c).join("src"))
                    .unwrap()
                    .map(|e| e.unwrap().path())
            })
            .filter(|p| p.extension().is_some_and(|x| x == "rs"))
            .collect();
    files.sort();

    let mut sites = Vec::new();
    for path in files {
        let src = std::fs::read_to_string(&path).unwrap();
        let rel = path.strip_prefix(&root).unwrap().display().to_string();
        let ast = syn::parse_file(&src)
            .unwrap_or_else(|e| panic!("{rel} does not parse: {e}"));
        let mut lector = Lector { file: rel, sites: Vec::new() };
        syn::visit::Visit::visit_file(&mut lector, &ast);
        sites.extend(lector.sites);
    }
    sites.sort();
    sites.dedup();
    sites
}
```

- [ ] **Step 5: Confirm every file parses before comparing anything**

```bash
cargo test --test catalogus the_ast_extractor_agrees 2>&1 | head -30
```

If this panics with `<file> does not parse`, the `syn` version is too old for something the source uses — `liss-abi/src/custos.rs` and `defectus.rs` use edition-2024 `#[unsafe(no_mangle)]` attribute syntax and `c"…"` literals. Raise the pin (`cargo update -p syn`) until every file parses. Do not proceed on a file that does not parse: a parse failure would silently drop every site in it if the `unwrap_or_else` were softened.

- [ ] **Step 6: Drive the differential test to green, or to an enumerated list**

```bash
cargo test --test catalogus the_ast_extractor_agrees 2>&1 | head -80
```

Work through the two lists until they are empty. Expect the residue to be line-number drift from the span column of the table above — fix by taking a different sub-span, not by adjusting the expected value.

If a genuine behavioural difference remains, it is a **finding, not a failure**: the scanner's 300-byte forward window can reach past a site's own arguments into a sibling's literal, and the AST cannot. For each such row, add a line to §"Differences from the scanner" in `2026-08-09-test-harness-libraries-design.md` naming the file, the site, both patterns, and which one is right. Then relax the test to an explicit allowlist of exactly those rows.

- [ ] **Step 7: Run the whole catalogus target**

```bash
cargo test --test catalogus
```

Expected: PASS. The snapshot has not moved — `catalogus()` still calls `extract_sites`.

- [ ] **Step 8: Commit**

```bash
git add crates/lingua-serpentis/Cargo.toml crates/lingua-serpentis/tests/catalogus.rs docs/
git commit -m "test(catalogus): a parser beside the scanner, and a test that they agree"
```

---

### Task 5: Switch over, and retire the scanner

**Files:**
- Modify: `crates/lingua-serpentis/tests/catalogus.rs`
- Modify: `crates/lingua-serpentis/tests/snapshots/catalogus__catalogus.snap`

- [ ] **Step 1: Rename, so every caller moves at once**

Delete `extract_sites`, `production_source`, `opening_quote`, `literal_after` and the `MARKERS` constant. Delete `the_ast_extractor_agrees_with_the_scanner`. Rename `extract_sites_ast` to `extract_sites`. `catalogus()` and the three surviving meta-tests already call that name and need no edit.

- [ ] **Step 2: Run the meta-tests first**

```bash
cargo test --test catalogus -- --skip catalogus
```

Expected: PASS, three tests. Keep all three even though the AST makes them structurally impossible to fail — they are cheap, and they pin the intent:
- `byte_and_c_string_literals_are_extracted_like_any_other` still earns its place: `primum_literale`'s three `syn::Lit` arms are exactly what it guards.
- `a_diagnostic_entry_points_own_definition_is_not_a_site` and `a_marker_in_a_comment_is_prose_and_a_marker_in_code_is_not` now hold by construction — an `ItemFn` is not an `Expr::Call`, and a token stream has no comments.

- [ ] **Step 3: Run the snapshot test**

```bash
cargo test --test catalogus catalogus
```

- [ ] **Step 4: Judge the diff before blessing anything**

If it passes, nothing moved and there is nothing to bless — skip to Step 6. If it fails, read the diff `insta` already printed in the test output:

```bash
cargo test --test catalogus catalogus 2>&1 | head -80
```

(`cargo insta` is a separate binary and may not be in the dev shell; the failure output carries the same diff.) Check the header line first. A changed `123 sites` / `70 fixtured` / `0 ambiguous` / `48 uncovered` / `5 unparsed` means a site was gained, lost or reclassified, and **each such row must already be listed in the design document's "Differences from the scanner" section from Task 4 Step 6.** If a row appears here that is not in that list, stop and go back to Task 4 — an unexplained row is exactly the silent regression `CLAUDE.md` warns about.

- [ ] **Step 5: Bless once, with the differences recorded**

```bash
INSTA_UPDATE=always cargo test --test catalogus
git diff crates/lingua-serpentis/tests/snapshots/catalogus__catalogus.snap
```

- [ ] **Step 6: Both test commands**

```bash
cargo test && cargo test --manifest-path crates/liss-abi/Cargo.toml
```

- [ ] **Step 7: Commit**

```bash
git add crates/lingua-serpentis/ docs/
git commit -m "test(catalogus): the ratchet reads the tree, not the bytes"
```

---

### Task 6: Retire the source rules the scanner imposed

The point of the whole exercise. Until this task lands, the project is still obeying constraints that no longer exist.

**Files:**
- Modify: `CLAUDE.md` (the block-comment bullet)
- Modify: `docs/internals/testing.md` (the `catalogus.rs` section)

- [ ] **Step 1: Narrow the block-comment prohibition in `CLAUDE.md`**

The bullet currently reads *"Never write a `/* … */` block comment under `crates/*/src`"* and gives two reasons: `catalogus.rs`'s scanner, and `liss-abi/tests/abi.rs`'s `c"…"` grep. Only the first is gone. Rewrite it so the rule applies to `crates/liss-abi/src` alone and cites `abi.rs`'s `sententiae` as the sole remaining reason. Do not delete the bullet.

Leave the "Comments move the catalogue snapshot" bullet exactly as it is — the snapshot still records `file:line`, and adding a comment above a site still moves it.

- [ ] **Step 2: Rewrite `docs/internals/testing.md`'s catalogue section**

The section runs from `## The diagnostic catalogue: catalogus.rs` to `## The robustness invariant`. Replace:

- **"The eleven markers"** → the eleven-row AST table from Task 4 Step 3, keeping the existing prose on *why* each marker exists (`ProgramError::in_file` is deliberately absent; the three `String`-shaped markers exist for `roman.rs`; the last three are the back end's and were deferred to Task 19 so patterns stayed unique). That reasoning is unaffected by how the sites are found.
- **"Finding the message literal"** → `primum_literale`: the first string, byte-string or C-string literal in the site's own token stream, descending into groups. Delete the 300-byte window, `char_boundary`, and `opening_quote` — including the `lexer.rs` `°` anecdote, which was a symptom of the window.
- **"The four guards"** → **one** guard. Keep guard 4 verbatim. Replace 1, 2 and 3 with a short paragraph stating they are now structural: a function definition is an `ItemFn` and never an `Expr::Call`; a token stream contains no comments; a `NUNTIUS_*` use is an `Expr::Path` and only its declaration is an `Item::Const`/`Item::Static`. Say that the three meta-tests are kept as regression tests even though they can no longer fail by construction.
- **`production_source`** → the `#[cfg(test)]` attribute check in `visit_item`. Note this is strictly more precise than truncating at the first `"\n#[cfg(test)]\n"`, which relied on every file having at most one such module running to EOF.

Keep untouched: what it scans (six crates, non-recursive `read_dir`, nothing under `src/bin/`), `matches` and its anchoring, the four statuses, and the `fixtured`-means-uniquely-attributable discussion. None of that changed.

- [ ] **Step 3: Update the two counts `testing.md` states**

`cli.rs` gained two tests in Task 2. Find the line reading *"one named test each, every one spawning the binary through `support::proc`"* and the *"nineteen of the twenty-three tests were untouched"* sentence, and correct the total. `testing.md`'s own header promises that *"where a number is stated, a test asserts it"* — confirm against `cargo test --test cli 2>&1 | grep 'test result'`.

- [ ] **Step 4: Verify the documents describe the code**

```bash
grep -rn "opening_quote\|literal_after\|production_source\|300-byte\|300 bytes" docs/ CLAUDE.md
grep -rn "block comment" docs/ CLAUDE.md
```

Expected: the first prints nothing. The second prints only the narrowed `liss-abi` rule and `testing.md`'s corresponding note.

- [ ] **Step 5: Full verification, then commit**

```bash
cargo test && cargo test --manifest-path crates/liss-abi/Cargo.toml
git add CLAUDE.md docs/
git commit -m "docs(testing): the rules the scanner imposed on the source, lifted"
```

---

## Verification

Run from the repo root, inside `nix develop`, after Task 6.

**1. Both workspaces.**

```bash
cargo test
cargo test --manifest-path crates/liss-abi/Cargo.toml
```

Every target passes. `cli.rs` reports two more tests than at Task 0; nothing else changes count.

**2. The ratchet did not quietly lose anything.**

```bash
head -6 crates/lingua-serpentis/tests/snapshots/catalogus__catalogus.snap
```

The header reads `123 sites · 70 fixtured · 0 ambiguous · 48 uncovered · 5 unparsed`, or differs only in ways enumerated in `docs/superpowers/specs/2026-08-09-test-harness-libraries-design.md`.

**3. The corpus is untouched.**

```bash
cargo test --test conformance 2>&1 | tail -5
git status --porcelain conformance/
```

118 fixtures pass and `conformance/` has no modifications. This plan touches no expectation file; if `git status` shows one, something blessed a regression.

**4. No runtime dependency was added.**

```bash
cargo tree -e normal | grep -Ei "syn|quote|proc-macro2|tempfile"
```

**No output.** All four appear only under `cargo tree -e dev`. This is the check spec §11 turns on.

**5. The prohibition is really lifted — the point of the exercise.**

By hand, in `crates/lingua-serpentis/src/checker.rs`, insert this line immediately above `message: format!("variabilis ignota: '{}'", name),` (find it with `grep -n "variabilis ignota" crates/lingua-serpentis/src/checker.rs`):

```rust
            /* a block comment, and a marker in prose: NUNTIUS_ */
```

Then:

```bash
cargo test --test catalogus -- --skip catalogus
git checkout crates/lingua-serpentis/src/checker.rs
```

The three meta-tests must PASS with that block comment in place. Under the old scanner the `NUNTIUS_` inside it would have become a phantom site. `catalogus` itself is skipped because the inserted line shifts every following line number, which the snapshot legitimately records.

**6. The second workspace's own scanner still holds its assumption.**

```bash
cargo test --manifest-path crates/liss-abi/Cargo.toml every_latin_sentence
```

Passes. `liss-abi/tests/abi.rs`'s `sententiae` was deliberately left alone, which is why `CLAUDE.md` narrows the block-comment rule to `crates/liss-abi/src` rather than deleting it.
