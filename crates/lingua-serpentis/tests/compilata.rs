//! **The differential.** Every case under `conformance/` run *both* ways — the
//! interpreter in process, the compiler through `liss run` — with the two
//! `Rendered` values required equal, byte for byte, on all three of stdout,
//! stderr and exit status.
//!
//! This is spec §10.1's property made checkable: "the compiled path reproduces
//! the corpus" stops being an assertion and becomes a thing `cargo test` knows.
//! `conformance.rs` already pins the interpreter against the blessed files, so
//! equality here is what carries the blessing across to the compiler without a
//! single new expectation file.
//!
//! # Scaffolding, with a defined lifetime
//!
//! Task 19 deletes this file along with the interpreter, at which point
//! `conformance.rs` inherits the job unchanged — it calls `support::run_fixture`,
//! and that is the one function that switches from interpreting to compiling.
//! Nothing here is meant to outlive the second backend, which is why it mirrors
//! `conformance.rs`'s shape exactly and reuses `support::discover`,
//! `Fixture::target`, `Fixture::expected` and `support::run_fixture` as they
//! stand rather than growing a corpus notion of its own.
//!
//! # The compiled side goes through `liss run`, and nothing is subtracted
//!
//! Not the raw image. **Latinity warnings are a front-end product**: `liss run`
//! writes them from the *parent*, before it spawns the child (spec §9.5), so
//! the bytes a caller sees are the parent's warnings followed by the child's own
//! diagnostics — which is exactly the single stream the blessed `nuntii` files
//! record, because the interpreter had only one process to write them from.
//!
//! `liss-codegen/tests/support`'s `casus_par_est` strips those warnings from the
//! *expectation*, and its `interpretatum` clears them structurally, because a
//! bare image has no parent to emit them. Both are right for a raw-image
//! comparison and both would be **wrong here**: a compiled path that silently
//! dropped its warnings would pass, and catching precisely that is why this file
//! exists. So: no filter, no normalisation, no subtraction, on either side.
//!
//! **`Monitum` is not a front-end-only prefix**, which is the trap a filter
//! would fall into even if one were wanted. `liss-abi`'s `ingressus.rs` writes
//! `Monitum: ` at **run time** for D-10's truncated-exit-status note, and
//! `conformance/monita/status-exitus-redactus` is that line and nothing else.
//! It is in scope here like every other fixture.
//!
//! # Why the two processes' streams stay in order
//!
//! The parent's stderr and the child's are the *same open file description* —
//! `Stdio::from(File)` hands the child a dup of the handle, so the two share one
//! offset — and the parent flushes before it spawns. Captured to a file rather
//! than a pipe for the same reason `support::proc` does it: a fixture that
//! outruns a pipe buffer would deadlock a harness that waits before it reads.
mod support;

use libtest_mimic::{Arguments, Failed, Trial};
use lingua_serpentis::Rendered;
use std::fs::File;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// The compiling driver, built by cargo because this test names it.
const LISS: &str = env!("CARGO_BIN_EXE_liss");

/// How long one fixture may take under `liss run` before it is called a
/// divergence rather than waited on.
///
/// Every fixture terminates under the interpreter — `conformance.rs` would hang
/// otherwise — so a compiled run that does not terminate is a real finding, and
/// the only useful thing to do with it is *report* it. Without a deadline the
/// whole suite would hang instead, on a machine where nobody can see which case
/// did it. Generous, because it is a backstop and not a performance budget: the
/// slowest fixture in the corpus is two orders of magnitude inside it.
const MORA: Duration = Duration::from_secs(120);

/// How often the deadline is checked. Short enough that the loop adds nothing
/// measurable to a fixture that takes tens of milliseconds.
const PAUSA: Duration = Duration::from_millis(2);

/// A fixture the two backends are **meant** to disagree about, with both
/// answers pinned exactly.
///
/// A skip list would be the wrong shape. "These two need not agree" is a hole;
/// "the interpreter says exactly this and the compiler says exactly that" is a
/// second assertion, and it is the one that notices when either side moves.
struct Divergentia {
    /// The trial name, `<categoria>::<casus>`.
    casus: &'static str,
    /// Why the disagreement is by design, in one sentence.
    causa: &'static str,
    /// What `liss run` must print, byte for byte.
    emissio: &'static str,
    nuntii: &'static str,
    exitus: i32,
}

