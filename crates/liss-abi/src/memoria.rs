use core::sync::atomic::{AtomicUsize, Ordering};

use crate::{LISS_MAGIC, LissHdr};

static VIVAE: AtomicUsize = AtomicUsize::new(0);

#[unsafe(no_mangle)]
pub extern "C" fn liss_live_count() -> u64 {
    VIVAE.load(Ordering::Relaxed) as u64
}

/// `bytes` of uninitialised, 8-aligned storage, counted.
///
/// # Safety
/// The caller must initialise the whole header before anything reads it, and
/// must eventually pass the pointer to [`liberare`] exactly once.
pub unsafe fn allocare(bytes: usize) -> *mut u8 {
    let p = unsafe { libc::malloc(bytes) }.cast::<u8>();
    if p.is_null() {
        crate::defectus::memoria_exhausta();
    }
    VIVAE.fetch_add(1, Ordering::Relaxed);
    p
}

/// As [`allocare`], but zeroed.
///
/// # Safety
/// As [`allocare`].
pub unsafe fn allocare_nulla(bytes: usize) -> *mut u8 {
    let p = unsafe { libc::calloc(1, bytes) }.cast::<u8>();
    if p.is_null() {
        crate::defectus::memoria_exhausta();
    }
    VIVAE.fetch_add(1, Ordering::Relaxed);
    p
}

/// Return a block from [`allocare`] to the allocator, counted.
///
/// # Safety
/// `p` must have come from [`allocare`]/[`allocare_nulla`] and must not have
/// been freed already. Callers that hold a `LissHdr` should use [`abolere`],
/// which checks the magic first.
pub unsafe fn liberare(p: *mut u8) {
    unsafe { libc::free(p.cast()) };
    VIVAE.fetch_sub(1, Ordering::Relaxed);
}

/// Refuse to go on unless `h` still looks like a live Lingua Serpentis object.
///
/// # Safety
/// `h` must be readable for `size_of::<LissHdr>()` bytes, or null (which ends
/// the process).
pub unsafe fn verifica(h: *const LissHdr) {
    if h.is_null() || unsafe { (*h).magic } != LISS_MAGIC {
        crate::defectus::memoria_violata();
    }
}

/// Check the magic, clear it, and free — the only way a header ever reaches
/// the allocator.
///
/// # Safety
/// `h` must be null, or a header from [`allocare`] that has not been freed.
pub unsafe fn abolere(h: *mut LissHdr) {
    if h.is_null() {
        return;
    }
    unsafe {
        verifica(h);
        // D-27: a plain store here is dead-store-eliminated before the `free`.
        core::ptr::write_volatile(&raw mut (*h).magic, 0);
        liberare(h.cast());
    }
}
