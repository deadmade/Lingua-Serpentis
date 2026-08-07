//! `T lista` — the only recursive type, and the only place a kind code is read
//! at run time.
//!
//! Lists are **1-based**, and every element operation goes through `ekind`:
//! freeing, copying, comparing and printing a list are one walk each, dispatched
//! on the container's descriptor rather than on a tag per element. Nesting is
//! unbounded and needs no interning table, because the element kind of `k` is
//! `k - GRADUS` in closed form.
//!
//! Construction is `liss_list_new` plus **direct stores**: codegen allocates a
//! header of the final length and writes each element word at
//! `ONUS + 8*i` itself. There is no per-element setter, because there is
//! nothing for one to do that a store does not.

use core::ffi::c_char;
use core::ptr;

use liss_runtime::Roman;

use crate::defectus::{index_extra_fines, memoria_exhausta, memoria_violata};
use crate::memoria::{abolere, allocare_nulla, verifica};
use crate::numeri::{int_novus, roman_of};
use crate::{GRADUS, LISS_MAGIC, LissHdr, LissInt, LissList, LissText, ONUS, basis, profunditas};

/// Check the magic *and* that this header names a list.
///
/// # Safety
/// `p` must be null or readable for a header.
pub(crate) unsafe fn verifica_seriem(p: *const LissList) {
    unsafe { verifica(p.cast()) };
    let kind = unsafe { (*p).h.kind };
    if profunditas(kind) == 0 || kind as u64 != unsafe { (*p).ekind } + GRADUS as u64 {
        memoria_violata();
    }
}

/// The item array. `ONUS` for a list as for the other two layouts — one
/// constant for codegen, not three.
///
/// # Safety
/// `p` must be a live `LissList`.
#[inline]
pub(crate) unsafe fn elementa(p: *const LissList) -> *mut u64 {
    unsafe { p.cast::<u8>().cast_mut().add(ONUS).cast::<u64>() }
}

/// Bytes for a list of `len` elements, refusing an overflow rather than
/// wrapping into a short allocation.
fn quantum(len: u64) -> usize {
    let Ok(len) = usize::try_from(len) else {
        memoria_exhausta()
    };
    let Some(payload) = len.checked_mul(8) else {
        memoria_exhausta()
    };
    let Some(total) = ONUS.checked_add(payload) else {
        memoria_exhausta()
    };
    total
}

/// A list header of `len` zeroed element words, element kind `ekind`.
///
/// The header's own `kind` is `ekind + GRADUS`, computed here so codegen passes
/// one immediate rather than two that could disagree.
///
/// # Safety
/// Returns a fresh owned pointer. Every element word must be stored before
/// anything reads the list.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn liss_list_new(ekind: u64, len: u64) -> *mut LissList {
    let Ok(k) = u32::try_from(ekind) else {
        memoria_violata()
    };
    let Some(kind) = k.checked_add(GRADUS) else {
        memoria_violata()
    };
    let p = unsafe { allocare_nulla(quantum(len)) }.cast::<LissList>();
    unsafe {
        (*p).h = LissHdr {
            magic: LISS_MAGIC,
            kind,
        };
        (*p).len = len;
        (*p).ekind = ekind;
    }
    p
}

/// Free one element word according to its kind.
///
/// A zero word is skipped: `Bool` and `Char` own nothing, and for the three
/// pointer kinds a null can only be an element of a list caught part-built —
/// which nothing frees, because a raise never unwinds. Skipping is what keeps
/// that case from becoming a null dereference on a path Task 9's handler may
/// yet take.
///
/// # Safety
/// `word` must be a live value of kind `ekind` the caller owns.
unsafe fn liberare_verbum(word: u64, ekind: u32) {
    if word == 0 {
        return;
    }
    if profunditas(ekind) > 0 {
        unsafe { liss_list_free(word as *mut LissList) };
        return;
    }
    match basis(ekind) {
        crate::KIND_BOOL | crate::KIND_CHAR => {}
        crate::KIND_INT | crate::KIND_FRAC_UNCIA | crate::KIND_FRAC_SCRIPULUM => {
            unsafe { crate::numeri::liss_int_free(word as *mut LissInt) };
        }
        crate::KIND_TEXT => unsafe { crate::textus::liss_text_free(word as *mut LissText) },
        _ => memoria_violata(),
    }
}

