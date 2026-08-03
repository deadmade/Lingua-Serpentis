# Conformance and Robustness Suite Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a fixture-driven conformance corpus, a no-crash robustness invariant, an end-to-end CLI suite, and a diagnostic-catalogue ratchet for Lingua Serpentis — recording every defect found without fixing any of them.

**Architecture:** A corpus of program folders at the workspace root (`conformance/`) is executed by test targets inside `crates/lingua-serpentis` (the only place `CARGO_BIN_EXE_lingua-serpentis` exists and the only place Cargo builds the binary). Curated fixtures run in-process through `run_dir_with_warnings`/`run_file_with_warnings` for speed; the CLI suite and the fuzzer spawn the real binary, because aborts, broken pipes and process exit status are unobservable in-process. One shared `render()` function, extracted from `main.rs`, guarantees the in-process runner asserts exactly what a user sees.

**Tech Stack:** Rust 2024 edition, rustc 1.97 / cargo 1.97. Dev-dependencies only: `libtest-mimic 0.8` (one named test per fixture), `insta 1` (catalogue snapshot), `proptest 1` (generation with shrinking). Runtime dependencies stay at zero.

**Spec:** `docs/superpowers/specs/2026-08-03-conformance-and-robustness-suite-design.md`

## Global Constraints

- **No interpreter behaviour changes.** The only production edit in this entire plan is Task 1's `render()` extraction, which is behaviour-preserving. Everything else is tests, fixtures and docs.
- **Every defect found is recorded, never fixed.** A `defectus/` fixture pins today's behaviour so `cargo test` stays green, plus an entry in `docs/defectus/2026-08-03-index.md`.
- **Runtime dependencies stay at zero.** `libtest-mimic`, `insta`, `proptest` go under `[dev-dependencies]` only. Fixture walking, diffing, blessing, mutation and the subprocess runner are hand-rolled against `std`.
- **Exact versions verified to build on this toolchain:** `libtest-mimic = "0.8"` (0.8.2), `insta = "1"` (1.48.0), `proptest = "1"` (1.11.0).
- **All diagnostic text is Latin and is asserted verbatim.** Never `contains()`. The existing suite's `contains("typ")` style is exactly the weakness this replaces. **The sole exception is text the OS supplies** (errno strings): assert the fixed prefix with `starts_with` and comment why. Two sites qualify today, both in `cli.rs`, both on `"Error reading file: "`. Everything else — every fixture in `conformance/`, every other assertion — is byte-exact.
- **Every `errores/` fixture that produces a line number puts it on a line other than 1.** Structural errors (`Error: munus 'initium' in programmate non inventum`) carry no line and are exempt.
- **Corpus path:** `concat!(env!("CARGO_MANIFEST_DIR"), "/../../conformance")`. **Binary path:** `env!("CARGO_BIN_EXE_lingua-serpentis")`. **Repo root:** `concat!(env!("CARGO_MANIFEST_DIR"), "/../..")`.
- **Fixture directory names must avoid `.gitignore` collisions.** The repo's Python-derived `.gitignore` unanchoredly ignores `lib/`, `lib64/`, `build/`, `dist/`, `var/`, `parts/`, `eggs/`, `sdist/`, `wheels/`, `env/`, `venv/`, `ENV/`, `instance/`, `cover/`, `htmlcov/`. Never name a fixture case or a package folder inside `programma/` any of these.
- **Commit after every task, with an explicit pathspec.** Branch is `rust-rewrite`. The user has unrelated work **already staged in the index** (`.envrc`, `flake.nix`, `flake.lock`, `.gitignore`, `.idea/` deletions). `git add <paths> && git commit -m "…"` does **not** protect it — `git commit` with no pathspec commits the *whole index*, sweeping their work in. Every commit step in the tasks below is written in that unsafe form; use this instead:

  ```bash
  git add <path> [<path>…]                    # ONLY for paths git does not track yet
  git commit --only <path> [<path>…] -m "message"
  ```

  `--only` restricts the commit to the listed paths and leaves the rest of the index untouched. It cannot name an **untracked** path, though — git 2.54 rejects it with `pathspec … did not match any file(s) known to git` — so new files need a `git add` of those exact paths first. Never `git add -A` or `git add .`. Include `Cargo.lock` in the pathspec whenever the task changes a manifest, or it is left as a stray uncommitted diff. After committing, `git status --short` must still show all ten of the user's staged entries.

---

## File Structure

| file | responsibility |
|---|---|
| `crates/lingua-serpentis/src/lib.rs` | **modified** — adds `Rendered` + `render()`, the single definition of what a user sees |
| `crates/lingua-serpentis/src/main.rs` | **modified** — `finish()` delegates to `render()` |
| `crates/lingua-serpentis/Cargo.toml` | **modified** — `[dev-dependencies]`, `[[test]]` harness declarations |
| `crates/lingua-serpentis/tests/support/mod.rs` | fixture model, discovery, in-process runner, diffing, blessing |
| `crates/lingua-serpentis/tests/support/proc.rs` | subprocess runner with timeout and signal classification |
| `crates/lingua-serpentis/tests/support/diag.rs` | diagnostic grammar parser and well-formedness rules |
| `crates/lingua-serpentis/tests/support/mutate.rs` | deterministic corpus mutation |
| `crates/lingua-serpentis/tests/conformance.rs` | `libtest-mimic` main; one trial per fixture |
| `crates/lingua-serpentis/tests/cli.rs` | end-to-end binary behaviour |
| `crates/lingua-serpentis/tests/robustness.rs` | the no-crash / well-formed invariant over generated input |
| `crates/lingua-serpentis/tests/catalogus.rs` | diagnostic extraction + `insta` snapshot ratchet |
| `conformance/{currunt,errores,monita,defectus}/<casus>/` | the corpus — data, not code |
| `docs/defectus/2026-08-03-index.md` | the defect register |

### Fixture format (locked here, refines spec §4.1)

```
conformance/<categoria>/<casus>/
├── programma/          the program folder (omit when VIA is present)
├── VIA                 optional: repo-relative path to an external program,
│                       e.g. "examples/calc.liss" — used instead of programma/
├── emissio             expected stdout        (absent ⇒ empty)
├── nuntii              expected stderr        (absent ⇒ empty)
├── exitus              expected exit status   (absent ⇒ 0)
└── NOTA.md             optional prose; REQUIRED under defectus/
```

`VIA` is an addition to the spec, for two reasons: it lets `currunt/` reference `examples/*.liss` instead of copying them (copies drift), and single-file targets exercise `run_file_with_warnings`, which strips `file` from warnings and is therefore a **different rendering path** from folder mode. Both paths must be covered. The runner dispatches on `is_dir()`, exactly as `main.rs` does.

---

### Task 1: Extract the CLI rendering into the library

The suite must assert exactly what a user sees. Today `main.rs::finish` inlines that rendering, so an in-process runner would have to reimplement it — and a reimplementation that drifts is worse than no test. This is the only production-code change in the plan.

**Files:**
- Modify: `crates/lingua-serpentis/src/lib.rs` (append after line 94)
- Modify: `crates/lingua-serpentis/src/main.rs:5-20`

**Interfaces:**
- Consumes: nothing.
- Produces: `lingua_serpentis::Rendered { stdout: String, stderr: String, code: i32 }` and `lingua_serpentis::render<E: Display>(Result<(String, u8, Vec<Warning>), E>) -> Rendered`. Every later task uses these.

- [ ] **Step 1: Write the failing tests**

Append to `crates/lingua-serpentis/src/lib.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use error::ProgramError;

    #[test]
    fn warnings_go_to_stderr_and_output_to_stdout() {
        let r = render(Ok::<_, ProgramError>((
            "V\n".to_string(),
            0,
            vec![Warning { message: "res".into(), file: None, line: Some(2) }],
        )));
        assert_eq!(r.stdout, "V\n");
        assert_eq!(r.stderr, "Monitum in linea 2: res\n");
        assert_eq!(r.code, 0);
    }

    #[test]
    fn every_warning_gets_its_own_line_in_order() {
        let w = |m: &str| Warning { message: m.into(), file: None, line: None };
        let r = render(Ok::<_, ProgramError>((
            String::new(),
            0,
            vec![w("prima"), w("secunda")],
        )));
        assert_eq!(r.stderr, "Monitum: prima\nMonitum: secunda\n");
    }

    #[test]
    fn the_programs_exit_status_is_carried_through() {
        let r = render(Ok::<_, ProgramError>((String::new(), 5, vec![])));
        assert_eq!(r.code, 5);
    }

    #[test]
    fn an_error_renders_to_stderr_with_status_one_and_no_stdout() {
        let r = render(Err::<(String, u8, Vec<Warning>), _>(
            ProgramError::structural("munus 'initium' in programmate non inventum"),
        ));
        assert_eq!(r.stdout, "");
        assert_eq!(r.stderr, "Error: munus 'initium' in programmate non inventum\n");
        assert_eq!(r.code, 1);
    }
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p lingua-serpentis --lib render`
Expected: FAIL — `cannot find function 'render' in this scope`.

- [ ] **Step 3: Implement `Rendered` and `render`**

Append to `crates/lingua-serpentis/src/lib.rs`, **above** the `#[cfg(test)] mod tests` block:

```rust
/// Everything a completed run puts in front of the user: the two streams and
/// the process status. Kept in the library so tests can assert exactly what
/// the CLI writes without spawning a process.
#[derive(Debug, Clone, PartialEq)]
pub struct Rendered {
    pub stdout: String,
    pub stderr: String,
    pub code: i32,
}

/// Render a pipeline result the way the CLI does: warnings to stderr — so they
/// never contaminate piped program output — program output to stdout, the
/// program's own exit status on success and 1 on any error.
pub fn render<E: std::fmt::Display>(
    result: Result<(String, u8, Vec<Warning>), E>,
) -> Rendered {
    match result {
        Ok((stdout, code, warnings)) => {
            let mut stderr = String::new();
            for w in &warnings {
                stderr.push_str(&w.to_string());
                stderr.push('\n');
            }
            Rendered { stdout, stderr, code: code as i32 }
        }
        Err(e) => Rendered { stdout: String::new(), stderr: format!("{e}\n"), code: 1 },
    }
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p lingua-serpentis --lib render`
Expected: PASS, 4 tests.

- [ ] **Step 5: Rewrite `finish` to delegate**

Replace `crates/lingua-serpentis/src/main.rs` lines 1-20 with:

```rust
use lingua_serpentis::{render, run_dir_with_warnings, run_file_with_warnings, Warning};
use std::path::Path;
use std::process::exit;

fn finish<E: std::fmt::Display>(result: Result<(String, u8, Vec<Warning>), E>) -> ! {
    let r = render(result);
    // Warnings and errors go to stderr so they never contaminate program output.
    eprint!("{}", r.stderr);
    print!("{}", r.stdout);
    exit(r.code);
}
```

- [ ] **Step 6: Verify the CLI is byte-for-byte unchanged**

Run:
```bash
cargo build -p lingua-serpentis
printf 'munus initium reddit ni {\n  ni result = °V;\n  scribe(result);\n  redde °III;\n}' > /tmp/liss-check.liss
./target/debug/lingua-serpentis /tmp/liss-check.liss; echo "exit=$?"
```
Expected, exactly:
```
Monitum in linea 2: nomen 'result' fortasse non Latinum est
V
exit=3
```

- [ ] **Step 7: Run the whole existing suite**

Run: `cargo test --workspace`
Expected: PASS, 246 tests (242 existing + 4 new).

- [ ] **Step 8: Commit**

```bash
git add crates/lingua-serpentis/src/lib.rs crates/lingua-serpentis/src/main.rs
git commit -m "refactor(cli): extract render() so tests can assert what a user sees

Behaviour-preserving: finish() now delegates stream and exit-status
rendering to the library, giving the conformance runner one definition to
assert against instead of a reimplementation that could drift."
```

---

### Task 2: The fixture runner

**Files:**
- Create: `crates/lingua-serpentis/tests/support/mod.rs`
- Create: `crates/lingua-serpentis/tests/conformance.rs`
- Create: `conformance/currunt/salve/programma/principale.liss`
- Create: `conformance/currunt/salve/emissio`
- Create: `conformance/errores/typus-let/programma/principale.liss`
- Create: `conformance/errores/typus-let/nuntii`
- Create: `conformance/errores/typus-let/exitus`
- Modify: `crates/lingua-serpentis/Cargo.toml`

