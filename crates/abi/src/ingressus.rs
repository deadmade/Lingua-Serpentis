use core::ffi::{c_char, c_int};

use crate::LissInt;

const MONITUM_PRAEFIXUM: &[u8] = b"monitum: ";

unsafe extern "C" {
    /// Compiled code's entry point: the lowering of the user's `munus initium
    /// reddit ni`. Returns one owned `ni`, which is this function's to free.
    fn liss_initium() -> *mut LissInt;
}

/// The program.
///
/// # Safety
/// Called by `__libc_start_main`, once, on the main thread. Never returns:
/// [`crate::defectus::liss_exit`] ends the process.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn main(_argc: c_int, _argv: *const *const c_char) -> c_int {
    // First, and before anything can recurse.
    unsafe { crate::custos::liss_custos_institue() };

    let reditus = unsafe { liss_initium() };
    // Read it, then free it: `roman_of` copies the digits out, so the pointer is
    // dead by the time `exit_status` sees the value.
    let numerus = unsafe { crate::numeri::roman_of(reditus) };
    unsafe { crate::numeri::liss_int_free(reditus) };

    let (status, monitum) = runtime::exit_status(&numerus);
    if let Some(m) = monitum {
        // D-10; pinned by conformance/monita/status-exitus-redactus.
        erratum(MONITUM_PRAEFIXUM);
        erratum(m.message.as_bytes());
        erratum(b"\n");
    }

    unsafe { crate::defectus::liss_exit(c_int::from(status)) }
}

fn erratum(octeti: &[u8]) {
    unsafe { crate::scriptio::liss_scribe_erratum(octeti.as_ptr(), octeti.len() as u64) };
}
