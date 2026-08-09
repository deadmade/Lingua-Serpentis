//! Linking, in process and in memory (spec §9.3).

use libwild::{FileSystem, FileType, InputFileData, Linker, OutputFileData, OutputOptions};
use std::collections::HashMap;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use crate::Defectus;

/// Objects in, image out.
pub trait Nexus: Send + Sync {
    /// Link `artefacta`, in the order given, into an executable named `nomen`.
    fn iunge(&self, artefacta: &[(&str, &[u8])], nomen: &str) -> Result<Vec<u8>, Defectus>;
}

/// Name → bytes. Every file that exists, for the duration of one link.
type Tabula = HashMap<PathBuf, Arc<Vec<u8>>>;

/// Take the lock, poisoned or not.
fn aperi(tabula: &Mutex<Tabula>) -> std::sync::MutexGuard<'_, Tabula> {
    tabula.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[derive(Clone, Default)]
struct MemoriaFs {
    tabula: Arc<Mutex<Tabula>>,
}

#[derive(Debug)]
struct Ingressus(Arc<Vec<u8>>);

impl InputFileData for Ingressus {
    fn bytes(&self) -> &[u8] {
        &self.0
    }
}

struct Egressus {
    via: PathBuf,
    octeti: Vec<u8>,
    tabula: Arc<Mutex<Tabula>>,
}

impl OutputFileData for Egressus {
    fn bytes(&self) -> &[u8] {
        &self.octeti
    }
    fn bytes_mut(&mut self) -> &mut [u8] {
        &mut self.octeti
    }
    fn finish(mut self) -> libwild::error::Result {
        let octeti = std::mem::take(&mut self.octeti);
        aperi(&self.tabula).insert(self.via.clone(), Arc::new(octeti));
        Ok(())
    }
}

impl FileSystem for MemoriaFs {
    type Input = Ingressus;
    type Output = Egressus;

    fn open_input(
        &self,
        via: &Path,
        _prepopulate_maps: bool,
    ) -> libwild::error::Result<(Self::Input, Option<Arc<File>>)> {
        let octeti = aperi(&self.tabula)
            .get(via)
            .cloned()
            .ok_or_else(|| libwild::error!("no in-memory file {}", via.display()))?;
        Ok((Ingressus(octeti), None))
    }

    fn file_type(&self, via: &Path) -> libwild::error::Result<FileType> {
        if aperi(&self.tabula).contains_key(via) {
            Ok(FileType::File)
        } else {
            Err(std::io::Error::new(std::io::ErrorKind::NotFound, "absent").into())
        }
    }

    fn canonicalize(&self, via: &Path) -> libwild::error::Result<PathBuf> {
        Ok(via.to_path_buf())
    }

    fn remove_file(&self, via: &Path) -> libwild::error::Result<()> {
        aperi(&self.tabula).remove(via);
        Ok(())
    }

    fn rename_file(&self, via: &Path, nova: &Path) -> libwild::error::Result<()> {
        let mut tabula = aperi(&self.tabula);
        let Some(octeti) = tabula.remove(via) else {
            return Err(std::io::Error::new(std::io::ErrorKind::NotFound, "absent").into());
        };
        tabula.insert(nova.to_path_buf(), octeti);
        Ok(())
    }

    fn create_output(
        &self,
        via: Arc<Path>,
        optiones: OutputOptions,
    ) -> libwild::error::Result<Self::Output> {
        let mensura = usize::try_from(optiones.size)
            .map_err(|_| libwild::error!("output too large for this platform"))?;
        Ok(Egressus {
            via: via.to_path_buf(),
            octeti: vec![0; mensura],
            tabula: Arc::clone(&self.tabula),
        })
    }

    fn write_auxiliary(&self, via: &Path, octeti: &[u8]) -> libwild::error::Result<()> {
        aperi(&self.tabula).insert(via.to_path_buf(), Arc::new(octeti.to_vec()));
        Ok(())
    }
}

pub struct WildNexus;

