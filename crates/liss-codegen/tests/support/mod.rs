//! What every "compile it and run it" test needs: the front end, the linker,
//! and a temporary directory to `execve` out of.
//!
//! Shared by `salve.rs` and `scalares.rs` so the pipeline is spelled once. The
//! interesting helper is [`par_est`], which compiles and runs one source and
//! requires its three streams to match a **recorded** expectation.
//!
//! # The expectations here were recorded from the interpreter, and when
//!
//! Until Task 19 this was a *differential*: `par_est` ran each source both ways
//! and required the two `Rendered` values equal, so the oracle was the live
//! interpreter the corpus is blessed against rather than a hand-copied string.
//! That is what made one test per operator cheap enough to write, and it is
//! why forty-seven of them exist.
//!
//! Task 19 deletes the interpreter (spec §3.1), so on **2026-08-08**, with both
//! backends still standing, every one of those forty-seven answers was captured
//! from the interpreter and written into its call site. The oracle did not
//! change; it stopped being re-derived on every run and became a recorded fact.
//! That is spec §10.4's discipline — *"everything that uses the interpreter as a
//! source of truth happens while it still exists"* — applied to this file
//! rather than to the corpus.
//!
//! **So no expectation below was ever produced by the compiler.** Not one was
//! blessed from the backend under test, which is the property that would have
//! been silently lost by re-blessing these after the deletion, and the reason
//! the capture had to precede it. A later reader may treat every string here as
//! the tree-walker's own answer.

#![allow(dead_code)] // each test target uses a different subset

use liss_codegen::nexus::{Nexus, WildNexus};
use liss_codegen::target::Target;
use lingua_serpentis::ast::{ParsedFile, ParsedProgram};
use lingua_serpentis::checker::check_program_typed;
use lingua_serpentis::lexer::tokenize;
use lingua_serpentis::parser::parse_file;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The workspace root, found by walking up from this crate.
pub fn radix() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    while !p.join("conformance").is_dir() {
        assert!(p.pop(), "no `conformance/` above the manifest directory");
    }
    p
}

/// Lower one source file to the typed IR, exactly as the driver will.
pub fn ir(via: &Path, fons: &str) -> liss_ir::Programma {
    let tokens = tokenize(fons).expect("the fixture lexes");
    let ast = parse_file(&tokens).expect("the fixture parses");
    let mut prog = ParsedProgram {
        root: vec![ParsedFile { path: via.display().to_string(), ast }],
        packages: BTreeMap::new(),
    };
    check_program_typed(&mut prog).expect("the fixture type-checks")
}

/// The same, in **single-file mode**: `Func::file` is cleared, so a diagnostic
/// reads `Error in linea N:` rather than `Error in 'fons.liss', linea N:`.
///
/// Spec §4.1 gives this job to whoever constructs the `Programma`, because the
/// checker has no single-file signal and `liss run` inherits the child's stdio
/// and so cannot rewrite the sentence afterwards. Task 16 owns it in the CLI;
/// here it is what lets [`differentia`] compare against
/// `run_file_with_warnings`, which strips the file from its errors the same way.
pub fn ir_solum(fons: &str) -> liss_ir::Programma {
    let mut prog = ir(Path::new("fons.liss"), fons);
    for f in &mut prog.funcs {
        f.file = None;
    }
    prog
}

/// Lower a whole **program folder** — a root plus its packages — exactly as
/// `run_dir_with_warnings` does, but stopping at the typed IR.
///
/// `Func::file` is left alone here: a folder program's diagnostics name their
/// file (`Error in 'principale.liss', linea N:`), which is the difference
/// [`ir_solum`] exists to erase for single-file sources.
pub fn ir_scrinii(dir: &Path) -> liss_ir::Programma {
    ir_programmatis(lingua_serpentis::program::load_dir(dir).expect("the folder loads"))
}