**Interfaces:**
- Consumes: `lingua_serpentis::{render, Rendered, run_dir_with_warnings, run_file_with_warnings}` from Task 1.
- Produces: `support::{corpus_root, repo_root, discover, Fixture, Expected, run_fixture, blessing, bless, compare}`. `Fixture { name: String, dir: PathBuf }` with methods `target() -> PathBuf` and `expected() -> Expected`. `Expected { stdout: String, stderr: String, code: i32 }`. Tasks 3-8 add fixtures only; Tasks 9-12 reuse `corpus_root`, `repo_root` and `discover`.

- [ ] **Step 1: Add the dev-dependency and the harness declaration**

Append to `crates/lingua-serpentis/Cargo.toml`:

```toml
[dev-dependencies]
libtest-mimic = "0.8"

[[test]]
name = "conformance"
harness = false
```

Note: declaring one `[[test]]` does **not** disable autodiscovery of the others — `tests/programs.rs` keeps its default harness. This was verified on this toolchain.

- [ ] **Step 2: Write the two starter fixtures**

`conformance/currunt/salve/programma/principale.liss`:
```
munus initium reddit ni {
    scribe("Salve, munde!");
    redde nullus;
}
```
`conformance/currunt/salve/emissio` (note the trailing newline, which `scribe` emits):
```
Salve, munde!
```

`conformance/errores/typus-let/programma/principale.liss`:
```
munus initium reddit ni {
  ni x = "salve";
  redde nullus;
}
```
`conformance/errores/typus-let/nuntii`:
```
Error in 'principale.liss', linea 2: 'x' typum 'ni' expectat, sed 'voc' datum est
```
`conformance/errores/typus-let/exitus`:
```
1
```

- [ ] **Step 3: Write the support module**

Create `crates/lingua-serpentis/tests/support/mod.rs`:

```rust
#![allow(dead_code)] // each test target uses a different subset

use lingua_serpentis::Rendered;
use std::path::{Path, PathBuf};

/// The four corpus categories, in the order they are reported.
pub const CATEGORIES: [&str; 4] = ["currunt", "errores", "monita", "defectus"];

pub fn repo_root() -> PathBuf {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    p.canonicalize().unwrap_or_else(|e| panic!("repo root {}: {e}", p.display()))
}

pub fn corpus_root() -> PathBuf {
    let p = repo_root().join("conformance");
    p.canonicalize().unwrap_or_else(|e| panic!("corpus {}: {e}", p.display()))
}

/// One case directory. `name` is `<categoria>::<casus>`, which is what
/// libtest-mimic reports and what `cargo test` filters on.
#[derive(Debug, Clone)]
pub struct Fixture {
    pub name: String,
    pub dir: PathBuf,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Expected {
    pub stdout: String,
    pub stderr: String,
    pub code: i32,
}

fn read_or_empty(p: &Path) -> String {
    std::fs::read_to_string(p).unwrap_or_default()
}

impl Fixture {
    /// What gets run: an external program named by `VIA`, else `programma/`.
    pub fn target(&self) -> PathBuf {
        let via = self.dir.join("VIA");
        if via.is_file() {
            let rel = std::fs::read_to_string(&via)
                .unwrap_or_else(|e| panic!("{}: VIA: {e}", self.name));
            return repo_root().join(rel.trim());
        }
        self.dir.join("programma")
    }

    pub fn expected(&self) -> Expected {
        let exitus = self.dir.join("exitus");
        let code = if exitus.is_file() {
            let s = read_or_empty(&exitus);
            s.trim().parse().unwrap_or_else(|_| {
                panic!("{}: 'exitus' must be an integer, got {s:?}", self.name)
            })
        } else {
            0
        };
        Expected {
            stdout: read_or_empty(&self.dir.join("emissio")),
            stderr: read_or_empty(&self.dir.join("nuntii")),
            code,
        }
    }
}

/// Every case directory under every category, sorted, so the report is stable.
pub fn discover() -> Vec<Fixture> {
    let root = corpus_root();
    let mut out = Vec::new();
    for categoria in CATEGORIES {
        let dir = root.join(categoria);
        if !dir.is_dir() {
            continue;
        }
        let mut cases: Vec<PathBuf> = std::fs::read_dir(&dir)
            .unwrap_or_else(|e| panic!("cannot read {}: {e}", dir.display()))
            .map(|e| e.unwrap().path())
            .filter(|p| p.is_dir())
            .collect();
        cases.sort();
        for case in cases {
            let casus = case.file_name().unwrap().to_string_lossy().into_owned();
            out.push(Fixture { name: format!("{categoria}::{casus}"), dir: case });
        }
    }
    out
}

/// Run a fixture the way `main.rs` does: a directory is a program folder, a
/// file is a single-source program. The two paths render warnings
/// differently — folder mode attributes them to a file, single-file mode does
/// not — so both must be exercised.
pub fn run_fixture(f: &Fixture) -> Rendered {
    let target = f.target();
    if target.is_dir() {
        lingua_serpentis::render(lingua_serpentis::run_dir_with_warnings(&target))
    } else {
        let src = std::fs::read_to_string(&target)
            .unwrap_or_else(|e| panic!("{}: cannot read {}: {e}", f.name, target.display()));
        lingua_serpentis::render(lingua_serpentis::run_file_with_warnings(&src))
    }
}

pub fn blessing() -> bool {
    std::env::var_os("LISS_BLESS").is_some_and(|v| v != "0")
}

/// Rewrite the expectation files from actual output. An empty stream removes
/// its file rather than leaving an empty one, so `git status` shows intent.
pub fn bless(f: &Fixture, got: &Rendered) {
    write_or_remove(&f.dir.join("emissio"), &got.stdout);
    write_or_remove(&f.dir.join("nuntii"), &got.stderr);
    let exitus = if got.code == 0 { String::new() } else { format!("{}\n", got.code) };
    write_or_remove(&f.dir.join("exitus"), &exitus);
}

fn write_or_remove(p: &Path, content: &str) {
    if content.is_empty() {
        let _ = std::fs::remove_file(p);
    } else {
        std::fs::write(p, content).unwrap_or_else(|e| panic!("write {}: {e}", p.display()));
    }
}

pub fn compare(f: &Fixture, got: &Rendered) -> Result<(), String> {
    let want = f.expected();
    let mut report = String::new();
    if let Some(d) = diff("emissio (stdout)", &want.stdout, &got.stdout) {
        report.push_str(&d);
    }
    if let Some(d) = diff("nuntii (stderr)", &want.stderr, &got.stderr) {
        report.push_str(&d);
    }
    if want.code != got.code {
        report.push_str(&format!(
            "--- exitus ---\n  expected: {}\n  actual:   {}\n",
            want.code, got.code
        ));
    }
    if report.is_empty() {
        Ok(())
    } else {
        report.push_str("\nre-bless with: LISS_BLESS=1 cargo test --test conformance\n");
        Err(report)
    }
}

fn diff(label: &str, want: &str, got: &str) -> Option<String> {
    if want == got {
        return None;
    }
    let mut s = format!("--- {label}: expected ---\n{want}");
    if !s.ends_with('\n') {
        s.push('\n');
    }
    s.push_str(&format!("--- {label}: actual ---\n{got}"));
    if !s.ends_with('\n') {
        s.push('\n');
    }
    let w: Vec<&str> = want.lines().collect();
    let g: Vec<&str> = got.lines().collect();
    if let Some(i) = (0..w.len().max(g.len())).find(|&i| w.get(i) != g.get(i)) {
        s.push_str(&format!(
            "--- {label}: first difference at line {} ---\n  expected: {:?}\n  actual:   {:?}\n",
            i + 1,
            w.get(i),
            g.get(i)
        ));
    }
    Some(s)
}
```

- [ ] **Step 4: Write the conformance harness**

Create `crates/lingua-serpentis/tests/conformance.rs`:

```rust
//! The conformance corpus: every case under `conformance/` becomes one named
//! test. Cases run in-process — a subprocess is only needed for what the
//! library cannot observe, which lives in `cli.rs` and `robustness.rs`.
mod support;

use libtest_mimic::{Arguments, Failed, Trial};

fn main() {
    let args = Arguments::from_args();
    let fixtures = support::discover();
    assert!(
        !fixtures.is_empty(),
        "no fixtures found under {}",
        support::corpus_root().display()
    );
    let trials: Vec<Trial> = fixtures
        .into_iter()
        .map(|f| Trial::test(f.name.clone(), move || run(&f)))
        .collect();
    libtest_mimic::run(&args, trials).exit();
}

fn run(f: &support::Fixture) -> Result<(), Failed> {
    let got = support::run_fixture(f);
    if support::blessing() {
        support::bless(f, &got);
        return Ok(());
    }
    support::compare(f, &got).map_err(Failed::from)
}
```

- [ ] **Step 5: Run the harness**

Run: `cargo test --test conformance`
Expected: PASS, 2 tests, named `currunt::salve` and `errores::typus-let`.

- [ ] **Step 6: Verify a wrong expectation actually fails, and that blessing fixes it**

Run:
```bash
printf 'Vale, munde!\n' > conformance/currunt/salve/emissio
cargo test --test conformance currunt::salve
```
Expected: FAIL, with a diff showing `expected: "Vale, munde!"` / `actual: "Salve, munde!"`.

Then:
```bash
LISS_BLESS=1 cargo test --test conformance currunt::salve
git diff --stat conformance/
cargo test --test conformance
```
Expected: the file is restored to `Salve, munde!` and both tests pass.

- [ ] **Step 7: Commit**

```bash
git add crates/lingua-serpentis/Cargo.toml crates/lingua-serpentis/tests/support/mod.rs \
        crates/lingua-serpentis/tests/conformance.rs conformance/
git commit -m "test(conformance): fixture-driven runner with one named test per case

Each case is a program folder plus expected stdout/stderr/exit status, so
file attribution is testable. LISS_BLESS=1 regenerates expectations."
```

---

### Task 3: `currunt/` — the examples, run as programs

Today four of five shipped examples are `include_str!`'d and `examples/salve.liss` is referenced by nothing. `computatrum/` is wired file-by-file, so a new file added to it joins no test. Referencing the examples through `VIA` fixes all three.

**Files:**
- Create: `conformance/currunt/exemplum-{salve,calc,praecisio,series,computatrum}/{VIA,emissio}`
- Create: `conformance/currunt/exemplum-calc/nuntii`

**Interfaces:**
- Consumes: `support::Fixture::target()` VIA handling from Task 2.
- Produces: nothing consumed by later tasks.

- [ ] **Step 1: Create the five VIA fixtures**

```bash
for e in salve calc praecisio series; do
  mkdir -p conformance/currunt/exemplum-$e
  printf 'examples/%s.liss\n' $e > conformance/currunt/exemplum-$e/VIA
done
mkdir -p conformance/currunt/exemplum-computatrum
printf 'examples/computatrum\n' > conformance/currunt/exemplum-computatrum/VIA
```

- [ ] **Step 2: Verify they fail before blessing**

Run: `cargo test --test conformance currunt::exemplum`
Expected: FAIL — five cases, each reporting an empty `emissio` against real output.

- [ ] **Step 3: Bless and read the diff**

Run:
```bash
LISS_BLESS=1 cargo test --test conformance currunt::exemplum
git status --short conformance/
cat conformance/currunt/exemplum-calc/nuntii
```

Expected `conformance/currunt/exemplum-calc/nuntii`:
```
Monitum in linea 1: nomen 'summare' fortasse non Latinum est
```

**This warning is defect D-18.** `2026-07-19-latin-identifier-checker-design.md` §12 leaves open whether programmer-Latin infinitives like `summare` need a supplemental word list, to be added "only if a real example fails". A real example does fail — this one — and the supplement was never added, so the open question was answered by reality and nobody acted on it. Keep the blessed file: the fixture is what makes it visible. Task 8 writes it up.

`exemplum-praecisio` will also warn, on `tertia_brevis` and `tertia_longa`. That is **not** a defect and gets no ID: §9.1 lists "a name with a digit/underscore" as a specified **negative** for `latinitas::is_latin`, so the checker is behaving exactly as designed on identifiers the example's author chose. Bless it and leave it.

- [ ] **Step 4: Verify the blessed expectations**

Run: `cargo test --test conformance currunt::exemplum`
Expected: PASS, 5 tests.

