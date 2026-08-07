//! The runtime's entry point (spec §6): `main`, and the four things it owes.
//!
//! Compiled code provides `liss_initium`; this crate provides `main`. Because
//! the entry point is `#[unsafe(no_mangle)] extern "C" fn main`, `lang_start` is
//! bypassed and `std::rt::init` never runs — which is what §8.1 turns to
//! advantage, and what makes the first line below load-bearing rather than
//! defensive.
//!
//! # Why the installer is called here, and not from `.init_array`
//!
//! A constructor would remove the need to *remember*, and on this target it
//! would work: musl's `__libc_start_main` runs `.init_array` before `main`, and
//! since `std::rt::init` never runs there is no std `SIGSEGV` handler to lose
//! the race to — the reason [`crate::custos`] rejects the mechanism on a host
//! build does not apply here. It is still not what this module does, for three
//! reasons:
//!
//! 1. **The host build and the musl build must be the same program.** A
//!    constructor is inert on the host (`std::rt::init` overwrites the
//!    disposition immediately afterwards) and live on musl. Shipping a
//!    mechanism that is *provably wrong* in the configuration `cargo test`
//!    exercises means the thing tested is not the thing shipped.
//! 2. **`liss_custos_institue` must run once**, and a constructor is an
//!    invisible second entry: the next reader who adds the obvious explicit
//!    call gets two `sigaltstack`s and two `sigaction`s with nothing to say so.
//! 3. **A call is checkable and a constructor is barely so.** Task 10's test
//!    reads the prelinked runtime object and asserts that `main`'s relocations
//!    name `liss_custos_institue` *at a lower offset than* `liss_initium` —
//!    i.e. that the installer is called, from `main`, first. Against
//!    `.init_array` the strongest available statement is "some pointer is in the
//!    section", which is where the Task 9 tripwire already was.
//!
//! So: one call, first line, and a test that reads the shipped object rather
//! than the source it came from.
//!
//! # The rest of the sequence
//!
//! `liss_custos_institue` also sets `SIGPIPE` to `SIG_IGN` (D-06, spec §6), so
//! that is not repeated here. Then:
//!
//! - call `liss_initium`, which returns one owned `ni`;
//! - convert it with `liss_runtime::exit_status` — the value mod CCLVI, shifted
//!   non-negative — and **free it**, because the leak counter is checked two
//!   lines later and a returned value nobody freed is a leak like any other;
//! - write the D-10 truncation note to stderr if there is one, *before* exiting,
//!   as `Monitum: …` — the same sentence and the same prefix the CLI wrote, so
//!   `conformance/monita/status-exitus-redactus` reads identically;
//! - hand the status to [`crate::defectus::liss_exit`], which flushes stdout,
//!   checks the live-allocation count, and exits — **1**, not 70, for a leak.
//!
//! The order matters in one place only and it is the one the fixture pins: the
//! note reaches descriptor 2 before the process ends. Stream *interleaving* is
//! not observable (spec §6), so nothing else here is ordered by design.

use core::ffi::{c_char, c_int};

use crate::LissInt;

/// The prefix `Warning`'s `Display` writes for a warning with no file and no
/// line — `error.rs`'s fourth arm. Written out here because this crate cannot
/// depend on `lingua-serpentis`, and a `b"…"` literal rather than a `c"…"` one
/// because it is a fragment, not a sentence: `tests/abi.rs` collects the `c"…"`
/// literals as the crate's Latin vocabulary and a bare prefix is not one.
const MONITUM_PRAEFIXUM: &[u8] = b"Monitum: ";

unsafe extern "C" {
    /// Compiled code's entry point (spec §4.3): the lowering of the user's
    /// `munus initium reddit ni`. Returns one owned `ni`, which is this
    /// function's to free.
    ///
    /// Undefined in the runtime object on purpose — it is the one symbol the
    /// program link supplies, and Task 10's test asserts it is still undefined
    /// in the embedded blob.
    fn liss_initium() -> *mut LissInt;
}

/// The program.
///
/// # Safety
/// Called by `__libc_start_main`, once, on the main thread. Never returns:
/// [`crate::defectus::liss_exit`] ends the process.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn main(_argc: c_int, _argv: *const *const c_char) -> c_int {
    // First, and before anything can recurse. See the module doc.
    unsafe { crate::custos::liss_custos_institue() };

    let reditus = unsafe { liss_initium() };
    // Read it, then free it: `roman_of` copies the digits out, so the pointer
    // is dead by the time `exit_status` sees the value. Freeing before the
    // conversion would be reading freed memory; freeing after `liss_exit` is
    // not a thing that happens.
    let numerus = unsafe { crate::numeri::roman_of(reditus) };
    unsafe { crate::numeri::liss_int_free(reditus) };

    let (status, monitum) = liss_runtime::exit_status(&numerus);
    if let Some(m) = monitum {
        // D-10: the note reaches stderr before the process ends. Unbuffered, so
        // it cannot queue behind program output that a closed descriptor 1
        // would then lose.
        erratum(MONITUM_PRAEFIXUM);
        erratum(m.message.as_bytes());
        erratum(b"\n");
    }

    unsafe { crate::defectus::liss_exit(c_int::from(status)) }
}

/// One unbuffered write to stderr, as `defectus::erratum` and `custos::erratum`
/// do it.
fn erratum(octeti: &[u8]) {
    unsafe { crate::scriptio::liss_scribe_erratum(octeti.as_ptr(), octeti.len() as u64) };
}