/// Lower a **multi-file program written in the test** — root files and package
/// files named by their program-relative path, exactly as
/// `lingua_serpentis::run_program_with_warnings` takes them.
///
/// The disk-free twin of [`ir_scrinii`], and the reason cross-package codegen
/// tests need no fixture folder: `"principale.liss"` is a root file and
/// `"mathematica/summa.liss"` is in the `mathematica` package, which is the
/// whole of the folder rule (`program::assemble`). Sharing the *loaded* program
/// shape with [`ir_scrinii`] rather than the parse is what keeps the two from
/// drifting into different notions of what a package is.
pub fn ir_fasciculorum(scidae: &[(&str, &str)]) -> liss_ir::Programma {
    let files = scidae
        .iter()
        .map(|(via, fons)| lingua_serpentis::program::SourceFile {
            path: (*via).to_owned(),
            src: (*fons).to_owned(),
        })
        .collect();
    ir_programmatis(lingua_serpentis::program::assemble(files).expect("the program assembles"))
}

/// Parse and check one already-loaded program, stopping at the typed IR.
///
/// `Func::file` is left alone here: a folder program's diagnostics name their
/// file (`Error in 'principale.liss', linea N:`), which is the difference
/// [`ir_solum`] exists to erase for single-file sources.
fn ir_programmatis(prog: lingua_serpentis::program::Program) -> liss_ir::Programma {
    let mut parsed = parsata(prog);
    check_program_typed(&mut parsed).expect("the fixture type-checks")
}

/// Lex and parse every file of a loaded program, stopping short of the checker
/// — which is exactly where `lingua_serpentis::run_loaded` runs the Latinity
/// lint, and therefore the state [`monita`] needs.
fn parsata(prog: lingua_serpentis::program::Program) -> ParsedProgram {
    let uno = |f: lingua_serpentis::program::SourceFile| {
        let tokens = tokenize(&f.src).expect("the fixture lexes");
        let ast = parse_file(&tokens).expect("the fixture parses");
        ParsedFile { path: f.path, ast }
    };
    ParsedProgram {
        root: prog.root.into_iter().map(uno).collect(),
        packages: prog
            .packages
            .into_iter()
            .map(|(n, fs)| (n, fs.into_iter().map(uno).collect()))
            .collect(),
    }
}

/// **The checker's Latinity warnings for one conformance case, obtained
/// structurally** — the exact `Vec<Warning>` `run_loaded` would attach, from
/// the same `latinity::lint_program` call on the same `ParsedProgram`.
///
/// This exists because a compiled program never prints one of these, and the
/// blessed `nuntii` files record the interpreter's stderr, where there is only
/// one process and so only one stream. Spec §9.5 has the CLI write the Latinity
/// warnings itself and *then* spawn the child; the child's stderr carries its
/// own diagnostics and nothing else. [`casus_par_est`] therefore subtracts
/// exactly these lines from the expectation before comparing.
///
/// # Why this is not a `Monitum` prefix filter, which is what it used to be
///
/// Because **the runtime writes `Monitum` lines too, and they are the child's
/// own.** `liss-abi`'s `ingressus.rs` defines `MONITUM_PRAEFIXUM: &[u8] =
/// b"Monitum: "` and its `main` writes the D-10 truncated-exit-status note
/// through it at run time — `conformance/monita/status-exitus-redactus` is
/// precisely that line, and `munus initium reddit ni { redde °CCC; }` produces
/// it from a compiled image today. A filter keyed on the word would have
/// deleted a diagnostic the compiled program is *required* to emit, and the
/// earlier comment here claimed the opposite was exact. It was not.
///
/// The subtraction is now by identity rather than by shape: whatever
/// `lint_program` returns comes out, and everything else — the D-10 note
/// included — stays in the expectation and must be matched byte for byte.
///
/// The two loading modes have to be mirrored exactly, because the rendered
/// text differs. A **folder** program's warnings name their file
/// (`Monitum in 'principale.liss', linea 2: …`); a **single-source** program's
/// do not, because `run_file_with_warnings` parses it as `fons.liss` and then
/// clears `Warning::file` — which is the same distinction [`ir_solum`] draws
/// for `Func::file`.
fn monita(via: &Path) -> Vec<lingua_serpentis::Warning> {
    use lingua_serpentis::program::{Program, SourceFile};

    if via.is_dir() {
        let prog = lingua_serpentis::program::load_dir(via).expect("the folder loads");
        return lingua_serpentis::latinity::lint_program(&parsata(prog));
    }
    let src = std::fs::read_to_string(via)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", via.display()));
    let prog = Program {
        root: vec![SourceFile { path: "fons.liss".to_owned(), src }],
        packages: BTreeMap::new(),
    };
    let mut monita = lingua_serpentis::latinity::lint_program(&parsata(prog));
    for m in &mut monita {
        m.file = None;
    }
    monita
}

