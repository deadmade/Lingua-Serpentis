//! The inflection point: a Lingua Serpentis program that is *executed as a
//! native binary* rather than walked by an interpreter.
//!
//! The program is `conformance/currunt/exemplum-salve` — `Const(Text)`,
//! `Print`, `Return(0)` and nothing else, the narrowest source that exercises
//! the whole pipeline: checker → typed IR → Cranelift → ELF object → libwild →
//! `execve`. The fixture is read from `conformance/`, not restated here, so the
//! compiled path is held to the same bytes the interpreted path is (spec
//! §10.1) — and the fixture itself is untouched, still passing interpreted.
//!
//! # Why the whole link is exercised and not just the object
//!
//! An object file that "looks right" proves nothing: Task 10's four embedded
//! blobs carry 80 general-dynamic TLS relocations on x86_64 that a static,
//! non-dynamic image requires the linker to relax to local-exec. libwild does
//! relax them — measured before a line of codegen was written — but that is a
//! property of the linker plus these blobs plus these flags, and only running
//! the produced image says so.

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
fn radix() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    while !p.join("conformance").is_dir() {
        assert!(p.pop(), "no `conformance/` above the manifest directory");
    }
    p
}

/// Lower one source file to the typed IR, exactly as the driver will.
fn ir(via: &Path, fons: &str) -> liss_ir::Programma {
    let tokens = tokenize(fons).expect("the fixture lexes");
    let ast = parse_file(&tokens).expect("the fixture parses");
    let mut prog = ParsedProgram {
        root: vec![ParsedFile { path: via.display().to_string(), ast }],
        packages: BTreeMap::new(),
    };
    check_program_typed(&mut prog).expect("the fixture type-checks")
}

/// Write `imago` somewhere executable, run it, and give back
/// `(status, stdout, stderr)`.
///
/// The path is unlinked *after* the wait rather than before the exec, because
/// spec §9.5's `/proc/self/fd` dance belongs to the CLI task; here a test
/// directory that the harness removes is enough.
fn exsequere(imago: &[u8], nomen: &str) -> (Option<i32>, String, String) {
    let dir = std::env::temp_dir().join(format!("liss-salve-{}-{nomen}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a scratch directory");
    let via = dir.join("imago");
    std::fs::write(&via, imago).expect("the image is written");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&via, std::fs::Permissions::from_mode(0o700))
            .expect("the image is made executable");
    }
    let out = Command::new(&via).output().expect("the image executes");
    let _ = std::fs::remove_dir_all(&dir);
    (
        out.status.code(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// **The task.** `exemplum-salve`, compiled, linked, run — and its stdout
/// compared against the same `emissio` the interpreted path is compared
/// against.
#[test]
fn exemplum_salve_becomes_a_static_binary_that_prints() {
    let Some(machina) = Target::hospes() else {
        eprintln!("not x86_64 or aarch64; nothing to run here");
        return;
    };
    let radix = radix();
    let fixtura = radix.join("conformance/currunt/exemplum-salve");
    let via = fixtura.join("VIA");
    let relativa = std::fs::read_to_string(&via).expect("VIA").trim().to_owned();
    let fons = std::fs::read_to_string(radix.join(&relativa)).expect("the source");
    let emissio = std::fs::read_to_string(fixtura.join("emissio")).expect("emissio");

    let prog = ir(Path::new(&relativa), &fons);
    let objectum = liss_codegen::compila(&prog, &machina).expect("the program compiles");
    let runtime = machina.runtime();
    let imago = WildNexus
        .iunge(&runtime.artefacta(&objectum), "imago")
        .expect("the program links");

    // A static, non-dynamic ELF of this architecture, and nothing else.
    let caput = &imago[..20.min(imago.len())];
    assert_eq!(&caput[..4], b"\x7fELF", "the image is not an ELF");
    assert_eq!(caput[4], 2, "the image is not ELF64");
    assert_eq!(caput[16], 2, "the image is not ET_EXEC — a static link must not be a PIE");
    assert_eq!(
        u16::from_le_bytes([caput[18], caput[19]]),
        machina.machina(),
        "the image is the wrong architecture",
    );

    let (status, stdout, stderr) = exsequere(&imago, "principale");
    assert_eq!(stdout, emissio, "stdout does not match the fixture's `emissio`");
    assert_eq!(stderr, "", "a clean run must say nothing on stderr");
    assert_eq!(status, Some(0), "`redde nullus` must exit 0");

    println!(
        "{}: objectum {} B, imago {} B",
        machina.triple(),
        objectum.len(),
        imago.len(),
    );
}

/// The two edges of the same three constructs, run rather than inspected: an
/// **empty** text literal, and a **non-zero** return.
///
/// Empty is the interesting one. `liss_text_new` and `liss_int_new` both take
/// a `(ptr, len)` pair and both document `len == 0` as not reading the pointer,
/// which is what lets `constans` emit a null rather than a blob —
/// `cranelift-object` gives no symbol to a zero-length data object, so a
/// literal `""` would otherwise be a pointer to nothing at all. The return
/// exercises the other half: `°CXXIII` is a real digit blob, three bytes of
/// little-endian `Roman`, and its journey out through `exit_status` is what
/// says the blob was read correctly.
#[test]
fn an_empty_literal_and_a_non_zero_return_survive_the_round_trip() {
    let Some(machina) = Target::hospes() else { return };
    let fons = "munus initium reddit ni {\n    scribe(\"\");\n    redde °CXXIII;\n}\n";
    let prog = ir(Path::new("principale.liss"), fons);
    let objectum = liss_codegen::compila(&prog, &machina).expect("the program compiles");
    let imago = WildNexus
        .iunge(&machina.runtime().artefacta(&objectum), "imago")
        .expect("the program links");

    let (status, stdout, stderr) = exsequere(&imago, "vacuum");
    assert_eq!(stdout, "\n", "`scribe(\"\")` prints one empty line");
    assert_eq!(stderr, "", "no leak, and nothing else to say");
    assert_eq!(status, Some(123), "`redde °CXXIII` exits 123");
}

/// The same program compiled for **both** targets, asserting only what can be
/// asserted without running it: that the object is this target's architecture.
///
/// It is the guard on spec §9.3's one codegen rule — `Signature::call_conv`
/// always comes from `module.isa().default_call_conv()`. A literal call
/// convention compiles and passes the test above on the host, and fails here
/// only if it is wrong for the *other* target; a wrong-but-valid literal would
/// still slip through, which is why the rule is also stated at the one place
/// signatures are built.
#[test]
fn both_targets_emit_an_object_of_their_own_architecture() {
    let radix = radix();
    let fons = std::fs::read_to_string(radix.join("examples/salve.liss")).expect("the source");
    let prog = ir(Path::new("examples/salve.liss"), &fons);

    for machina in Target::OMNES {
        let objectum = liss_codegen::compila(&prog, &machina).expect("the program compiles");
        assert_eq!(&objectum[..4], b"\x7fELF");
        assert_eq!(
            u16::from_le_bytes([objectum[18], objectum[19]]),
            machina.machina(),
            "{}: the object is the wrong architecture",
            machina.triple(),
        );
    }
}
