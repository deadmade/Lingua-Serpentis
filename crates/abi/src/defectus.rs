use core::ffi::{CStr, c_char};
use core::ptr;

use runtime::Roman;

use crate::LissInt;
use crate::numeri::roman_of;
use crate::scriptio::{Erratum, erratum};

/// Raise, and never return. `fmt` is the sentence; up to two `{}` holes are
/// filled from `arg0` and `arg1` as Roman numerals. Null and empty `file` both
/// mean "no file", and `line` 0 means "no line".
///
/// # Safety
/// `file` and `fmt` must be null or NUL-terminated; `arg0`/`arg1` must be null
/// or live `LissInt`s.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn liss_panic(
    file: *const c_char,
    line: u64,
    fmt: *const c_char,
    arg0: *const LissInt,
    arg1: *const LissInt,
) -> ! {
    let nomen = unsafe { chorda(file) };
    let forma = unsafe { chorda(fmt) };

    // Validate before writing a byte: a hole with no argument renders as a
    // literal `{}`, which `judge` would accept as a plausible diagnostic.
    if lacunae(forma) != usize::from(!arg0.is_null()) + usize::from(!arg1.is_null()) {
        memoria_violata();
    }

    // D-08. `effunde` and not `liss_flush`, which is the success path's and
    // exits 0.
    let _ = unsafe { crate::scriptio::effunde() };

    // The sentence first, then where it happened: the first two levels of the
    // grammar in 2026-08-09-caret-diagnostics-design.md §3. The third — the source
    // line under a caret — needs the source, which a running program does not have,
    // and each level is independent of the one below precisely so this can stop here.
    erratum(b"erratum: ");

    let mut argumenta = [arg0, arg1].into_iter().filter(|p| !p.is_null());
    let mut reliquum = forma;
    while let Some(i) = positio(reliquum) {
        erratum(&reliquum[..i]);
        match argumenta.next() {
            Some(p) => unsafe { roman_of(p) }.display_ad(&mut Erratum),
            None => memoria_violata(),
        }
        reliquum = &reliquum[i + 2..];
    }
    erratum(reliquum);
    erratum(b"\n");

    match (nomen.is_empty(), line) {
        (false, 0) => {
            erratum(b"  in '");
            erratum(nomen);
            erratum(b"'\n");
        }
        (false, n) => {
            erratum(b"  in '");
            erratum(nomen);
            erratum(b"', linea ");
            Roman::from_u64(n).display_ad(&mut Erratum);
            erratum(b"\n");
        }
        (true, 0) => {}
        (true, n) => {
            erratum(b"  in linea ");
            Roman::from_u64(n).display_ad(&mut Erratum);
            erratum(b"\n");
        }
    }

    unsafe { libc::_exit(1) }
}

/// A C string's bytes, borrowed; empty for null.
///
/// # Safety
/// `p` must be null or NUL-terminated.
unsafe fn chorda<'a>(p: *const c_char) -> &'a [u8] {
    if p.is_null() {
        b""
    } else {
        unsafe { CStr::from_ptr(p) }.to_bytes()
    }
}

/// Byte offset of the next `{}`.
fn positio(forma: &[u8]) -> Option<usize> {
    forma.windows(2).position(|par| par == b"{}")
}

/// How many `{}` holes a format string has.
fn lacunae(mut forma: &[u8]) -> usize {
    let mut n = 0;
    while let Some(i) = positio(forma) {
        n += 1;
        forma = &forma[i + 2..];
    }
    n
}

/// End the program: flush, check the live-allocation count, exit. A leak is
/// exit **1**, not a distinctive code: `judge` treats anything outside
/// {0, 1, 101} as the program's own choice.
///
/// # Safety
/// Ends the process. Nothing runs after it.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn liss_exit(status: i32) -> ! {
    unsafe { crate::scriptio::liss_flush() };
    if crate::memoria::liss_live_count() != 0 {
        unsafe {
            liss_panic(
                ptr::null(),
                0,
                c"memoria non liberata est".as_ptr(),
                ptr::null(),
                ptr::null(),
            )
        }
    }
    unsafe { libc::_exit(status) }
}

/// One helper for the `ni`, `mnr` and `nr` sites, so the sentence stays unique
/// in the workspace and `catalogus.rs` can attribute it.
pub(crate) fn divisio_per_nullum(file: *const c_char, line: u64) -> ! {
    unsafe {
        liss_panic(
            file,
            line,
            c"divisio per nullum".as_ptr(),
            ptr::null(),
            ptr::null(),
        )
    }
}

/// `%` by zero. Deliberately *not* folded into `divisio_per_nullum` with an
/// interpolated operator: one pattern with a hole would cost the catalogue the
/// ability to tell the two apart.
pub(crate) fn modulus_per_nullum(file: *const c_char, line: u64) -> ! {
    unsafe {
        liss_panic(
            file,
            line,
            c"modulus per nullum".as_ptr(),
            ptr::null(),
            ptr::null(),
        )
    }
}

/// A list subscript outside `I..=longitudo`, from either `xs[i]` or
/// `xs[i] = v`. **Both numbers are Roman** — D-12.
pub(crate) fn index_extra_fines(
    idx: *const LissInt,
    longitudo: u64,
    file: *const c_char,
    line: u64,
) -> ! {
    // Allocated on the raising path and never freed: nothing unwinds.
    let n = unsafe { crate::numeri::int_novus(crate::KIND_INT, &Roman::from_u64(longitudo)) };
    unsafe {
        liss_panic(
            file,
            line,
            c"index extra fines: {} (longitudo {})".as_ptr(),
            idx,
            n,
        )
    }
}

/// A header without [`LISS_MAGIC`], a digit above 9, text that is not UTF-8, a
/// kind code naming no type. The tail (`verificatio`, against the signal
/// handler's `signum`) must stay distinct: `catalogus.rs` grants `fixtured`
/// only to a workspace-unique pattern.
pub(crate) fn memoria_violata() -> ! {
    unsafe {
        liss_panic(
            ptr::null(),
            0,
            c"defectus internus: memoria violata est: verificatio".as_ptr(),
            ptr::null(),
            ptr::null(),
        )
    }
}

/// `malloc` returned null, or a length computation overflowed `usize`.
/// Permanently UNCOVERED by design: no fixture can exhaust the address space.
pub(crate) fn memoria_exhausta() -> ! {
    unsafe {
        liss_panic(
            ptr::null(),
            0,
            c"memoria exhausta est".as_ptr(),
            ptr::null(),
            ptr::null(),
        )
    }
}