/// One conformance case's blessed expectation, read the way
/// `tests/support::Fixture` reads it and **byte for byte**: a missing stream
/// file is an empty stream, a missing `exitus` is status 0. Nothing is
/// filtered, normalised or re-line-ended here — see [`casus_par_est`] for the
/// one subtraction that happens, and why it is applied there rather than here.
pub fn expectatum(casus: &str) -> lingua_serpentis::Rendered {
    let dir = radix().join("conformance/currunt").join(casus);
    let lege = |n: &str| std::fs::read_to_string(dir.join(n)).unwrap_or_default();
    let exitus = lege("exitus");
    lingua_serpentis::Rendered {
        stdout: lege("emissio"),
        stderr: lege("nuntii"),
        code: exitus.trim().parse().unwrap_or(0),
    }
}

/// What a conformance case runs, by the corpus's own rule: an external program
/// named by `VIA`, otherwise the case's own `programma/`. A **directory** is a
/// program folder and a **file** is a single-source program, and the two lower
/// differently — see [`ir_solum`].
pub fn casus_via(casus: &str) -> PathBuf {
    let dir = radix().join("conformance/currunt").join(casus);
    let via = dir.join("VIA");
    match std::fs::read_to_string(&via) {
        Ok(rel) => radix().join(rel.trim()),
        Err(_) => dir.join("programma"),
    }
}

/// **The checkpoint assertion.** Compile and run one conformance case and
/// require it to match the blessed expectation byte for byte — stdout, stderr
/// and exit status.
///
/// Exit status is not an afterthought in a `memoria-*` case: it is the whole
/// assertion. Spec §7.5 has the runtime check the live-allocation count on the
/// success path and exit **1** with `memoria non liberata est` if it is
/// nonzero, and `abolere` clears a header's magic **volatilely** before freeing
/// so that a double free reports `defectus internus: memoria violata est`
/// rather than corrupting silently. (Volatilely as of this task: a plain store
/// there was dead-store-eliminated in every shipped build — D-27.) So "exit 0
/// with the blessed stdout" *is* "every allocation this program made was freed
/// exactly once", with one exception: a redundant free of a **slot** is a no-op,
/// because `Corpus::libera` leaves the slot null. Only a redundant free of a
/// temporary reaches the magic.
///
/// # The one subtraction, and it is by identity
///
/// The blessed `nuntii` is the *interpreter's* stderr, which begins with the
/// checker's Latinity warnings; a compiled image never writes one, because the
/// CLI writes them and then spawns the child (spec §9.5). So exactly those
/// lines — the ones [`monita`] derives from the same `lint_program` call the
/// interpreter makes, rendered the way `lingua_serpentis::render` renders them
/// — are stripped from the **front** of the expectation, which is where
/// `render` puts them.
///
/// `strip_prefix` rather than a filter, and the failure is the assertion's:
/// if those exact lines are not the beginning of the blessed stderr, nothing is
/// removed and the comparison fails loudly with both strings in view. Every
/// byte after them, the runtime's own `Monitum: ` note (D-10) included, stays
/// and must match.
pub fn casus_par_est(casus: &str) {
    let machina = Target::hospes().expect("the host is x86_64 or aarch64");
    let via = casus_via(casus);
    let prog = if via.is_dir() {
        ir_scrinii(&via)
    } else {
        let fons = std::fs::read_to_string(&via)
            .unwrap_or_else(|e| panic!("cannot read {}: {e}", via.display()));
        ir_solum(&fons)
    };
    let (code, stdout, stderr) = exsequere(&imago(&prog, machina), casus);
    let compilatum = lingua_serpentis::Rendered {
        stdout,
        stderr,
        code: code.expect("the image was not killed by a signal"),
    };

    let mut expectatum = expectatum(casus);
    let praefixum: String =
        monita(&via).iter().map(|m| format!("{m}\n")).collect();
    if let Some(reliquum) = expectatum.stderr.strip_prefix(&praefixum) {
        expectatum.stderr = reliquum.to_owned();
    }
    assert_eq!(
        expectatum,
        compilatum,
        "compiled `{casus}` does not match its blessed expectation, less the \
         {} Latinity warning(s) the CLI writes rather than the child",
        praefixum.lines().count(),
    );
}

