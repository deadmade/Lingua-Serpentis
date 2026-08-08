//! `liss-abi` — the C ABI a compiled Lingua Serpentis program calls.
//!
//! This is the memory-safety-critical layer, and the only crate in the
//! workspace containing `unsafe`. Everything above it — the lexer, the parser,
//! the checker, the typed IR — deals in Rust values; everything below it is
//! machine words that codegen emits. The rules those words obey are stated
//! once, here, and are the contract Tasks 11–15 emit calls against.
//!
//! # The calling convention (spec §5.4)
//!
//! **Every operation takes ownership of its owned arguments and returns
//! exactly one new owned value.** Two families are exempt, and both are
//! lifecycle rather than operations:
//!
//! - `liss_*_copy` **borrows**. It is what `Load` calls to read a slot without
//!   emptying it; a consuming copy would leave the slot dangling.
//! - `liss_list_put` **borrows the list it mutates** (it consumes its index and
//!   its value, and frees the element it displaces).
//!
//! Everything else consumes. That is what makes a statement's expression tree
//! leave exactly one live temporary — its root — so codegen never maintains a
//! set of pending temporaries.
//!
//! There is no `liss_enter`/`liss_leave`. With the recursion cap gone (spec
//! §3.3) calls carry no bookkeeping whatsoever; stack exhaustion is caught by
//! the signal handler of §8 instead.
//!
//! # One machine word per value (spec §5.1)
//!
//! | type | word |
//! |---|---|
//! | `veritas` | `i8`, 0 or 1 |
//! | `lit` | `i32`, a Unicode scalar value |
//! | `ni` | `*mut LissInt` |
//! | `mnr`, `nr` | `*mut LissInt`, a count of a grain the *type* fixes |
//! | `voc` | `*mut LissText` |
//! | `T lista` | `*mut LissList` |
//!
//! Where a word is passed generically — a list element, `liss_list_get`'s
//! result — it is a `u64`, and the two scalar types are **zero-extended** into
//! it. There are no `liss_bool_*` or `liss_char_*` entry points: both are
//! immediates that codegen operates on directly, and neither is owned.
//!
//! # Diagnostics
//!
//! Every runtime diagnostic is written as an **inline literal at its
//! `liss_panic` call site**, never as a reference to a shared constant table
//! (spec §10.3). `catalogus.rs` extracts the first string literal within 300
//! bytes of a marker; a site that forwards a constant carries no literal of its
//! own and reads `UNPARSED`, hiding the message from the ratchet. Where two
//! call sites would raise the same sentence they share one `-> !` helper in
//! [`defectus`], which keeps the literal unique in the workspace and so
//! uniquely attributable — the same reason `interp.rs` collapsed its three
//! `divisio per nullum` arms into one closure (D-24).
//!
//! The message pointers are **C string literals** (`c"…"`). That is not
//! decoration: it means the sentence appears exactly once at the call site,
//! with no separate byte length to keep in sync and no `\0` escape for
//! `catalogus` to mangle, while still being a plain `*const c_char` for codegen
//! to hand over.

use core::mem::{align_of, size_of};

#[cfg(unix)]
pub mod custos;
pub mod defectus;
/// The runtime's `main` (spec §6). Behind the `ingressus` feature, which the
/// musl cross-build in `liss-codegen/build.rs` is the only thing that turns on:
/// this crate is *also* built as an `rlib` for its own tests, every test binary
/// already defines `main`, and an ungated one here is a `duplicate symbol: main`
/// at link time — measured, not feared. See the module doc for why the guard is
/// installed by a call there rather than by an `.init_array` constructor.
#[cfg(all(unix, feature = "ingressus"))]
pub mod ingressus;

pub mod memoria;
pub mod numeri;
pub mod scriptio;
pub mod series;
pub mod textus;

/// `'LISS'` — stamped into every heap object's header and checked on **every**
/// free (spec §5.2).
///
/// It turns a double free or a use-after-free into a deterministic Latin exit
/// rather than musl heap corruption, and it covers the live-allocation
/// counter's one blind spot: an extra free plus a missing free nets to zero,
/// and only the magic can tell that apart from a balanced program.
pub const LISS_MAGIC: u32 = 0x4C49_5353;

