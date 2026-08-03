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