/// **One entry, and the milestone predicted it.**
///
/// Spec §3.3 removes the recursion limit: the interpreter caps at 64 frames
/// (`interp.rs`'s `VOCATIONES_MAXIMAE`) and raises `vocatio nimis alte
/// implicata` from the call's own line, while a compiled program recurses until
/// the guard page and §8's handler turns the `SIGSEGV` into
/// `acervus vocationum exhaustus est` with no line at all (§8.4). The two
/// sentences describe different events, and only one of them is the language's.
///
/// Spec §10.1 already names this as **the one expectation that changes** in the
/// whole milestone, and the plan hands the change to Task 19, which renames the
/// case to `errores/acervus-exhaustus` and rewrites its `nuntii` — at which
/// point this file is deleted and this entry with it. Until then the corpus is
/// blessed against the interpreter, so the compiled answer has nowhere to live
/// except here.
///
/// **D-26 is not here, and must not become an entry.** A terminator in a `dum`
/// init clause diverges too, but it is deliberately not a fixture — one would
/// bless a backend's answer as the language's — so it never reaches this table.
/// It is excluded in `generative.rs`, where a generator could otherwise
/// synthesise it, and pinned by `regimen.rs`'s `caput_dum_terminatum_componitur`.
const DIVERGENTIAE: [Divergentia; 1] = [Divergentia {
    casus: "errores::vocatio-nimis-alte-implicata",
    causa: "spec §3.3 removes the interpreter's 64-frame cap; Task 19 re-blesses \
            this case as `errores/acervus-exhaustus`",
    emissio: "",
    nuntii: "Error: acervus vocationum exhaustus est\n",
    exitus: 1,
}];

impl Divergentia {
    /// Both sides, each against its own pinned answer.
    ///
    /// The interpreted side is checked against the blessed files rather than
    /// against a second copy of them written here: if the interpreter's answer
    /// ever moves, this entry stops describing anything real and says so, and
    /// it says so in the target whose job is to know that the two backends
    /// differ — not only in `conformance.rs`, which cannot see the other side.
    fn tenetur(
        &self,
        f: &support::Fixture,
        interpretatum: &Rendered,
        compilatum: &Rendered,
    ) -> Result<(), String> {
        let expectatum = f.expected();
        let mut s = String::new();
        for (label, want, got) in [
            ("interpretatum: emissio", &expectatum.stdout, &interpretatum.stdout),
            ("interpretatum: nuntii", &expectatum.stderr, &interpretatum.stderr),
            ("compilatum: emissio", &self.emissio.to_owned(), &compilatum.stdout),
            ("compilatum: nuntii", &self.nuntii.to_owned(), &compilatum.stderr),
        ] {
            if let Some(d) = support::diff(label, want, got) {
                s.push_str(&d);
            }
        }
        if expectatum.code != interpretatum.code {
            s.push_str(&format!(
                "--- interpretatum: exitus ---\n  expected: {}\n  actual:   {}\n",
                expectatum.code, interpretatum.code
            ));
        }
        if self.exitus != compilatum.code {
            s.push_str(&format!(
                "--- compilatum: exitus ---\n  expected: {}\n  actual:   {}\n",
                self.exitus, compilatum.code
            ));
        }
        if s.is_empty() {
            return Ok(());
        }
        s.push_str(&format!(
            "\nthis case is a **pinned divergence**: {}.\nBoth answers are \
             asserted, so a change to either one lands here. If the two \
             backends now agree, delete the entry from `DIVERGENTIAE`.\n",
            self.causa
        ));
        Err(s)
    }
}

fn main() {
    let args = Arguments::from_args();
    let fixtures = support::discover();
    assert!(
        !fixtures.is_empty(),
        "no fixtures found under {}",
        support::corpus_root().display()
    );
    // A pinned divergence that names nothing is an allowance nobody can see
    // expiring: Task 19 renames exactly this case, and a stale entry would then
    // sit here silently permitting a fixture that no longer exists.
    for d in &DIVERGENTIAE {
        assert!(
            fixtures.iter().any(|f| f.name == d.casus),
            "DIVERGENTIAE names `{}`, which is not a fixture",
            d.casus
        );
    }
    let trials: Vec<Trial> = fixtures
        .into_iter()
        .map(|f| Trial::test(f.name.clone(), move || differt(&f)))
        .collect();
    libtest_mimic::run(&args, trials).exit();
}

/// One fixture, both ways.
///
/// **`LISS_BLESS` is deliberately not honoured.** There is nothing here to
/// bless: this target compares two backends against each other and never reads
/// an expectation file to decide anything. A blessing run rewrites the corpus
/// from the interpreter, and this differential is exactly the check that must
/// still hold afterwards.
fn differt(f: &support::Fixture) -> Result<(), Failed> {
    let interpretatum = support::run_fixture(f);
    let compilatum = compilatum(f).map_err(Failed::from)?;
    match DIVERGENTIAE.iter().find(|d| d.casus == f.name) {
        Some(d) => d.tenetur(f, &interpretatum, &compilatum).map_err(Failed::from),
        None if interpretatum == compilatum => Ok(()),
        None => Err(Failed::from(relatio(f, &interpretatum, &compilatum))),
    }
}

