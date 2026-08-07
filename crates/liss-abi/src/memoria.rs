//! Allocation, and the proof that it balances (spec §7.5).
//!
//! Every heap object in a compiled program passes through exactly one
//! [`allocare`]/[`liberare`] pair, which maintain a live count. `main` checks
//! it on the success path; a nonzero count is `memoria non liberata est` and
//! exit **1** — see [`crate::defectus::liss_exit`].
//!
//! The counter is **always on, in every build**: one relaxed atomic against a
//! `malloc`. Making it debug-only would have meant "debug build" becoming a
//! property of *which prebuilt musl archive is embedded*, doubling the artefact
//! matrix; and an environment-variable gate would make behaviour depend on the
//! environment. If this counter ever fires the compiler is broken, and a cheap
//! loud invariant is the right shape for that.

use core::sync::atomic::{AtomicUsize, Ordering};

use crate::{LISS_MAGIC, LissHdr};

/// How many blocks [`allocare`] has handed out and [`liberare`] has not taken
/// back. `Relaxed` throughout: compiled programs are single-threaded, so this
/// synchronises nothing — it is a counter, not a lock.
static VIVAE: AtomicUsize = AtomicUsize::new(0);

/// The live-allocation count. Exposed so the runtime's `main` (Task 9) and the
/// ABI's own tests can read it without reaching into this module.
#[unsafe(no_mangle)]
pub extern "C" fn liss_live_count() -> u64 {
    VIVAE.load(Ordering::Relaxed) as u64
}

/// `bytes` of uninitialised, 8-aligned storage, counted.
///
/// `malloc` is 16-aligned on both targets, which covers every layout here
/// (all three are `align_of` 8).
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
/// Used for `LissList`, whose items codegen fills with direct stores after the
/// header comes back: zeroing means a list caught part-built — because an
/// element expression raised between two stores — holds null words rather than
/// whatever `malloc` returned. Nothing frees such a list (a raise never
/// unwinds, spec §6), but "deterministic" beats "whatever was on the heap" for
/// a structure the signal handler of §8 may yet walk past.
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
/// This is the **read** guard, and null is fatal here: nothing in a compiled
/// program can read a value it does not have. Freeing is the other case, and
/// [`abolere`] treats null as a no-op — see the rule stated there.
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
/// **A null pointer is a no-op**, exactly as C's `free` is, and this is a
/// contract of the ABI rather than defensive padding.
///
/// The reason is §7.3's teardown rule meeting how the checker builds
/// `Block::declares`: `pop_scope` yields every slot bound *anywhere* in the
/// block, wherever the `Let` sits. So in
///
/// ```text
/// munus initium reddit ni {
///   si (verus) { redde nullus; }
///   voc s = "salve";
///   redde nullus;
/// }
/// ```
///
/// `body.declares` holds `s`, and the `redde` inside the `si` frees a slot
/// whose `Let` never ran. Cranelift requires a `Variable` be defined on every
/// path reaching a use, so codegen defines every slot to zero on entry — and
/// that free sees null. Refusing it would turn a legal program that prints
/// nothing and exits 0 into exit 1 with a corruption diagnostic, and the
/// robustness harness would report an invariant violation.
///
/// The alternative is codegen proving per-slot, per-path initialisation, which
/// is precisely the bookkeeping §5.4 exists to abolish. One branch here keeps
/// §7.3.4's "one uniform walk over `Block::declares`, with no special case"
/// literally true.
///
/// # The magic is cleared with a **volatile** write, and that is not decoration
///
/// Clearing it before the free is what makes an immediate double free — or a
/// read after one — deterministic rather than probabilistic: the block reads as
/// not-a-`LISS`-object from the moment it is handed back, instead of only once
/// `malloc` has given it out again and something has overwritten the first four
/// bytes.
///
/// A plain `(*h).magic = 0` **does not do that in a release build, and did not**
/// (D-27). `free` ends the object's lifetime, so LLVM's dead-store elimination
/// removes any store that immediately precedes it — correctly, by the language
/// rules, since nothing may legally observe the result. Nothing *legal*: this
/// store exists precisely for the illegal reader. The shipped
/// `liss_text_free` disassembled to a null test, a magic test, a kind test,
/// `call free` and the counter decrement, with **no write to the magic
/// anywhere**, and the same held of `liss_int_free`, `liss_list_free`'s element
/// loop and this function. It was uniform, not one inlining accident.
///
/// `write_volatile` is the fix and the whole of it: a volatile access is an
/// observable side effect that no optimiser may remove, and it costs one `movl`
/// that was always meant to be there.
///
/// This matters out of proportion to its size. Spec §5.2 makes the magic word
/// the *entire* answer to the live counter's blind spot — an extra free plus a
/// missing free nets to zero, and only the magic can tell that apart from a
/// balanced program. Codegen's null-store rule (`emissio.rs`'s `libera`) makes a
/// redundant free of a *slot* benign; a redundant free of a **temporary** has no
/// such protection and this is what catches it.
///
/// # Safety
/// `h` must be null, or a header from [`allocare`] that has not been freed.
pub unsafe fn abolere(h: *mut LissHdr) {
    if h.is_null() {
        return;
    }
    unsafe {
        verifica(h);
        core::ptr::write_volatile(&raw mut (*h).magic, 0);
        liberare(h.cast());
    }
}
