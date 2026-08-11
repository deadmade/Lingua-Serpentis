use core::ptr;

use crate::defectus::memoria_violata;
use crate::memoria::{abolere, allocare, verifica};
use crate::{LISS_MAGIC, LissHdr, LissText, ONUS};

/// Check the magic *and* that this header names text.
///
/// # Safety
/// `p` must be null or readable for a header.
pub(crate) unsafe fn verifica_textum(p: *const LissText) {
    unsafe { verifica(p.cast()) };
    if unsafe { (*p).h.kind } != crate::KIND_TEXT {
        memoria_violata();
    }
}

/// The payload as a `&str`, borrowed.
///
/// # Safety
/// `p` must be a live `LissText`, and must outlive the returned reference.
pub(crate) unsafe fn textum_of<'a>(p: *const LissText) -> &'a str {
    unsafe { verifica_textum(p) };
    let len = unsafe { (*p).len as usize };
    let bytes = unsafe { core::slice::from_raw_parts(p.cast::<u8>().add(ONUS), len) };
    match core::str::from_utf8(bytes) {
        Ok(s) => s,
        Err(_) => memoria_violata(),
    }
}

/// Allocate a `LissText` over `bytes`, which must already be valid UTF-8.
///
/// # Safety
/// Returns a fresh owned pointer the caller must free.
unsafe fn text_novus(bytes: &[u8]) -> *mut LissText {
    let Some(quantum) = ONUS.checked_add(bytes.len()) else {
        crate::defectus::memoria_exhausta()
    };
    let p = unsafe { allocare(quantum) }.cast::<LissText>();
    unsafe {
        (*p).h = LissHdr {
            magic: LISS_MAGIC,
            kind: crate::KIND_TEXT,
        };
        (*p).len = bytes.len() as u64;
        (*p)._rsv = 0;
        ptr::copy_nonoverlapping(bytes.as_ptr(), p.cast::<u8>().add(ONUS), bytes.len());
    }
    p
}

/// A text constant, built from a byte blob codegen emits as static data. The
/// bytes are validated as UTF-8 once, here.
///
/// # Safety
/// `bytes` must be readable for `len` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn liss_text_new(bytes: *const u8, len: u64) -> *mut LissText {
    let b = if len == 0 {
        &[][..]
    } else {
        unsafe { core::slice::from_raw_parts(bytes, len as usize) }
    };
    if core::str::from_utf8(b).is_err() {
        memoria_violata();
    }
    unsafe { text_novus(b) }
}

/// A private copy. **Borrows** `p`.
///
/// # Safety
/// `p` must be a live `LissText`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn liss_text_copy(p: *const LissText) -> *mut LissText {
    unsafe { verifica_textum(p) };
    // The magic vouches for the object, not for its length field.
    let Ok(len) = usize::try_from(unsafe { (*p).len }) else {
        memoria_violata()
    };
    let Some(quantum) = ONUS.checked_add(len) else {
        memoria_violata()
    };
    let q = unsafe { allocare(quantum) };
    unsafe { ptr::copy_nonoverlapping(p.cast::<u8>(), q, quantum) };
    q.cast()
}

/// Free. Checks the magic first. **Null is a no-op**.
///
/// # Safety
/// `p` must be null, or a live `LissText` the caller owns that has not been
/// freed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn liss_text_free(p: *mut LissText) {
    if p.is_null() {
        return;
    }
    unsafe { verifica_textum(p) };
    unsafe { abolere(p.cast()) };
}

/// `a + b` — concatenation. Consumes both.
///
/// # Safety
/// Both must be live `LissText`s the caller owns.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn liss_text_concat(a: *mut LissText, b: *mut LissText) -> *mut LissText {
    unsafe { verifica_textum(a) };
    unsafe { verifica_textum(b) };
    let (la, lb) = unsafe { ((*a).len as usize, (*b).len as usize) };
    let Some(len) = la.checked_add(lb) else {
        crate::defectus::memoria_exhausta()
    };
    let Some(quantum) = ONUS.checked_add(len) else {
        crate::defectus::memoria_exhausta()
    };
    let p = unsafe { allocare(quantum) }.cast::<LissText>();
    unsafe {
        (*p).h = LissHdr {
            magic: LISS_MAGIC,
            kind: crate::KIND_TEXT,
        };
        (*p).len = len as u64;
        (*p)._rsv = 0;
        let dest = p.cast::<u8>().add(ONUS);
        ptr::copy_nonoverlapping(a.cast::<u8>().add(ONUS), dest, la);
        ptr::copy_nonoverlapping(b.cast::<u8>().add(ONUS), dest.add(la), lb);
    }
    unsafe { liss_text_free(a) };
    unsafe { liss_text_free(b) };
    p
}

/// Three-way compare, consuming both: `-1`, `0` or `1`. Byte order over the
/// UTF-8 payload.
///
/// # Safety
/// As [`liss_text_concat`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn liss_text_cmp(a: *mut LissText, b: *mut LissText) -> i8 {
    let ordo = unsafe { textum_of(a) }.cmp(unsafe { textum_of(b) });
    unsafe { liss_text_free(a) };
    unsafe { liss_text_free(b) };
    ordo as i8
}
