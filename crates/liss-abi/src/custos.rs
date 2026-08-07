//! `custos` (spec §8): stack exhaustion, kept inside the error model.
//!
//! Removing the recursion cap means deep recursion faults on the guard page,
//! and the robustness invariant does not permit signal death — `judge` maps a
//! signalled process straight to a `no abort` violation, and **D-04 recorded
//! exactly this** (`SIGABRT`, exit 134) as "outside the error model entirely".
//! So the fault is caught here and reported as an ordinary exit-1 Latin
//! diagnostic.
//!
//! **`std::rt::init` never runs** (spec §8.1). The entry point is
//! `#[unsafe(no_mangle)] extern "C" fn main`, so `lang_start` is bypassed, std
//! installs no `SIGSEGV` handler, and there is no English `has overflowed its
//! stack` to suppress and nothing to coexist with. The disposition is ours
//! outright. **Invariant:** should the entry point ever become a Rust
//! `fn main`, [`liss_custos_institue`] must run *after* `std::rt::init` so that
//! it overrides std's handler rather than being overridden by it. The test
//! target is precisely that situation and installs from `main`.
//!
//! # Getting the bounds right
//!
//! The main thread's stack **grows on demand**, so every interface that reports
//! the stack as it currently *exists* reports a small fraction of what it may
//! become, while the guard page sits at the bottom of the *potential* stack,
//! megabytes below. Measured on musl, x86_64, `RLIMIT_STACK` 16 MiB:
//!
//! | method | reports | |
//! |---|---|---|
//! | `pthread_getattr_np` + `pthread_attr_getstack` | 124 KiB | wrong |
//! | `getrlimit` against a local in `main` | 16 MiB, top ~32 KiB low | wrong |
//! | `/proc/self/maps` `[stack]` | 152 KiB | wrong |
//! | **`[stack]` high address − `RLIMIT_STACK`** | **16 MiB** | **right** |
//!
//! The first and third describe only the pages already *touched*. The second
//! gets the size right and the top wrong, because `argv`, the environment and
//! the auxv sit above `main`'s frame. So: [`fines`] takes the high address of
//! the `[stack]` mapping and subtracts `RLIMIT_STACK`, falling back to the
//! known-approximate second method only where `/proc` is not mounted, and
//! treating `RLIM_INFINITY` as bounds-unknown.
//!
//! # The handler
//!
//! It may use only `write(2)`, `_exit(2)`, atomic loads on already-initialised
//! statics, and integer comparison. No allocation, no formatting, no
//! `std::io` — it locks. **It never reads `ucontext_t`:** recovering the
//! faulting stack pointer from `uc_mcontext` would mean hand-rolled musl struct
//! offsets with no header to check them against, and `si_addr` against
//! precomputed bounds is architecture-independent — which is why this module
//! needs no per-target code beyond the two signal numbers.
//!
//! The discriminator can afford to be generous, and its failure mode is the
//! point: **both branches are well-formed exit-1 Latin diagnostics**, so a
//! misclassification yields a wrong-but-well-formed message, never an invariant
//! violation. A Lingua Serpentis program has no pointers, no FFI and no
//! `unsafe`, so the only ways compiled code can fault are stack exhaustion and
//! a compiler bug.
//!
//! # `SIGPIPE`
//!
//! Also set here, and for the same reason: `std::rt::init` never ran, so the
//! runtime does not inherit std's `SIG_IGN` and the default disposition would
//! kill the child on `prog | head`. `scriptio::scribe_ad` already treats the
//! `EPIPE` that then surfaces as an ordinary end of output and exits 0
//! (spec §6); without the disposition change that code is unreachable. This is
//! D-06 relocating from the CLI into the runtime, unchanged in behaviour.

use core::ffi::{CStr, c_char, c_int, c_void};
use core::ptr;
use core::sync::atomic::{AtomicUsize, Ordering};

/// The alternate stack, in bytes. A fixed **64 KiB**, deliberately not
/// `SIGSTKSZ`: musl's is a small compile-time constant, and on aarch64 with SVE
/// the true minimum can exceed it. 64 KiB is unconditionally enough for a
/// handler that allocates nothing.
const MENSURA_ALTERNA: usize = 64 * 1024;

