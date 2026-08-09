//! aarch64 images run under `qemu-aarch64` — the second target's only execution anywhere.

mod support;

use codegen::target::Target;
use object::read::elf::{FileHeader, ProgramHeader};
use std::path::Path;
use std::process::Command;

const AEMULATOR: &str = "qemu-aarch64";

/// An aarch64 image from one single-file source.
fn imago_peregrina(fons: &str) -> Vec<u8> {
    support::imago(&support::ir_solum(fons), Target::Aarch64)
}

/// Run an aarch64 image under the emulator; `(status, stdout, stderr)`.
fn exsequere_aemulatum(imago: &[u8], nomen: &str) -> (Option<i32>, String, String) {
    exsequere_aemulatum_cum(imago, nomen, &[])
}

/// [`exsequere_aemulatum`], with environment variables for the emulator.
fn exsequere_aemulatum_cum(
    imago: &[u8],
    nomen: &str,
    ambitus: &[(&str, &str)],
) -> (Option<i32>, String, String) {
    let dir = std::env::temp_dir().join(format!("liss-a64-{}-{nomen}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a scratch directory");
    let via = dir.join("imago");
    std::fs::write(&via, imago).expect("the image is written");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&via, std::fs::Permissions::from_mode(0o700))
            .expect("the image is made executable");
    }
    // `current_dir` is containment: an uncaught guest signal drops a 36 MB core here.
    let mut cmd = Command::new(AEMULATOR);
    cmd.arg(&via).current_dir(&dir);
    for (clavis, valor) in ambitus {
        cmd.env(clavis, valor);
    }
    let exitus = cmd.output();
    let _ = std::fs::remove_dir_all(&dir);
    let exitus = exitus.unwrap_or_else(|e| {
        panic!(
            "`{AEMULATOR}` did not run ({e}). It is in the dev shell (Task 1); \
             outside it, the second target can be built but never executed, and \
             a skipped cross-architecture test is worse than none"
        )
    });
    (
        exitus.status.code(),
        String::from_utf8_lossy(&exitus.stdout).into_owned(),
        String::from_utf8_lossy(&exitus.stderr).into_owned(),
    )
}

fn imago_est_statica_aarch64(imago: &[u8]) {
    let caput = object::elf::FileHeader64::<object::Endianness>::parse(imago)
        .expect("the image is an ELF64 file");
    let ordo = caput.endian().expect("a known endianness");
    assert_eq!(
        caput.e_machine(ordo).0,
        Target::Aarch64.machina(),
        "the image is not EM_AARCH64",
    );
    assert_eq!(
        caput.e_type(ordo),
        object::elf::ET_EXEC,
        "the image is not ET_EXEC — a static link must not be a PIE",
    );
    let capita = caput
        .program_headers(ordo, imago)
        .expect("the image has program headers");
    // An empty slice would satisfy every `!any` below vacuously.
    assert!(!capita.is_empty(), "the image has no program headers");
    for (genus, nomen, causa) in [
        (
            object::elf::PT_INTERP,
            "PT_INTERP",
            "it wants a dynamic loader",
        ),
        (
            object::elf::PT_DYNAMIC,
            "PT_DYNAMIC",
            "it carries a dynamic section",
        ),
    ] {
        assert!(
            !capita.iter().any(|c| c.p_type(ordo) == genus),
            "the image has a {nomen}: {causa}, and spec §2 says it runs on any \
             Linux of its architecture with nothing else installed",
        );
    }
}

#[test]
fn exemplum_salve_currit_sub_aemulatore() {
    let radix = support::radix();
    let fixtura = radix.join("conformance/currunt/exemplum-salve");
    let relativa = std::fs::read_to_string(fixtura.join("VIA"))
        .expect("VIA")
        .trim()
        .to_owned();
    let fons = std::fs::read_to_string(radix.join(&relativa)).expect("the source");
    let emissio = std::fs::read_to_string(fixtura.join("emissio")).expect("emissio");

    let prog = support::ir(Path::new(&relativa), &fons);
    let imago = support::imago(&prog, Target::Aarch64);
    imago_est_statica_aarch64(&imago);

    let (status, stdout, stderr) = exsequere_aemulatum(&imago, "salve");
    assert_eq!(
        stdout, emissio,
        "stdout does not match the fixture's `emissio`"
    );
    assert_eq!(stderr, "", "a clean run must say nothing on stderr");
    assert_eq!(status, Some(0), "`redde nullus` must exit 0");

    println!("{}: imago {} B", Target::Aarch64.triple(), imago.len());
}

