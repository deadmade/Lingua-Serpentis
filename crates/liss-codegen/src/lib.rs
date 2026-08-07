//! `liss-codegen` — the back end: the embedded musl runtime today, Cranelift
//! and libwild from Task 11.
//!
//! What this crate ships right now is the answer to spec §2's first constraint,
//! **no rustc on the machine running the compiler**: `build.rs` cross-builds
//! `liss-abi` for both musl targets at *compiler* build time, prelinks each
//! with `libc.a` and the CRT objects, and the bytes land in the binary. A
//! program link is then five objects and no toolchain — see [`target::Runtime`].

pub mod target;