Sanity-check two of the blessed files by hand:
- `conformance/currunt/exemplum-calc/emissio` must be `X̅X̅X̅DXXXII` plus a newline.
- `conformance/currunt/exemplum-computatrum/emissio` must be `XLII\nXLII\n`.

- [ ] **Step 5: Commit**

```bash
git add conformance/currunt/
git commit -m "test(conformance): run every shipped example as a fixture

Via VIA references rather than copies, so the fixtures cannot drift from
examples/. salve.liss was previously referenced by no test at all, and
computatrum/ is now picked up as a folder, so files added to it join the
suite automatically.

Records D-18: calc.liss warns on 'summare' though the identifier-checker
design says it should lint cleanly."
```

---

### Task 4: `errores/` — lexer and parser diagnostics

**Files:**
- Create: `conformance/errores/<casus>/programma/principale.liss` + `nuntii` + `exitus` for each case below.

**Interfaces:**
- Consumes: Task 2's runner.
- Produces: nothing consumed by later tasks; Task 13's catalogue reads the fixtures back.

All expected strings below were captured from `target/debug/lingua-serpentis` on 2026-08-03 and are exact.

- [ ] **Step 1: Write the fixture programs**

Each `programma/principale.liss` (all deliberately place the error below line 1):

| casus | program |
|---|---|
| `chorda-non-clausa` | `munus initium reddit ni {`⏎`  voc s = "salve;`⏎`  redde nullus;`⏎`}` |
| `signum-ignotum` | `munus initium reddit ni {`⏎`  ni x = 5;`⏎`  redde nullus;`⏎`}` |
| `expectatum-semicolon` | `munus initium reddit ni {`⏎`  ni x = °I`⏎`  redde nullus;`⏎`}` |
| `expectatum-rparen` | `munus initium reddit ni {`⏎`  scribe(°I;`⏎`  redde nullus;`⏎`}` |
| `numerus-romanus-vacuus` | `munus initium reddit ni {`⏎`  ni x = °;`⏎`  redde nullus;`⏎`}` |
| `numerus-romanus-invalidus` | `munus initium reddit ni {`⏎`  ni x = °X/;`⏎`  redde nullus;`⏎`}` |
| `expressio-expectata` | `munus initium reddit ni {`⏎`  ni x = ;`⏎`  redde nullus;`⏎`}` |
| `declarationes-solae` | `scribe(°I);`⏎`munus initium reddit ni { redde nullus; }` |

- [ ] **Step 2: Write the expected `nuntii` by hand, then verify by running**

Write these `nuntii` files (each with a trailing newline) and `exitus` files containing `1`:

```
chorda-non-clausa         Error in 'principale.liss', linea 2: chorda non clausa
signum-ignotum            Error in 'principale.liss', linea 2: signum ignotum: '5'
expectatum-semicolon      Error in 'principale.liss', linea 3: expectatum: Semicolon
expectatum-rparen         Error in 'principale.liss', linea 2: expectatum: RParen
numerus-romanus-vacuus    Error in 'principale.liss', linea 2: numerus Romanus vacuus
numerus-romanus-invalidus Error in 'principale.liss', linea 2: numerus Romanus invalidus: 'X/'
expressio-expectata       Error in 'principale.liss', linea 2: expressio expectata
```

Write them by hand rather than blessing: a blessed error fixture proves nothing, because blessing accepts whatever the implementation currently does. `declarationes-solae` is the exception — bless it and check the message names `nihil nisi declarationes`.

- [ ] **Step 3: Run and confirm every one passes**

Run: `cargo test --test conformance errores::`
Expected: PASS. Any failure here means a hand-written expectation is wrong — read the diff and correct the expectation, not the program.

Note `expectatum-semicolon` reports **linea 3**, not 2: the parser reports at the token *after* the missing one. That is deliberate in the expectation and is worth its own comment.

- [ ] **Step 4: Record the two defects this task exposes**

Create `conformance/errores/expectatum-semicolon/NOTA.md`:
```markdown
`expectatum: Semicolon` prints the Rust `TokenKind` Debug name — see D-02.
The line is 3, not 2: the parser reports at the token following the missing
semicolon. Pinned deliberately.
```

Create `conformance/errores/signum-ignotum/NOTA.md`:
```markdown
An Arabic digit gets the lexer's catch-all with no hint that `°V` exists —
the likeliest beginner mistake in a Roman-numeral language. See D-15.
```

- [ ] **Step 5: Commit**

```bash
git add conformance/errores/
git commit -m "test(conformance): lexer and parser diagnostics, asserted verbatim

Eight cases, each with the error below line 1 — no test in the workspace
previously asserted a parser line number on a multi-line source.

Records D-02 (expectatum: Semicolon leaks a Rust type name) and D-15."
```

---

### Task 5: `errores/` — checker diagnostics

**Files:**
- Create: `conformance/errores/<casus>/programma/**` + `nuntii` + `exitus`.

**Interfaces:**
- Consumes: Task 2's runner. Produces: nothing.

- [ ] **Step 1: Write the single-file cases**

| casus | program (line 1 is `munus initium reddit ni {` unless shown) | expected `nuntii` |
|---|---|---|
| `typus-litterae` | `  lit c = "ab";` | `Error in 'principale.liss', linea 2: 'c' typum 'lit' expectat, sed 'voc' datum est` |
| `operatio-binaria` | `  ni x = °V + "a";` | `Error in 'principale.liss', linea 2: operatio typis 'ni' et 'voc' non convenit` |
| `variabilis-ignota` | `  scribe(y);` | `Error in 'principale.liss', linea 2: variabilis ignota: 'y'` |
| `condicio-non-veritas` | `  si (°V) { scribe(°I); }` | `Error in 'principale.liss', linea 2: condicio 'veritas' esse debet` |
| `functio-ignota` | `  scribe(nusquam());` | `Error in 'principale.liss', linea 2: functio ignota: 'nusquam'` |
| `index-seriei-non-ni` | `  ni lista xs = [°I];`⏎`  ni y = xs["a"];` | `Error in 'principale.liss', linea 3: index seriei 'ni' esse debet` |

Each program is wrapped as:
```
munus initium reddit ni {
<the body line(s) above>
  redde nullus;
}
```
and each `exitus` contains `1`.

`index-seriei-non-ni` matters specifically: the survey found that the two existing tests claiming to cover this message use a **fractional** index, which dies one step earlier in `resolve_expr`. Both emission sites were therefore unreached despite two passing `is_err()` assertions. A text index reaches it.

- [ ] **Step 2: Write the multi-statement cases**

`conformance/errores/numerus-argumentorum/programma/principale.liss`:
```
munus geminus capit (ni a) reddit ni { redde a; }
munus initium reddit ni {
  scribe(geminus(°I, °II));
  redde nullus;
}
```
`nuntii`: `Error in 'principale.liss', linea 3: functio 'geminus' numerum argumentorum falsum accepit`

`conformance/errores/munus-nidificatum/programma/principale.liss`:
```
munus exterior reddit ni {
  munus interior reddit ni { redde °I; }
  redde °I;
}
munus initium reddit ni { redde nullus; }
```
`nuntii`: `Error in 'principale.liss', linea 2: functio 'interior' intra corpus definiri non potest; functiones nidificatae non sustinentur`

- [ ] **Step 3: Write the multi-file attribution case**

This is the case the whole folder-fixture format exists for.

`conformance/errores/typus-reditus-in-fasciculo/programma/principale.liss`:
```
affer mathematica;
munus initium reddit ni { redde mathematica.summa(°I, °II); }
```
`conformance/errores/typus-reditus-in-fasciculo/programma/mathematica/summa.liss`:
```
// prima linea
munus summa capit (ni a, ni b) reddit ni { redde "salve"; }
```
`nuntii`:
```
Error in 'mathematica/summa.liss', linea 2: reditus typum 'ni' expectat, sed 'voc' datum est
```
`exitus`: `1`

The assertion is that the error names **`mathematica/summa.liss`**, not `principale.liss` and not a bare line number.

- [ ] **Step 4: Add the package and structural cases**

| casus | programma | expected `nuntii` |
|---|---|---|
| `fasciculus-non-inventus` | `principale.liss`: `affer ignota;`⏎`munus initium reddit ni { redde nullus; }` | `Error in 'principale.liss', linea 1: fasciculus 'ignota' non inventus` |
| `fasciculus-non-allatus` | `principale.liss` using `mathematica.summa(...)` without `affer`, plus `mathematica/summa.liss` | `Error in 'principale.liss', linea 1: fasciculus 'mathematica' non allatus est` |
| `functio-bis-definita` | two root files both defining `munus duplum` | `Error in 'principale.liss', linea 1: functio 'duplum' bis definita` |
| `initium-non-inventum` | one root file defining only `munus f reddit ni { redde nullus; }` | `Error: munus 'initium' in programmate non inventum` |
| `initium-cum-parametris` | `munus initium capit (ni a) reddit ni { redde nullus; }` | `Error in 'principale.liss', linea 1: munus 'initium' nihil capere et 'ni' reddere debet` |
| `fasciculus-in-fasciculo` | `mathematica/interior/x.liss` present | `Error: fasciculus in fasciculo non licet: 'mathematica/interior'` |

`initium-non-inventum` and `fasciculus-in-fasciculo` are the structural cases exempt from the line-number rule — they carry no file and no line by design, and asserting the bare `Error: ` prefix is the point.

- [ ] **Step 5: Run**

Run: `cargo test --test conformance errores::`
Expected: PASS, ~21 cases.

- [ ] **Step 6: Commit**

```bash
git add conformance/errores/
git commit -m "test(conformance): checker diagnostics incl. multi-file attribution

index-seriei-non-ni closes a real gap: the two existing tests for that
message use a fractional index that dies one step earlier, so both emission
sites were unreached despite passing is_err() assertions."
```

---

### Task 6: `errores/` — runtime and program-assembly diagnostics

**Files:**
- Create: `conformance/errores/{divisio-per-nullum,modulus-per-nullum,index-extra-fines,nomen-fasciculi-non-validum}/**`
- Create: `conformance/defectus/D-08-emissio-perdita/**`

**Interfaces:**
- Consumes: Task 2's runner. Produces: nothing.

- [ ] **Step 1: The three runtime errors**

Each wrapped in `munus initium reddit ni { ... redde nullus; }`:

| casus | body | expected `nuntii` |
|---|---|---|
| `divisio-per-nullum` | `  ni z = °I - °I;`⏎`  scribe(°V / z);` | `Error in 'principale.liss', linea 3: divisio per nullum` |
| `modulus-per-nullum` | `  ni z = °I - °I;`⏎`  scribe(°V % z);` | `Error in 'principale.liss', linea 3: modulus per nullum` |
| `index-extra-fines` | `  ni lista xs = [°I, °II];`⏎`  scribe(xs[°X]);` | `Error in 'principale.liss', linea 3: index extra fines: X (longitudo 2)` |

`modulus-per-nullum` had zero test coverage while its sibling `divisio-per-nullum` was tested — pairing them is the point. `index-extra-fines` pins **defect D-12**: the index is a Roman numeral and the length is an Arabic `2`, in a language whose design states "no Arabic number anywhere a user can see".

Create `conformance/errores/index-extra-fines/NOTA.md`:
```markdown
`longitudo 2` is Arabic — the only user-visible Arabic number left in the
language. Pinned as-is; see D-12.
```

- [ ] **Step 2: The package-name case**

`conformance/errores/nomen-fasciculi-non-validum/programma/principale.liss`:
```
munus initium reddit ni { redde nullus; }
```
plus `conformance/errores/nomen-fasciculi-non-validum/programma/si/x.liss`:
```
munus quinque reddit ni { redde °V; }
```
`nuntii`: `Error: nomen fasciculi 'si' non validum est`

A keyword cannot be a package name because the folder name must lex as exactly one identifier.

- [ ] **Step 3: Pin defect D-08 — a runtime error discards prior output**

`conformance/defectus/D-08-emissio-perdita/programma/principale.liss`:
```
munus initium reddit ni {
  scribe("ante");
  ni z = °I - °I;
  scribe(°V / z);
  redde nullus;
}
```
`nuntii`: `Error in 'principale.liss', linea 4: divisio per nullum`
`exitus`: `1`
`emissio`: **absent** — and that is the defect.