impl Nexus for WildNexus {
    fn iunge(&self, artefacta: &[(&str, &[u8])], nomen: &str) -> Result<Vec<u8>, Defectus> {
        let machina = machina_ex_artefactis(artefacta)?;

        let fs = MemoriaFs::default();
        {
            let mut tabula = aperi(&fs.tabula);
            for (nomen, octeti) in artefacta {
                tabula.insert(PathBuf::from(*nomen), Arc::new(octeti.to_vec()));
            }
        }

        let mut argv: Vec<String> = vec![
            "wild".to_owned(),
            "-m".to_owned(),
            machina.to_owned(),
            "-static".to_owned(),
            "--no-pie".to_owned(),
            "--gc-sections".to_owned(),
            "-o".to_owned(),
            nomen.to_owned(),
        ];
        argv.extend(artefacta.iter().map(|(n, _)| (*n).to_owned()));

        let argumenta = || argv.iter().map(String::as_str);
        let mut args = libwild::Args::new(argumenta)
            .map_err(|_| Defectus::novus("imago non coniuncta est: argumenta"))?;
        // Set before `parse`: `parse` itself can warn.
        args.on_warning(Box::new(|_| {}));
        args.parse(argumenta)
            .map_err(|_| Defectus::novus("imago non coniuncta est: perscrutatio"))?;

        let nexus = Linker::with_file_system(fs.clone());
        {
            // The output borrows the linker's arena; both must go before the map is read.
            let _egressus =
                nexus.run(&args).map_err(|_| Defectus::novus("imago non coniuncta est: nexus"))?;
        }
        drop(nexus);

        let imago = aperi(&fs.tabula)
            .remove(Path::new(nomen))
            .ok_or_else(|| Defectus::novus("imago non coniuncta est: imago abest"))?;
        Ok(Arc::try_unwrap(imago).unwrap_or_else(|a| (*a).clone()))
    }
}

/// The `-m` emulation the inputs call for, read from the objects themselves.
fn machina_ex_artefactis(artefacta: &[(&str, &[u8])]) -> Result<&'static str, Defectus> {
    let mut inventa: Option<u16> = None;
    for (_, octeti) in artefacta {
        if octeti.len() < 20 || &octeti[..4] != b"\x7fELF" {
            return Err(Defectus::novus("defectus internus: artefactum non est ELF"));
        }
        let machina = u16::from_le_bytes([octeti[18], octeti[19]]);
        match inventa {
            None => inventa = Some(machina),
            Some(prior) if prior == machina => {}
            Some(_) => {
                return Err(Defectus::novus(
                    "defectus internus: artefacta machinis diversis composita sunt",
                ));
            }
        }
    }
    match inventa {
        Some(m) => crate::target::Target::OMNES
            .into_iter()
            .find(|t| t.machina() == m)
            .map(|t| t.emulatio())
            .ok_or_else(|| Defectus::novus("defectus internus: machina ignota est")),
        None => Err(Defectus::novus("defectus internus: nihil coniungendum est")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::target::Target;

    #[test]
    fn an_empty_link_is_refused() {
        let e = WildNexus.iunge(&[], "imago").unwrap_err();
        assert_eq!(e.nuntius, "defectus internus: nihil coniungendum est");
    }

    #[test]
    fn artefacts_of_two_architectures_are_refused() {
        let x = Target::X86_64.runtime();
        let a = Target::Aarch64.runtime();
        let e = WildNexus
            .iunge(&[("crt1.o", x.crt1), ("altera.o", a.crt1)], "imago")
            .unwrap_err();
        assert_eq!(
            e.nuntius,
            "defectus internus: artefacta machinis diversis composita sunt"
        );
    }

    #[test]
    fn a_link_that_cannot_resolve_fails_in_latin() {
        let r = Target::X86_64.runtime();
        let e = WildNexus
            .iunge(
                &[
                    ("crt1.o", r.crt1),
                    ("crti.o", r.crti),
                    ("runtime.o", r.nucleus),
                    ("crtn.o", r.crtn),
                ],
                "imago",
            )
            .unwrap_err();
        assert_eq!(e.nuntius, "imago non coniuncta est: nexus");
    }

    #[test]
    fn the_emulation_comes_from_the_objects() {
        for meta in Target::OMNES {
            let r = meta.runtime();
            assert_eq!(
                machina_ex_artefactis(&[("crt1.o", r.crt1)]).unwrap(),
                meta.emulatio(),
            );
        }
    }
}
