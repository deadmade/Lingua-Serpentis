//! Linking, in process and in memory (spec §9.3).
//!
//! [`WildNexus`] hands libwild a [`FileSystem`] that is a `HashMap`, so the
//! five inputs are never written anywhere and the finished image is never read
//! from anywhere — it is returned as bytes and the caller decides what a file
//! is. No `cc`, no `ld`, no temporary directory, no toolchain on the machine.
//!
//! # The risk this was measured against first
//!
//! The prelinked runtime of [`crate::target`] carries **general-dynamic** TLS
//! relocations — 36 `R_X86_64_TLSGD`, 16 `TLSLD` and 28 `DTPOFF32` on x86_64;
//! 53 each of `R_AARCH64_TLSDESC_{CALL,ADR_PAGE21,ADD_LO12,LD64_LO12}` on
//! aarch64 — all of them from Rust's std thread-locals, which are compiled GD
//! because a `staticlib` does not know it will end up in a static executable.
//! A static, non-dynamic image has no `__tls_get_addr` to call and no dynamic
//! relocations to apply, so the **linker** must rewrite every one of those
//! sequences to local-exec. If it cannot, either the link fails or the image
//! faults at its first TLS access, and no amount of code generation helps.
//!
//! It was checked before any of this was written, and libwild does relax them:
//! the x86_64 image contains zero TLS relocations, no dynamic section, and the
//! canonical `data16 data16 data16 mov %fs:0x0,%rax` rewrite at every former GD
//! site; the aarch64 image likewise, and it runs under `qemu-aarch64`. The one
//! surviving `__tls_get_addr` in each image is musl's own, called from its
//! static `dl_iterate_phdr` — hand-written code, not a relaxed relocation.
//!
//! # Why libwild's own words never reach a user
//!
//! Two rules, both from spec §9.3 and both easy to omit because omitting them
//! is silent:
//!
//! - **`args.on_warning` is installed.** libwild's default sink writes English
//!   to stderr. The language's one standing rule is that everything the user
//!   sees is Latin, and the robustness harness enforces it with an English
//!   denylist — so a libwild warning would not merely be ugly, it would fail a
//!   test far away from here with no hint of where it came from.
//! - **`libwild::error::Error` is never forwarded.** Every link failure becomes
//!   the same structural Latin diagnostic, for D-22's reason: an English string
//!   interpolated into a Latin sentence was a defect, not a convenience.
//!
//! # `libwild::run` cannot be used
//!
//! The free function hard-codes `OsFileSystem`. The only way in is
//! `Linker::with_file_system(fs).run(&args)`.

use libwild::{FileSystem, FileType, InputFileData, Linker, OutputFileData, OutputOptions};
use std::collections::HashMap;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use crate::Defectus;

/// Objects in, image out.
///
/// The trait exists so that the Windows answer is one new implementor and no
/// other change (spec §9.4) — libwild's `Args` is `Elf | MachO | Wasm` and has
/// no PE/COFF backend, so that answer does not exist yet and this seam is where
/// it will go.
pub trait Nexus: Send + Sync {
    /// Link `artefacta`, in the order given, into an executable named `nomen`.
    ///
    /// The names are filenames only in the sense that libwild's argv wants
    /// strings; nothing with those names exists on disk.
    fn iunge(&self, artefacta: &[(&str, &[u8])], nomen: &str) -> Result<Vec<u8>, Defectus>;
}

/// The in-memory filesystem libwild reads its inputs from and writes its output
/// to. A `HashMap` behind a `Mutex`, because libwild links in parallel.
#[derive(Clone, Default)]
struct MemoriaFs {
    tabula: Arc<Mutex<HashMap<PathBuf, Arc<Vec<u8>>>>>,
}

/// One input file, as libwild sees it.
#[derive(Debug)]
struct Ingressus(Arc<Vec<u8>>);

impl InputFileData for Ingressus {
    fn bytes(&self) -> &[u8] {
        &self.0
    }
}

/// The output image, sized up front by libwild and filled in place.
struct Egressus {
    via: PathBuf,
    octeti: Vec<u8>,
    tabula: Arc<Mutex<HashMap<PathBuf, Arc<Vec<u8>>>>>,
}

impl OutputFileData for Egressus {
    fn bytes(&self) -> &[u8] {
        &self.octeti
    }
    fn bytes_mut(&mut self) -> &mut [u8] {
        &mut self.octeti
    }
    /// libwild's commit point. The bytes land in the map, where [`Nexus::iunge`]
    /// picks them up after the linker has been dropped.
    fn finish(mut self) -> libwild::error::Result {
        let octeti = std::mem::take(&mut self.octeti);
        self.tabula.lock().unwrap().insert(self.via.clone(), Arc::new(octeti));
        Ok(())
    }
}