/// Run one fixture through `liss run` and render what came back.
///
/// The argument is `Fixture::target()` verbatim — the corpus's own rule, a
/// `VIA`-named program or the case's `programma/` — so the compiler is handed
/// the same path the interpreter was, and single-file mode and folder mode are
/// chosen by `liss` on the same evidence `support::run_fixture` uses. That
/// matters: the two modes render diagnostics differently, and a harness that
/// picked for them would be able to hide the difference.
fn compilatum(f: &support::Fixture) -> Result<Rendered, String> {
    let via = f.target();
    let dir = support::proc::temp_dir("liss-compilata");
    let emissio = dir.path().join("emissio");
    let nuntii = dir.path().join("nuntii");

    let aperi = |p: &std::path::Path| {
        File::create(p).map_err(|e| format!("cannot create {}: {e}", p.display()))
    };
    let out = aperi(&emissio)?;
    let err = aperi(&nuntii)?;

    let mut child = Command::new(LISS)
        .arg("run")
        .arg(&via)
        .stdin(Stdio::null())
        .stdout(Stdio::from(out))
        .stderr(Stdio::from(err))
        .spawn()
        .map_err(|e| format!("cannot spawn `{LISS} run {}`: {e}", via.display()))?;

    let terminus = Instant::now() + MORA;
    let status = loop {
        match child.try_wait() {
            Ok(Some(s)) => break s,
            Ok(None) if Instant::now() >= terminus => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!(
                    "`liss run {}` was still running after {MORA:?}; the \
                     interpreter finished, so this is a divergence and not a \
                     slow machine",
                    via.display()
                ));
            }
            Ok(None) => std::thread::sleep(PAUSA),
            Err(e) => return Err(format!("wait on `liss run {}`: {e}", via.display())),
        }
    };

    let lege = |p: &std::path::Path| {
        std::fs::read(p)
            .map(|b| String::from_utf8_lossy(&b).into_owned())
            .map_err(|e| format!("cannot read {}: {e}", p.display()))
    };
    let stdout = lege(&emissio)?;
    let stderr = lege(&nuntii)?;

    // `code()` is `None` exactly when a signal ended the process, and under
    // `liss run` that process is the *parent*: the driver maps a signalled
    // child to `Error: programma abrupte cessavit` and exit 1 (§9.5), so this
    // arm means the compiler itself was killed. Reported rather than unwrapped,
    // because the whole value of this target is the list of what diverged.
    let code = status.code().ok_or_else(|| {
        format!(
            "`liss run {}` was killed by a signal; stdout {stdout:?}, stderr {stderr:?}",
            via.display()
        )
    })?;

    Ok(Rendered { stdout, stderr, code })
}

/// What a divergence looks like when someone has to act on it.
///
/// Three per-stream diffs, and then the one fact that says *which* backend
/// moved: whether the interpreter still reproduces the blessed expectation. If
/// it does — and `conformance.rs` says it does — the compiled column is the one
/// to read. Stating it here rather than leaving it to be inferred is what makes
/// a failing run of this target usable as Task 19's precondition list.
fn relatio(f: &support::Fixture, interpretatum: &Rendered, compilatum: &Rendered) -> String {
    let mut s = String::new();
    for (label, i, c) in [
        ("emissio (stdout)", &interpretatum.stdout, &compilatum.stdout),
        ("nuntii (stderr)", &interpretatum.stderr, &compilatum.stderr),
    ] {
        if let Some(d) = support::diff(label, i, c) {
            s.push_str(&d);
        }
    }
    if interpretatum.code != compilatum.code {
        s.push_str(&format!(
            "--- exitus ---\n  interpretatum: {}\n  compilatum:    {}\n",
            interpretatum.code, compilatum.code
        ));
    }
    let expectatum = f.expected();
    let par = interpretatum.stdout == expectatum.stdout
        && interpretatum.stderr == expectatum.stderr
        && interpretatum.code == expectatum.code;
    s.push_str(&format!(
        "\nthe `expected` column above is the **interpreter**, which {} its \
         blessed expectation.\nran: {LISS} run {}\n",
        if par { "still reproduces" } else { "no longer reproduces" },
        f.target().display(),
    ));
    s
}
