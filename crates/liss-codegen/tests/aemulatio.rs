//! **aarch64, run rather than merely built.**
//!
//! Spec §2 names two architectures. Every codegen test in this crate builds an
//! object for both — `regimen.rs`'s `regimen_utrique_machinae_componitur` and
//! `series.rs`'s `series_utrique_machinae_componuntur` exist for exactly that —
//! but until this file only the *host*'s image was ever executed. A verifier
//! that runs clean and a register allocator that finds an assignment say the
//! object is well formed; they say nothing about whether the program is right.
//!
//! Earlier tasks did run aarch64 images, by hand, and wrote the result into a
//! report: Task 10 ran a stand-in `liss_initium` under `qemu-aarch64`, Task 11
//! ran `exemplum-salve`. Evidence in a report is evidence that decays. This
//! file is that evidence turned into assertions the suite re-derives on every
//! run.
//!
//! # `qemu-aarch64` is required, not preferred
//!
//! Task 1 put it in the dev shell, and
//! `lingua-serpentis/tests/instrumenta.rs::qemu_aarch64_is_on_the_path` fails
//! loudly if it is missing — the same shape as the musl-toolchain assertions
//! beside it, and for the same reason. Nothing here skips: a cross-architecture
//! test that quietly does not run is worse than no cross-architecture test at
//! all, because the suite reports green either way and only one of those greens
//! means anything.
//!
//! # Where `qemu-user` differs from a kernel, and whether it matters
//!
//! It matters for §8 and nowhere else, so it is worth being exact. Measured on
//! this host with a hand-built aarch64 probe:
//!
//! - **`si_addr` is faithful.** A wild read of `0x1000` is delivered as
//!   `SIGSEGV` with `si_addr == 0x1000`; a stack overflow is delivered as
//!   `SIGSEGV` with `si_addr` four bytes below the stack mapping. Both
//!   branches of §8.2's discriminator are therefore reachable and correctly
//!   discriminated under emulation, which is what
//!   [`acervus_exhaustus_sub_aemulatore`] relies on — and
//!   [`acervus_displacitus_est_defectus_internus`] then demonstrates *inside
//!   this suite*, rather than in a probe, by displacing the guest stack and
//!   watching the classification move with it.
//! - **The guest stack does not grow on demand.** `qemu-user` maps the whole
//!   `RLIMIT_STACK` up front, so the `[stack]` mapping's low address *equals*
//!   the derived floor. The measurement §8.2 is built on — that every interface
//!   reporting the stack as it currently exists reports a fraction of what it
//!   may become — is simply not observable here, and `liss-abi`'s
//!   `the_bounds_are_the_mapping_top_minus_the_limit`, which asserts the strict
//!   inequality, would fail under emulation. That test stays on the host, where
//!   the fact it pins is true.
//! - **`SIGBUS` cannot be provoked.** It is checked by inspecting the installer
//!   instead — `target.rs`'s `the_guard_traps_both_sigsegv_and_sigbus`.
//! - **The page size is the host's.** 4 KiB here, so no 16 or 64 KiB kernel is
//!   reachable from this suite at all; `the_page_size_is_asked_of_sysconf_and_
//!   never_assumed` checks that the number is asked for rather than assumed,
//!   and that is the whole of what can be checked from here.
//! - **`RLIMIT_STACK` is an undeclared input to §8 under emulation**, silent in
//!   both directions — unlimited makes the guard test pass vacuously, and below
//!   8 MiB it fails on a correct build, because `qemu-user` floors the guest
//!   stack at 8 MiB whatever the limit says. Asserted in `instrumenta.rs`, and
//!   made self-diagnosing by the negative control here.
//!
//! # Why TLSDESC is not asserted
//!
//! Task 11 confirmed the aarch64 TLSDESC → local-exec relaxation by hand and
//! this file does not re-check it. The reason is not "running the image is
//! stronger" — running exercises only the TLS sites the four programs here
//! actually execute, not all 53 in the image. It is that **an unrelaxed TLSDESC
//! is not representable in what ships**: the linked aarch64 image has zero
//! relocations of any kind and no dynamic section, so there is nowhere for one
//! to live, and [`imago_est_statica_aarch64`]'s `PT_DYNAMIC` assertion already
//! covers every site statically. `nexus.rs`'s module doc records the original
//! measurement.

