#![allow(dead_code)] // each test target uses a different subset

use lingua_serpentis::Rendered;
use std::path::{Path, PathBuf};

pub mod diag;
pub mod mutate;
pub mod proc;

/// The compiling driver, built by cargo because these targets name it.
pub const LISS: &str = env!("CARGO_BIN_EXE_liss");

/// How long one program may take under `liss run` before it is called a hang
/// rather than waited on.
///
/// Generous, because it is a backstop and not a performance budget: `liss run`
/// on the slowest thing in the corpus is two orders of magnitude inside it.
/// Without a deadline a program that does not terminate hangs the whole suite
/// instead, on a machine where nobody can see which one did it.
const MORA: std::time::Duration = std::time::Duration::from_secs(120);

/// How often the deadline is checked. Short enough to add nothing measurable to
/// a run that takes tens of milliseconds.
const PAUSA: std::time::Duration = std::time::Duration::from_millis(2);

/// **Compile and run one program through `liss run`, and render its two streams
/// and status the way a caller sees them.**
///
/// `target` is a path exactly as a user would type it: a `.liss` file is a
/// single-source program and a directory is a program folder, and `liss`
/// chooses between them on the same evidence. That distinction is load-bearing
/// rather than incidental — the two modes render diagnostics differently
/// (`Error in linea N:` against `Error in 'f.liss', linea N:`), so a harness
/// that decided for `liss` would be able to hide the difference.
///
/// # Why `liss run` and not the image
///
/// **Latinity warnings are a front-end product.** The parent writes them and
/// *then* spawns the child (spec §9.5), so the bytes a caller sees are the
/// parent's warnings followed by the child's own diagnostics — which is exactly
/// what the blessed `nuntii` files record, the interpreter having had only one
/// process to write from. A harness that ran the bare image would see none of
/// them and would have to re-bless three `nuntii` files empty, deleting the
/// corpus's only coverage of Latinity warnings in the name of a harness detail.
///
/// # Why the two processes' streams stay in order
///
/// The parent's stderr and the child's are the *same open file description* —
/// `Stdio::from(File)` hands the child a dup of the handle, so the two share one
/// offset — and the parent flushes before it spawns. Captured to files rather
/// than pipes because a program that outruns a pipe buffer would deadlock a
/// harness that waits before it reads, and some of these do so on purpose.
pub fn curre(target: &Path) -> Result<Rendered, String> {
    use std::process::{Command, Stdio};

    let dir = proc::temp_dir("liss-curre");
    let emissio = dir.path().join("emissio");
    let nuntii = dir.path().join("nuntii");
    let aperi = |p: &Path| {
        std::fs::File::create(p).map_err(|e| format!("cannot create {}: {e}", p.display()))
    };
    let out = aperi(&emissio)?;
    let err = aperi(&nuntii)?;

    let mut child = Command::new(LISS)
        .arg("run")
        .arg(target)
        .stdin(Stdio::null())
        .stdout(Stdio::from(out))
        .stderr(Stdio::from(err))
        .spawn()
        .map_err(|e| format!("cannot spawn `{LISS} run {}`: {e}", target.display()))?;

    let terminus = std::time::Instant::now() + MORA;
    let status = loop {
        match child.try_wait() {
            Ok(Some(s)) => break s,
            Ok(None) if std::time::Instant::now() >= terminus => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!(
                    "`liss run {}` was still running after {MORA:?}",
                    target.display()
                ));
            }
            Ok(None) => std::thread::sleep(PAUSA),
            Err(e) => return Err(format!("wait on `liss run {}`: {e}", target.display())),
        }
    };

    let lege = |p: &Path| {
        std::fs::read(p)
            .map(|b| String::from_utf8_lossy(&b).into_owned())
            .map_err(|e| format!("cannot read {}: {e}", p.display()))
    };
    let stdout = lege(&emissio)?;
    let stderr = lege(&nuntii)?;

    // `code()` is `None` exactly when a signal ended the process, and the
    // process here is the compiling **parent**: `liss` maps a signalled child to
    // `Error: programma abrupte cessavit` and exit 1 (§9.5), so this arm means
    // the compiler itself was killed. Reported rather than unwrapped, because a
    // panic here would name the harness instead of the failure.
    let code = status.code().ok_or_else(|| {
        format!(
            "`liss run {}` was killed by a signal; stdout {stdout:?}, stderr {stderr:?}",
            target.display()
        )
    })?;

    Ok(Rendered { stdout, stderr, code })
}

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

/// One stream's difference, with the first line that differs called out.
///
/// `pub` for `compilata.rs`, which compares two *backends* rather than a
/// backend against a blessing: its "expected" column is the interpreter and its
/// "actual" is the compiler, and it says so where it prints this. Sharing the
/// function rather than re-spelling it is what keeps the two targets from
/// drifting into different ideas of what a first difference is.
pub fn diff(label: &str, want: &str, got: &str) -> Option<String> {
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
