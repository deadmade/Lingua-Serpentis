//! `custos` (spec §8). The missing `libtest` harness is load-bearing: docs/internals/testing.md.

use std::io::Write;
use std::process::{Command, Output};

use abi::custos::{fines, liss_custos_institue};
use abi::scriptio::{liss_flush, liss_print_text};
use abi::textus::liss_text_new;

/// Written before the recursion; only the handler's flush can still deliver it (D-08).
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

fn scaena(nomen: &str) {
    unsafe {
        match nomen {
            "abyssus" => {
                liss_custos_institue();
                // Deliberately not flushed.
                liss_print_text(liss_text_new(ANTE.as_ptr(), ANTE.len() as u64));
                let _ = abyssus(0);
            }
            "violatio" => {
                liss_custos_institue();
                // A fault nowhere near the stack; non-null so LLVM does not fold it into a trap.
                let vagus = 0x1000usize as *const u8;
                let _ = std::ptr::read_volatile(vagus);
            }
            "fistula" => {
                // Undo `std::rt::init`: a compiled program never had `SIG_IGN` (spec §8.1).
                libc::signal(libc::SIGPIPE, libc::SIG_DFL);
                liss_custos_institue();
                frange_fistulam();

                liss_print_text(liss_text_new(ANTE.as_ptr(), ANTE.len() as u64));
                liss_flush();
                // Only reachable if the write neither raised `SIGPIPE` nor returned `EPIPE`.
                libc::_exit(7);
            }
            // The same two faults with stdout closed, and different amounts of pending output.
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

/// A descriptor 1 whose reader is gone, so the next write meets `EPIPE`.
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

/// Recursion the optimiser cannot turn into a loop or a tail call.
#[inline(never)]
#[allow(unconditional_recursion, reason = "exhausting the stack is the point")]
fn abyssus(n: u64) -> u64 {
    let mut pad = [0u8; 256];
    unsafe { std::ptr::write_volatile(pad.as_mut_ptr(), (n & 0xff) as u8) };
    let altius = abyssus(n + 1);
    let cauda = unsafe { std::ptr::read_volatile(pad.as_ptr()) } as u64;
    altius.wrapping_add(cauda)
}

/// The `[stack]` mapping from `/proc/self/maps` — the naive answer, not the derived one.
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

fn the_bounds_are_the_mapping_top_minus_the_limit() {
    let modus = modus_acervi();
    if modus == libc::RLIM_INFINITY {
        // Bounds-unknown by design (spec §8.2).
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

    assert_eq!(culmen, mappa_summa, "the top is the mapping's high address");
    assert_eq!(culmen - fundus, modus, "the size is RLIMIT_STACK");

    assert!(
        fundus < mappa_ima,
        "the real floor is below the mapping's floor",
    );
    assert!(
        mappa_summa - mappa_ima < modus / 8,
        "the mapping is a small fraction of what the stack may grow into",
    );

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
        "erratum: acervus vocationum exhaustus est\n",
    );
    // D-08
    assert_eq!(String::from_utf8_lossy(&out.stdout), format!("{ANTE}\n"));
}

fn a_fault_away_from_the_stack_is_an_internal_defect() {
    let out = in_subprocesso("violatio");
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(
        String::from_utf8_lossy(&out.stderr),
        "erratum: defectus internus: memoria violata est: signum\n",
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

fn a_closed_pipe_never_decides_the_exit_status() {
    for (scaena, nuntius) in [
        (
            "fistula-abyssus",
            "erratum: acervus vocationum exhaustus est\n",
        ),
        (
            "fistula-violatio",
            "erratum: defectus internus: memoria violata est: signum\n",
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
        "({} entry point in abi/src) ",
        if inventum { "an" } else { "no" }
    );
}
