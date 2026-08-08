//! What every "compile it and run it" test needs: the front end, the linker,
//! and a temporary directory to `execve` out of.
//!
//! Shared by `salve.rs` and `scalares.rs` so the pipeline is spelled once. The
//! interesting helper is [`differentia`], which runs one source **both** ways —
//! interpreted and compiled — and returns the pair. That is spec §10.1's
//! property in miniature, three tasks before `tests/compilata.rs` makes it the
//! whole suite's job, and it is what makes a scalar test cheap enough to write
//! one per operator: the oracle is the interpreter the corpus is blessed
//! against, not a hand-copied expectation.

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
    let uno = |f: lingua_serpentis::program::SourceFile| {
        let tokens = tokenize(&f.src).expect("the fixture lexes");
        let ast = parse_file(&tokens).expect("the fixture parses");
        ParsedFile { path: f.path, ast }
    };
    let mut parsed = ParsedProgram {
        root: prog.root.into_iter().map(uno).collect(),
        packages: prog
            .packages
            .into_iter()
            .map(|(n, fs)| (n, fs.into_iter().map(uno).collect()))
            .collect(),
    };
    check_program_typed(&mut parsed).expect("the fixture type-checks")
}

/// One conformance case's blessed expectation, read the way
/// `tests/support::Fixture` reads it: a missing stream file is an empty stream,
/// a missing `exitus` is status 0.
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
    assert_eq!(
        expectatum(casus),
        compilatum,
        "compiled `{casus}` does not match its blessed expectation",
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

/// What the interpreter makes of one pipeline result, with its **warnings
/// cleared**.
///
/// Clearing them is not a convenience: a compiled program never prints them.
/// The checker's Latinity warnings are written by the CLI, before it spawns the
/// child (spec §9.5), so the child's stderr carries its own diagnostics and
/// nothing else. Clearing them is what makes stderr comparable byte for byte
/// rather than merely comparable when the source happens to draw none — which
/// would have quietly excluded every program with a non-Latin identifier in it,
/// and cross-package tests are exactly where those live.
fn interpretatum(
    mut effectus: Result<
        (String, u8, Vec<lingua_serpentis::Warning>),
        lingua_serpentis::ProgramError,
    >,
) -> lingua_serpentis::Rendered {
    match &mut effectus {
        Ok((_, _, monita)) => monita.clear(),
        Err(e) => e.warnings.clear(),
    }
    lingua_serpentis::render(effectus)
}

/// What a linked image made of the same program.
fn compilatum(exitus: (Option<i32>, String, String)) -> lingua_serpentis::Rendered {
    let (code, stdout, stderr) = exitus;
    lingua_serpentis::Rendered {
        stdout,
        stderr,
        code: code.expect("the image was not killed by a signal"),
    }
}

/// **The differential.** One source, run interpreted and compiled, both
/// renderings returned as `(interpretatum, compilatum)`.
pub fn differentia(
    fons: &str,
    nomen: &str,
) -> (lingua_serpentis::Rendered, lingua_serpentis::Rendered) {
    (
        interpretatum(lingua_serpentis::run_file_with_warnings(fons)),
        compilatum(curre(fons, nomen)),
    )
}

/// Assert that a source produces byte-identical output, stderr and status from
/// both backends.
pub fn par_est(fons: &str, nomen: &str) {
    let (i, c) = differentia(fons, nomen);
    assert_eq!(
        i, c,
        "the two backends disagree on:\n{fons}\ninterpretatum: {i:?}\ncompilatum:    {c:?}",
    );
}

/// [`par_est`] for a **multi-file** program.
///
/// **The interpreter is only an oracle 64 frames deep** (`interp.rs`'s
/// `VOCATIONES_MAXIMAE`), so nothing that recurses further may come through
/// here — that is `curre_fasciculos` and a written expectation, because the two
/// backends are *meant* to disagree about depth until the interpreter is gone.
pub fn par_est_fasciculorum(scidae: &[(&str, &str)], nomen: &str) {
    let i = interpretatum(lingua_serpentis::run_program_with_warnings(scidae));
    let c = compilatum(curre_fasciculos(scidae, nomen));
    assert_eq!(
        i, c,
        "the two backends disagree on {scidae:?}\n\
         interpretatum: {i:?}\ncompilatum:    {c:?}",
    );
}
