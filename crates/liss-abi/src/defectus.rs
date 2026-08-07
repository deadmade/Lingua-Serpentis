//! Raising, and ending.
//!
//! `liss_panic` never unwinds: it flushes, writes one Latin sentence to stderr
//! and `_exit`s (spec §6). Everything live leaks, exactly as the interpreter
//! dropped everything on its error path — which is why the leak assertion of
//! §7.5 runs only on the *success* path, and why unwinding for `redde` /
//! `abrumpe` / `continua` is a purely syntactic problem with no landing pads.

use core::ffi::{CStr, c_char};
use core::ptr;

use liss_runtime::Roman;

use crate::LissInt;
use crate::numeri::roman_of;

/// Raise, and never return.
///
/// `fmt` is the sentence, as a C string literal written **inline at the call
/// site** (spec §10.3). Up to two `{}` holes are filled from `arg0` and `arg1`,
/// rendered as Roman numerals — which is what lets
/// `index extra fines: {} (longitudo {})` be one literal at one site rather
/// than a message assembled somewhere the diagnostic catalogue cannot see.
///
/// `file` is the source path (null or empty for single-file mode) and `line`
/// the statement's line, both supplied by codegen: a signal handler has no line
/// table and this design emits no debug information (spec §8.4), so there is no
/// ambient location for the runtime to consult. The four shapes reproduce
/// `ProgramError`'s `Display` exactly:
///
/// ```text
/// Error in 'principale.liss', linea 3: index extra fines: X (longitudo II)
/// Error in linea 3: divisio per nullum
/// Error in 'principale.liss': ...
/// Error: memoria non liberata est
/// ```
///
/// **Termination.** Reading an argument checks its magic and its digits, and a
/// corrupt one calls [`memoria_violata`], which re-enters here — with both
/// arguments null. That inner call converts nothing, so the recursion is
/// bounded at depth two by construction rather than by a flag.
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
    // D-08: everything `scribe` produced before the failure still reaches
    // stdout. The interpreter used to discard it with the error.
    unsafe { crate::scriptio::liss_flush() };

    let mut nuntius = String::new();
    match (unsafe { scida(file) }, line) {
        (Some(p), 0) => nuntius.push_str(&format!("Error in '{p}': ")),
        (Some(p), n) => nuntius.push_str(&format!("Error in '{p}', linea {n}: ")),
        (None, 0) => nuntius.push_str("Error: "),
        (None, n) => nuntius.push_str(&format!("Error in linea {n}: ")),
    }

    let forma = if fmt.is_null() {
        String::new()
    } else {
        String::from_utf8_lossy(unsafe { CStr::from_ptr(fmt) }.to_bytes()).into_owned()
    };
    let mut argumenta = [arg0, arg1].into_iter();
    let mut reliquum = forma.as_str();
    while let Some(i) = reliquum.find("{}") {
        nuntius.push_str(&reliquum[..i]);
        match argumenta.next() {
            Some(p) if !p.is_null() => nuntius.push_str(&unsafe { roman_of(p) }.display()),
            // A hole with no argument is a bug in the caller, not something a
            // user should see garbage for. Leaving it verbatim keeps the
            // sentence recognisably the one the catalogue records.
            _ => nuntius.push_str("{}"),
        }
        reliquum = &reliquum[i + 2..];
    }
    nuntius.push_str(reliquum);
    nuntius.push('\n');

    unsafe { crate::scriptio::liss_scribe_erratum(nuntius.as_ptr(), nuntius.len() as u64) };
    unsafe { libc::_exit(1) }
}

/// End the program: flush, check the live-allocation count, exit.
///
/// Exit **1** for a leak specifically, and not a distinctive code such as 70.
/// `judge` treats any status outside {0, 1, 101} as "this came from `initium`,
/// therefore it is the program's own choice and legitimate" — a leak reported
/// as 70 would be silently accepted by the one harness most meant to catch it.
/// The diagnostic text carries the distinction; the exit code carries the
/// invariant.
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

/// A `ni` or fraction was divided by zero. One helper for the `ni`, `mnr` and
/// `nr` sites, so the sentence is unique in the workspace and the catalogue can
/// attribute it — the same consolidation `interp.rs` made for D-24.
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
/// interpolated operator: it is a different diagnostic, and one pattern with a
/// hole would cost the catalogue the ability to tell the two apart.
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

/// A list subscript outside `I..=longitudo`, from either the reading form
/// (`xs[i]`) or the assigning form (`xs[i] = v`).
///
/// **Both numbers are Roman.** The length interpolated as a bare `usize` was
/// D-12 — the one place in the language a user could see an Arabic number. An
/// empty list reads `longitudo nullus`.
pub(crate) fn index_extra_fines(
    idx: *const LissInt,
    longitudo: u64,
    file: *const c_char,
    line: u64,
) -> ! {
    // Allocated on the raising path and never freed: nothing unwinds, and the
    // leak assertion runs only where the program succeeded.
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
/// kind code naming no type. Every one of them means a pointer reached this
/// crate that no compiled program could have produced, so the honest report is
/// an internal defect rather than a language-level error.
///
/// Structural — no file, no line — which `parse_diagnostic` classifies as
/// `Diagnostic::Structural` and `judge` accepts without a location check. It
/// shares its wording with the signal handler's non-stack branch (spec §8.2),
/// which is the same fact about the same program.
pub(crate) fn memoria_violata() -> ! {
    unsafe {
        liss_panic(
            ptr::null(),
            0,
            c"defectus internus: memoria violata est".as_ptr(),
            ptr::null(),
            ptr::null(),
        )
    }
}

/// `malloc` returned null, or a length computation overflowed `usize`.
///
/// New vocabulary, and the one sentence in this crate the spec does not already
/// name. It is needed because the alternative to reporting it is ignoring a
/// null from `malloc`, and permanently UNCOVERED by design: no fixture can
/// exhaust a 4 MiB-floor address space on purpose.
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

/// The source path a diagnostic is attributed to, or `None`.
///
/// Both null and empty mean "no file": spec §4.1 lowers `Func::file: None` to a
/// zero-length name, and a codegen that emits a null pointer instead means the
/// same thing. Accepting both costs one comparison and removes a way for the
/// two halves of the contract to disagree.
unsafe fn scida(file: *const c_char) -> Option<String> {
    if file.is_null() {
        return None;
    }
    let bytes = unsafe { CStr::from_ptr(file) }.to_bytes();
    if bytes.is_empty() {
        None
    } else {
        Some(String::from_utf8_lossy(bytes).into_owned())
    }
}
