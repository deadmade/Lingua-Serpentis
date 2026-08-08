//! `liss-codegen` — the back end: the typed IR in, a static ELF out.
//!
//! Three pieces, in the order a program passes through them:
//!
//! - [`target`] — the two targets and the four musl blobs embedded for each,
//!   put into the compiler's `.rodata` at *its* build time by `build.rs`. This
//!   is spec §2's first constraint made good: **no rustc on the machine running
//!   the compiler**, because there is no toolchain to invoke.
//! - [`emissio`] — Cranelift. [`compila`] takes a [`liss_ir::Programma`] and
//!   gives back one relocatable ELF object.
//! - [`nexus`] — libwild, driven entirely from memory. [`nexus::WildNexus`]
//!   takes that object plus the four blobs and gives back the finished image.
//!
//! Nothing here touches the filesystem. `compila` returns bytes and `iunge`
//! returns bytes; whoever wants a file writes one, with its own permissions.

pub mod emissio;
pub mod nexus;
pub mod target;

pub use emissio::compila;

/// A back-end failure.
///
/// Structural and always Latin — it carries no line, because there is no line:
/// by the time the IR exists the source has type-checked, so everything that
/// can still go wrong is either the machine (out of memory) or this compiler
/// (a bug). Both read the same to the person holding the terminal, and the
/// register is the runtime's: subject, perfect passive participle, `est`.
///
/// **No foreign error is ever interpolated into one of these.** Cranelift's
/// `ModuleError`, `object`'s write errors and `libwild::error::Error` all say
/// their piece in English; forwarding any of them would put English on a user's
/// stderr, which is D-22 exactly — an OS string spliced into a Latin sentence
/// was a defect then and is one now. What the English *would* have said is
/// recovered from a debugger or a stack trace, not from the diagnostic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Defectus {
    pub nuntius: String,
}

impl Defectus {
    /// The one constructor. Spec §10.3 makes `Defectus::novus(` a catalogue
    /// marker, so the sentence must be written **inline at the call site** and
    /// never behind a shared constant — a site that forwards a constant carries
    /// no literal of its own and the ratchet reports it `UNPARSED`.
    pub fn novus(nuntius: impl Into<String>) -> Defectus {
        Defectus { nuntius: nuntius.into() }
    }
}

impl std::fmt::Display for Defectus {
    /// The same shape `ProgramError`'s structural arm writes, so a back-end
    /// failure is indistinguishable in form from a front-end one.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Error: {}", self.nuntius)
    }
}

impl std::error::Error for Defectus {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_defect_formats_as_a_structural_error() {
        let d = Defectus::novus("imago non coniuncta est: nexus");
        assert_eq!(d.to_string(), "Error: imago non coniuncta est: nexus");
    }
}
