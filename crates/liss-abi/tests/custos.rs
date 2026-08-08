//! `custos` (spec §8): the guard page, and the two signal dispositions
//! `std::rt::init` never installs for us.
//!
//! **This target has no `libtest` harness, and that is load-bearing.** Every
//! bound §8.2 derives describes the *main* thread's stack — the `[stack]`
//! mapping, which grows on demand — and `libtest` spawns a thread per test even
//! at `--test-threads=1`. A recursion on a worker would fault in an ordinary
//! `mmap` megabytes from the band and take the handler's other branch, so the
//! test would pass or fail for reasons unrelated to what it claims to check.
//! Owning `main` is the only way to put the scenario on the thread the runtime
//! actually runs on.
//!
//! It is also the situation spec §8.1 records as an invariant: `main` here is a
//! Rust `fn main`, so `std::rt::init` *has* run and std *has* installed a
//! `SIGSEGV` handler and an alternate stack of its own. `liss_custos_institue`
//! is called afterwards and must override both — which the `abyssus` scenario
//! proves, because before this task that child died with std's English
//! `has overflowed its stack` and `SIGABRT`.
//!
//! Every assertion needs a process that *dies*, so — as in `abi.rs` — the
//! binary re-executes itself with `LISS_CUSTOS_SCAENA` naming one scenario. A
//! test-only hook that made the handler return would have tested a handler no
//! compiled program ever runs.

use std::io::Write;
use std::process::{Command, Output};

use liss_abi::custos::{fines, liss_custos_institue};
use liss_abi::scriptio::{liss_flush, liss_print_text};
use liss_abi::textus::liss_text_new;

/// What the buffered `scribe` writes before the recursion, and what the
/// handler's flush must still deliver to descriptor 1 (D-08).
const ANTE: &str = "ante abyssum";

fn main() {
    if let Ok(nomen) = std::env::var("LISS_CUSTOS_SCAENA") {
        scaena(&nomen);
        return;
    }

    let probationes: [(&str, fn()); 6] = [
        (
            "the_bounds_are_the_mapping_top_minus_the_limit",
            the_bounds_are_the_mapping_top_minus_the_limit,
        ),
        (
            "deep_recursion_is_a_latin_exit_and_never_a_signal",
            deep_recursion_is_a_latin_exit_and_never_a_signal,
        ),
        (
            "a_fault_away_from_the_stack_is_an_internal_defect",
            a_fault_away_from_the_stack_is_an_internal_defect,
        ),
        (
            "a_closed_pipe_is_an_ordinary_end_of_output",
            a_closed_pipe_is_an_ordinary_end_of_output,
        ),
        (
            "a_closed_pipe_never_decides_the_exit_status",
            a_closed_pipe_never_decides_the_exit_status,
        ),
        (
            "a_main_in_this_crate_must_install_the_guard",
            a_main_in_this_crate_must_install_the_guard,
        ),
    ];

    println!("\nrunning {} tests", probationes.len());
    let mut lapsae = 0usize;
    for (nomen, probatio) in probationes {
        print!("test {nomen} ... ");
        let _ = std::io::stdout().flush();
        match std::panic::catch_unwind(probatio) {
            Ok(()) => println!("ok"),
            Err(_) => {
                println!("FAILED");
                lapsae += 1;
            }
        }
    }
    println!(
        "\ntest result: {}. {} passed; {lapsae} failed\n",
        if lapsae == 0 { "ok" } else { "FAILED" },
        probationes.len() - lapsae,
    );
    if lapsae > 0 {
        std::process::exit(1);
    }
}

// ------------------------------------------------------------------ scenarios

fn scaena(nomen: &str) {
    unsafe {
        match nomen {
            "abyssus" => {
                liss_custos_institue();
                // Into the buffer, and deliberately not flushed: only the
                // handler can still get this to descriptor 1.
                liss_print_text(liss_text_new(ANTE.as_ptr(), ANTE.len() as u64));
                let _ = abyssus(0);
            }
            "violatio" => {
                liss_custos_institue();
                // A fault nowhere near the stack. `mmap_min_addr` is 64 KiB by
                // default, so this page is unmapped; a non-null constant keeps
                // LLVM from folding the access into a trap instruction, which
                // would raise `SIGILL` and test nothing.
                let vagus = 0x1000usize as *const u8;
                let _ = std::ptr::read_volatile(vagus);
            }
            "fistula" => {
                // Undo `std::rt::init`. A compiled program never had `SIG_IGN`
                // to begin with (spec §8.1); this child has to give it back to
                // stand where that program stands. Without the reset the
                // scenario would pass on std's disposition and check nothing.
                libc::signal(libc::SIGPIPE, libc::SIG_DFL);
                liss_custos_institue();
                frange_fistulam();

                liss_print_text(liss_text_new(ANTE.as_ptr(), ANTE.len() as u64));
                liss_flush();
                // Only reachable if the write neither raised `SIGPIPE` nor
                // returned `EPIPE`.
                libc::_exit(7);
            }
            // The same two faults as `abyssus` and `violatio`, with stdout
            // already closed — and deliberately with different amounts of
            // pending output, because that is the variable that used to decide
            // the exit status. `abyssus` leaves 13 bytes in the buffer, so the
            // flush reached a `write` and met `EPIPE`; `violatio` leaves none,
            // so the flush short-circuited and never saw it.
            "fistula-abyssus" => {
                liss_custos_institue();
                frange_fistulam();
                liss_print_text(liss_text_new(ANTE.as_ptr(), ANTE.len() as u64));
                let _ = abyssus(0);
            }
            "fistula-violatio" => {
                liss_custos_institue();
                frange_fistulam();
                let vagus = 0x1000usize as *const u8;
                let _ = std::ptr::read_volatile(vagus);
            }
            other => panic!("unknown scenario {other}"),
        }
    }
    unreachable!("every scenario ends the process");
}

