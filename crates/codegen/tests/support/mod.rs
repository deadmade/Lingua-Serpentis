//! The pipeline spelled once. Every recorded expectation here came from the
//! interpreter (spec §10.4) — see docs/internals/testing.md.

#![allow(dead_code)] // each test target uses a different subset

use codegen::nexus::{Nexus, WildNexus};
use codegen::target::Target;
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
pub fn ir(via: &Path, fons: &str) -> ir::Programma {
    let tokens = tokenize(fons).expect("the fixture lexes");
    let ast = parse_file(&tokens).expect("the fixture parses");
    let prog = ParsedProgram {
        root: vec![ParsedFile { path: via.display().to_string(), src: fons.into(), ast }],
        packages: BTreeMap::new(),
    };
    check_program_typed(&prog).expect("the fixture type-checks")
}

/// The same, in single-file mode: `Func::file` is cleared, so a diagnostic reads
/// `Error in linea N:` rather than `Error in 'fons.liss', linea N:` (spec §4.1).
pub fn ir_solum(fons: &str) -> ir::Programma {
    let mut prog = ir(Path::new("fons.liss"), fons);
    for f in &mut prog.funcs {
        f.file = None;
    }
    prog
}

/// Lower a whole program folder — a root plus its packages — to the typed IR.
pub fn ir_scrinii(dir: &Path) -> ir::Programma {
    ir_programmatis(lingua_serpentis::program::load_dir(dir).expect("the folder loads"))
}

/// The disk-free twin of [`ir_scrinii`]: root files and package files named by
/// their program-relative path.
pub fn ir_fasciculorum(scidae: &[(&str, &str)]) -> ir::Programma {
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
fn ir_programmatis(prog: lingua_serpentis::program::Program) -> ir::Programma {
    let parsed = parsata(prog);
    check_program_typed(&parsed).expect("the fixture type-checks")
}

/// Lex and parse every file of a loaded program, stopping short of the checker.
fn parsata(prog: lingua_serpentis::program::Program) -> ParsedProgram {
    let uno = |f: lingua_serpentis::program::SourceFile| {
        let tokens = tokenize(&f.src).expect("the fixture lexes");
        let ast = parse_file(&tokens).expect("the fixture parses");
        ParsedFile { path: f.path, src: f.src, ast }
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

/// The checker's Latinity warnings for one case, from the same
/// `latinity::lint_program` call the CLI makes (spec §9.5).
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

/// One conformance case's blessed expectation, byte for byte: a missing stream
/// file is an empty stream, a missing `exitus` is status 0.
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

/// What a conformance case runs: the program `VIA` names, otherwise the case's own
/// `programma/`. A directory is a program folder, a file a single-source program.
pub fn casus_via(casus: &str) -> PathBuf {
    let dir = radix().join("conformance/currunt").join(casus);
    let via = dir.join("VIA");
    match std::fs::read_to_string(&via) {
        Ok(rel) => radix().join(rel.trim()),
        Err(_) => dir.join("programma"),
    }
}

/// Compile and run one `conformance/currunt` case against its blessed expectation,
/// less the Latinity warnings [`monita`] subtracts by identity.
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

/// The ABI entry points one program calls, in emission order, read off the object's
/// relocations. Only sound within one basic block, so callers use straight-line programs.
pub fn vocationes(prog: &ir::Programma, interesting: &[&str]) -> Vec<String> {
    use object::{Object, ObjectSection, ObjectSymbol};

    let octeti = codegen::compila(prog, &Target::X86_64).expect("the program compiles");
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

/// `ETXTBSY`, spelled out: `libc` is not among this crate's dependencies (spec §11).
fn libc_etxtbsy() -> i32 {
    26
}

/// Write `imago` somewhere executable and run it; `(status, stdout, stderr)`.
pub fn exsequere(imago: &[u8], nomen: &str) -> (Option<i32>, String, String) {
    // A guard, not a bare path: the four `expect`s below leaked the directory on a panic.
    let dir = tempfile::Builder::new()
        .prefix(&format!("liss-{nomen}-"))
        .tempdir()
        .expect("a scratch directory");
    let via = dir.path().join("imago");
    std::fs::write(&via, imago).expect("the image is written");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&via, std::fs::Permissions::from_mode(0o700))
            .expect("the image is made executable");
    }
    // `ETXTBSY` is a race between test threads, transient by construction.
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
    (
        out.status.code(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// Compile and link one `Programma` for the host.
pub fn imago(prog: &ir::Programma, machina: Target) -> Vec<u8> {
    let objectum = codegen::compila(prog, &machina).expect("the program compiles");
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

/// The recorded answer as a `Rendered`, so one `assert_eq!` covers all three streams.
fn exspectatum(emissio: &str, nuntii: &str, exitus: i32) -> lingua_serpentis::Rendered {
    lingua_serpentis::Rendered {
        stdout: emissio.to_owned(),
        stderr: nuntii.to_owned(),
        code: exitus,
    }
}

/// Compile, link and run one source; all three streams must match the recorded answer.
pub fn par_est(fons: &str, nomen: &str, emissio: &str) {
    par_est_plene(fons, nomen, emissio, "", 0)
}

/// [`par_est`] for a program that fails at run time: nothing on stdout, the
/// diagnostic on stderr, exit 1.
pub fn par_est_erroris(fons: &str, nomen: &str, nuntii: &str) {
    par_est_plene(fons, nomen, "", nuntii, 1)
}

/// [`par_est`] with all three streams given.
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

/// [`par_est_erroris`] for a multi-file program.
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
