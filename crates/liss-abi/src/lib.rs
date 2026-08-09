use core::mem::{align_of, size_of};

#[cfg(unix)]
pub mod custos;
pub mod defectus;
/// The runtime's `main`, behind the `ingressus` feature: test binaries define their own.
#[cfg(all(unix, feature = "ingressus"))]
pub mod ingressus;

pub mod memoria;
pub mod numeri;
pub mod scriptio;
pub mod series;
pub mod textus;

/// `'LISS'` — stamped into every heap object's header and checked on **every**
/// free (spec §5.2).
pub const LISS_MAGIC: u32 = 0x4C49_5353;

/// The offset of the payload in **all three** heap layouts. `LissText::_rsv`
/// exists solely to buy this and must not be removed.
pub use liss_runtime::dispositio::ONUS;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct LissHdr {
    pub magic: u32,
    pub kind: u32,
}

/// A `ni`, or an `mnr`/`nr` grain count. The payload at [`ONUS`] is `Roman`'s
/// own representation: `len` canonical little-endian digits, each `0..=9`.
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
    /// Reserved, and load-bearing: it is what puts `bytes` at [`ONUS`].
    pub _rsv: u64,
    // bytes: [u8; len] at ONUS
}

/// A `T lista`. The payload at [`ONUS`] is `len` machine words, each an element
/// of kind `ekind`.
#[repr(C)]
pub struct LissList {
    pub h: LissHdr,
    pub len: u64,
    /// The **element** kind. `h.kind` is `ekind + GRADUS`.
    pub ekind: u64,
    // items: [u64; len] at ONUS
}

/// The proof that the number the compiler emits is the number this crate reads.
const _: () = {
    assert!(size_of::<LissInt>() == ONUS);
    assert!(size_of::<LissText>() == ONUS);
    assert!(size_of::<LissList>() == ONUS);
    assert!(align_of::<LissInt>() == 8);
    assert!(align_of::<LissText>() == 8);
    assert!(align_of::<LissList>() == 8);
    assert!(size_of::<LissHdr>() == 8);
    // `size_of::<u64>()`, not a literal 8: the element array *is* a `[u64]`.
    assert!(liss_runtime::dispositio::VERBUM == size_of::<u64>());
};

// Kind codes (spec §5.3), defined in `liss_runtime::genus` and re-exported here.
pub use liss_runtime::genus::{
    GRADUS, KIND_BOOL, KIND_CHAR, KIND_FRAC_SCRIPULUM, KIND_FRAC_UNCIA, KIND_INT, KIND_TEXT, basis,
    granum, profunditas,
};
