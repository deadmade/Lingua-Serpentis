//! Output (spec §6): the buffer, the flush, and `scribe`'s six shapes.
//!
//! **The buffer is a `static mut [u8; 8192]` with a separate `AtomicUsize`
//! length.** Not a `Vec`: no allocator involvement, no `realloc` window, no
//! lock. Task 9's signal handler walks this buffer from a signal context to
//! flush pending output before it reports stack exhaustion, and a `Vec` could
//! be mid-`realloc` when the fault arrives — which would lose D-08 for exactly
//! the case §8 exists to handle.
//!
//! **`scribe` writes the bytes first, then stores the new length with
//! `Release`.** That ordering is the whole guarantee: any length the handler
//! observes names bytes that are already fully written, so stack-overflow
//! output is never torn. The load side is `Relaxed` because there is no second
//! writer — a compiled program is single-threaded — and the handler runs on the
//! same thread, so the release-store is what it needs, not a matching acquire
//! in this file.
//!
//! **Writing decides nothing about the exit status.** [`scribe_ad`] reports a
//! dead descriptor to its caller and never ends the process itself, because the
//! right answer differs between the success path and the failing ones and only
//! the caller knows which it is on. See [`effunde`].
//!
//! **Stream ordering is not observable and must not be designed for.** The
//! fixture format stores stdout and stderr in separate files, so nothing in the
//! suite can see how the two interleave. What is owed is only: everything
//! `scribe` produced before a failure still reaches stdout (D-08), warnings
//! still reach stderr (D-09), and the truncated-exit note reaches stderr
//! (D-10). All three hold with direct, unordered writes to the two descriptors.

use core::ffi::c_int;
use core::ptr;
use core::sync::atomic::{AtomicUsize, Ordering};

use liss_runtime::scriptio::{Sink, scribe_bool, scribe_char, scribe_frac, scribe_int, scribe_text};

use crate::defectus::memoria_violata;
use crate::{LissInt, LissList, LissText, basis, granum, profunditas};

const CAPACITAS: usize = 8192;

/// Pending stdout. See the module doc for why this is an array and not a `Vec`.
static mut PIXIS: [u8; CAPACITAS] = [0; CAPACITAS];

/// How much of [`PIXIS`] is written. Published with `Release` after the bytes.
static LONGITUDO: AtomicUsize = AtomicUsize::new(0);

/// Append to the pending-output buffer, flushing whenever it fills.
///
/// # Safety
/// Single-threaded. A second writer would race the buffer, which no ordering
/// here defends against and nothing in a compiled program can create.
pub(crate) unsafe fn scribe(bytes: &[u8]) {
    let mut reliquum = bytes;
    while !reliquum.is_empty() {
        let n = LONGITUDO.load(Ordering::Relaxed);
        if n == CAPACITAS {
            unsafe { liss_flush() };
            continue;
        }
        let quantum = reliquum.len().min(CAPACITAS - n);
        unsafe {
            let basis = (&raw mut PIXIS).cast::<u8>();
            ptr::copy_nonoverlapping(reliquum.as_ptr(), basis.add(n), quantum);
        }
        // The bytes are in place before the length names them. This is the
        // line the signal handler depends on; see the module doc.
        LONGITUDO.store(n + quantum, Ordering::Release);
        reliquum = &reliquum[quantum..];
    }
}