mod support;

use liss_codegen::target::Target;
use object::read::elf::{FileHeader, ProgramHeader};
use std::path::Path;
use std::process::Command;

/// The emulator. Not a `PathBuf` to search for: `Command` searches `PATH`, and
/// the one thing worth doing here is turning "not found" into a sentence that
/// names the dev shell rather than into `NotFound`.
const AEMULATOR: &str = "qemu-aarch64";

/// Build an image for **aarch64** from one single-file source.
fn imago_peregrina(fons: &str) -> Vec<u8> {
    support::imago(&support::ir_solum(fons), Target::Aarch64)
}

/// Write an aarch64 image somewhere and run it under the emulator;
/// `(status, stdout, stderr)`.
///
/// The host's own [`support::exsequere`] cannot be reused: it `execve`s the
/// image directly, which on an x86_64 kernel with no `binfmt_misc` registration
/// is `ENOEXEC`. So the image is an *argument* here rather than the program,
/// and the `ETXTBSY` race that helper guards against does not arise — the file
/// is opened for reading by the emulator, never executed.
fn exsequere_aemulatum(imago: &[u8], nomen: &str) -> (Option<i32>, String, String) {
    exsequere_aemulatum_cum(imago, nomen, &[])
}

/// [`exsequere_aemulatum`], with environment variables for the emulator.
///
/// The one caller that needs this is
/// [`acervus_displacitus_est_defectus_internus`], which sets `QEMU_STACK_SIZE`
/// to move the guest stack out from under the bounds the runtime derives.
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
    // **`current_dir` is not tidiness, it is containment.** On an *uncaught*
    // guest signal `qemu-user` writes a 36 MB core dump named
    // `qemu_<image>_<timestamp>_<pid>.core` into its working directory — which,
    // for a test spawned by cargo, is the crate root. That is precisely the
    // situation [`acervus_exhaustus_sub_aemulatore`] exists to detect, so the
    // one run that finds the regression is the one that would litter the
    // repository with it. Pointing the emulator at the scratch directory puts
    // the core beside the image, and `remove_dir_all` takes both.
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

/// **The checkpoint's fallback claim, kept as well as the stronger one.**
///
/// A static ELF executable of this architecture with no interpreter: the three
/// things that distinguish a program this design ships from a dynamically
/// linked one, from a PIE, and from the *other* target's object. Asserted on
/// the same bytes that are then run, so a run that passes for the wrong reason
/// — the host's own image, say — cannot.
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
    // Non-emptiness first: an empty slice satisfies every `!any` below
    // vacuously, and "the image has no program headers at all" is a stranger
    // failure than the ones this function is looking for, not a milder one.
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

/// **The checkpoint.** `exemplum-salve`, cross-compiled to aarch64, linked, and
/// **run under `qemu-aarch64`** — against the same `emissio` the interpreted
/// path and the host-compiled path are both held to.
///
/// This is `salve.rs`'s `exemplum_salve_becomes_a_static_binary_that_prints`
/// with the host swapped out. Task 11 did it by hand and put the byte count in
/// a report; the suite has never done it.
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

/// **The D-10 note and a truncated status survive the round trip.**
///
/// `conformance/monita/status-exitus-redactus` is the whole fixture: `redde
/// °CCC` exits 44, and the runtime says so on stderr in Latin with the
/// truncated value rendered as a Roman numeral. Three things have to be right
/// at once for this to pass on aarch64 — the three-byte `Roman` digit blob
/// reaching `liss_exit` intact, the comparison that notices the truncation, and
/// the `Monitum: ` line the runtime writes at *run* time — and Task 10 checked
/// them by hand on a stand-in `liss_initium`, never on a compiled program.
///
/// The expectation is read from the fixture rather than restated, so this
/// cannot drift from the corpus, and the fixture itself is untouched.
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