`conformance/defectus/D-08-emissio-perdita/NOTA.md`:
```markdown
# D-08 · A runtime error discards everything already printed

`scribe("ante")` runs before the division, but stdout is empty: `scribe`
accumulates into `Interp::out` and `lib.rs`'s `Err` arm never returns it.

- **Observed:** stdout empty, stderr the division error, exit 1.
- **Expected:** stdout `ante\n`, then the error.
- **Cause:** `interp.rs` accumulates output in `Interp::out`; `run_loaded`
  (`lib.rs:44`) discards it on the error path.

This fixture pins the WRONG behaviour so the suite stays green. When D-08 is
fixed this test will fail — that failure is the fix's acceptance criterion.
Move the fixture to `errores/` with `emissio` containing `ante` at that point.
```

- [ ] **Step 4: Run**

Run: `cargo test --test conformance`
Expected: PASS, all cases including `defectus::D-08-emissio-perdita`.

- [ ] **Step 5: Commit**

```bash
git add conformance/
git commit -m "test(conformance): runtime and assembly diagnostics

Pairs modulus-per-nullum with the already-tested divisio-per-nullum, pins
D-12 (Arabic 'longitudo 2') and D-08 (a runtime error discards everything
already printed)."
```

---

### Task 7: `monita/` — warnings, including cross-file ordering

No test in the workspace has ever had warnings coming from two different files at once, so `lint_program`'s ordering — package-name warnings first with neither file nor line, then root files by path, then package files — is entirely emergent and unpinned. All three `Warning` Display arms are exercised here.

**Files:**
- Create: `conformance/monita/{ordo-trans-scidas,fons-unicus,programma-Latinum}/**`

**Interfaces:**
- Consumes: Task 2's runner, including `VIA` for the single-file case.
- Produces: nothing.

- [ ] **Step 1: The cross-file ordering case**

`conformance/monita/ordo-trans-scidas/programma/principale.liss`:
```
affer foobar;
munus initium reddit ni {
  ni result = foobar.quinque();
  scribe(result);
  redde nullus;
}
```
`conformance/monita/ordo-trans-scidas/programma/foobar/x.liss`:
```
munus quinque reddit ni {
  ni answer = °V;
  redde answer;
}
```
`emissio`:
```
V
```
`nuntii` — all three Display arms, in exactly this order:
```
Monitum: nomen fasciculi 'foobar' fortasse non Latinum est
Monitum in 'principale.liss', linea 3: nomen 'result' fortasse non Latinum est
Monitum in 'foobar/x.liss', linea 2: nomen 'answer' fortasse non Latinum est
```
No `exitus` file (exit 0 — warnings never stop a program).

- [ ] **Step 2: The single-file case (different rendering path)**

`conformance/monita/fons-unicus/programma/principale.liss` cannot be used — folder mode always attributes a file. Use `VIA` pointing at a single file instead:

Create `conformance/monita/fons-unicus/fons.liss`:
```
munus initium reddit ni {
  ni result = °V;
  scribe(result);
  redde nullus;
}
```
Create `conformance/monita/fons-unicus/VIA`:
```
conformance/monita/fons-unicus/fons.liss
```
`emissio`:
```
V
```
`nuntii`:
```
Monitum in linea 2: nomen 'result' fortasse non Latinum est
```

The assertion is that single-file mode emits `Monitum in linea 2:` with **no file**, unlike folder mode's `Monitum in 'principale.liss', linea 2:`.

- [ ] **Step 3: The negative case**

`conformance/monita/programma-Latinum/programma/principale.liss`:
```
affer mathematica;
munus initium reddit ni {
  scribe(mathematica.summa(°XX, °XXII));
  redde nullus;
}
```
`conformance/monita/programma-Latinum/programma/mathematica/summa.liss`:
```
munus summa capit (ni primus, ni secundus) reddit ni {
  redde primus + secundus;
}
```
`emissio`:
```
XLII
```
No `nuntii` file — fully Latin identifiers must produce **no** warnings at all. This is the case that catches a latinity regression that starts warning on valid Latin.

- [ ] **Step 4: Run**

Run: `cargo test --test conformance monita::`
Expected: PASS, 3 tests.

- [ ] **Step 5: Commit**

```bash
git add conformance/monita/
git commit -m "test(conformance): warnings, incl. the never-tested cross-file ordering

All three Warning Display arms in one fixture, plus single-file mode (which
omits the file) and a fully-Latin negative case."
```

---

### Task 8: The defect register

**Files:**
- Create: `docs/defectus/2026-08-03-index.md`
- Create: `conformance/defectus/D-18-calc-monet/NOTA.md`

**Interfaces:**
- Consumes: defect IDs referenced by Tasks 3-7. Produces: the register that Tasks 9-12 append to.

- [ ] **Step 1: Write the register**

Create `docs/defectus/2026-08-03-index.md` with a header explaining the convention, then one section per defect. Copy the 21 confirmed defects from spec §8.1 verbatim, expanding each to the full form. Use exactly this shape:

```markdown
# Lingua Serpentis — Defect Register

Opened 2026-08-03 by the conformance and robustness suite
(`docs/superpowers/specs/2026-08-03-conformance-and-robustness-suite-design.md`).

**Nothing here is fixed.** Each defect has a fixture under
`conformance/defectus/` pinning today's behaviour so `cargo test` stays green.
Fixing a defect makes its fixture fail — that failure is the acceptance
criterion. Move the fixture to the right category with the corrected
expectation as part of the fix.

| ID | severity | one-line | covered by |
|----|----------|----------|-----------|
| D-01 | high | No all-paths-return analysis; the type system is unsound | `defectus/D-01-reditus-omissus` |
| D-02 | medium | `expectatum: Semicolon` leaks a Rust `TokenKind` Debug name | `errores/expectatum-semicolon`, rule `no Rust type names` |
| D-03 | high | Line numbers wrong after a multi-line string | `defectus/D-03-linea-post-chordam` |
| D-04 | high | Unbounded recursion aborts (SIGABRT, exit 134) | `cli.rs` (ignored), rule `no abort` |
| D-05 | medium | Deep expression nesting aborts in the parser | `robustness.rs` pathology, rule `no abort` |
| D-06 | medium | Broken pipe panics (exit 101) | `cli.rs` (ignored), rule `no panic` |
| D-07 | medium | `abrumpe`/`continua` outside a loop is a silent no-op | `defectus/D-07-abrumpe-extra-iterationem` |
| D-08 | high | A runtime error discards everything already printed | `defectus/D-08-emissio-perdita` |
| D-09 | medium | Any error discards all Latinity warnings | `defectus/D-09-monita-perdita` |
| D-10 | low | Exit status silently truncated / wrapped | `cli.rs` |
| D-11 | low | Two English diagnostics in `main.rs` | `cli.rs`, rule `Latin only` |
| D-12 | low | Arabic numerals in `index extra fines: X (longitudo 2)` | `errores/index-extra-fines`, rule `no Arabic digits` |
| D-13 | medium | Roman numeral form never validated (`°IIX` = 10) | `defectus/D-13-numerus-non-canonicus` |
| D-14 | medium | `°X/°II` lexes as one numeral; whitespace is load-bearing | `defectus/D-14-numerus-sine-spatio` |
| D-15 | low | Arabic digits get the lexer's catch-all with no hint | `errores/signum-ignotum` |
| D-16 | medium | Duplicate parameter names accepted; last binding wins | `defectus/D-16-parametri-gemini` |
| D-17 | medium | A stray hidden directory with a `.liss` kills the program | `cli.rs` |
| D-18 | low | `examples/calc.liss` warns on `summare`; the design's §12 open question was answered by reality and never acted on | `currunt/exemplum-calc` |
| D-19 | medium | ~50 unit tests run against the laxer legacy API | register only — no test |
| D-20 | low | `x = x * x` in a loop runs forever | rule `terminates` |
| D-21 | low | `mag_divmod`'s zero guard is a `debug_assert` | register only — unreachable today |

---

### D-01 · Nulla analysis reditus — the type system is unsound

- **Severity:** high — a checker-approved program produces a value of the wrong type
- **Repro:** `conformance/defectus/D-01-reditus-omissus/`
- **Observed:** `munus nomen reddit voc { si (falsus) { redde "a"; } }` called from
  `initium` prints `nullus` and exits 0 — a declared-`voc` call yielded an `Int`.
- **Expected:** a checker error naming the missing return.
- **Cause:** `interp.rs:167` ends every call body with `Ok(Value::Int(Roman::zero()))`
  regardless of declared type; `checker.rs` only checks the `redde` statements
  actually written.
- **Note:** this is what promotes four "defensive" interpreter messages
  (`operatio binaria invalida`, `condicio non est veritas`, `comparatio invalida`,
  `subscriptio invalida`) from unreachable to reachable. Any fix must special-case
  `initium`, whose fall-through the packages design licenses deliberately and which
  `interp.rs::falling_off_initium_exits_zero` pins as correct.
```

- [ ] **Step 2: Add a fixture for every defect that does not have one yet**

Tasks 3-7 already produced fixtures for D-08, D-12, D-15, D-18 and D-02. Create a `conformance/defectus/D-NN-<nomen>/` for each remaining defect that is reachable through the fixture runner:

- `D-01-reditus-omissus` — the program above; `emissio` is `nullus\n`, exit 0.
- `D-03-linea-post-chordam` — a multi-line string then an error two lines later;
  pins `linea 4` where the correct answer is `linea 5`.
- `D-07-abrumpe-extra-iterationem` — `scribe(°I); abrumpe; scribe(°II);` in `initium`;
  pins `emissio` of `I\nII\n`, exit 0.
- `D-09-monita-perdita` — a program with a non-Latin name *and* a runtime error;
  pins an empty `nuntii` apart from the error, proving the warning was dropped.
- `D-13-numerus-non-canonicus` — `scribe(°IIX + °VV + °IM);` pins `MXIX`.
- `D-14-numerus-sine-spatio` — `scribe(°X/°II);` pins `numerus Romanus invalidus: 'X/'`.
- `D-16-parametri-gemini` — `munus f capit (ni a, voc a) reddit voc { redde a; }` pins that
  the last binding wins.

Defects D-04, D-05, D-06, D-10, D-11, D-17, D-19, D-20, D-21 are **not** fixture-shaped —
they need a subprocess, a filesystem manipulation, or no repro at all. They are covered
by Tasks 9-12 and their register entries name the test that covers them instead.

Every `defectus/` fixture must have a `NOTA.md` in the D-08 shape from Task 6.

- [ ] **Step 3: Verify each defect fixture actually reproduces its defect**

Run: `cargo test --test conformance defectus::`
Expected: PASS. Then hand-check three of them against the register: the observed
value in the `NOTA.md` must match the blessed `emissio`/`nuntii` exactly.

- [ ] **Step 4: Commit**

```bash
git add docs/defectus/ conformance/defectus/
git commit -m "docs: defect register, with a pinning fixture per reproducible defect

21 confirmed defects, each with a repro and the responsible file:line.
Nothing is fixed: the fixtures pin today's behaviour so the suite is green,
and a fix makes its fixture fail on purpose."
```

---

### Task 9: The CLI suite

Nothing in the repo has ever spawned the binary. `main.rs` is entirely uncovered, and `run_dir_with_warnings` — the whole disk pipeline — has one caller and no tests.

**Files:**
- Create: `crates/lingua-serpentis/tests/support/proc.rs`
- Create: `crates/lingua-serpentis/tests/cli.rs`
- Modify: `crates/lingua-serpentis/tests/support/mod.rs` (add `pub mod proc;`)

**Interfaces:**
- Consumes: `support::repo_root` from Task 2.
- Produces: `support::proc::{run_binary, Outcome, temp_dir, TempDir}`. `Outcome` is consumed by Tasks 10-12.

- [ ] **Step 1: Write the subprocess runner**

Create `crates/lingua-serpentis/tests/support/proc.rs`:

```rust
//! Spawning the real binary. Needed for everything the library cannot
//! observe: aborts, broken pipes, real process exit status, stream routing
//! and argv handling.
#![allow(dead_code)]

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

pub const BINARY: &str = env!("CARGO_BIN_EXE_lingua-serpentis");

/// How a run of the binary ended.
#[derive(Debug, Clone, PartialEq)]
pub enum Outcome {
    /// Exited normally with this status.
    Exited { code: i32, stdout: String, stderr: String },
    /// Killed by a signal — SIGABRT from a stack overflow, SIGSEGV, ...
    Signalled { signal: i32, stdout: String, stderr: String },
    /// Still running when the deadline passed.
    TimedOut,
}

impl Outcome {
    pub fn stderr(&self) -> &str {
        match self {
            Outcome::Exited { stderr, .. } | Outcome::Signalled { stderr, .. } => stderr,
            Outcome::TimedOut => "",
        }
    }
    pub fn stdout(&self) -> &str {
        match self {
            Outcome::Exited { stdout, .. } | Outcome::Signalled { stdout, .. } => stdout,
            Outcome::TimedOut => "",
        }
    }
}

/// Run the binary with `args`, capturing both streams to temp files rather
/// than pipes — a pipe would deadlock as soon as the child fills the buffer,
/// which some of these inputs do on purpose.
pub fn run_binary(args: &[&std::ffi::OsStr], timeout: Duration) -> Outcome {
    let dir = temp_dir("liss-proc");
    let out_path = dir.path().join("stdout");
    let err_path = dir.path().join("stderr");
    let out = std::fs::File::create(&out_path).unwrap();
    let err = std::fs::File::create(&err_path).unwrap();

    let mut child = Command::new(BINARY)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::from(out))
        .stderr(Stdio::from(err))
        .spawn()
        .expect("spawn lingua-serpentis");

    let deadline = Instant::now() + timeout;
    let status = loop {
        match child.try_wait().expect("try_wait") {
            Some(s) => break s,
            None if Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                return Outcome::TimedOut;
            }
            None => std::thread::sleep(Duration::from_millis(5)),
        }
    };

    let stdout = String::from_utf8_lossy(&std::fs::read(&out_path).unwrap()).into_owned();
    let stderr = String::from_utf8_lossy(&std::fs::read(&err_path).unwrap()).into_owned();

    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        if let Some(signal) = status.signal() {
            return Outcome::Signalled { signal, stdout, stderr };
        }
    }
    Outcome::Exited { code: status.code().unwrap_or(-1), stdout, stderr }
}

/// Like `run_binary`, but reads a little of the child's stdout and then closes
/// the pipe while the child is still writing. This is the only way to provoke
/// a broken pipe, which is not observable through a file redirect.
pub fn run_closing_stdout_early(args: &[&std::ffi::OsStr], timeout: Duration) -> Outcome {
    use std::io::Read;

    let dir = temp_dir("liss-pipe");
    let err_path = dir.path().join("stderr");
    let err = std::fs::File::create(&err_path).unwrap();

    let mut child = Command::new(BINARY)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::from(err))
        .spawn()
        .expect("spawn lingua-serpentis");

    {
        let mut out = child.stdout.take().expect("piped stdout");
        let mut buf = [0u8; 64];
        let _ = out.read(&mut buf);
        // `out` drops here, closing the read end while the child writes on.
    }

    let deadline = Instant::now() + timeout;
    let status = loop {
        match child.try_wait().expect("try_wait") {
            Some(s) => break s,
            None if Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                return Outcome::TimedOut;
            }
            None => std::thread::sleep(Duration::from_millis(5)),
        }
    };

    let stderr = String::from_utf8_lossy(&std::fs::read(&err_path).unwrap()).into_owned();
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        if let Some(signal) = status.signal() {
            return Outcome::Signalled { signal, stdout: String::new(), stderr };
        }
    }
    Outcome::Exited { code: status.code().unwrap_or(-1), stdout: String::new(), stderr }
}

/// A directory removed on drop. Hand-rolled because runtime deps stay at zero
/// and `tempfile` is not worth a dev-dep for twenty lines.
pub struct TempDir(PathBuf);

impl TempDir {
    pub fn path(&self) -> &Path {
        &self.0
    }
    /// Write a file, creating parent directories. Path is `/`-separated and
    /// relative to the temp root.
    pub fn write(&self, rel: &str, content: &str) -> PathBuf {
        let p = self.0.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        let mut f = std::fs::File::create(&p).unwrap();
        f.write_all(content.as_bytes()).unwrap();
        p
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

static COUNTER: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

pub fn temp_dir(prefix: &str) -> TempDir {
    let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let p = std::env::temp_dir().join(format!("{prefix}-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).unwrap();
    TempDir(p)
}
```

Add to `crates/lingua-serpentis/tests/support/mod.rs`, after the `use` block:
```rust
pub mod proc;
```

- [ ] **Step 2: Write the CLI tests**

Create `crates/lingua-serpentis/tests/cli.rs`. This target uses the **default** harness, so no `[[test]]` entry is needed.

```rust
//! End-to-end behaviour of the `lingua-serpentis` binary. Everything here
//! needs a real process: argv, the stdout/stderr split, exit status, and the
//! failure modes that abort rather than return.
mod support;

use std::ffi::OsStr;
use std::time::Duration;
use support::proc::{run_binary, run_closing_stdout_early, temp_dir, Outcome};

const T: Duration = Duration::from_secs(20);

fn run(args: &[&str]) -> Outcome {
    let owned: Vec<&OsStr> = args.iter().map(OsStr::new).collect();
    run_binary(&owned, T)
}

fn exited(o: &Outcome) -> (i32, &str, &str) {
    match o {
        Outcome::Exited { code, stdout, stderr } => (*code, stdout.as_str(), stderr.as_str()),
        other => panic!("expected a normal exit, got {other:?}"),
    }
}

#[test]
fn no_arguments_prints_usage_to_stderr() {
    let o = run(&[]);
    let (code, out, err) = exited(&o);
    assert_eq!(code, 1);
    assert_eq!(out, "");
    assert_eq!(err, "Usus: lingua-serpentis <via.liss | directorium>\n");
}

#[test]
fn two_arguments_prints_usage_to_stderr() {
    let o = run(&["a.liss", "b.liss"]);
    let (code, _, err) = exited(&o);
    assert_eq!(code, 1);
    assert_eq!(err, "Usus: lingua-serpentis <via.liss | directorium>\n");
}

#[test]
fn a_missing_file_reports_in_english_defect_d11() {
    let d = temp_dir("liss-cli");
    let missing = d.path().join("nusquam.liss");
    let o = run(&[missing.to_str().unwrap()]);
    let (code, _, err) = exited(&o);
    assert_eq!(code, 1);
    // D-11: English, in a language whose every other diagnostic is Latin.
    // Prefix, not verbatim: the errno tail is OS-supplied. The prefix itself
    // is the defect, and it is fully ours.
    assert!(err.starts_with("Error reading file: "), "got: {err:?}");
}

#[test]
fn a_non_liss_extension_is_refused_in_english_defect_d11() {
    let d = temp_dir("liss-cli");
    let p = d.write("notae.txt", "munus initium reddit ni { redde nullus; }");
    let o = run(&[p.to_str().unwrap()]);
    let (code, _, err) = exited(&o);
    assert_eq!(code, 1);
    assert_eq!(err, "Error: file must have .liss extension\n");
}

#[test]
fn a_nonexistent_directory_gets_the_extension_error_defect_d11() {
    // The is_dir() check runs first, so a path that is neither file nor
    // directory falls through to the extension branch and gets a message
    // about extensions rather than about the missing path.
    let d = temp_dir("liss-cli");
    let missing = d.path().join("nusquam");
    let o = run(&[missing.to_str().unwrap()]);
    let (code, _, err) = exited(&o);
    assert_eq!(code, 1);
    assert_eq!(err, "Error: file must have .liss extension\n");
}

#[test]
fn a_directory_named_dot_liss_runs_as_a_program_folder() {
    // is_dir() is checked before the extension, so this is a folder program.
    let d = temp_dir("liss-cli");
    d.write("scaena.liss/principale.liss",
            "munus initium reddit ni { scribe(\"ex directorio\"); redde nullus; }");
    let o = run(&[d.path().join("scaena.liss").to_str().unwrap()]);
    let (code, out, err) = exited(&o);
    assert_eq!((code, out, err), (0, "ex directorio\n", ""));
}

#[test]
fn warnings_go_to_stderr_and_output_to_stdout_separately() {
    let d = temp_dir("liss-cli");
    let p = d.write("fons.liss",
        "munus initium reddit ni {\n  ni result = °V;\n  scribe(result);\n  redde nullus;\n}");
    let o = run(&[p.to_str().unwrap()]);
    let (code, out, err) = exited(&o);
    assert_eq!(code, 0);
    // The contract: a piped stdout is never contaminated by warnings.
    assert_eq!(out, "V\n");
    assert_eq!(err, "Monitum in linea 2: nomen 'result' fortasse non Latinum est\n");
}

#[test]
fn the_process_exit_status_comes_from_initium() {
    let d = temp_dir("liss-cli");
    let p = d.write("fons.liss", "munus initium reddit ni { redde °III; }");
    assert_eq!(exited(&run(&[p.to_str().unwrap()])).0, 3);
}

#[test]
fn an_out_of_range_exit_status_is_silently_truncated_defect_d10() {
    let d = temp_dir("liss-cli");
    let p = d.write("fons.liss", "munus initium reddit ni { redde °C.C.C; }");
    // D-10: 300 does not fit in a process exit status and is wrapped, not refused.
    assert_eq!(exited(&run(&[p.to_str().unwrap()])).0, 116);
}

#[test]
fn a_negative_exit_status_is_silently_wrapped_defect_d10() {
    let d = temp_dir("liss-cli");
    let p = d.write("fons.liss", "munus initium reddit ni { redde -°I; }");
    assert_eq!(exited(&run(&[p.to_str().unwrap()])).0, 255);
}

#[test]
fn a_hidden_directory_containing_liss_kills_the_program_defect_d17() {
    let d = temp_dir("liss-cli");
    d.write("prog/principale.liss", "munus initium reddit ni { redde nullus; }");
    d.write("prog/.git/vestigium.liss", "munus x reddit ni { redde °I; }");
    let o = run(&[d.path().join("prog").to_str().unwrap()]);
    let (code, _, err) = exited(&o);
    // D-17: hidden directories are not skipped by load_dir.
    assert_eq!(code, 1);
    assert_eq!(err, "Error: nomen fasciculi '.git' non validum est\n");
}

#[test]
fn non_liss_files_in_a_program_folder_are_ignored() {
    let d = temp_dir("liss-cli");
    d.write("prog/principale.liss",
            "munus initium reddit ni { scribe(\"salve\"); redde nullus; }");
    d.write("prog/notae.txt", "non est programma");
    d.write("prog/vacua/intus/.keep", "");
    let o = run(&[d.path().join("prog").to_str().unwrap()]);
    assert_eq!(exited(&o).0, 0);
    assert_eq!(exited(&o).1, "salve\n");
}

#[test]
fn an_unreadable_file_is_reported_not_panicked() {
    let d = temp_dir("liss-cli");
    let p = d.write("fons.liss", "munus initium reddit ni { redde nullus; }");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o000)).unwrap();
    }
    let o = run(&[p.to_str().unwrap()]);
    let (code, _, err) = exited(&o);
    assert_eq!(code, 1);
    // Prefix, not verbatim: the OS supplies the errno tail ("Permission denied
    // (os error 13)"), which is not portable. The only carve-out the Global
    // Constraints allow.
    assert!(err.starts_with("Error reading file: "), "got: {err:?}");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o644)).unwrap();
    }
}

#[test]
fn a_non_utf8_source_file_is_reported_not_panicked() {
    let d = temp_dir("liss-cli");
    let p = d.path().join("fons.liss");
    std::fs::write(&p, [0xff, 0xfe, 0x00, 0x62]).unwrap();
    let o = run(&[p.to_str().unwrap()]);
    let (code, _, err) = exited(&o);
    assert_eq!(code, 1);
    assert_eq!(err, "Error reading file: stream did not contain valid UTF-8\n");
}
```

- [ ] **Step 3: Run the CLI suite**

Run: `cargo test --test cli`
Expected: PASS, 15 tests. If `a_missing_file_reports_in_english_defect_d11` fails on a
non-Linux OS the message text differs — that is acceptable; gate it with `#[cfg(target_os = "linux")]`
rather than weakening the assertion.

- [ ] **Step 4: Add the two crash tests, as `#[ignore]` with a defect note**

These assert the behaviour we *want* and therefore fail today. They are the one place
in the plan where `#[ignore]` is right: there is no sensible "current behaviour" to bless
when the process aborts.

Append to `crates/lingua-serpentis/tests/cli.rs`:

