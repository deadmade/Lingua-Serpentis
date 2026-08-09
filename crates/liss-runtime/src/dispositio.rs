//! The heap layout constants both ends of the ABI have to agree on (spec §5.2).

/// The offset of the payload in all three heap layouts.
pub const ONUS: usize = 24;

/// One element word of a `lista`, in bytes.
pub const VERBUM: usize = 8;