/// Write pending output to fd 1 and empty the buffer, reporting whether it got
/// out. `false` means the descriptor is gone.
///
/// **It does not decide the exit status, and that is the whole design.**
/// A flush with nothing pending returns at `n == 0` without reaching a `write`,
/// so it never sees `EPIPE`; while the write chose the status, the *same* fault
/// therefore ended differently depending on how much output the program
/// happened to have buffered — exit 0 with a silent stderr for a program that
/// had printed something, exit 1 with its diagnostic for one that had not.
/// Nothing stated that rule because nobody designed it.
///
/// A closed stdout now means one thing — the bytes did not get out — and each
/// caller says what that is worth:
///
/// - the **success** path ([`liss_flush`], and `liss_exit` through it) treats it
///   as an ordinary end of output and exits 0: D-06, and `prog | head`;
/// - the **failing** paths (`defectus::liss_panic`, and `custos`'s signal
///   handler) ignore it, write their sentence to descriptor 2 and exit 1.
///
/// `judge` is what settles that and rules out the middle: exit 0 carrying a
/// diagnostic is a `clean success` violation, exit 1 carrying none is
/// `diagnostic present`. Exit 1 *with* the sentence is the only failing outcome
/// that satisfies the invariant, so the flush must not be able to pre-empt it.
///
/// The length is cleared only after the bytes are out, so a handler that fires
/// between the two writes the same bytes twice rather than losing them; and it
/// cannot fire *inside* the `write`, because a `SIGSEGV` arrives from an
/// instruction this thread executes and there are none between here and the
/// syscall.
///
/// # Safety
/// Single-threaded, as [`scribe`].
pub(crate) unsafe fn effunde() -> bool {
    let n = LONGITUDO.load(Ordering::Relaxed);
    if n == 0 {
        return true;
    }
    let emissum = unsafe { scribe_ad(1, (&raw const PIXIS).cast::<u8>(), n) };
    LONGITUDO.store(0, Ordering::Release);
    emissum
}

/// Flush on the **success** path: on overflow from [`scribe`], and at exit from
/// `liss_exit`.
///
/// D-06 lives here. `std::rt::init` never runs (spec §8.1), so this runtime does
/// not inherit std's `SIG_IGN` for `SIGPIPE` and the default disposition would
/// kill the child on `prog | head`; `custos::liss_custos_institue` sets it at
/// startup, and the `EPIPE` that then surfaces is an ordinary end of output, so
/// exit 0 is what every Unix tool does — `main.rs::write_or_exit`'s semantics,
/// reproduced at the source rather than in the CLI.
///
/// # Safety
/// Single-threaded, as [`scribe`]. Ends the process if descriptor 1 is gone.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn liss_flush() {
    if !unsafe { effunde() } {
        unsafe { libc::_exit(0) }
    }
}

/// Write `len` bytes to stderr, unbuffered.
///
/// Diagnostics and warnings are not program output and must not queue behind
/// it: D-09 requires a warning to reach stderr even when the program then
/// fails, and D-10 requires the truncated-exit-status note to reach it before
/// the process ends.
///
/// **A failure here is ignored, in both directions.** This is `mone`'s entry
/// point too, and a warning that cannot be delivered is no reason to fail a
/// program that otherwise succeeded; while the failing paths that also call it
/// reach `_exit(1)` by their own control flow whether or not the sentence
/// arrived — which is what stops a program whose error channel closed from
/// claiming success.
///
/// # Safety
/// `p` must be readable for `len` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn liss_scribe_erratum(p: *const u8, len: u64) {
    let _ = unsafe { scribe_ad(2, p, len as usize) };
}

/// `write(2)` until everything is out. `false` means the descriptor is gone and
/// the bytes did not get out.
///
/// Every failure is the same outcome, `EPIPE` or not. The alternative was the
/// old non-`EPIPE` arm's `_exit(1)`, which produced exit 1 with an empty
/// stderr — `judge`'s `diagnostic present` violation, raised from inside the
/// code whose job is to keep the program's failures well-formed. Saying more
/// about a stream that may itself be broken is not worth a second failed write.
unsafe fn scribe_ad(fd: c_int, mut p: *const u8, mut n: usize) -> bool {
    while n > 0 {
        let scriptum = unsafe { libc::write(fd, p.cast(), n) };
        if scriptum > 0 {
            p = unsafe { p.add(scriptum as usize) };
            n -= scriptum as usize;
            continue;
        }
        if scriptum == 0 {
            // `write` returning 0 for a non-empty buffer would spin forever on
            // a stale `EINTR`, and nothing useful can be done with a descriptor
            // that does it.
            return false;
        }
        if unsafe { *libc::__errno_location() } == libc::EINTR {
            continue;
        }
        return false;
    }
    true
}

/// The buffer, as a `liss-runtime` formatter sink.
struct Pixis;

impl Sink for Pixis {
    fn push(&mut self, bytes: &[u8]) {
        unsafe { scribe(bytes) };
    }
}

