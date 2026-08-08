//! The heap layout constant both ends of the ABI have to agree on (spec §5.2).
//!
//! # Why this is here and not in `liss-abi`
//!
//! Exactly the argument [`crate::genus`] makes for the kind codes, and it
//! arrives for the same reason. `liss-abi` owns the three `#[repr(C)]` headers
//! and *was* the sole definition of this offset — but codegen is now a
//! **writer** of the layout as well as a caller of the ABI: `liss_list_new`
//! deliberately returns uninitialised memory and leaves codegen to store each
//! element word itself. `liss-abi` is cross-compiled for musl and never linked
//! into the compiler (spec §11), so `liss-codegen` cannot import from it; the
//! alternative was a second `24` in `emissio.rs` held to this one by a comment,
//! which is precisely the arrangement Task 12 removed for `KIND_INT`.
//!
//! `liss-abi` re-exports this, so `liss_abi::ONUS` still names it, and its
//! `size_of` assertions still hold the three headers to it — those are the
//! compile-time proof that the number the compiler emits is the number the
//! runtime reads, and moving the constant up here is what puts them on the same
//! side of that proof.

/// The offset of the payload in **all three** heap layouts — a `ni`'s digits,
/// a `voc`'s bytes, a `lista`'s element words.
///
/// One constant rather than three, so codegen carries one number: element `i`
/// of a list is at `ONUS + 8*i`. `LissText::_rsv` exists solely to buy this and
/// must not be removed.
pub const ONUS: usize = 24;

/// One element word of a `lista`, in bytes. Every element is one machine word
/// whatever its kind (spec §5.1) — the two narrow scalars zero-extended into
/// it — which is what makes the payload an array rather than a packed record
/// codegen would have to compute offsets into.
pub const VERBUM: usize = 8;