/// D-10, on aarch64.
#[test]
fn nota_de_statu_redacto_transit_aemulationem() {
    let fixtura = support::radix().join("conformance/monita/status-exitus-redactus");
    let via = fixtura.join("programma");
    let nuntii = std::fs::read_to_string(fixtura.join("nuntii")).expect("nuntii");
    let exitus: i32 = std::fs::read_to_string(fixtura.join("exitus"))
        .expect("exitus")
        .trim()
        .parse()
        .expect("a status");

    let imago = support::imago(&support::ir_scrinii(&via), Target::Aarch64);
    let (status, stdout, stderr) = exsequere_aemulatum(&imago, "redactus");
    assert_eq!(stdout, "", "the program prints nothing");
    assert_eq!(
        stderr, nuntii,
        "the D-10 note did not survive the round trip"
    );
    assert_eq!(status, Some(exitus), "`redde °CCC` exits 44");
}

/// `conformance/errores/acervus-exhaustus`'s message, restated: the only place that holds it on aarch64.
#[test]
fn acervus_exhaustus_sub_aemulatore() {
    let imago = imago_peregrina(
        "munus profundum capit (ni n) reddit ni {\n    \
             redde profundum(n + °I);\n}\n\
         munus initium reddit ni {\n    \
             redde profundum(°I);\n}\n",
    );
    let (status, stdout, stderr) = exsequere_aemulatum(&imago, "acervus");
    assert_eq!(
        stderr, "erratum: acervus vocationum exhaustus est\n",
        "the guard page fault must become a Latin diagnostic, not a signal",
    );
    assert_eq!(
        status,
        Some(1),
        "a signalled process is a `no abort` violation — D-04's shape, and \
         spec §8's whole subject",
    );
    assert_eq!(stdout, "", "nothing was printed before the recursion");
}

/// The negative control for [`acervus_exhaustus_sub_aemulatore`].
#[test]
fn acervus_displacitus_est_defectus_internus() {
    let imago = imago_peregrina(
        "munus profundum capit (ni n) reddit ni {\n    \
             redde profundum(n + °I);\n}\n\
         munus initium reddit ni {\n    \
             redde profundum(°I);\n}\n",
    );
    let (status, stdout, stderr) =
        exsequere_aemulatum_cum(&imago, "displacitus", &[("QEMU_STACK_SIZE", "1048576")]);
    assert_eq!(
        stderr, "erratum: defectus internus: memoria violata est: signum\n",
        "with the guest stack displaced from the derived bounds, the fault is \
         outside the band and must take the discriminator's other branch. The \
         same sentence as `acervus_exhaustus_sub_aemulatore` here would mean \
         the bounds are unknown and the band is never consulted",
    );
    assert_eq!(status, Some(1), "both branches exit 1 (spec §8.2)");
    assert_eq!(stdout, "", "nothing was printed before the recursion");
}

/// `conformance/currunt/memoria-adde-in-ansa`, the corpus's densest allocation case.
#[test]
fn memoria_liberatur_sub_aemulatore() {
    const CASUS: &str = "memoria-adde-in-ansa";
    let expectatum = support::expectatum(CASUS);
    let imago = support::imago(
        &support::ir_scrinii(&support::casus_via(CASUS)),
        Target::Aarch64,
    );
    let (status, stdout, stderr) = exsequere_aemulatum(&imago, CASUS);
    assert_eq!(stdout, expectatum.stdout, "stdout does not match `emissio`");
    assert_eq!(stderr, "", "no leak, and nothing else to say");
    assert_eq!(
        status,
        Some(0),
        "exit 0 is the live-allocation count at zero (spec §7.5)",
    );
}