/// **The ABI entry points one program calls, in the order codegen emitted
/// them** — read off the object's relocations, filtered to `interesting`.
///
/// This is the oracle for the ownership rules a program cannot observe. Two
/// frees in the wrong order leak nothing and print the same thing (§7.3.2);
/// a copy taken after the free it should precede reads memory the allocator
/// has not handed out again, so it usually returns the right bytes (§7.3.3).
/// Both are visible here and nowhere else in this suite.
///
/// **Only sound within one basic block.** Relocation offsets are `.text`
/// *layout* order, and Cranelift orders blocks for the branch predictor, not
/// for the reader — a loop body routinely lands after the code that follows the
/// loop. Callers must therefore use straight-line programs. Within a block,
/// layout order is emission order, because a call has side effects and nothing
/// reorders one.
pub fn vocationes(prog: &liss_ir::Programma, interesting: &[&str]) -> Vec<String> {
    use object::{Object, ObjectSection, ObjectSymbol};

    let octeti = liss_codegen::compila(prog, &Target::X86_64).expect("the program compiles");
    let fasciculus = object::File::parse(&octeti[..]).expect("a relocatable object");
    let mut ordo = Vec::new();
    for sectio in fasciculus.sections() {
        for (_, r) in sectio.relocations() {
            let object::RelocationTarget::Symbol(i) = r.target() else { continue };
            let Ok(s) = fasciculus.symbol_by_index(i) else { continue };
            let Ok(nomen) = s.name() else { continue };
            if interesting.contains(&nomen) {
                ordo.push(nomen.to_owned());
            }
        }
    }
    ordo
}

/// `ETXTBSY`, spelled out rather than pulled in: this crate's dev-dependencies
/// are the ones spec §11 allows, and `libc` is not among them. The number is
/// 26 on both Linux targets and the constant has never moved.
fn libc_etxtbsy() -> i32 {
    26
}

