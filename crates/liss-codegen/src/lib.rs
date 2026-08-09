//! The back end: the typed IR in, a static ELF out.

pub mod emissio;
pub mod nexus;
pub mod target;

pub use emissio::compila;

/// A back-end failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Defectus {
    pub nuntius: String,
}

impl Defectus {
    pub fn novus(nuntius: impl Into<String>) -> Defectus {
        Defectus { nuntius: nuntius.into() }
    }
}

impl std::fmt::Display for Defectus {
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