/// The guard band below the low bound: **1 MiB**, not one page. Linux's
/// `stack_guard_gap` defaults to 256 pages, so a fault caused by stack growth
/// can land that far below the mapping.
const LIMES: usize = 1 << 20;

/// The page size to assume if `sysconf` cannot say — never the answer, only a
/// floor. aarch64 kernels commonly ship 16 KiB and 64 KiB pages, which is why
/// [`pagina`] asks rather than assuming; but `_SC_PAGESIZE` cannot fail on
/// Linux, and the 1 MiB band swallows the difference either way.
const PAGINA_MINIMA: usize = 4096;

/// Not a `c"…"` literal, and that is deliberate: `tests/abi.rs` reads every one
/// of those out of this crate's source as a user-visible Latin sentence, and a
/// path is neither.
const VIA_MAPPARUM: &[u8] = b"/proc/self/maps\0";

/// `Error: ` — the prefix `parse_diagnostic` looks for.
const NUNTIUS_PRAEFIXUM: &[u8] = b"Error: ";

/// The stack of calls has been exhausted. Keeping the `vocatio` root means a
/// reader who knew the interpreter's `vocatio nimis alte implicata` recognises
/// its successor. Structural — no file, no line — because a signal handler has
/// a return address and no line table (spec §8.4).
const NUNTIUS_ACERVUS: &CStr = c"acervus vocationum exhaustus est";

/// A fault nowhere near the stack, which in a language with no pointers, no FFI
/// and no `unsafe` can only be a compiler bug.
///
/// The sentence is `defectus::memoria_violata`'s, written out a second time
/// rather than shared: that helper reaches it through `liss_panic`, which this
/// handler must not call, and §10.3 wants the literal inline at the site the
/// catalogue attributes it to. This site is not one — the marker is
/// `liss_panic(` — so the duplicate costs the catalogue nothing, and
/// `tests/abi.rs`'s sentence list deduplicates it.
const NUNTIUS_MEMORIA: &CStr = c"defectus internus: memoria violata est";

/// 16-byte aligned, because the kernel hands the handler this buffer as a stack
/// and the ABI wants the frame aligned.
#[repr(C, align(16))]
struct AcervusAlternus([u8; MENSURA_ALTERNA]);

/// The alternate stack. **`static`, not allocated:** by the time the handler
/// runs there is no stack left to allocate one on. All-zero, so it costs `.bss`
/// and not image size.
static mut ALTERNUS: AcervusAlternus = AcervusAlternus([0; MENSURA_ALTERNA]);

/// The low bound of the stack, computed once at initialisation and **never in
/// the handler**. Zero means bounds-unknown, which the handler reports as
/// exhaustion — in a program whose only unbounded recursion is user-written
/// that is overwhelmingly the right guess, and a wrong guess still satisfies
/// the invariant.
static FUNDUS: AtomicUsize = AtomicUsize::new(0);

/// The page size, likewise computed once.
static PAGINA: AtomicUsize = AtomicUsize::new(0);

