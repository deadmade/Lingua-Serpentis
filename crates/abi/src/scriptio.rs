use core::ffi::c_int;
use core::ptr;
use core::sync::atomic::{AtomicUsize, Ordering};

use runtime::scriptio::{Sink, scribe_bool, scribe_char, scribe_frac, scribe_int, scribe_text};

use crate::defectus::memoria_violata;
use crate::{LissInt, LissList, LissText, basis, granum, profunditas};

const CAPACITAS: usize = 8192;

/// Pending stdout. An array and not a `Vec`: the signal handler walks it (D-08).
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
        // The bytes are in place before the length names them; the handler relies on it.
        LONGITUDO.store(n + quantum, Ordering::Release);
        reliquum = &reliquum[quantum..];
    }
}

/// Write pending output to fd 1 and empty the buffer, reporting whether it got
/// out. `false` means the descriptor is gone. **It does not decide the exit
/// status**; the caller does. Pinned by `fistula-abyssus` and `fistula-violatio`
/// in `tests/custos.rs`.
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
/// `liss_exit`. D-06: a closed descriptor 1 is an ordinary end of output.
///
/// # Safety
/// Single-threaded, as [`scribe`]. Ends the process if descriptor 1 is gone.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn liss_flush() {
    if !unsafe { effunde() } {
        unsafe { libc::_exit(0) }
    }
}

/// Write `len` bytes to stderr, unbuffered — D-09 and D-10 both require it not
/// to queue behind program output. **A failure here is ignored.**
///
/// # Safety
/// `p` must be readable for `len` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn liss_scribe_erratum(p: *const u8, len: u64) {
    let _ = unsafe { scribe_ad(2, p, len as usize) };
}

/// One unbuffered write to stderr, for this crate's own diagnostics.
pub(crate) fn erratum(octeti: &[u8]) {
    unsafe { liss_scribe_erratum(octeti.as_ptr(), octeti.len() as u64) };
}

/// `write(2)` until everything is out. `false` means the descriptor is gone and
/// the bytes did not get out. Every failure is the same outcome, `EPIPE` or not.
unsafe fn scribe_ad(fd: c_int, mut p: *const u8, mut n: usize) -> bool {
    while n > 0 {
        let scriptum = unsafe { libc::write(fd, p.cast(), n) };
        if scriptum > 0 {
            p = unsafe { p.add(scriptum as usize) };
            n -= scriptum as usize;
            continue;
        }
        if scriptum == 0 {
            // A non-empty buffer that writes 0 would spin forever.
            return false;
        }
        if unsafe { *libc::__errno_location() } == libc::EINTR {
            continue;
        }
        return false;
    }
    true
}

/// The buffer, as a `runtime` formatter sink.
struct Pixis;

impl Sink for Pixis {
    fn push(&mut self, bytes: &[u8]) {
        unsafe { scribe(bytes) };
    }
}

/// Stderr, unbuffered, as a `runtime` formatter sink. D-28
pub(crate) struct Erratum;

impl Sink for Erratum {
    fn push(&mut self, bytes: &[u8]) {
        erratum(bytes);
    }
}

/// Render one machine word of kind `kind`, without a trailing newline and
/// without consuming it.
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

/// `[a, b, c]`, driven by `ekind` rather than by a tag on each element.
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