/// Give this process a descriptor 1 whose reader is gone, so the next write to
/// it meets `EPIPE`.
unsafe fn frange_fistulam() {
    let mut fistula = [0 as libc::c_int; 2];
    assert_eq!(unsafe { libc::pipe(fistula.as_mut_ptr()) }, 0, "pipe");
    unsafe { libc::close(fistula[0]) };
    assert!(unsafe { libc::dup2(fistula[1], 1) } >= 0, "dup2");
}

fn in_subprocesso(scaena: &str) -> Output {
    Command::new(std::env::current_exe().expect("the test binary knows its own path"))
        .env("LISS_CUSTOS_SCAENA", scaena)
        .output()
        .expect("the test binary re-executes itself")
}

/// Recursion the optimiser cannot turn into a loop: each frame keeps a padding
/// array alive *across* the recursive call with a volatile write before and a
/// volatile read after, so the frame cannot be elided and the call cannot
/// become a tail call.
#[inline(never)]
#[allow(unconditional_recursion, reason = "exhausting the stack is the point")]
fn abyssus(n: u64) -> u64 {
    let mut pad = [0u8; 256];
    unsafe { std::ptr::write_volatile(pad.as_mut_ptr(), (n & 0xff) as u8) };
    let altius = abyssus(n + 1);
    let cauda = unsafe { std::ptr::read_volatile(pad.as_ptr()) } as u64;
    altius.wrapping_add(cauda)
}

/// The `[stack]` mapping, read straight from `/proc/self/maps` — the naive
/// answer this test exists to distinguish the derived one from.
fn mappa() -> (usize, usize) {
    let maps = std::fs::read_to_string("/proc/self/maps").expect("/proc is mounted");
    let linea = maps
        .lines()
        .find(|l| l.ends_with("[stack]"))
        .expect("the main thread has a [stack] mapping");
    let campus = linea.split_whitespace().next().expect("an address range");
    let (ima, summa) = campus.split_once('-').expect("lo-hi");
    (
        usize::from_str_radix(ima, 16).expect("hex"),
        usize::from_str_radix(summa, 16).expect("hex"),
    )
}

fn modus_acervi() -> u64 {
    unsafe {
        let mut modus: libc::rlimit = std::mem::zeroed();
        assert_eq!(
            libc::getrlimit(libc::RLIMIT_STACK, &mut modus),
            0,
            "getrlimit",
        );
        modus.rlim_cur
    }
}

// ---------------------------------------------------------------------- tests

/// Print the derived bounds beside the three numbers they come from, and assert
/// they agree — so the probe's finding stays true rather than being
/// re-discovered the next time someone reaches for the obvious interface.
fn the_bounds_are_the_mapping_top_minus_the_limit() {
    let modus = modus_acervi();
    if modus == libc::RLIM_INFINITY {
        // `RLIM_INFINITY` is bounds-unknown by design (spec §8.2); the
        // handler's fallback branch covers it, and it reports exhaustion,
        // which `deep_recursion_is_a_latin_exit_and_never_a_signal` still pins.
        println!("(RLIMIT_STACK is unlimited here; bounds are unknown by design) ");
        return;
    }
    let modus = modus as usize;
    let ancora = 0u64;
    let locus = &raw const ancora as usize;
    let (mappa_ima, mappa_summa) = mappa();
    let (fundus, culmen) = fines().expect("a finite RLIMIT_STACK and a mounted /proc");
    println!(
        "\n  derivatum   0x{fundus:012x}-0x{culmen:012x}  {} KiB\
         \n  [stack]     0x{mappa_ima:012x}-0x{mappa_summa:012x}  {} KiB\
         \n  RLIMIT      {} KiB\
         \n  locus       0x{locus:012x}",
        (culmen - fundus) / 1024,
        (mappa_summa - mappa_ima) / 1024,
        modus / 1024,
    );

    // The derivation itself.
    assert_eq!(culmen, mappa_summa, "the top is the mapping's high address");
    assert_eq!(culmen - fundus, modus, "the size is RLIMIT_STACK");

    // The probe's finding: the `[stack]` mapping describes only the pages
    // already *touched*, so its own low address — what `pthread_getattr_np` and
    // a bare `/proc` read both report — sits megabytes above the floor the
    // guard page actually defends.
    assert!(
        fundus < mappa_ima,
        "the real floor is below the mapping's floor",
    );
    assert!(
        mappa_summa - mappa_ima < modus / 8,
        "the mapping is a small fraction of what the stack may grow into",
    );

    // And the band contains the running stack, which also confirms this is the
    // main thread: no worker's locals live in `[stack]`.
    assert!(
        fundus <= locus && locus < culmen,
        "the band holds the stack",
    );
    assert!(
        mappa_ima <= locus && locus < mappa_summa,
        "the scenarios must run on the main thread",
    );
}