```rust
#[test]
#[ignore = "D-06: Rust ignores SIGPIPE and print! panics; exit 101 with a backtrace note"]
fn a_closed_pipe_is_not_a_panic() {
    // A real reproduction, not a placeholder: read a little of the child's
    // stdout, close the pipe, and let it keep writing. Every other Unix tool
    // exits quietly here.
    let d = temp_dir("liss-cli");
    let p = d.write(
        "fons.liss",
        "munus initium reddit ni {\n  ni i = nullus;\n  \
         dum (i = nullus; i minor_est °M.M.M.M.M; i = i + °I) {\n    \
         scribe(\"linea satis longa ad tubum implendum\");\n  }\n  redde nullus;\n}",
    );
    let o = run_closing_stdout_early(&[OsStr::new(p.to_str().unwrap())], T);
    match o {
        Outcome::Exited { code, stderr, .. } => {
            assert_ne!(code, 101, "panicked on a closed pipe; stderr: {stderr:?}");
            assert!(!stderr.contains("panicked"), "stderr: {stderr:?}");
        }
        Outcome::Signalled { signal, .. } => assert_eq!(signal, 13, "expected SIGPIPE or a clean exit"),
        Outcome::TimedOut => panic!("hung on a closed pipe"),
    }
}

#[test]
#[ignore = "D-04: unbounded recursion aborts with SIGABRT (exit 134), outside the error model"]
fn unbounded_recursion_is_a_clean_latin_error() {
    let d = temp_dir("liss-cli");
    let p = d.write("fons.liss",
        "munus recurre capit (ni n) reddit ni { redde recurre(n + °I); }\n\
         munus initium reddit ni { scribe(recurre(°I)); redde nullus; }");
    let o = run(&[p.to_str().unwrap()]);
    let (code, _, err) = exited(&o); // panics today: the process is Signalled
    assert_eq!(code, 1);
    assert!(err.contains("alte"), "expected a Latin depth error, got {err:?}");
}
```

Run: `cargo test --test cli -- --ignored`
Expected: both FAIL, with messages naming D-04 and D-06. This is the intended state.

- [ ] **Step 5: Commit**

```bash
git add crates/lingua-serpentis/tests/support/proc.rs crates/lingua-serpentis/tests/cli.rs \
        crates/lingua-serpentis/tests/support/mod.rs
git commit -m "test(cli): end-to-end binary behaviour, previously untested entirely

Nothing in the repo had ever spawned the binary. Covers argv, the
stdout/stderr split, real exit status, and the disk pipeline that
run_dir_with_warnings had one caller and no tests for.

Pins D-10, D-11, D-17. D-04 and D-06 are #[ignore]d, failing on purpose."
```

---

### Task 10: The robustness invariant and its curated pathologies

**Files:**
- Create: `crates/lingua-serpentis/tests/support/diag.rs`
- Create: `crates/lingua-serpentis/tests/robustness.rs`
- Modify: `crates/lingua-serpentis/tests/support/mod.rs` (add `pub mod diag;`)

**Interfaces:**
- Consumes: `support::proc::{run_binary, Outcome, temp_dir}` from Task 9.
- Produces: `support::diag::{Diagnostic, parse_diagnostic, Verdict, judge}`. Tasks 11-12 call `judge`.

- [ ] **Step 1: Write the diagnostic grammar and the well-formedness rules**

Create `crates/lingua-serpentis/tests/support/diag.rs`:

```rust
//! The shape every diagnostic must have, and the rules it must satisfy. This
//! is the specification the fuzzer checks against.
#![allow(dead_code)]

use super::proc::Outcome;

#[derive(Debug, Clone, PartialEq)]
pub enum Diagnostic {
    /// `Error: <message>` — structural, about the program's shape.
    Structural { message: String },
    /// `Error in linea N: <message>` — single-source mode.
    Line { line: usize, message: String },
    /// `Error in '<file>', linea N: <message>` — folder mode.
    FileLine { file: String, line: usize, message: String },
}

impl Diagnostic {
    pub fn message(&self) -> &str {
        match self {
            Diagnostic::Structural { message }
            | Diagnostic::Line { message, .. }
            | Diagnostic::FileLine { message, .. } => message,
        }
    }
    pub fn line(&self) -> Option<usize> {
        match self {
            Diagnostic::Structural { .. } => None,
            Diagnostic::Line { line, .. } | Diagnostic::FileLine { line, .. } => Some(*line),
        }
    }
}

/// Parse one rendered diagnostic line. Returns `None` when the line does not
/// match the grammar at all — which is itself a finding.
pub fn parse_diagnostic(s: &str) -> Option<Diagnostic> {
    let rest = s.strip_prefix("Error")?;
    if let Some(m) = rest.strip_prefix(": ") {
        return Some(Diagnostic::Structural { message: m.to_string() });
    }
    let rest = rest.strip_prefix(" in ")?;
    if let Some(m) = rest.strip_prefix("linea ") {
        let (n, msg) = m.split_once(": ")?;
        return Some(Diagnostic::Line { line: n.parse().ok()?, message: msg.to_string() });
    }
    // `'<file>', linea N: <message>`
    let rest = rest.strip_prefix('\'')?;
    let (file, rest) = rest.split_once("', linea ")?;
    let (n, msg) = rest.split_once(": ")?;
    Some(Diagnostic::FileLine {
        file: file.to_string(),
        line: n.parse().ok()?,
        message: msg.to_string(),
    })
}

/// Rust `TokenKind` variant names. None of these may ever reach a user.
pub const RUST_TYPE_NAMES: [&str; 24] = [
    "Semicolon", "RParen", "LParen", "RBrace", "LBrace", "RBracket", "LBracket",
    "Assign", "Comma", "Dot", "Ident", "TyInt", "TyFloat", "TyDouble", "TyText",
    "TyChar", "TyBool", "Takes", "Yields", "Lista", "Import", "Percent", "Star", "Slash",
];

/// Words that betray an untranslated English string.
pub const ENGLISH_WORDS: [&str; 8] = [
    "file", "must", "have", "extension", "reading", "directory", "invalid", "expected",
];

/// Rules whose violations are already in the defect register. A violation of
/// any OTHER rule is a new finding and fails the test that saw it. Shrinking
/// this array is how progress gets measured — never add to it without adding
/// a register entry too.
pub const RECORDED_RULES: [&str; 6] = [
    "no abort",           // D-04, D-05
    "terminates",         // D-20
    "no panic",           // D-06
    "no Arabic digits",   // D-12
    "no Rust type names", // D-02
    "Latin only",         // D-11
];

/// A run either satisfies the invariant or names the rule it broke. The
/// mapping from rule to recorded defect lives in
/// `docs/defectus/2026-08-03-index.md`, not here, so this file stays a
/// specification rather than a bug list — and so one rule can cover several
/// defects (both stack overflows break `no abort`).
#[derive(Debug, Clone, PartialEq)]
pub enum Verdict {
    /// Ran to completion, or failed with a well-formed Latin diagnostic.
    Acceptable,
    Violation { rule: &'static str, detail: String },
}

/// Strip quoted spans. They are user input echoed back — an identifier, a
/// numeral, a character — and the lexical rules below are about text the
/// *language* wrote. Without this, `signum ignotum: '5'` would trip the
/// no-Arabic-digits rule for quoting the user's own `5` back at them, and
/// `variabilis ignota: 'Semicolon'` would trip the Rust-type-name rule for a
/// user who named a variable `Semicolon`.
fn without_quoted(msg: &str) -> String {
    let mut out = String::new();
    let mut quote: Option<char> = None;
    for c in msg.chars() {
        match quote {
            Some(q) if c == q => quote = None,
            Some(_) => {}
            None if c == '\'' || c == '"' => quote = Some(c),
            None => out.push(c),
        }
    }
    out
}

/// Judge one run against the invariant. `sources` maps program-relative path
/// to source text, so line numbers can be range-checked.
pub fn judge(outcome: &Outcome, sources: &[(String, String)]) -> Verdict {
    let violation = |rule: &'static str, detail: String| Verdict::Violation { rule, detail };

    let (code, stderr) = match outcome {
        Outcome::TimedOut => return violation("terminates", "still running at the deadline".into()),
        Outcome::Signalled { signal, .. } => {
            return violation("no abort", format!("killed by signal {signal}"))
        }
        Outcome::Exited { code, stderr, .. } => (*code, stderr.as_str()),
    };

    if code == 0 {
        for line in stderr.lines() {
            if !line.starts_with("Monitum") {
                return violation("clean success", format!("exit 0 but stderr has {line:?}"));
            }
        }
        return Verdict::Acceptable;
    }

    if code == 101 {
        return violation("no panic", format!("exit 101; stderr {stderr:?}"));
    }
    if code != 1 {
        // Any other status came from `initium`, which is legitimate.
        return Verdict::Acceptable;
    }

    let Some(last) = stderr.lines().last() else {
        return violation("diagnostic present", "exit 1 with empty stderr".into());
    };
    let Some(d) = parse_diagnostic(last) else {
        return violation("diagnostic grammar", format!("unparseable: {last:?}"));
    };

    if let Diagnostic::FileLine { file, .. } = &d {
        if !sources.iter().any(|(p, _)| p == file) {
            return violation("file exists", format!("names {file:?}, not in the program"));
        }
    }
    if let Some(line) = d.line() {
        let src = match &d {
            Diagnostic::FileLine { file, .. } => {
                sources.iter().find(|(p, _)| p == file).map(|(_, s)| s.as_str())
            }
            _ => sources.first().map(|(_, s)| s.as_str()),
        };
        if let Some(src) = src {
            let n = src.lines().count().max(1);
            if line == 0 || line > n {
                return violation(
                    "line in range",
                    format!("linea {line} but the file has {n} lines"),
                );
            }
        }
    }

    let msg = d.message();
    if msg.is_empty() {
        return violation("message non-empty", format!("{last:?}"));
    }
    let bare = without_quoted(msg);
    // English before digits: `Error: file must have .liss extension` should be
    // reported as an untranslated string, not as a stray numeral.
    let lower = bare.to_lowercase();
    if let Some(w) = ENGLISH_WORDS
        .iter()
        .find(|w| lower.split(|c: char| !c.is_alphabetic()).any(|t| t == **w))
    {
        return violation("Latin only", format!("{msg:?} contains {w:?}"));
    }
    if bare.chars().any(|c| c.is_ascii_digit()) {
        return violation("no Arabic digits", format!("{msg:?}"));
    }
    if let Some(w) = RUST_TYPE_NAMES.iter().find(|w| bare.contains(**w)) {
        return violation("no Rust type names", format!("{msg:?} contains {w:?}"));
    }
    Verdict::Acceptable
}
```

Add `pub mod diag;` next to `pub mod proc;` in `support/mod.rs`.

- [ ] **Step 2: Write a test for the diagnostic parser itself**

The parser is the specification; a wrong parser silently passes everything. Create
`crates/lingua-serpentis/tests/robustness.rs` starting with:

```rust
//! The robustness invariant: within a timeout, the process terminates with
//! either exit 0 and a stderr of warnings only, or exit 1 and a well-formed
//! Latin diagnostic. Anything else is a defect.
//!
//! Every input runs in its own process, because a stack overflow aborts and
//! cannot be caught in-process.
mod support;

use std::ffi::OsStr;
use std::time::Duration;
use support::diag::{judge, parse_diagnostic, Diagnostic, Verdict, RECORDED_RULES};
use support::proc::{run_binary, temp_dir};

const T: Duration = Duration::from_secs(10);

#[test]
fn the_diagnostic_grammar_parses_all_three_shapes() {
    assert_eq!(
        parse_diagnostic("Error: munus 'initium' in programmate non inventum"),
        Some(Diagnostic::Structural {
            message: "munus 'initium' in programmate non inventum".into()
        })
    );
    assert_eq!(
        parse_diagnostic("Error in linea 2: chorda non clausa"),
        Some(Diagnostic::Line { line: 2, message: "chorda non clausa".into() })
    );
    assert_eq!(
        parse_diagnostic("Error in 'mathematica/summa.liss', linea 2: res mala"),
        Some(Diagnostic::FileLine {
            file: "mathematica/summa.liss".into(),
            line: 2,
            message: "res mala".into(),
        })
    );
    assert_eq!(parse_diagnostic("Monitum in linea 2: nomen"), None);
    assert_eq!(parse_diagnostic("thread 'main' panicked"), None);
}
```

- [ ] **Step 3: Run it**

Run: `cargo test --test robustness the_diagnostic_grammar`
Expected: PASS.

- [ ] **Step 4: Add the runner helper and the curated pathologies**