// The English in the `libwild::error!` calls below is deliberate and is not a
// user-visible string: this trait's error type is libwild's, these values never
// leave the linker, and `iunge` replaces whatever comes back with one Latin
// sentence of its own. They exist so that a `MemoriaFs` bug is legible in a
// debugger rather than silent.
impl FileSystem for MemoriaFs {
    type Input = Ingressus;
    type Output = Egressus;

    fn open_input(
        &self,
        via: &Path,
        _prepopulate_maps: bool,
    ) -> libwild::error::Result<(Self::Input, Option<Arc<File>>)> {
        let octeti = self
            .tabula
            .lock()
            .unwrap()
            .get(via)
            .cloned()
            .ok_or_else(|| libwild::error!("no in-memory file {}", via.display()))?;
        // `None`: there is no descriptor behind this, so libwild must not try
        // to mmap it.
        Ok((Ingressus(octeti), None))
    }

    fn file_type(&self, via: &Path) -> libwild::error::Result<FileType> {
        if self.tabula.lock().unwrap().contains_key(via) {
            Ok(FileType::File)
        } else {
            Err(std::io::Error::new(std::io::ErrorKind::NotFound, "absent").into())
        }
    }

    fn canonicalize(&self, via: &Path) -> libwild::error::Result<PathBuf> {
        // Every name is already canonical: they are map keys, not paths.
        Ok(via.to_path_buf())
    }

    fn remove_file(&self, via: &Path) -> libwild::error::Result<()> {
        self.tabula.lock().unwrap().remove(via);
        Ok(())
    }

    fn rename_file(&self, via: &Path, nova: &Path) -> libwild::error::Result<()> {
        let mut tabula = self.tabula.lock().unwrap();
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
        self.tabula.lock().unwrap().insert(via.to_path_buf(), Arc::new(octeti.to_vec()));
        Ok(())
    }
}

/// The one implementor: libwild.
pub struct WildNexus;

impl Nexus for WildNexus {
    fn iunge(&self, artefacta: &[(&str, &[u8])], nomen: &str) -> Result<Vec<u8>, Defectus> {
        let machina = machina_ex_artefactis(artefacta)?;

        let fs = MemoriaFs::default();
        {
            let mut tabula = fs.tabula.lock().unwrap();
            for (nomen, octeti) in artefacta {
                tabula.insert(PathBuf::from(*nomen), Arc::new(octeti.to_vec()));
            }
        }

        // `-static --no-pie` is spec §2's promise: one ELF that runs on any
        // Linux of its architecture with nothing else installed.
        // `--gc-sections` is worth ~0.92 MB on the image, because the prelinked
        // runtime carries the whole of `libc.a` and most of it is unreachable
        // from any one program.
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
            .map_err(|_| Defectus::novus("imago non coniuncta est"))?;
        // Spec §9.3, and the reason it is set before `parse` rather than after:
        // `parse` itself can warn.
        args.on_warning(Box::new(|_| {}));
        args.parse(argumenta).map_err(|_| Defectus::novus("imago non coniuncta est"))?;

        let nexus = Linker::with_file_system(fs.clone());
        {
            // The output borrows the linker's arena; both must go before the
            // map is read, or the image is still half-owned by libwild.
            let _egressus =
                nexus.run(&args).map_err(|_| Defectus::novus("imago non coniuncta est"))?;
        }
        drop(nexus);

        let imago = fs
            .tabula
            .lock()
            .unwrap()
            .remove(Path::new(nomen))
            .ok_or_else(|| Defectus::novus("imago non coniuncta est"))?;
        // The linker is gone, so this `Arc` is the only handle and the bytes
        // move out rather than being copied.
        Ok(Arc::try_unwrap(imago).unwrap_or_else(|a| (*a).clone()))
    }
}

/// The `-m` emulation the inputs call for, read from the objects themselves.
///
/// libwild infers nothing from an argv that names no architecture, and the
/// alternative — passing a [`crate::target::Target`] alongside the bytes —
/// would let a caller link one target's object against the other's runtime and
/// be told so only by the relocation failures. The `e_machine` at offset 18 of
/// every input is the fact that settles it, and every input must agree.
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

    /// Nothing to link is a diagnostic, not a zero-byte image.
    #[test]
    fn an_empty_link_is_refused() {
        let e = WildNexus.iunge(&[], "imago").unwrap_err();
        assert_eq!(e.nuntius, "defectus internus: nihil coniungendum est");
    }

    /// The one mistake this seam exists to catch: an x86_64 object linked
    /// against the aarch64 runtime. Without the check libwild is handed
    /// `-m elf_x86_64` for aarch64 inputs and fails somewhere in relocation
    /// processing, which becomes the same opaque `imago non coniuncta est` with
    /// nothing to say what went wrong.
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

    /// A link with nothing defining `liss_initium` fails, and says so in this
    /// crate's own sentence — libwild reports an undefined symbol in English,
    /// and none of that English survives the boundary (spec §9.3, D-22).
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
        assert_eq!(e.nuntius, "imago non coniuncta est");
    }

    /// Each target's `emulatio` is derivable from its own blobs.
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