/// Render one machine word of kind `kind`, without a trailing newline and
/// without consuming it.
///
/// This is the whole of spec §5.5's list case: `ekind` is the only thing that
/// says how to read an element, and the closed-form kind code makes the
/// recursion one subtraction deep per level.
///
/// # Safety
/// `word` must be a live value of kind `kind`.
pub(crate) unsafe fn scribe_verbum(word: u64, kind: u32, out: &mut impl Sink) {
    if profunditas(kind) > 0 {
        unsafe { scribe_seriem(word as *const LissList, out) };
        return;
    }
    match basis(kind) {
        crate::KIND_BOOL => scribe_bool(word != 0, out),
        crate::KIND_CHAR => match char::from_u32(word as u32) {
            Some(c) => scribe_char(c, out),
            None => memoria_violata(),
        },
        crate::KIND_INT => scribe_int(&unsafe { crate::numeri::roman_of(word as *const LissInt) }, out),
        crate::KIND_FRAC_UNCIA | crate::KIND_FRAC_SCRIPULUM => scribe_frac(
            &unsafe { crate::numeri::roman_of(word as *const LissInt) },
            granum(basis(kind)).unwrap_or_else(|| memoria_violata()),
            out,
        ),
        crate::KIND_TEXT => scribe_text(unsafe { crate::textus::textum_of(word as *const LissText) }, out),
        _ => memoria_violata(),
    }
}

/// `[a, b, c]` — `Value::display`'s list arm, driven by `ekind` instead of a
/// tag on each element.
unsafe fn scribe_seriem(p: *const LissList, out: &mut impl Sink) {
    unsafe { crate::series::verifica_seriem(p) };
    let (len, ekind) = unsafe { ((*p).len, (*p).ekind as u32) };
    out.push(b"[");
    for i in 0..len {
        if i > 0 {
            out.push(b", ");
        }
        unsafe { scribe_verbum(*crate::series::elementa(p).add(i as usize), ekind, out) };
    }
    out.push(b"]");
}

/// Finish a `scribe` statement: one newline, exactly as `StmtKind::Print` has
/// always appended one.
fn linea(out: &mut impl Sink) {
    out.push(b"\n");
}

/// `scribe(veritas)`. Not owned, so nothing is freed.
#[unsafe(no_mangle)]
pub extern "C" fn liss_print_bool(v: i8) {
    scribe_bool(v != 0, &mut Pixis);
    linea(&mut Pixis);
}

/// `scribe(lit)`. Not owned, so nothing is freed.
///
/// # Safety
/// Ends the process if `c` is not a Unicode scalar value.
#[unsafe(no_mangle)]
pub extern "C" fn liss_print_char(c: i32) {
    match char::from_u32(c as u32) {
        Some(c) => scribe_char(c, &mut Pixis),
        None => memoria_violata(),
    }
    linea(&mut Pixis);
}

/// `scribe(ni)`. Consumes `p`.
///
/// # Safety
/// `p` must be a live `LissInt` the caller owns.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn liss_print_int(p: *mut LissInt) {
    scribe_int(&unsafe { crate::numeri::roman_of(p) }, &mut Pixis);
    linea(&mut Pixis);
    unsafe { crate::numeri::liss_int_free(p) };
}

/// `scribe(mnr)` / `scribe(nr)`. `granum` is XII or CCLXXXVIII. Consumes `p`.
///
/// # Safety
/// `p` must be a live `LissInt` the caller owns.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn liss_print_frac(p: *mut LissInt, granum: u64) {
    if granum != 12 && granum != 288 {
        memoria_violata();
    }
    scribe_frac(&unsafe { crate::numeri::roman_of(p) }, granum, &mut Pixis);
    linea(&mut Pixis);
    unsafe { crate::numeri::liss_int_free(p) };
}

/// `scribe(voc)`. Consumes `p`.
///
/// # Safety
/// `p` must be a live `LissText` the caller owns.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn liss_print_text(p: *mut LissText) {
    scribe_text(unsafe { crate::textus::textum_of(p) }, &mut Pixis);
    linea(&mut Pixis);
    unsafe { crate::textus::liss_text_free(p) };
}

/// `scribe(T lista)`. Consumes `p`, recursing through `ekind` to any depth.
///
/// # Safety
/// `p` must be a live `LissList` the caller owns.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn liss_print_list(p: *mut LissList) {
    unsafe { scribe_seriem(p, &mut Pixis) };
    linea(&mut Pixis);
    unsafe { crate::series::liss_list_free(p) };
}