Append to `crates/lingua-serpentis/tests/robustness.rs`:

```rust
/// Run one single-file source through the binary and judge it.
fn judge_source(name: &str, src: &str) -> Verdict {
    let d = temp_dir("liss-rob");
    let p = d.write("fons.liss", src);
    let outcome = run_binary(&[OsStr::new(p.to_str().unwrap())], T);
    let verdict = judge(&outcome, &[("fons.liss".to_string(), src.to_string())]);
    if let Verdict::Violation { rule, detail } = &verdict {
        eprintln!("[{name}] {rule}: {detail}");
    }
    verdict
}

/// Every curated pathology with the rule it is known to break — `None` means
/// it must satisfy the invariant. The rule, not a defect ID: both stack
/// overflows break `no abort`, and the register is what maps rules to IDs.
fn pathologies() -> Vec<(&'static str, String, Option<&'static str>)> {
    vec![
        ("vacuum", String::new(), None),
        ("spatia sola", "   \n\t\n".to_string(), None),
        ("BOM", "\u{feff}munus initium reddit ni { redde nullus; }".to_string(), None),
        (
            "CRLF",
            "munus initium reddit ni {\r\n  scribe(°I);\r\n  redde nullus;\r\n}".to_string(),
            None,
        ),
        (
            "nomen longissimum",
            format!(
                "munus initium reddit ni {{ ni {} = °I; redde nullus; }}",
                "a".repeat(100_000)
            ),
            None,
        ),
        (
            "numerus longissimus",
            format!(
                "munus initium reddit ni {{ scribe(°{}); redde nullus; }}",
                "M".repeat(5_000)
            ),
            None,
        ),
        (
            "series longissima",
            format!(
                "munus initium reddit ni {{ ni lista xs = [{}°I]; scribe(longitudo(xs)); redde nullus; }}",
                "°I, ".repeat(5_000)
            ),
            None,
        ),
        (
            "parentheses nimium implicatae",
            format!(
                "munus initium reddit ni {{ scribe({}°I{}); redde nullus; }}",
                "(".repeat(20_000),
                ")".repeat(20_000)
            ),
            Some("no abort"), // D-05
        ),
        (
            "recursio infinita",
            "munus recurre capit (ni n) reddit ni { redde recurre(n + °I); }\n\
             munus initium reddit ni { scribe(recurre(°I)); redde nullus; }"
                .to_string(),
            Some("no abort"), // D-04
        ),
        (
            "index extra fines",
            "munus initium reddit ni {\n  ni lista xs = [°I];\n  scribe(xs[°X]);\n  redde nullus;\n}"
                .to_string(),
            Some("no Arabic digits"), // D-12 — `longitudo 1` is Arabic
        ),
        (
            // D-03 is NOT detectable by this invariant: the reported line is
            // wrong but still inside the file, so `line in range` passes. It is
            // caught by conformance/defectus/D-03-linea-post-chordam instead.
            // Kept here to prove the input at least stays inside the error model.
            "chorda multilinea",
            "munus initium reddit ni {\n  voc s = \"prima\nsecunda\ntertia\";\n  ni x = °I\n  redde nullus;\n}"
                .to_string(),
            None,
        ),
    ]
}

#[test]
fn every_curated_pathology_matches_its_recorded_verdict() {
    let mut surprises = Vec::new();
    for (name, src, expected_rule) in pathologies() {
        let verdict = judge_source(name, &src);
        match (&verdict, expected_rule) {
            (Verdict::Acceptable, None) => {}
            (Verdict::Violation { rule, .. }, Some(want)) if *rule == want => {}
            (Verdict::Acceptable, Some(want)) => surprises.push(format!(
                "{name}: expected to break the {want:?} rule, but it now passes — \
                 if a defect was fixed, update pathologies() and the register"
            )),
            (v, w) => surprises.push(format!("{name}: expected {w:?}, got {v:?}")),
        }
    }
    assert!(surprises.is_empty(), "{}", surprises.join("\n"));
}
```

- [ ] **Step 5: Run and record what it finds**

Run: `cargo test --test robustness -- --nocapture`
Expected: PASS. Any `surprises` entry is either a new defect (add it to the register
and to `pathologies()`) or a stale expectation.

Note the two-way ratchet: if a defect gets fixed, the test fails telling you to update
the register. That is deliberate — the allowlist must shrink, not rot.

- [ ] **Step 6: Commit**

```bash
git add crates/lingua-serpentis/tests/support/diag.rs \
        crates/lingua-serpentis/tests/robustness.rs \
        crates/lingua-serpentis/tests/support/mod.rs
git commit -m "test(robustness): the no-crash, well-formed-diagnostic invariant

Judges every run: exit 0 with warnings only, or exit 1 with a diagnostic
that parses, names a real file, points at a line that exists, and contains
no Arabic digits, Rust type names or English.

Known violations carry a defect ID rather than weakening the rule, and the
test fails if a recorded defect silently starts passing."
```

---

### Task 11: Mutation fuzzing over the corpus

**Files:**
- Create: `crates/lingua-serpentis/tests/support/mutate.rs`
- Modify: `crates/lingua-serpentis/tests/support/mod.rs` (add `pub mod mutate;`)
- Modify: `crates/lingua-serpentis/tests/robustness.rs`

**Interfaces:**
- Consumes: `support::{corpus_root, repo_root}`, `support::diag::judge`.
- Produces: `support::mutate::{Rng, mutate}`.

- [ ] **Step 1: Write the mutator**

Create `crates/lingua-serpentis/tests/support/mutate.rs`:

```rust
//! Deterministic source mutation. A fixed seed means a failure reproduces
//! exactly; `LISS_FUZZ_SEED` varies it for a soak run.
#![allow(dead_code)]

/// SplitMix64 — small, fast, and good enough to shuffle bytes with.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed)
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 { 0 } else { (self.next_u64() % n as u64) as usize }
    }
}

/// Characters worth injecting: the ones the lexer treats specially.
const INTERESTING: [char; 16] =
    ['°', '"', '\'', '{', '}', '(', ')', '[', ']', ';', ',', '.', '/', ':', '\n', '5'];

/// Produce one mutation of `src`. Operating on `char`s rather than bytes keeps
/// the output valid UTF-8, so we test the language rather than `read_to_string`.
pub fn mutate(src: &str, rng: &mut Rng) -> String {
    let mut cs: Vec<char> = src.chars().collect();
    if cs.is_empty() {
        return "°".to_string();
    }
    match rng.below(7) {
        0 => {
            let i = rng.below(cs.len());
            cs.remove(i);
        }
        1 => {
            let i = rng.below(cs.len());
            cs.insert(i, cs[i]);
        }
        2 => {
            let (i, j) = (rng.below(cs.len()), rng.below(cs.len()));
            cs.swap(i, j);
        }
        3 => {
            let i = rng.below(cs.len());
            cs[i] = INTERESTING[rng.below(INTERESTING.len())];
        }
        4 => {
            let i = rng.below(cs.len());
            cs.truncate(i);
        }
        5 => {
            let i = rng.below(cs.len());
            cs.insert(i, INTERESTING[rng.below(INTERESTING.len())]);
        }
        _ => {
            // Duplicate a whole span — good at producing unbalanced brackets.
            let i = rng.below(cs.len());
            let len = 1 + rng.below(16.min(cs.len() - i));
            let span: Vec<char> = cs[i..i + len].to_vec();
            let at = rng.below(cs.len());
            for (k, c) in span.into_iter().enumerate() {
                cs.insert(at + k, c);
            }
        }
    }
    cs.into_iter().collect()
}

/// Every `.liss` source in the repo, as (label, text) — the mutation seed corpus.
pub fn seed_corpus(repo_root: &std::path::Path) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for base in ["examples", "conformance"] {
        collect(&repo_root.join(base), &mut out);
    }
    out.sort();
    out
}

fn collect(dir: &std::path::Path, out: &mut Vec<(String, String)>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            collect(&p, out);
        } else if p.extension().is_some_and(|x| x == "liss") {
            if let Ok(s) = std::fs::read_to_string(&p) {
                out.push((p.display().to_string(), s));
            }
        }
    }
}
```

Add `pub mod mutate;` to `support/mod.rs`.

- [ ] **Step 2: Add the fuzz test**

Append to `crates/lingua-serpentis/tests/robustness.rs`:

```rust
fn env_usize(key: &str, default: usize) -> usize {
    std::env::var(key).ok().and_then(|v| v.parse().ok()).unwrap_or(default)
}

#[test]
fn no_mutation_of_any_known_program_escapes_the_error_model() {
    use support::mutate::{mutate, seed_corpus, Rng};

    let seeds = seed_corpus(&support::repo_root());
    assert!(!seeds.is_empty(), "no .liss files found to mutate");

    let cases = env_usize("LISS_FUZZ_CASES", 300);
    let mut rng = Rng::new(env_usize("LISS_FUZZ_SEED", 0x5EED) as u64);

    let mut violations = Vec::new();
    for n in 0..cases {
        let (label, base) = &seeds[rng.below(seeds.len())];
        let src = mutate(base, &mut rng);
        match judge_source(&format!("mutatio {n} ex {label}"), &src) {
            Verdict::Acceptable => {}
            Verdict::Violation { rule, .. } if RECORDED_RULES.contains(&rule) => {}
            Verdict::Violation { rule, detail } => {
                violations.push(format!("--- {rule}: {detail}\n--- source ---\n{src}\n"));
                if violations.len() >= 5 {
                    break; // five repros is plenty; stop burning time
                }
            }
        }
    }
    assert!(
        violations.is_empty(),
        "{} new robustness violation(s):\n{}",
        violations.len(),
        violations.join("\n")
    );
}
```

- [ ] **Step 3: Run at the default budget**

Run: `cargo test --test robustness no_mutation -- --nocapture`
Expected: PASS in well under a minute.

- [ ] **Step 4: Run a soak and triage what it finds**

Run: `LISS_FUZZ_CASES=5000 cargo test --test robustness no_mutation -- --nocapture`

Every reported violation is a **new defect**. For each: minimise the source by hand,
add a `defectus/` fixture (or a `pathologies()` entry when it needs a subprocess), add
a register entry with the next free `D-NN`, and add the ID to `judge`'s allowlist so the
default run is green. Do **not** fix the interpreter.

- [ ] **Step 5: Commit**

```bash
git add crates/lingua-serpentis/tests/support/mutate.rs \
        crates/lingua-serpentis/tests/robustness.rs \
        crates/lingua-serpentis/tests/support/mod.rs \
        conformance/ docs/defectus/
git commit -m "test(robustness): deterministic mutation fuzzing over every .liss

Mutates examples and fixtures at the char level so the input stays valid
UTF-8 and the language is under test rather than read_to_string.
LISS_FUZZ_CASES and LISS_FUZZ_SEED drive a soak."
```

---

### Task 12: Property-based generation

Mutation reaches the lexer and parser well but rarely produces a program that type-checks, so the checker and interpreter stay shallow. Generated well-formed programs reach them, and proptest shrinks a failure to a minimal repro.

**Files:**
- Modify: `crates/lingua-serpentis/Cargo.toml` (add `proptest`)
- Create: `crates/lingua-serpentis/tests/generative.rs`

**Interfaces:**
- Consumes: `support::diag::judge`, `support::proc::{run_binary, temp_dir}`.
- Produces: nothing consumed later.

- [ ] **Step 1: Add the dependency**

In `crates/lingua-serpentis/Cargo.toml`, extend `[dev-dependencies]`:

```toml
proptest = "1"
```

- [ ] **Step 2: Write the generator and the property**

Create `crates/lingua-serpentis/tests/generative.rs`:

```rust
//! Generated well-formed programs. Mutation rarely gets past the parser, so
//! this is what exercises the checker and interpreter in depth. proptest
//! shrinks a failure to a minimal repro, which a hand-rolled generator cannot.
mod support;

use proptest::prelude::*;
use std::ffi::OsStr;
use std::time::Duration;
use support::diag::{judge, Verdict, RECORDED_RULES};
use support::proc::{run_binary, temp_dir};

const T: Duration = Duration::from_secs(10);

/// A generated expression, rendered to source. Only `ni`-typed shapes, so the
/// programs mostly type-check and reach the interpreter.
#[derive(Debug, Clone)]
enum Expr {
    Num(u32),
    Var(&'static str),
    Bin(Box<Expr>, &'static str, Box<Expr>),
    Neg(Box<Expr>),
}

const VARS: [&str; 3] = ["a", "b", "c"];
const OPS: [&str; 5] = ["+", "-", "*", "/", "%"];

fn roman(mut n: u32) -> String {
    if n == 0 {
        return "nullus".to_string();
    }
    const TABULA: [(u32, &str); 13] = [
        (1000, "M"), (900, "CM"), (500, "D"), (400, "CD"), (100, "C"), (90, "XC"),
        (50, "L"), (40, "XL"), (10, "X"), (9, "IX"), (5, "V"), (4, "IV"), (1, "I"),
    ];
    let mut s = String::from("°");
    for (v, sym) in TABULA {
        while n >= v {
            s.push_str(sym);
            n -= v;
        }
    }
    s
}

impl Expr {
    fn render(&self) -> String {
        match self {
            Expr::Num(n) => roman(*n),
            Expr::Var(v) => v.to_string(),
            Expr::Bin(l, op, r) => format!("({} {} {})", l.render(), op, r.render()),
            Expr::Neg(e) => format!("(-{})", e.render()),
        }
    }
}

fn expr() -> impl Strategy<Value = Expr> {
    let leaf = prop_oneof![
        (0u32..200).prop_map(Expr::Num),
        proptest::sample::select(VARS.as_slice()).prop_map(Expr::Var),
    ];
    leaf.prop_recursive(4, 24, 2, |inner| {
        prop_oneof![
            (inner.clone(), proptest::sample::select(OPS.as_slice()), inner.clone())
                .prop_map(|(l, op, r)| Expr::Bin(Box::new(l), op, Box::new(r))),
            inner.prop_map(|e| Expr::Neg(Box::new(e))),
        ]
    })
}

/// Wrap an expression in a complete, declaration-only program.
fn program(e: &Expr) -> String {
    format!(
        "munus computa capit (ni a, ni b, ni c) reddit ni {{\n  redde {};\n}}\n\
         munus initium reddit ni {{\n  scribe(computa(°VII, °III, °II));\n  redde nullus;\n}}\n",
        e.render()
    )
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 200, max_shrink_iters: 2000, ..ProptestConfig::default() })]

    #[test]
    fn generated_programs_never_escape_the_error_model(e in expr()) {
        let src = program(&e);
        let d = temp_dir("liss-gen");
        let p = d.write("fons.liss", &src);
        let outcome = run_binary(&[OsStr::new(p.to_str().unwrap())], T);
        let verdict = judge(&outcome, &[("fons.liss".to_string(), src.clone())]);
        match verdict {
            // Divide/modulo by zero is a legitimate, well-formed error, so
            // `judge` already calls it Acceptable.
            Verdict::Acceptable => {}
            Verdict::Violation { rule, .. } if RECORDED_RULES.contains(&rule) => {}
            Verdict::Violation { rule, detail } => {
                prop_assert!(false, "{rule}: {detail}\n--- source ---\n{src}");
            }
        }
    }
}
```

- [ ] **Step 3: Run it**

Run: `cargo test --test generative -- --nocapture`
Expected: PASS, 200 generated cases. Runtime is dominated by process spawns; expect
tens of seconds.

- [ ] **Step 4: Triage any counterexample**

proptest writes a regression file at
`crates/lingua-serpentis/tests/generative.proptest-regressions`. **Commit it** — it turns
a found counterexample into a permanent test case. For each counterexample, add a
register entry and a `defectus/` fixture as in Task 11 Step 4, and do not fix the
interpreter.

- [ ] **Step 5: Commit**

```bash
git add crates/lingua-serpentis/Cargo.toml crates/lingua-serpentis/tests/generative.rs
git add crates/lingua-serpentis/tests/generative.proptest-regressions 2>/dev/null || true
git commit -m "test(robustness): property-based generation of well-formed programs

Mutation rarely gets past the parser; generated programs reach the checker
and interpreter, and proptest shrinks any counterexample to a minimal repro."
```

---

### Task 13: The catalogue ratchet

Without this, the suite is a snapshot of one afternoon's audit. With it, a new diagnostic added without a fixture fails review.

**Files:**
- Modify: `crates/lingua-serpentis/Cargo.toml` (add `insta`)
- Create: `crates/lingua-serpentis/tests/catalogus.rs`

**Interfaces:**
- Consumes: `support::{repo_root, discover, run_fixture}`.
- Produces: `crates/lingua-serpentis/tests/snapshots/catalogus__catalogus.snap`.

- [ ] **Step 1: Add the dependency**

Extend `[dev-dependencies]` in `crates/lingua-serpentis/Cargo.toml`:

```toml
insta = "1"
```

- [ ] **Step 2: Write the extractor and the snapshot**

Create `crates/lingua-serpentis/tests/catalogus.rs`:

```rust
//! The catalogue of every diagnostic the source can emit, and whether a
//! fixture produces it. A new message added without a fixture changes the
//! snapshot and fails review — which is what keeps this suite from rotting.
mod support;

use std::path::Path;

/// Markers that introduce a user-facing message. `ProgramError::in_file` is
/// deliberately absent: it re-wraps an existing error rather than producing one.
const MARKERS: [&str; 5] = [
    "LinguaError {",
    ".err(",
    "ProgramError::structural(",
    "Warning {",
    "eprintln!(",
];

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Site {
    file: String,
    line: usize,
    /// The format string as written, e.g. `variabilis ignota: '{}'`.
    pattern: String,
}

/// Source with its `#[cfg(test)]` module removed. Every file in this workspace
/// has at most one, and it always runs to EOF, so truncating is exact.
fn production_source(path: &Path) -> String {
    let text = std::fs::read_to_string(path).unwrap();
    match text.find("\n#[cfg(test)]\n") {
        Some(i) => text[..i + 1].to_string(),
        None => text,
    }
}

/// The first string literal after `from`, within `window` bytes and before the
/// next marker. Returns None when the site has no literal of its own.
fn literal_after(src: &str, from: usize, window: usize) -> Option<String> {
    let end = src.len().min(from + window);
    let hay = &src[from..end];
    let stop = MARKERS.iter().filter_map(|m| hay.find(m)).min().unwrap_or(hay.len());
    let hay = &hay[..stop];
    let start = hay.find('"')? + 1;
    let rest = &hay[start..];
    let mut out = String::new();
    let mut chars = rest.chars();
    while let Some(c) = chars.next() {
        match c {
            '"' => return Some(out),
            '\\' => {
                if let Some(n) = chars.next() {
                    out.push(match n { 'n' => '\n', 't' => '\t', other => other });
                }
            }
            _ => out.push(c),
        }
    }
    None
}

fn extract_sites() -> Vec<Site> {
    let root = support::repo_root();
    let mut files: Vec<_> = ["lingua-serpentis", "latinitas"]
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
        let src = production_source(&path);
        let rel = path.strip_prefix(&root).unwrap().display().to_string();
        for marker in MARKERS {
            let mut at = 0;
            while let Some(i) = src[at..].find(marker) {
                let abs = at + i;
                let line = src[..abs].lines().count();
                let pattern = literal_after(&src, abs + marker.len(), 300)
                    .unwrap_or_else(|| "UNPARSED".to_string());
                sites.push(Site { file: rel.clone(), line, pattern });
                at = abs + marker.len();
            }
        }
    }
    sites.sort();
    sites.dedup();
    sites
}

/// Does `observed` match the format string `pattern`? Compares the literal
/// segments between `{...}` holes, in order.
fn matches(pattern: &str, observed: &str) -> bool {
    let mut segments = Vec::new();
    let mut rest = pattern;
    loop {
        match rest.find('{') {
            None => {
                segments.push(rest);
                break;
            }
            Some(i) => {
                segments.push(&rest[..i]);
                match rest[i..].find('}') {
                    None => break,
                    Some(j) => rest = &rest[i + j + 1..],
                }
            }
        }
    }
    let mut cursor = 0;
    for seg in segments {
        if seg.is_empty() {
            continue;
        }
        match observed[cursor..].find(seg) {
            Some(k) => cursor += k + seg.len(),
            None => return false,
        }
    }
    true
}

/// Every message the corpus actually produces, from both streams.
fn observed_messages() -> Vec<String> {
    let mut out = Vec::new();
    for f in support::discover() {
        let r = support::run_fixture(&f);
        for line in r.stderr.lines() {
            let msg = line
                .split_once(": ")
                .map(|(_, m)| m.to_string())
                .unwrap_or_else(|| line.to_string());
            out.push(msg);
        }
    }
    out.sort();
    out.dedup();
    out
}

#[test]
fn catalogus() {
    let sites = extract_sites();
    let observed = observed_messages();

    let mut covered = 0;
    let mut uncovered = 0;
    let mut unparsed = 0;
    let mut lines = Vec::new();
    for s in &sites {
        let status = if s.pattern == "UNPARSED" {
            unparsed += 1;
            "UNPARSED".to_string()
        } else if observed.iter().any(|o| matches(&s.pattern, o)) {
            covered += 1;
            "fixtured".to_string()
        } else {
            uncovered += 1;
            "UNCOVERED".to_string()
        };
        lines.push(format!("{}:{}  {}  → {status}", s.file, s.line, s.pattern));
    }

    let report = format!(
        "{} sites · {covered} fixtured · {uncovered} uncovered · {unparsed} unparsed\n\n{}",
        sites.len(),
        lines.join("\n")
    );
    insta::assert_snapshot!("catalogus", report);
}
```

- [ ] **Step 3: Create the snapshot**

Run: `INSTA_UPDATE=always cargo test --test catalogus`
Then read the generated
`crates/lingua-serpentis/tests/snapshots/catalogus__catalogus.snap` end to end.

This file is the deliverable. Check three things:
1. The `UNPARSED` count is small. Each one is a site whose message the grep could not
   find — e.g. `lexer.rs:40`'s `LinguaError { message: m, line }`, which forwards a
   message built elsewhere. If the count is large, widen the window in `literal_after`.
2. Every `UNCOVERED` line is either genuinely unreachable (record it in the register as
   dead) or a fixture you still owe. Add fixtures for the reachable ones and re-run.
3. No site is missing. Cross-check the total against
   `grep -c` for each marker in the production sources.

- [ ] **Step 4: Verify the ratchet actually catches a new message**

Run:
```bash
# temporarily add a message with no fixture
sed -i 's|pub fn assemble|pub fn probatio_temporaria() -> ProgramError { ProgramError::structural("probatio temporaria") }\n\npub fn assemble|' \
  crates/lingua-serpentis/src/program.rs
cargo test --test catalogus
```
Expected: FAIL, with the snapshot diff showing a new `→ UNCOVERED` line and the counts
changed.

Then revert:
```bash
git checkout crates/lingua-serpentis/src/program.rs
cargo test --test catalogus
```
Expected: PASS.

- [ ] **Step 5: Run the entire suite**

Run: `cargo test --workspace`
Expected: PASS — the 242 pre-existing tests, the 4 from Task 1, the conformance corpus,
the CLI suite, robustness, generative, and the catalogue. Two `#[ignore]`d CLI tests are
reported as ignored.

- [ ] **Step 6: Commit**

```bash
git add crates/lingua-serpentis/Cargo.toml crates/lingua-serpentis/tests/catalogus.rs \
        crates/lingua-serpentis/tests/snapshots/ conformance/ docs/defectus/
git commit -m "test(catalogus): snapshot every diagnostic and whether a fixture hits it

Turns the audit into a standing ratchet: a new message added without a
fixture changes the snapshot and fails review. UNCOVERED and DEAD are legal
states, but visible ones — the header count is what the project chooses to
live with."
```

---

## Final Verification

- [ ] `cargo test --workspace` passes.
- [ ] `cargo test --test cli -- --ignored` shows exactly the two crash tests failing, naming D-04 and D-06.
- [ ] `LISS_FUZZ_CASES=5000 cargo test --test robustness` passes, or every violation it reports has been added to the register.
- [ ] The catalogue snapshot header shows every site fixtured, `UNCOVERED` with a reason, or `UNPARSED`.
- [ ] `docs/defectus/2026-08-03-index.md` has an entry for every defect ID referenced anywhere in the tests — grep for `D-[0-9]` across `crates/lingua-serpentis/tests/` and `conformance/` and check each against the register.
- [ ] `git diff master --stat -- crates/lingua-serpentis/src/` shows **only** `lib.rs` and `main.rs`, and only the Task 1 extraction.