/// The offset of the payload in **all three** heap layouts, so codegen carries
/// one constant rather than three. `LissText::_rsv` exists solely to buy this
/// and must not be removed.
///
/// **Defined in `liss_runtime::dispositio`, not here**, for the reason the kind
/// codes are: codegen writes list element words itself — `liss_list_new` has no
/// per-element setter by design — and this crate is never linked into the
/// compiler (spec §11), so a second `24` in `emissio.rs` was the only other way.
/// The `size_of` assertions below are what hold the three headers to it, and
/// they are still here, which is what makes the shared constant a proof rather
/// than a promise.
pub use liss_runtime::dispositio::ONUS;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct LissHdr {
    pub magic: u32,
    pub kind: u32,
}

/// A `ni`, or an `mnr`/`nr` grain count. The payload at [`ONUS`] is `Roman`'s
/// own representation: `len` little-endian digits, each `0..=9`, canonical (no
/// trailing zeros, and the empty payload the unique zero).
#[repr(C)]
pub struct LissInt {
    pub h: LissHdr,
    pub len: u64,
    pub neg: u64,
    // digits: [u8; len] at ONUS
}

/// A `voc`. The payload at [`ONUS`] is `len` bytes of UTF-8.
#[repr(C)]
pub struct LissText {
    pub h: LissHdr,
    pub len: u64,
    /// Reserved, and load-bearing: it is what puts `bytes` at [`ONUS`], the
    /// same offset as `LissInt`'s digits and `LissList`'s items.
    pub _rsv: u64,
    // bytes: [u8; len] at ONUS
}

/// A `T lista`. The payload at [`ONUS`] is `len` machine words, each an element
/// of kind `ekind`.
#[repr(C)]
pub struct LissList {
    pub h: LissHdr,
    pub len: u64,
    /// The **element** kind, so the runtime can free, copy, compare and print
    /// recursively without a type environment. `h.kind` is `ekind + GRADUS`.
    pub ekind: u64,
    // items: [u64; len] at ONUS
}

/// **The proof that the number the compiler emits is the number this crate
/// reads.** `ONUS` and `VERBUM` are defined in `liss_runtime::dispositio` so
/// codegen can import them (spec §11 forbids it importing from *here*), and
/// these assertions are what stop that sharing from being a promise: the musl
/// cross-build of this crate is a prerequisite of every compiled program
/// (`liss-codegen/build.rs`, which also re-runs on `liss-runtime/src`), so a
/// constant that drifted from the layout is a **build failure** rather than a
/// wrong answer.
///
/// `VERBUM` needs its own line and did not have one. `ONUS` is held by the
/// three `size_of`s; the element **stride** is held by nothing at all, because
/// the reader never spells it — [`series::elementa`] returns a `*mut u64` and
/// every access is `.add(i)`, which is a hard-wired 8. So a `VERBUM` of 16
/// would have codegen store element `i` at `ONUS + 16*i` while this crate reads
/// it at `ONUS + 8*i`: `quantum` would over-allocate (harmless), every element
/// past the first would read as a zero word, and `liberare_verbum` and
/// `copiare_verbum` both *skip* a zero word. Silent wrong answers and silent
/// leaks, and nothing would fail to compile. Now something does.
const _: () = {
    assert!(size_of::<LissInt>() == ONUS);
    assert!(size_of::<LissText>() == ONUS);
    assert!(size_of::<LissList>() == ONUS);
    assert!(align_of::<LissInt>() == 8);
    assert!(align_of::<LissText>() == 8);
    assert!(align_of::<LissList>() == 8);
    assert!(size_of::<LissHdr>() == 8);
    // `size_of::<u64>()`, not a literal 8: the element array *is* a `[u64]`,
    // and `elementa`'s `.add(i)` scales by exactly this.
    assert!(liss_runtime::dispositio::VERBUM == size_of::<u64>());
};

// Kind codes (spec §5.3): `kind = base | (depth << 3)`. `ni lista` is 10,
// `ni lista lista` is 18, the element kind of `k` is `k - 8`. Closed form,
// unbounded nesting, no interning table. Codegen only ever *writes* these as
// compile-time immediates; the runtime reads them to walk lists.
//
// **They are defined in `liss_runtime::genus`, not here**, and re-exported so
// `liss_abi::KIND_INT` still names the same constant. This crate is
// cross-compiled for musl and is never linked into the compiler (spec §11), so
// `liss-codegen` — the only *writer* of these numbers — cannot import from it;
// it used to carry a copied `GENUS_NI = 2` with a comment as the only thing
// tying the two together. `liss-runtime` is a dependency of both and needs no
// new row in spec §11's table, which is what makes one definition possible.
pub use liss_runtime::genus::{
    GRADUS, KIND_BOOL, KIND_CHAR, KIND_FRAC_SCRIPULUM, KIND_FRAC_UNCIA, KIND_INT, KIND_TEXT, basis,
    granum, profunditas,
};