/// Install the guard: bounds, alternate stack, `SIGSEGV`/`SIGBUS`, `SIGPIPE`.
///
/// Everything expensive happens here, before `liss_initium`. Failure is silent
/// and leaves the default dispositions — the pre-Task-9 behaviour — because
/// there is nothing a program can usefully do about a kernel that refuses
/// `sigaltstack`, and reporting it would need a sentence the catalogue would
/// then have to carry for a case that cannot arise: both calls take a valid
/// signal number and a `static` buffer of a size the kernel already accepts.
///
/// # Safety
/// Call **once**, from the main thread, before any other entry point. The
/// bounds describe the `[stack]` mapping, and the alternate stack is per-thread.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn liss_custos_institue() {
    let pagina = pagina();
    PAGINA.store(pagina, Ordering::Relaxed);
    if let Some((fundus, _)) = fines() {
        FUNDUS.store(fundus, Ordering::Relaxed);
    }

    // D-06, relocated: see the module doc.
    unsafe { libc::signal(libc::SIGPIPE, libc::SIG_IGN) };

    unsafe {
        let mut alternus: libc::stack_t = core::mem::zeroed();
        alternus.ss_sp = (&raw mut ALTERNUS).cast::<c_void>();
        alternus.ss_size = MENSURA_ALTERNA;
        alternus.ss_flags = 0;
        if libc::sigaltstack(&alternus, ptr::null_mut()) != 0 {
            // A handler with no stack of its own would fault on entry, which is
            // worse than not installing one.
            return;
        }

        let mut actio: libc::sigaction = core::mem::zeroed();
        actio.sa_sigaction = manus as *const () as usize;
        // `SA_ONSTACK` because there is no stack left, `SA_SIGINFO` because
        // `si_addr` is the whole discriminator. **Both signals**: on aarch64 a
        // stack fault may arrive as either.
        actio.sa_flags = libc::SA_ONSTACK | libc::SA_SIGINFO;
        libc::sigemptyset(&mut actio.sa_mask);
        libc::sigaction(libc::SIGSEGV, &actio, ptr::null_mut());
        libc::sigaction(libc::SIGBUS, &actio, ptr::null_mut());
    }
}

/// The stack bounds as `(low, high)`, derived fresh. Pure: it installs nothing
/// and stores nothing, so tests can compare it against `/proc/self/maps`
/// without arming a handler.
pub fn fines() -> Option<(usize, usize)> {
    let pagina = pagina();
    match fines_ex_mappa(pagina) {
        Some(fines) => Some(fines),
        // Known-approximate, and reached only where `/proc` is not mounted:
        // `argv`, the environment and the auxv sit above `main`'s frame —
        // roughly 32 KiB — so the top comes out low and the floor with it.
        None => fines_ex_limite(&raw const pagina as usize, pagina),
    }
}

/// `sysconf(_SC_PAGESIZE)`, never a literal.
fn pagina() -> usize {
    let quaesitum = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
    match usize::try_from(quaesitum) {
        Ok(n) if n.is_power_of_two() => n,
        _ => PAGINA_MINIMA,
    }
}

/// The method that won: the `[stack]` mapping's high address, minus
/// `RLIMIT_STACK`.
fn fines_ex_mappa(pagina: usize) -> Option<(usize, usize)> {
    let culmen = culmen_mappae()?;
    let modus = modus_acervi()?;
    if modus >= culmen {
        return None;
    }
    Some(((culmen - modus) & !(pagina - 1), culmen))
}

/// The fallback: `RLIMIT_STACK` measured down from a local, whose page rounded
/// up stands in for the top.
fn fines_ex_limite(locus: usize, pagina: usize) -> Option<(usize, usize)> {
    let modus = modus_acervi()?;
    let culmen = locus.checked_add(pagina - 1)? & !(pagina - 1);
    if modus > culmen {
        return None;
    }
    Some((culmen - modus, culmen))
}

/// `RLIMIT_STACK`, in bytes. `None` for `RLIM_INFINITY`, which is
/// bounds-unknown: there is no floor to compare against.
fn modus_acervi() -> Option<usize> {
    let mut modus: libc::rlimit = unsafe { core::mem::zeroed() };
    if unsafe { libc::getrlimit(libc::RLIMIT_STACK, &mut modus) } != 0 {
        return None;
    }
    if modus.rlim_cur == libc::RLIM_INFINITY || modus.rlim_cur == 0 {
        return None;
    }
    usize::try_from(modus.rlim_cur).ok()
}