/// **Spec §8, on the second architecture, end to end.**
///
/// Unbounded recursion faults on the guard page; the handler installed by
/// `main` before `liss_initium` catches it and turns it into an ordinary exit-1
/// Latin diagnostic. `liss-abi`'s `tests/custos.rs` proves the handler, but on
/// the *host*, in a Rust test binary, against a Rust recursion. This proves it
/// for a **compiled Lingua Serpentis program on aarch64**, which is the thing
/// spec §3.3 removed the recursion cap in favour of.
///
/// The message is `conformance/errores/acervus-exhaustus`'s, restated here
/// rather than shared. It was `compilata.rs`'s `DIVERGENTIAE` entry until Task
/// 19 re-blessed the fixture and deleted that file; the restatement outlived
/// both, and now carries more weight than it did. **`compilata` and the corpus
/// run host images only**, so this assertion is the *only* thing in the
/// workspace holding that sentence on aarch64. A corpus-wide rename does not
/// touch it, which is the point and also the hazard: if the wording ever
/// changes, this is the second place to change.
///
/// This is the slow one: the recursion has 16 MiB of guest stack to fill under
/// emulation. Measured at roughly 1.4 s, and it is the reason this file's cost
/// is worth stating rather than assuming.
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
        stderr, "Error: acervus vocationum exhaustus est\n",
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

/// **The negative control: displace the guest stack and the *other* branch
/// fires.**
///
/// [`acervus_exhaustus_sub_aemulatore`] shows the right sentence coming out of
/// a stack overflow. On its own that is compatible with a handler that has no
/// bounds at all — spec §8.2's bounds-unknown fallback reports exhaustion
/// *unconditionally*, and `instrumenta.rs`'s
/// `the_stack_limit_is_finite_and_large_enough_for_the_guard_test` exists
/// precisely because `ulimit -s unlimited` puts the suite in that state and
/// everything still passes.
///
/// So this test proves the bounds are **known and the band computed**, by
/// moving the stack and requiring the classification to move with it.
/// `QEMU_STACK_SIZE` sets the guest stack independently of `RLIMIT_STACK`: at
/// 1 MiB the emulated `[stack]` mapping is 1 MiB, the runtime still subtracts
/// the inherited 16 MiB `RLIMIT_STACK` from its high address, and the derived
/// floor lands roughly 15 MiB *below* where the fault actually happens. The
/// band `[fundus − 1 MiB, fundus + pagina)` therefore does not contain
/// `si_addr`, and the discriminator must say so.
///
/// Three things fall out of four lines, and the third is why it is here rather
/// than in a report:
///
/// - the guard band is **derived**, not a constant that happens to match;
/// - `defectus internus: memoria violata est: signum` gets its **first coverage
///   on aarch64** — `liss-abi/tests/custos.rs` covers it on the host only;
/// - a `RLIMIT_STACK` misconfiguration becomes **self-diagnosing**: it makes
///   this test pass and [`acervus_exhaustus_sub_aemulatore`] fail, which is the
///   opposite pattern from a real §8 regression, where both fail.
///
/// It costs 0.10 s, because a 1 MiB stack fills sixteen times faster than the
/// default one.
///
/// **This is a statement about the emulator's configuration, not about the
/// program.** On real hardware `QEMU_STACK_SIZE` has no analogue and
/// `mapping-top − RLIMIT_STACK` is the true floor; nothing here suggests a
/// kernel could present this geometry.
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
        stderr, "Error: defectus internus: memoria violata est: signum\n",
        "with the guest stack displaced from the derived bounds, the fault is \
         outside the band and must take the discriminator's other branch. The \
         same sentence as `acervus_exhaustus_sub_aemulatore` here would mean \
         the bounds are unknown and the band is never consulted",
    );
    assert_eq!(status, Some(1), "both branches exit 1 (spec §8.2)");
    assert_eq!(stdout, "", "nothing was printed before the recursion");
}

/// **A program that allocates and frees leaves the live count at zero, on
/// aarch64.**
///
/// `conformance/currunt/memoria-adde-in-ansa` is the corpus's densest
/// allocation case: a list grown in a loop, a list of text, and a list of
/// lists. Spec §7.5 has the runtime check `liss_live_count` on the success path
/// and exit **1** with `memoria non liberata est` if it is nonzero, so "exit 0
/// with the blessed stdout" *is* "every allocation this program made was freed
/// exactly once".
///
/// Ownership is where a second architecture could plausibly go wrong without
/// the verifier noticing — `VERBUM`-scaled stores, argument registers, the
/// spilled temporaries a nested list forces — and nothing in this suite has
/// ever checked it anywhere but on the host.
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
