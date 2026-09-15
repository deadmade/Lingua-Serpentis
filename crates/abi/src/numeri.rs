//! `ni`, `mnr` and `nr` — all three a `*mut LissInt`; the grain is fixed by the type.

use core::ffi::c_char;
use core::ptr;

use runtime::{Roman, fraction};

use crate::defectus::{
    divisio_per_nullum, memoria_exhausta, memoria_violata, modulus_per_nullum, numerus_nimis_magnus,
};
use crate::memoria::{abolere, allocare, verifica};
use crate::{LISS_MAGIC, LissHdr, LissInt, ONUS, basis, profunditas};

/// Check the magic *and* that this header names a scalar number.
///
/// # Safety
/// `p` must be null or readable for a header.
pub(crate) unsafe fn verifica_numerum(p: *const LissInt) {
    unsafe { verifica(p.cast()) };
    let kind = unsafe { (*p).h.kind };
    if profunditas(kind) != 0
        || !matches!(
            basis(kind),
            crate::KIND_INT | crate::KIND_FRAC_UNCIA | crate::KIND_FRAC_SCRIPULUM
        )
    {
        memoria_violata();
    }
}

/// Read a `LissInt` back into a `Roman` — one `memcpy`, plus the digit check
/// that turns a corrupt payload into a Latin diagnostic rather than a panic.
///
/// # Safety
/// `p` must be a live `LissInt`.
pub(crate) unsafe fn roman_of(p: *const LissInt) -> Roman {
    unsafe { verifica_numerum(p) };
    let (len, neg) = unsafe { ((*p).len as usize, (*p).neg != 0) };
    let digiti = unsafe { core::slice::from_raw_parts(p.cast::<u8>().add(ONUS), len) };
    match Roman::from_digits(neg, digiti) {
        Some(r) => r,
        None => memoria_violata(),
    }
}

/// Box a `Roman` as a `LissInt` of the given kind — the other `memcpy`.
///
/// # Safety
/// Returns a fresh owned pointer the caller must free.
pub(crate) unsafe fn int_novus(kind: u32, r: &Roman) -> *mut LissInt {
    let digiti = r.digits();
    // D-20
    if digiti.len() > 38 {
        numerus_nimis_magnus()
    }
    let Some(bytes) = ONUS.checked_add(digiti.len()) else {
        memoria_exhausta()
    };
    let p = unsafe { allocare(bytes) }.cast::<LissInt>();
    unsafe {
        (*p).h = LissHdr {
            magic: LISS_MAGIC,
            kind,
        };
        (*p).len = digiti.len() as u64;
        // `Roman`'s `Ord` orders every negative below zero and forces zero
        // non-negative, so this is the canonical sign by construction.
        (*p).neg = u64::from(*r < Roman::zero());
        ptr::copy_nonoverlapping(digiti.as_ptr(), p.cast::<u8>().add(ONUS), digiti.len());
    }
    p
}

/// The kind stamped on a `LissInt`, so arithmetic can carry it to its result.
///
/// # Safety
/// `p` must be a live `LissInt`.
unsafe fn genus(p: *const LissInt) -> u32 {
    unsafe { (*p).h.kind }
}

/// An integer constant, built from a digit blob codegen emits as static data.
/// `digiti` are little-endian and each must be `0..=9`; `len` 0 is `nullus`.
///
/// # Safety
/// `digiti` must be readable for `len` bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn liss_int_new(
    kind: u32,
    neg: u64,
    digiti: *const u8,
    len: u64,
) -> *mut LissInt {
    let d = if len == 0 {
        &[][..]
    } else {
        unsafe { core::slice::from_raw_parts(digiti, len as usize) }
    };
    match Roman::from_digits(neg != 0, d) {
        Some(r) => unsafe { int_novus(kind, &r) },
        None => memoria_violata(),
    }
}

/// A private copy. **Borrows** `p`: this is what `Load` calls to read a slot
/// without emptying it. A raw copy of header plus payload, not a `Roman`
/// round-trip, so the digits need not be valid here.
///
/// # Safety
/// `p` must be a live `LissInt`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn liss_int_copy(p: *const LissInt) -> *mut LissInt {
    unsafe { verifica_numerum(p) };
    // The magic vouches for the object, not for its length field.
    let Ok(len) = usize::try_from(unsafe { (*p).len }) else {
        memoria_violata()
    };
    let Some(bytes) = ONUS.checked_add(len) else {
        memoria_violata()
    };
    let q = unsafe { allocare(bytes) };
    unsafe { ptr::copy_nonoverlapping(p.cast::<u8>(), q, bytes) };
    q.cast()
}

/// Free. Checks the magic first. **Null is a no-op**.
///
/// # Safety
/// `p` must be null, or a live `LissInt` the caller owns that has not been
/// freed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn liss_int_free(p: *mut LissInt) {
    if p.is_null() {
        return;
    }
    unsafe { verifica_numerum(p) };
    unsafe { abolere(p.cast()) };
}

/// `a + b`. Consumes both. Shared by `ni`, `mnr` and `nr`.
///
/// # Safety
/// Both must be live `LissInt`s the caller owns.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn liss_int_add(a: *mut LissInt, b: *mut LissInt) -> *mut LissInt {
    let (ra, rb) = unsafe { (roman_of(a), roman_of(b)) };
    let kind = unsafe { genus(a) };
    let out = unsafe { int_novus(kind, &ra.add(&rb)) };
    unsafe { liss_int_free(a) };
    unsafe { liss_int_free(b) };
    out
}