/// Copy one element word according to its kind — a deep copy for the three
/// pointer kinds, the word itself for the two immediates.
///
/// # Safety
/// `word` must be a live value of kind `ekind`.
unsafe fn copiare_verbum(word: u64, ekind: u32) -> u64 {
    if word == 0 {
        return 0;
    }
    if profunditas(ekind) > 0 {
        return unsafe { liss_list_copy(word as *const LissList) } as u64;
    }
    match basis(ekind) {
        crate::KIND_BOOL | crate::KIND_CHAR => word,
        crate::KIND_INT | crate::KIND_FRAC_UNCIA | crate::KIND_FRAC_SCRIPULUM => {
            unsafe { crate::numeri::liss_int_copy(word as *const LissInt) as u64 }
        }
        crate::KIND_TEXT => unsafe { crate::textus::liss_text_copy(word as *const LissText) as u64 },
        _ => memoria_violata(),
    }
}

/// Structural equality of one element word — the deep comparison `par_est`
/// has always had, since `Value`'s derived `PartialEq` recursed too.
///
/// # Safety
/// Both must be live values of kind `ekind`.
unsafe fn aequum_verbum(a: u64, b: u64, ekind: u32) -> bool {
    if profunditas(ekind) > 0 {
        return unsafe { aequae_series(a as *const LissList, b as *const LissList) };
    }
    match basis(ekind) {
        crate::KIND_BOOL | crate::KIND_CHAR => a == b,
        crate::KIND_INT | crate::KIND_FRAC_UNCIA | crate::KIND_FRAC_SCRIPULUM => unsafe {
            roman_of(a as *const LissInt) == roman_of(b as *const LissInt)
        },
        crate::KIND_TEXT => unsafe {
            crate::textus::textum_of(a as *const LissText)
                == crate::textus::textum_of(b as *const LissText)
        },
        _ => memoria_violata(),
    }
}

/// Borrowing deep equality, so the recursion does not consume what the public
/// entry point still has to free.
///
/// # Safety
/// Both must be live `LissList`s.
unsafe fn aequae_series(a: *const LissList, b: *const LissList) -> bool {
    unsafe { verifica_seriem(a) };
    unsafe { verifica_seriem(b) };
    let (la, lb) = unsafe { ((*a).len, (*b).len) };
    if la != lb || unsafe { (*a).ekind != (*b).ekind } {
        return false;
    }
    let ekind = unsafe { (*a).ekind as u32 };
    for i in 0..la as usize {
        let (wa, wb) = unsafe { (*elementa(a).add(i), *elementa(b).add(i)) };
        if !unsafe { aequum_verbum(wa, wb, ekind) } {
            return false;
        }
    }
    true
}

/// A private deep copy. **Borrows** `p` — see [`crate::numeri::liss_int_copy`].
///
/// # Safety
/// `p` must be a live `LissList`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn liss_list_copy(p: *const LissList) -> *mut LissList {
    unsafe { verifica_seriem(p) };
    let (len, ekind) = unsafe { ((*p).len, (*p).ekind) };
    let q = unsafe { liss_list_new(ekind, len) };
    for i in 0..len as usize {
        let word = unsafe { *elementa(p).add(i) };
        unsafe { *elementa(q).add(i) = copiare_verbum(word, ekind as u32) };
    }
    q
}

/// Free, recursively. Checks the magic of every header it reaches.
///
/// # Safety
/// `p` must be a live `LissList` the caller owns, and must not be freed twice.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn liss_list_free(p: *mut LissList) {
    unsafe { verifica_seriem(p) };
    let (len, ekind) = unsafe { ((*p).len, (*p).ekind as u32) };
    for i in 0..len as usize {
        unsafe { liberare_verbum(*elementa(p).add(i), ekind) };
    }
    unsafe { abolere(p.cast()) };
}

/// `longitudo(xs)` — the element count as a `ni`. Consumes `p`.
///
/// # Safety
/// `p` must be a live `LissList` the caller owns.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn liss_list_len(p: *mut LissList) -> *mut LissInt {
    unsafe { verifica_seriem(p) };
    let out = unsafe { int_novus(crate::KIND_INT, &Roman::from_u64((*p).len)) };
    unsafe { liss_list_free(p) };
    out
}

