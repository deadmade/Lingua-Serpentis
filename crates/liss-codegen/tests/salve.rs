//! The whole pipeline: checker → typed IR → Cranelift → ELF object → libwild → `execve`.

mod support;

use liss_codegen::nexus::{Nexus, WildNexus};
use liss_codegen::target::Target;
use std::path::Path;
use support::{exsequere, ir, radix};

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

/// Guards spec §9.3: `Signature::call_conv` always comes from `module.isa().default_call_conv()`.
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
