//! Stack exhaustion (spec §8), caught and reported as an exit-1 Latin diagnostic.

use core::ffi::{CStr, c_char, c_int, c_void};
use core::ptr;
use core::sync::atomic::{AtomicUsize, Ordering};

/// The alternate stack, in bytes. Deliberately not `SIGSTKSZ`: musl's is a small
/// compile-time constant, and on aarch64 with SVE the true minimum can exceed it.
const MENSURA_ALTERNA: usize = 64 * 1024;

/// The guard band below the low bound, not one page: Linux's `stack_guard_gap`
/// defaults to 256 pages.
const LIMES: usize = 1 << 20;

/// The page size to assume if `sysconf` cannot say — never the answer, only a floor.
const PAGINA_MINIMA: usize = 4096;

const VIA_MAPPARUM: &[u8] = b"/proc/self/maps\0";

/// `Error: ` — the prefix `parse_diagnostic` looks for. Deliberately **not**
/// named `NUNTIUS_*`: that name is a catalogue marker for a *message*, and this
/// is a prefix.
const PRAEFIXUM_ERRORIS: &[u8] = b"Error: ";

const NUNTIUS_ACERVUS: &CStr = c"acervus vocationum exhaustus est";

/// A fault nowhere near the stack, which in a language with no pointers, no FFI
/// and no `unsafe` can only be a compiler bug. The tail (`signum`) must stay
/// distinct from `defectus::memoria_violata`'s `verificatio`: `catalogus.rs`
/// grants `fixtured` only to a workspace-unique pattern.
const NUNTIUS_MEMORIA: &CStr = c"defectus internus: memoria violata est: signum";

/// 16-byte aligned, because the kernel hands the handler this buffer as a stack.
#[repr(C, align(16))]
struct AcervusAlternus([u8; MENSURA_ALTERNA]);

/// **`static`, not allocated:** by the time the handler runs there is no stack
/// left to allocate one on.
static mut ALTERNUS: AcervusAlternus = AcervusAlternus([0; MENSURA_ALTERNA]);

/// The low bound of the stack, computed once at initialisation and **never in
/// the handler**. Zero means bounds-unknown, which the handler reports as
/// exhaustion.
static FUNDUS: AtomicUsize = AtomicUsize::new(0);

static PAGINA: AtomicUsize = AtomicUsize::new(0);

/// Install the guard: bounds, alternate stack, `SIGSEGV`/`SIGBUS`, `SIGPIPE`.
///
/// **`main` must call this**, which nothing in this crate can enforce;
/// `tests/custos.rs`'s `a_main_in_this_crate_must_install_the_guard` does.
/// Failure is silent and leaves the default dispositions.
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

    // D-06: `std::rt::init` never ran, so std's `SIG_IGN` is not inherited.
    unsafe { libc::signal(libc::SIGPIPE, libc::SIG_IGN) };

    let _institutum = unsafe { pone_manum() };
}

/// The alternate stack and the two dispositions. `false` if the kernel refused
/// any of the three.
///
/// # Safety
/// Call once, from the main thread: `sigaltstack` is per-thread.
unsafe fn pone_manum() -> bool {
    unsafe {
        let mut alternus: libc::stack_t = core::mem::zeroed();
        alternus.ss_sp = (&raw mut ALTERNUS).cast::<c_void>();
        alternus.ss_size = MENSURA_ALTERNA;
        alternus.ss_flags = 0;
        if libc::sigaltstack(&alternus, ptr::null_mut()) != 0 {
            return false;
        }

        let mut actio: libc::sigaction = core::mem::zeroed();
        actio.sa_sigaction = manus as *const () as usize;
        // Both signals: on aarch64 a stack fault may arrive as either.
        actio.sa_flags = libc::SA_ONSTACK | libc::SA_SIGINFO;
        // Both in the mask, or a `SIGBUS` could re-enter the handler on the
        // alternate stack it is already using.
        libc::sigemptyset(&mut actio.sa_mask);
        libc::sigaddset(&mut actio.sa_mask, libc::SIGSEGV);
        libc::sigaddset(&mut actio.sa_mask, libc::SIGBUS);
        libc::sigaction(libc::SIGSEGV, &actio, ptr::null_mut()) == 0
            && libc::sigaction(libc::SIGBUS, &actio, ptr::null_mut()) == 0
    }
}

/// The stack bounds as `(low, high)`, derived fresh. Pure: it installs nothing
/// and stores nothing, so tests can compare it against `/proc/self/maps`.
pub fn fines() -> Option<(usize, usize)> {
    let pagina = pagina();
    match fines_ex_mappa(pagina) {
        Some(fines) => Some(fines),
        // Known-approximate, and reached only where `/proc` is not mounted.
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

/// The `[stack]` mapping's high address, minus `RLIMIT_STACK` — the only method
/// that reports the *potential* stack rather than the pages already touched.
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

/// The high address of the `[stack]` mapping, read with `open`/`read` over a
/// fixed buffer: nothing on this path may allocate.
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
    // Its own function so that no `?` inside it can return past the `close`.
    let culmen = unsafe { culmen_ex_fd(fd) };
    unsafe { libc::close(fd) };
    culmen
}

/// # Safety
/// `fd` must be open for reading.
unsafe fn culmen_ex_fd(fd: c_int) -> Option<usize> {
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
                // A truncated line simply fails to end with `[stack]`, which is
                // correct: the long lines are file mappings.
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
    inventum
}

/// `7ffc…000-7ffc…000 rw-p 00000000 00:00 0    [stack]` → the high address.
fn culmen_lineae(linea: &[u8]) -> Option<usize> {
    if !linea.ends_with(b"[stack]") {
        return None;
    }
    // The **first** `-` separates the two addresses: the `rw-p` that also has
    // one comes after a space.
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
    // D-08. The result is dropped: a closed descriptor 1 must not decide this
    // program's status.
    let _ = unsafe { crate::scriptio::effunde() };

    let nuntius = unsafe { discerne(notitia) };
    erratum(PRAEFIXUM_ERRORIS);
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