/// `a - b`. Consumes both. Shared by all three numeric types.
///
/// # Safety
/// As [`liss_int_add`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn liss_int_sub(a: *mut LissInt, b: *mut LissInt) -> *mut LissInt {
    let (ra, rb) = unsafe { (roman_of(a), roman_of(b)) };
    let kind = unsafe { genus(a) };
    let out = unsafe { int_novus(kind, &ra.sub(&rb)) };
    unsafe { liss_int_free(a) };
    unsafe { liss_int_free(b) };
    out
}

/// `a * b` for `ni` only — a fraction's product rescales, see
/// [`liss_frac_mul`]. Consumes both.
///
/// # Safety
/// As [`liss_int_add`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn liss_int_mul(a: *mut LissInt, b: *mut LissInt) -> *mut LissInt {
    let (ra, rb) = unsafe { (roman_of(a), roman_of(b)) };
    let kind = unsafe { genus(a) };
    let out = unsafe { int_novus(kind, &ra.mul(&rb)) };
    unsafe { liss_int_free(a) };
    unsafe { liss_int_free(b) };
    out
}

/// `-a`. Consumes `a`. Shared by all three numeric types.
///
/// # Safety
/// `a` must be a live `LissInt` the caller owns.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn liss_int_neg(a: *mut LissInt) -> *mut LissInt {
    let ra = unsafe { roman_of(a) };
    let kind = unsafe { genus(a) };
    let out = unsafe { int_novus(kind, &ra.neg()) };
    unsafe { liss_int_free(a) };
    out
}

/// `a / b`, truncating toward zero, for `ni` only. Consumes both. Raises
/// `divisio per nullum` when `b` is zero, and then frees neither operand.
///
/// # Safety
/// As [`liss_int_add`]; `scida` must be null or NUL-terminated.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn liss_int_div(
    a: *mut LissInt,
    b: *mut LissInt,
    scida: *const c_char,
    linea: u64,
) -> *mut LissInt {
    let (ra, rb) = unsafe { (roman_of(a), roman_of(b)) };
    let kind = unsafe { genus(a) };
    let Some((q, _)) = ra.div_rem(&rb) else {
        divisio_per_nullum(scida, linea)
    };
    let out = unsafe { int_novus(kind, &q) };
    unsafe { liss_int_free(a) };
    unsafe { liss_int_free(b) };
    out
}

/// `a % b`, the remainder's sign following the dividend — Rust's `i64` `%`.
/// Consumes both.
///
/// # Safety
/// As [`liss_int_div`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn liss_int_mod(
    a: *mut LissInt,
    b: *mut LissInt,
    scida: *const c_char,
    linea: u64,
) -> *mut LissInt {
    let (ra, rb) = unsafe { (roman_of(a), roman_of(b)) };
    let kind = unsafe { genus(a) };
    let Some((_, r)) = ra.div_rem(&rb) else {
        modulus_per_nullum(scida, linea)
    };
    let out = unsafe { int_novus(kind, &r) };
    unsafe { liss_int_free(a) };
    unsafe { liss_int_free(b) };
    out
}

/// Three-way compare, consuming both: `-1`, `0` or `1`.
///
/// # Safety
/// As [`liss_int_add`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn liss_int_cmp(a: *mut LissInt, b: *mut LissInt) -> i8 {
    let (ra, rb) = unsafe { (roman_of(a), roman_of(b)) };
    unsafe { liss_int_free(a) };
    unsafe { liss_int_free(b) };
    ra.cmp(&rb) as i8
}

/// `a * b` for `mnr`/`nr`: `(c1/g)*(c2/g)` back in grain units is
/// `trunc(c1*c2/g)`. `granum` is XII or CCLXXXVIII. Consumes both.
///
/// # Safety
/// As [`liss_int_add`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn liss_frac_mul(
    a: *mut LissInt,
    b: *mut LissInt,
    granum: u64,
) -> *mut LissInt {
    let g = exige_granum(granum);
    let (ra, rb) = unsafe { (roman_of(a), roman_of(b)) };
    let kind = unsafe { genus(a) };
    let Some(productum) = fraction::mul(&ra, &rb, &g) else {
        memoria_violata()
    };
    let out = unsafe { int_novus(kind, &productum) };
    unsafe { liss_int_free(a) };
    unsafe { liss_int_free(b) };
    out
}

/// `a / b` for `mnr`/`nr`: `trunc(c1*g/c2)`. Consumes both. Raises
/// `divisio per nullum` when the divisor count is zero.
///
/// # Safety
/// As [`liss_int_div`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn liss_frac_div(
    a: *mut LissInt,
    b: *mut LissInt,
    granum: u64,
    scida: *const c_char,
    linea: u64,
) -> *mut LissInt {
    let g = exige_granum(granum);
    let (ra, rb) = unsafe { (roman_of(a), roman_of(b)) };
    let kind = unsafe { genus(a) };
    let Some(q) = fraction::div(&ra, &rb, &g) else {
        divisio_per_nullum(scida, linea)
    };
    let out = unsafe { int_novus(kind, &q) };
    unsafe { liss_int_free(a) };
    unsafe { liss_int_free(b) };
    out
}

/// The two grains the language has, and nothing else.
fn exige_granum(granum: u64) -> Roman {
    if granum != 12 && granum != 288 {
        memoria_violata();
    }
    Roman::from_u64(granum)
}
