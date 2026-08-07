//! `ni`, `mnr` and `nr` — the boxed integer and the two grain counts.
//!
//! All three are a `*mut LissInt`. `mnr` counts unciae (1/12 of the `as`) and
//! `nr` counts scripula (1/288); the grain is a property of the *type*, so it
//! never travels in the value. That is why the three share `add`, `sub`, `neg`
//! and `cmp` outright and differ only in `mul`, `div` and formatting.
//!
//! `ni` is boxed unconditionally. A small-integer optimisation would be a
//! tagged value, which the ABI rules out, and unconditional boxing makes the
//! ownership rules uniform — every `ni` is exactly one allocation to free, with
//! no branch. The consequence, that `ni i = i + °I` allocates twice per
//! iteration, is stated in the Non-Goals rather than hidden.

use core::ffi::c_char;
use core::ptr;

use liss_runtime::{Roman, fraction};

use crate::defectus::{divisio_per_nullum, memoria_exhausta, memoria_violata, modulus_per_nullum};
use crate::memoria::{abolere, allocare, verifica};
use crate::{LISS_MAGIC, LissHdr, LissInt, ONUS, basis, profunditas};

/// Check the magic *and* that this header names a scalar number, which the
/// magic alone cannot: a `LissText` handed to an integer entry point has a
/// perfectly good magic and a payload that is not digits.
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

/// Read a `LissInt` back into a `Roman` — one `memcpy`, plus the digit check.
///
/// **`Roman::from_digits` returns `Option`, and that is the whole point of this
/// boundary.** A `LissInt` arriving with a digit above 9 is corrupt; left
/// unchecked it panics inside `render_digit`'s `unreachable!` or silently wraps
/// the `u8` addition in `mag_add` in a release build. Corrupt input must
/// produce a Latin diagnostic, never a panic and never silent wrapping.
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
/// The zero-copy alternative, a header pointing at the `Vec`'s own buffer,
/// costs a second allocation per integer. For numbers almost always under
/// twenty digits, a twenty-byte copy beats a `malloc`.
///
/// # Safety
/// Returns a fresh owned pointer the caller must free.
pub(crate) unsafe fn int_novus(kind: u32, r: &Roman) -> *mut LissInt {
    let digiti = r.digits();
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
        // `Roman` exposes no sign accessor, and does not need one: its `Ord`
        // orders every negative below zero and forces zero non-negative, so
        // this is the canonical sign by construction.
        (*p).neg = u64::from(*r < Roman::zero());
        ptr::copy_nonoverlapping(digiti.as_ptr(), p.cast::<u8>().add(ONUS), digiti.len());
    }
    p
}

/// The kind stamped on a `LissInt`, so arithmetic can carry it to its result.
///
/// Nothing in the runtime dispatches on a scalar's own kind — a list reads its
/// `ekind`, never its elements' headers — but propagating it keeps every header
/// honest rather than leaving `mnr` values labelled `ni`, and costs one load.
///
/// # Safety
/// `p` must be a live `LissInt`.
unsafe fn genus(p: *const LissInt) -> u32 {
    unsafe { (*p).h.kind }
}

/// An integer constant, built from a digit blob codegen emits as static data.
///
/// `digiti` are little-endian and each must be `0..=9`; `len` may be 0, which
/// is `nullus`. Trailing zeros are stripped on the way in, so the canonical
/// form is what lands on the heap regardless of how the blob was written.
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
/// without emptying it (spec §7.2 — `Load` is the only unconditional copy
/// site).
///
/// A raw copy of header plus payload rather than a `Roman` round-trip: it
/// preserves the kind, allocates once, and does not need the digits to be
/// valid — a corrupt integer is caught wherever it is *used*, by [`roman_of`].
///
/// # Safety
/// `p` must be a live `LissInt`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn liss_int_copy(p: *const LissInt) -> *mut LissInt {
    unsafe { verifica_numerum(p) };
    // `len` has passed the magic check but is not thereby trustworthy — the
    // magic says "a Lingua Serpentis object was here", not "its length field
    // is sane". Unchecked, a corrupt `len` near `usize::MAX` wraps to a small
    // allocation carrying a huge length, and the next read walks off it.
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

/// Free. Checks the magic first — every free does. **Null is a no-op**; see
/// [`crate::memoria::abolere`] for why that is a contract rather than padding.
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

/// `a + b`. Consumes both. Shared by `ni`, `mnr` and `nr`: adding two counts of
/// the same grain is adding two integers.
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

/// `a / b`, truncating toward zero, for `ni` only. Consumes both.
///
/// Raises `divisio per nullum` when `b` is zero, and then neither operand is
/// freed: a raise never unwinds, so everything live leaks by design and the
/// leak assertion runs only on the success path.
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
/// One entry point rather than four (`par_est`, `non_par_est`, `maior_est`,
/// `minor_est`): codegen compares the result against an immediate, which is one
/// instruction, and the runtime carries one implementation of the ordering.
///
/// # Safety
/// As [`liss_int_add`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn liss_int_cmp(a: *mut LissInt, b: *mut LissInt) -> i8 {
    let (ra, rb) = unsafe { (roman_of(a), roman_of(b)) };
    unsafe { liss_int_free(a) };
    unsafe { liss_int_free(b) };
    match ra.cmp(&rb) {
        core::cmp::Ordering::Less => -1,
        core::cmp::Ordering::Equal => 0,
        core::cmp::Ordering::Greater => 1,
    }
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
    let out = unsafe { int_novus(kind, &fraction::mul(&ra, &rb, &g)) };
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
///
/// `fraction::mul` divides by the grain and `expect`s it nonzero; a zero
/// arriving from a miscompiled call site would be a Rust panic crossing an
/// `extern "C"` boundary. Checking here turns it into a Latin exit, which is
/// the rule everywhere else in this crate.
fn exige_granum(granum: u64) -> Roman {
    if granum != 12 && granum != 288 {
        memoria_violata();
    }
    Roman::from_u64(granum)
}
