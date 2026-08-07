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

/// **The differential.** One source, run interpreted and compiled, both
/// renderings returned as `(interpretatum, compilatum)`.
///
/// The interpreter's **warnings are cleared before rendering**, and that is not
/// a convenience: a compiled program never prints them. The checker's Latinity
/// warnings are written by the CLI, before it spawns the child (spec §9.5), so
/// the child's stderr carries its own diagnostics and nothing else. Clearing
/// them is what makes stderr comparable byte for byte rather than merely
/// comparable when the source happens to draw none — which would have quietly
/// excluded every program with a non-Latin identifier in it.
pub fn differentia(
    fons: &str,
    nomen: &str,
) -> (lingua_serpentis::Rendered, lingua_serpentis::Rendered) {
    let mut effectus = lingua_serpentis::run_file_with_warnings(fons);
    match &mut effectus {
        Ok((_, _, monita)) => monita.clear(),
        Err(e) => e.warnings.clear(),
    }
    let interpretatum = lingua_serpentis::render(effectus);
    let (code, stdout, stderr) = curre(fons, nomen);
    let compilatum = lingua_serpentis::Rendered {
        stdout,
        stderr,
        code: code.expect("the image was not killed by a signal"),
    };
    (interpretatum, compilatum)
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