fn deep_recursion_is_a_latin_exit_and_never_a_signal() {
    let out = in_subprocesso("abyssus");
    assert_eq!(
        out.status.code(),
        Some(1),
        "a signalled process is a `no abort` violation, which is D-04's shape",
    );
    assert_eq!(
        String::from_utf8_lossy(&out.stderr),
        "Error: acervus vocationum exhaustus est\n",
    );
    // D-08 for the stack-exhaustion case: what `scribe` produced before the
    // fault was still in the buffer when the handler ran, and nothing else in
    // that process could have written it out.
    assert_eq!(String::from_utf8_lossy(&out.stdout), format!("{ANTE}\n"));
}

fn a_fault_away_from_the_stack_is_an_internal_defect() {
    let out = in_subprocesso("violatio");
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(
        String::from_utf8_lossy(&out.stderr),
        "Error: defectus internus: memoria violata est: signum\n",
        "the discriminator's other branch. `: signum` is the handler's tail, and \
         it is what distinguishes this from `memoria_violata`'s `: verificatio` \
         — a fault the kernel reported, not a value a `verifica` rejected",
    );
}

fn a_closed_pipe_is_an_ordinary_end_of_output() {
    let out = in_subprocesso("fistula");
    assert_eq!(
        out.status.code(),
        Some(0),
        "`prog | head` must not kill the child; D-06 moved into the runtime",
    );
    assert_eq!(String::from_utf8_lossy(&out.stderr), "");
}

/// A closed stdout is an end of *output*, and nothing more.
///
/// It used to be an end of the *program*, decided inside `scribe_ad`, and the
/// decision leaked into the failing paths: `liss_flush` short-circuits when the
/// buffer is empty, so it never reached a `write` and never saw `EPIPE`. The
/// same class of fault therefore reported differently depending on how much
/// output the program happened to have buffered — `fistula-abyssus` exited 0
/// with an empty stderr, `fistula-violatio` exited 1 with its sentence. `judge`
/// settles which of the two is right and rules out the middle: exit 0 with a
/// diagnostic is `clean success`, exit 1 without one is `diagnostic present`,
/// so exit 1 *with* the sentence is the only outcome that satisfies the
/// invariant.
fn a_closed_pipe_never_decides_the_exit_status() {
    for (scaena, nuntius) in [
        (
            "fistula-abyssus",
            "Error: acervus vocationum exhaustus est\n",
        ),
        (
            "fistula-violatio",
            "Error: defectus internus: memoria violata est: signum\n",
        ),
    ] {
        let out = in_subprocesso(scaena);
        assert_eq!(out.status.code(), Some(1), "{scaena} status");
        assert_eq!(
            String::from_utf8_lossy(&out.stderr),
            nuntius,
            "{scaena} stderr",
        );
    }
}

/// The Task 10 tripwire.
///
/// `liss_custos_institue` has no caller: `main` arrives with Task 10 (spec §6,
/// "the runtime provides `main`"), and a `main` that calls `liss_initium` and
/// forgets the installer would leave **every other check in this file
/// passing** — they all install explicitly — while D-04 stayed quietly open
/// until somebody ran a recursive program. Prose in a brief is not enough for
/// a hole that shows no symptom in the test suite.
///
/// So: crude on purpose, and it fires the moment an entry point appears in this
/// crate without the call. It asserts nothing about a crate that has no entry
/// point, because today that is the honest state.
fn a_main_in_this_crate_must_install_the_guard() {
    let dir = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/src"));
    let mut inventum = false;
    for intrans in std::fs::read_dir(dir).expect("src/ is readable") {
        let via = intrans.expect("a directory entry").path();
        if via.extension().is_none_or(|e| e != "rs") {
            continue;
        }
        let fons = std::fs::read_to_string(&via).expect("utf-8 source");
        if !fons.contains("fn main(") {
            continue;
        }
        inventum = true;
        assert!(
            fons.contains("liss_custos_institue("),
            "{}: an entry point must install the guard before `liss_initium`, \
             or deep recursion is a bare SIGSEGV again — spec §8, D-04",
            via.display(),
        );
    }
    print!(
        "({} entry point in liss-abi/src) ",
        if inventum { "an" } else { "no" }
    );
}