/// `xs[i]` — **1-based**, bounds-checked, returning a **deep copy**. Consumes
/// both `p` and `index`.
///
/// The copy is what `interp.rs:182` did on every element read, and it is what
/// keeps the no-aliasing invariant: after this call the element is in the list
/// and the result is a separate value the caller owns.
///
/// Out of range raises `index extra fines: {} (longitudo {})` with **both
/// numbers as Roman numerals** — the length interpolated as a bare `usize` was
/// D-12, the one place a user could see an Arabic number.
///
/// # Safety
/// Both must be live values the caller owns; `scida` must be null or
/// NUL-terminated.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn liss_list_get(
    p: *mut LissList,
    index: *mut LissInt,
    scida: *const c_char,
    linea: u64,
) -> u64 {
    unsafe { verifica_seriem(p) };
    let (len, ekind) = unsafe { ((*p).len, (*p).ekind as u32) };
    let Some(i) = (unsafe { intra_fines(index, len) }) else {
        index_extra_fines(index, len, scida, linea)
    };
    let word = unsafe { copiare_verbum(*elementa(p).add(i), ekind) };
    unsafe { crate::numeri::liss_int_free(index) };
    unsafe { liss_list_free(p) };
    word
}

/// `xs[i] = v` — 1-based and bounds-checked, freeing the element it displaces.
///
/// The **sole borrowing operation** in the ABI (spec §5.4): the list stays in
/// its slot, which is what makes `IndexAssign` a mutation rather than a
/// rebuild. `index` and `value` are consumed.
///
/// # Safety
/// As [`liss_list_get`]; `value` must be a live value of the list's element
/// kind that the caller owns.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn liss_list_put(
    p: *mut LissList,
    index: *mut LissInt,
    value: u64,
    scida: *const c_char,
    linea: u64,
) {
    unsafe { verifica_seriem(p) };
    let (len, ekind) = unsafe { ((*p).len, (*p).ekind as u32) };
    let Some(i) = (unsafe { intra_fines(index, len) }) else {
        index_extra_fines(index, len, scida, linea)
    };
    unsafe { liberare_verbum(*elementa(p).add(i), ekind) };
    unsafe { *elementa(p).add(i) = value };
    unsafe { crate::numeri::liss_int_free(index) };
}

/// `adde(xs, v)` — a new list one longer. Consumes `p` and takes ownership of
/// `value`.
///
/// **It steals the element words.** A header one longer, one `memcpy` of the
/// existing words, the new word stored, and only the *old header* freed — zero
/// element copies, because the caller already handed over a private copy of
/// every one of them. That is what keeps `adde` in a loop linear in words
/// rather than in deep copies.
///
/// # Safety
/// `p` must be a live `LissList` the caller owns; `value` must be a live value
/// of its element kind that the caller owns.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn liss_list_append(p: *mut LissList, value: u64) -> *mut LissList {
    unsafe { verifica_seriem(p) };
    let (len, ekind) = unsafe { ((*p).len, (*p).ekind) };
    let Some(auctus) = len.checked_add(1) else {
        memoria_exhausta()
    };
    let q = unsafe { liss_list_new(ekind, auctus) };
    unsafe { ptr::copy_nonoverlapping(elementa(p), elementa(q), len as usize) };
    unsafe { *elementa(q).add(len as usize) = value };
    // Only the old header. Its elements now belong to `q`, and freeing them
    // here is the one mistake this entry point exists to make impossible.
    unsafe { abolere(p.cast()) };
    q
}

/// Deep equality, consuming both: `1` or `0`.
///
/// Lists have `par_est` and `non_par_est` and no ordering, so there is no
/// three-way form to return — `Value`'s `PartialEq` recursed but its `Ord`
/// never covered `List`.
///
/// # Safety
/// Both must be live `LissList`s the caller owns.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn liss_list_eq(a: *mut LissList, b: *mut LissList) -> i8 {
    let aequa = unsafe { aequae_series(a, b) };
    unsafe { liss_list_free(a) };
    unsafe { liss_list_free(b) };
    i8::from(aequa)
}

/// The zero-based offset a 1-based subscript names, or `None` if it names none.
///
/// A subscript that does not fit `usize` at all — a hundred-digit `ni`, which
/// this language can spell — is out of range for every list, so `to_usize`
/// returning `None` and a subscript of zero take the same branch.
///
/// `then`, not `then_some`: `then_some` evaluates its argument eagerly, so the
/// zero subscript of `errores/index-infra-fines` computed `0usize - 1` before
/// the guard could reject it. That is an overflow panic in debug and a wrap to
/// `usize::MAX` in release — and a panic crossing an `extern "C"` boundary
/// aborts, which is `SIGABRT` and exit 134, exactly the shape D-04 recorded as
/// outside the error model entirely.
///
/// # Safety
/// `index` must be a live `LissInt`.
unsafe fn intra_fines(index: *const LissInt, len: u64) -> Option<usize> {
    let i = unsafe { roman_of(index) }.to_usize()?;
    (i >= 1 && i as u64 <= len).then(|| i - 1)
}