/// Write `imago` somewhere executable, run it, and give back
/// `(status, stdout, stderr)`.
///
/// The path is unlinked *after* the wait rather than before the exec, because
/// spec §9.5's `/proc/self/fd` dance belongs to the CLI task; here a test
/// directory that the harness removes is enough.
pub fn exsequere(imago: &[u8], nomen: &str) -> (Option<i32>, String, String) {
    let dir = std::env::temp_dir().join(format!("liss-{}-{nomen}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a scratch directory");
    let via = dir.join("imago");
    std::fs::write(&via, imago).expect("the image is written");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&via, std::fs::Permissions::from_mode(0o700))
            .expect("the image is made executable");
    }
    // **`ETXTBSY` is a race between test threads, not a failure of the image.**
    // libtest runs these in parallel; while this thread holds a write descriptor
    // on its own image, another thread's `Command` forks, and the child inherits
    // that descriptor for the moment between `fork` and `exec`. If this thread's
    // own `exec` lands inside that window the kernel refuses it, because a file
    // open for writing anywhere cannot be executed. It is transient by
    // construction — the child's `exec` closes the descriptor — so the answer is
    // to try again rather than to serialise every test in the file.
    let mut out = Command::new(&via).output();
    for _ in 0..64 {
        match &out {
            Err(e) if e.raw_os_error() == Some(libc_etxtbsy()) => {
                std::thread::sleep(std::time::Duration::from_millis(10));
                out = Command::new(&via).output();
            }
            _ => break,
        }
    }
    let out = out.expect("the image executes");
    let _ = std::fs::remove_dir_all(&dir);
    (
        out.status.code(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// Compile and link one `Programma` for the host.
pub fn imago(prog: &liss_ir::Programma, machina: Target) -> Vec<u8> {
    let objectum = liss_codegen::compila(prog, &machina).expect("the program compiles");
    WildNexus
        .iunge(&machina.runtime().artefacta(&objectum), "imago")
        .expect("the program links")
}

/// Compile, link and run one single-file source; `(status, stdout, stderr)`.
pub fn curre(fons: &str, nomen: &str) -> (Option<i32>, String, String) {
    let machina = Target::hospes().expect("the host is x86_64 or aarch64");
    exsequere(&imago(&ir_solum(fons), machina), nomen)
}

/// Compile, link and run one **multi-file** program; `(status, stdout, stderr)`.
pub fn curre_fasciculos(
    scidae: &[(&str, &str)],
    nomen: &str,
) -> (Option<i32>, String, String) {
    let machina = Target::hospes().expect("the host is x86_64 or aarch64");
    exsequere(&imago(&ir_fasciculorum(scidae), machina), nomen)
}

/// What a linked image made of one program.
fn compilatum(exitus: (Option<i32>, String, String)) -> lingua_serpentis::Rendered {
    let (code, stdout, stderr) = exitus;
    lingua_serpentis::Rendered {
        stdout,
        stderr,
        code: code.expect("the image was not killed by a signal"),
    }
}

/// The recorded answer, as a `Rendered`, so the comparison is one `assert_eq!`
/// over all three streams rather than three that can be forgotten separately.
fn exspectatum(emissio: &str, nuntii: &str, exitus: i32) -> lingua_serpentis::Rendered {
    lingua_serpentis::Rendered {
        stdout: emissio.to_owned(),
        stderr: nuntii.to_owned(),
        code: exitus,
    }
}

/// **Compile, link and run one source; require all three streams to match the
/// recorded expectation.**
///
/// `emissio` is the whole of stdout. A compiled image writes **no** Latinity
/// warnings — the CLI writes those and then spawns the child (spec §9.5) — so
/// stderr is empty and the status is 0 unless the program says otherwise; the
/// two shapes that do are [`par_est_erroris`] and [`par_est_plene`].
///
/// See this module's header for where every `emissio` below came from: the
/// interpreter, on 2026-08-08, while it still existed.
pub fn par_est(fons: &str, nomen: &str, emissio: &str) {
    par_est_plene(fons, nomen, emissio, "", 0)
}

/// [`par_est`] for a program that **fails at run time**: nothing on stdout, the
/// diagnostic on stderr, exit 1.
///
/// A shape of its own rather than a `par_est_plene` call with two constants,
/// because a runtime diagnostic is what most of `series.rs` is about and the
/// reader should be able to see at a glance which tests expect one.
pub fn par_est_erroris(fons: &str, nomen: &str, nuntii: &str) {
    par_est_plene(fons, nomen, "", nuntii, 1)
}

/// [`par_est`] with all three streams given: for the handful of programs whose
/// `initium` returns a status of its own.
pub fn par_est_plene(fons: &str, nomen: &str, emissio: &str, nuntii: &str, exitus: i32) {
    let c = compilatum(curre(fons, nomen));
    assert_eq!(
        exspectatum(emissio, nuntii, exitus),
        c,
        "the compiled program does not reproduce the recorded answer for:\n{fons}",
    );
}

/// [`par_est`] for a **multi-file** program.
pub fn par_est_fasciculorum(scidae: &[(&str, &str)], nomen: &str, emissio: &str) {
    par_est_fasciculorum_plene(scidae, nomen, emissio, "", 0)
}

/// [`par_est_erroris`] for a **multi-file** program. The diagnostic names the
/// package file it came from, which is the whole point of having one.
pub fn par_est_fasciculorum_erroris(scidae: &[(&str, &str)], nomen: &str, nuntii: &str) {
    par_est_fasciculorum_plene(scidae, nomen, "", nuntii, 1)
}

/// [`par_est_plene`] for a **multi-file** program.
pub fn par_est_fasciculorum_plene(
    scidae: &[(&str, &str)],
    nomen: &str,
    emissio: &str,
    nuntii: &str,
    exitus: i32,
) {
    let c = compilatum(curre_fasciculos(scidae, nomen));
    assert_eq!(
        exspectatum(emissio, nuntii, exitus),
        c,
        "the compiled program does not reproduce the recorded answer for {scidae:?}",
    );
}