/// The high address of the `[stack]` mapping.
///
/// Read with `open`/`read` over a fixed buffer rather than through `std::fs`:
/// this crate is `core` and `libc` throughout, and a `/proc` read that allocates
/// would be one more thing to reason about on a path that runs before the
/// program does.
fn culmen_mappae() -> Option<usize> {
    let fd = unsafe {
        libc::open(
            VIA_MAPPARUM.as_ptr().cast::<c_char>(),
            libc::O_RDONLY | libc::O_CLOEXEC,
        )
    };
    if fd < 0 {
        return None;
    }
    let mut lectio = [0u8; 4096];
    let mut linea = [0u8; 256];
    let mut longitudo = 0usize;
    let mut inventum = None;
    while inventum.is_none() {
        let acceptum =
            unsafe { libc::read(fd, lectio.as_mut_ptr().cast::<c_void>(), lectio.len()) };
        if acceptum < 0 {
            if unsafe { *libc::__errno_location() } == libc::EINTR {
                continue;
            }
            break;
        }
        let acceptum = match usize::try_from(acceptum) {
            Ok(0) | Err(_) => break,
            Ok(n) => n,
        };
        for &octetus in lectio.get(..acceptum)? {
            if octetus != b'\n' {
                // A line longer than the buffer is truncated, and then simply
                // fails to end with `[stack]` — which is correct, because the
                // long lines are file mappings and the anonymous ones are short.
                if longitudo < linea.len() {
                    linea[longitudo] = octetus;
                    longitudo += 1;
                }
                continue;
            }
            if inventum.is_none() {
                inventum = culmen_lineae(linea.get(..longitudo)?);
            }
            longitudo = 0;
        }
    }
    unsafe { libc::close(fd) };
    inventum
}

/// `7ffc…000-7ffc…000 rw-p 00000000 00:00 0    [stack]` → the high address.
fn culmen_lineae(linea: &[u8]) -> Option<usize> {
    if !linea.ends_with(b"[stack]") {
        return None;
    }
    // The **first** `-` separates the two addresses: hex digits contain none,
    // and the `rw-p` that does comes after a space.
    let vinculum = linea.iter().position(|&o| o == b'-')?;
    let reliquum = linea.get(vinculum + 1..)?;
    let spatium = reliquum.iter().position(|&o| o == b' ')?;
    hexadecimalis(reliquum.get(..spatium)?)
}

/// Lowercase or uppercase hex, with no prefix.
fn hexadecimalis(cifrae: &[u8]) -> Option<usize> {
    if cifrae.is_empty() {
        return None;
    }
    let mut numerus = 0usize;
    for &cifra in cifrae {
        let valor = match cifra {
            b'0'..=b'9' => cifra - b'0',
            b'a'..=b'f' => cifra - b'a' + 10,
            b'A'..=b'F' => cifra - b'A' + 10,
            _ => return None,
        };
        numerus = numerus.checked_mul(16)?.checked_add(usize::from(valor))?;
    }
    Some(numerus)
}

/// The handler. Everything it touches was computed before it could run.
unsafe extern "C" fn manus(_signum: c_int, notitia: *mut libc::siginfo_t, _contextus: *mut c_void) {
    // D-08, for the one case the flush-at-exit design could not have covered:
    // everything `scribe` produced before the fault still reaches stdout.
    // `liss_flush` is `write(2)` and integer arithmetic, and `scriptio` stores
    // the buffer's length with `Release` *after* the bytes, so the length read
    // here names bytes that are fully written.
    unsafe { crate::scriptio::liss_flush() };

    let nuntius = unsafe { discerne(notitia) };
    erratum(NUNTIUS_PRAEFIXUM);
    erratum(nuntius.to_bytes());
    erratum(b"\n");

    // `_exit`, never `exit`: `exit` runs `atexit` handlers on a corrupted stack.
    unsafe { libc::_exit(1) }
}

/// Which of the two sentences this fault is.
///
/// # Safety
/// `notitia` must be null or the `siginfo_t` the kernel supplied.
unsafe fn discerne(notitia: *const libc::siginfo_t) -> &'static CStr {
    let fundus = FUNDUS.load(Ordering::Relaxed);
    if fundus == 0 || notitia.is_null() {
        return NUNTIUS_ACERVUS;
    }
    let locus = unsafe { (*notitia).si_addr() } as usize;
    let ima = fundus.saturating_sub(LIMES);
    let summa = fundus.saturating_add(PAGINA.load(Ordering::Relaxed));
    if locus >= ima && locus < summa {
        NUNTIUS_ACERVUS
    } else {
        NUNTIUS_MEMORIA
    }
}

/// One unbuffered write to stderr, as `defectus::erratum` does it.
fn erratum(octeti: &[u8]) {
    unsafe { crate::scriptio::liss_scribe_erratum(octeti.as_ptr(), octeti.len() as u64) };
}
