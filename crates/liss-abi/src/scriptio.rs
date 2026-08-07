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

/// Write pending output to fd 1 and empty the buffer.
///
/// Called on overflow, at exit, on the raising path, and — from Task 9 — from
/// the signal handler. The length is cleared only after the bytes are out, so a
/// handler that fires between the two writes the same bytes twice rather than
/// losing them; and it cannot fire *inside* the `write`, because a `SIGSEGV`
/// arrives from an instruction this thread executes and there are none between
/// here and the syscall.
///
/// # Safety
/// Single-threaded, as [`scribe`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn liss_flush() {
    let n = LONGITUDO.load(Ordering::Relaxed);
    if n == 0 {
        return;
    }
    unsafe { scribe_ad(1, (&raw const PIXIS).cast::<u8>(), n) };
    LONGITUDO.store(0, Ordering::Release);
}

/// Write `len` bytes to stderr, unbuffered.
///
/// Diagnostics and warnings are not program output and must not queue behind
/// it: D-09 requires a warning to reach stderr even when the program then
/// fails, and D-10 requires the truncated-exit-status note to reach it before
/// the process ends.
///
/// # Safety
/// `p` must be readable for `len` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn liss_scribe_erratum(p: *const u8, len: u64) {
    unsafe { scribe_ad(2, p, len as usize) };
}

/// `write(2)` until everything is out, or the pipe is gone.
///
/// D-06 lives here now. `std::rt::init` never runs (spec §8.1), so this runtime
/// does not inherit std's `SIG_IGN` for `SIGPIPE` and the default disposition
/// would kill the child on `prog | head`. Task 9 sets `SIG_IGN` at startup; the
/// `EPIPE` that then surfaces here is an ordinary end of output, and exit 0 is
/// what every Unix tool does — reproducing `main.rs::write_or_exit`'s semantics
/// at the source rather than in the CLI.
unsafe fn scribe_ad(fd: c_int, mut p: *const u8, mut n: usize) {
    while n > 0 {
        let scriptum = unsafe { libc::write(fd, p.cast(), n) };
        if scriptum > 0 {
            p = unsafe { p.add(scriptum as usize) };
            n -= scriptum as usize;
            continue;
        }
        match unsafe { *libc::__errno_location() } {
            libc::EINTR => continue,
            libc::EPIPE => unsafe { libc::_exit(0) },
            // A real failure to write, on a stream that may itself be broken.
            // Saying so is not worth a second failed write.
            _ => unsafe { libc::_exit(1) },
        }
    }
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
