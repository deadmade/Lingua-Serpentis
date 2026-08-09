//! The two targets, and the runtime embedded for each (spec §9.2).

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Target {
    X86_64,
    Aarch64,
}

impl Target {
    /// Both, in the order `build.rs` builds them.
    pub const OMNES: [Target; 2] = [Target::X86_64, Target::Aarch64];

    pub const fn triple(self) -> &'static str {
        match self {
            Target::X86_64 => "x86_64-unknown-linux-musl",
            Target::Aarch64 => "aarch64-unknown-linux-musl",
        }
    }

    /// The ELF `e_machine` of every object this target's runtime carries.
    pub const fn machina(self) -> u16 {
        match self {
            Target::X86_64 => 62,
            Target::Aarch64 => 183,
        }
    }

    /// The linker's `-m` emulation name.
    pub const fn emulatio(self) -> &'static str {
        match self {
            Target::X86_64 => "elf_x86_64",
            Target::Aarch64 => "aarch64linux",
        }
    }

    /// The target this compiler is itself running on, if it is one of the two.
    pub fn hospes() -> Option<Target> {
        match std::env::consts::ARCH {
            "x86_64" => Some(Target::X86_64),
            "aarch64" => Some(Target::Aarch64),
            _ => None,
        }
    }

    /// Parse a `--target`: the full triple, or the architecture alone.
    pub fn ex_nomine(nomen: &str) -> Option<Target> {
        Target::OMNES
            .into_iter()
            .find(|m| nomen == m.triple() || nomen == m.triple().split('-').next().unwrap_or(""))
    }

    pub const fn runtime(self) -> Runtime {
        match self {
            Target::X86_64 => Runtime {
                meta: Target::X86_64,
                crt1: include_bytes!(concat!(
                    env!("OUT_DIR"),
                    "/x86_64-unknown-linux-musl/crt1.o"
                )),
                crti: include_bytes!(concat!(
                    env!("OUT_DIR"),
                    "/x86_64-unknown-linux-musl/crti.o"
                )),
                nucleus: include_bytes!(concat!(
                    env!("OUT_DIR"),
                    "/x86_64-unknown-linux-musl/runtime.o"
                )),
                crtn: include_bytes!(concat!(
                    env!("OUT_DIR"),
                    "/x86_64-unknown-linux-musl/crtn.o"
                )),
            },
            Target::Aarch64 => Runtime {
                meta: Target::Aarch64,
                crt1: include_bytes!(concat!(
                    env!("OUT_DIR"),
                    "/aarch64-unknown-linux-musl/crt1.o"
                )),
                crti: include_bytes!(concat!(
                    env!("OUT_DIR"),
                    "/aarch64-unknown-linux-musl/crti.o"
                )),
                nucleus: include_bytes!(concat!(
                    env!("OUT_DIR"),
                    "/aarch64-unknown-linux-musl/runtime.o"
                )),
                crtn: include_bytes!(concat!(
                    env!("OUT_DIR"),
                    "/aarch64-unknown-linux-musl/crtn.o"
                )),
            },
        }
    }
}

/// Everything a program link needs beyond the object codegen emits.
#[derive(Debug, Clone, Copy)]
pub struct Runtime {
    pub meta: Target,
    /// `_start`, which calls `__libc_start_main`, which calls `main`.
    pub crt1: &'static [u8],
    /// The prologue half of `.init`/`.fini`.
    pub crti: &'static [u8],
    /// `liss-abi` cross-built for musl and prelinked with `libc.a` and `libunwind.a`.
    pub nucleus: &'static [u8],
    /// The epilogue half. Must come last.
    pub crtn: &'static [u8],
}

impl Runtime {
    /// The link line, in order, with `programma` — the object codegen emitted — in its place.
    pub fn artefacta<'a>(&self, programma: &'a [u8]) -> [(&'static str, &'a [u8]); 5] {
        [
            ("crt1.o", self.crt1),
            ("crti.o", self.crti),
            ("programma.o", programma),
            ("runtime.o", self.nucleus),
            ("crtn.o", self.crtn),
        ]
    }

    /// How many bytes of this compiler binary this target's runtime accounts
    /// for.
    pub fn mensura(&self) -> usize {
        self.crt1.len() + self.crti.len() + self.nucleus.len() + self.crtn.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use object::read::elf::FileHeader;
    use object::{Object, ObjectSection, ObjectSymbol, RelocationTarget};
    use std::collections::{BTreeMap, BTreeSet};

    /// The ELF `e_machine` of a blob, read from the header.
    fn machina(octeti: &[u8]) -> u16 {
        let caput = object::elf::FileHeader64::<object::Endianness>::parse(octeti)
            .expect("an ELF64 object");
        let ordo = caput.endian().expect("a known endianness");
        caput.e_machine(ordo).0
    }

    #[test]
    fn the_machine_numbers_are_the_elf_ones() {
        assert_eq!(Target::X86_64.machina(), object::elf::EM_X86_64.0);
        assert_eq!(Target::Aarch64.machina(), object::elf::EM_AARCH64.0);
    }

    /// Every global symbol the object defines.
    fn definita(fasciculus: &object::File<'_>) -> Vec<String> {
        fasciculus
            .symbols()
            .filter(|s| s.is_global() && s.is_definition())
            .filter_map(|s| s.name().ok())
            .map(str::to_owned)
            .collect()
    }

    /// Every strong undefined symbol: what the program link must still supply.
    fn desiderata(fasciculus: &object::File<'_>) -> Vec<String> {
        fasciculus
            .symbols()
            .filter(|s| s.is_undefined() && !s.is_weak())
            .filter_map(|s| s.name().ok())
            .filter(|n| !n.is_empty())
            // Synthesised by the linker, never defined by an input.
            .filter(|n| *n != "_GLOBAL_OFFSET_TABLE_")
            .map(str::to_owned)
            .collect()
    }

    /// The offset, within `main`, of the first relocation naming each symbol.
    fn vocationes(fasciculus: &object::File<'_>) -> BTreeMap<String, u64> {
        let principium = fasciculus
            .symbol_by_name("main")
            .expect("the runtime object defines `main`");
        let index = principium
            .section_index()
            .expect("`main` is defined in a section");
        let sectio = fasciculus
            .section_by_index(index)
            .expect("that section exists");
        let ab = principium.address();
        let ad = ab + principium.size();

        let mut inventa: BTreeMap<String, u64> = BTreeMap::new();
        for (locus, relocatio) in sectio.relocations() {
            if locus < ab || locus >= ad {
                continue;
            }
            let RelocationTarget::Symbol(idx) = relocatio.target() else {
                continue;
            };
            let Ok(signum) = fasciculus.symbol_by_index(idx) else {
                continue;
            };
            let Ok(nomen) = signum.name() else { continue };
            inventa
                .entry(nomen.to_owned())
                .and_modify(|prius| *prius = (*prius).min(locus - ab))
                .or_insert(locus - ab);
        }
        inventa
    }

    /// The section-relative span of one defined function: `(ab, ad)`.
    fn spatium(fasciculus: &object::File<'_>, nomen: &str) -> (u64, u64) {
        let signum = fasciculus
            .symbol_by_name(nomen)
            .unwrap_or_else(|| panic!("the runtime object defines `{nomen}`"));
        let ab = signum.address();
        (ab, ab + signum.size())
    }

    /// Every offset inside `nomen`'s own body at which a relocation names
    /// `petitum`, relative to the start of that body.
    fn loci(fasciculus: &object::File<'_>, nomen: &str, petitum: &str) -> Vec<u64> {
        let (ab, ad) = spatium(fasciculus, nomen);
        let index = fasciculus
            .symbol_by_name(nomen)
            .and_then(|s| s.section_index())
            .expect("the function is defined in a section");
        let sectio = fasciculus
            .section_by_index(index)
            .expect("that section exists");
        let mut inventa = Vec::new();
        for (locus, relocatio) in sectio.relocations() {
            if locus < ab || locus >= ad {
                continue;
            }
            let RelocationTarget::Symbol(idx) = relocatio.target() else {
                continue;
            };
            let Ok(signum) = fasciculus.symbol_by_index(idx) else {
                continue;
            };
            if signum.name() == Ok(petitum) {
                inventa.push(locus - ab);
            }
        }
        inventa.sort_unstable();
        inventa
    }

    /// How many functions the reachable set may hold before [`clausura`] gives
    /// up rather than quietly scanning the object.
    const LIMES_CLAUSURAE: usize = 32;

    /// The trailing identifier of a v0-mangled symbol, for a readable message:
    /// `_RNvNtCs…_8liss_abi6custos8discerne` → `discerne`.
    fn nomen_breve(nomen: &str) -> &str {
        let cauda = nomen
            .rfind(|c: char| !(c.is_ascii_alphabetic() || c == '_'))
            .map_or(0, |i| i + 1);
        nomen
            .get(cauda..)
            .filter(|s| !s.is_empty())
            .unwrap_or(nomen)
    }

    /// Every named relocation reachable from `radix`, mapped to the function
    /// that makes it — following local, section-relative relocations
    /// transitively through the object.
    fn clausura(fasciculus: &object::File<'_>, radix: &str) -> (BTreeMap<String, String>, usize) {
        // (section, address) → (name, size), for every function in the object.
        let mut munera: BTreeMap<(usize, u64), (String, u64)> = BTreeMap::new();
        for signum in fasciculus.symbols() {
            if signum.kind() != object::SymbolKind::Text || signum.size() == 0 {
                continue;
            }
            let (Some(index), Ok(nomen)) = (signum.section_index(), signum.name()) else {
                continue;
            };
            munera.insert(
                (index.0, signum.address()),
                (nomen.to_owned(), signum.size()),
            );
        }

        let continens =
            |sectio: usize, addendum: i64, basis: u64| -> Option<(usize, u64, String, u64)> {
                let locus = u64::try_from((basis as i64).saturating_add(addendum).max(0)).ok()?;
                let ((index, ab), (nomen, mensura)) =
                    munera.range(..=(sectio, locus)).next_back()?;
                if *index != sectio || locus >= ab + mensura {
                    return None;
                }
                let comites = munera.range((sectio, 0)..=(sectio, u64::MAX)).count();
                assert_eq!(
                    comites,
                    1,
                    "section {sectio} holds {comites} functions, so a landing site \
                 can no longer be resolved by clamping a PC-relative addend to \
                 zero — the architecture's bias has to be undone exactly. \
                 `{}` is where the traversal stopped being sound",
                    nomen_breve(nomen),
                );
                Some((*index, *ab, nomen.clone(), *mensura))
            };

        let principium = fasciculus
            .symbols()
            .find(|s| s.name().is_ok_and(|n| n == radix || n.ends_with(radix)))
            .unwrap_or_else(|| panic!("the runtime object has no `{radix}`"));
        let index = principium
            .section_index()
            .expect("the root is defined in a section");

        let mut restant = vec![(
            index.0,
            principium.address(),
            principium.size(),
            principium.name().unwrap_or(radix).to_owned(),
        )];
        let mut visitata: BTreeSet<(usize, u64)> = BTreeSet::new();
        let mut inventa: BTreeMap<String, String> = BTreeMap::new();

        while let Some((sectio_index, ab, mensura, nomen)) = restant.pop() {
            if !visitata.insert((sectio_index, ab)) {
                continue;
            }
            assert!(
                visitata.len() <= LIMES_CLAUSURAE,
                "the set reachable from `{radix}` exceeds {LIMES_CLAUSURAE} \
                 functions. Either the handler grew a call into something large \
                 — which spec §8.2 forbids on its own — or this traversal has \
                 started following something it should stop at",
            );
            let Ok(sectio) = fasciculus.section_by_index(object::SectionIndex(sectio_index)) else {
                continue;
            };
            for (locus, relocatio) in sectio.relocations() {
                if locus < ab || locus >= ab + mensura {
                    continue;
                }
                let RelocationTarget::Symbol(idx) = relocatio.target() else {
                    continue;
                };
                let Ok(signum) = fasciculus.symbol_by_index(idx) else {
                    continue;
                };
                let Ok(petitum) = signum.name() else { continue };

                if !petitum.is_empty() {
                    // A named data target is a constant or a static, not something the
                    // handler calls.
                    if signum.kind() != object::SymbolKind::Text {
                        continue;
                    }
                    inventa
                        .entry(petitum.to_owned())
                        .or_insert_with(|| nomen_breve(&nomen).to_owned());
                    if !petitum.starts_with("liss_") {
                        continue;
                    }
                }
                let Some(destinatio) = signum.section_index() else {
                    continue;
                };
                if let Some((si, sub_ab, sub_nomen, sub_mensura)) =
                    continens(destinatio.0, relocatio.addend(), signum.address())
                {
                    restant.push((si, sub_ab, sub_mensura, sub_nomen));
                }
            }
        }
        (inventa, visitata.len())
    }

    /// The machine code of one defined function.
    fn machina_corporis<'a>(fasciculus: &'a object::File<'a>, nomen: &str) -> Vec<u8> {
        let (ab, ad) = spatium(fasciculus, nomen);
        let index = fasciculus
            .symbol_by_name(nomen)
            .and_then(|s| s.section_index())
            .expect("the function is defined in a section");
        let sectio = fasciculus
            .section_by_index(index)
            .expect("that section exists");
        let data = sectio.data().expect("the section has contents");
        let ab = usize::try_from(ab - sectio.address()).expect("an offset");
        let ad = usize::try_from(ad - sectio.address()).expect("an offset");
        data[ab..ad].to_vec()
    }

    /// aarch64 `MOVZ w<Rd>, #imm16` with `hw == 0`:
    /// `0x5280_0000 | imm16 << 5 | Rd`. Returns `(Rd, imm16)`.
    fn movz_w(instructio: u32) -> Option<(u32, u32)> {
        (instructio & 0xFFE0_0000 == 0x5280_0000)
            .then_some((instructio & 0x1F, (instructio >> 5) & 0xFFFF))
    }

    /// aarch64 `BL <imm26>` / `BLR <Xn>` — a call, and therefore the point past
    /// which nothing can be said about `w0`.
    fn vocatio(instructio: u32) -> bool {
        instructio & 0xFC00_0000 == 0x9400_0000 || instructio & 0xFFFF_FC1F == 0xD63F_0000
    }

    /// What aarch64 code put in `w0` for the call at `locus`, looked for over
    /// the preceding [`RETRO`] instructions and no further back than the
    /// previous call.
    fn argumentum_primum(corpus: &[u8], locus: u64) -> Option<u32> {
        let locus = usize::try_from(locus).ok()?;
        let mut retro = 1;
        while retro <= RETRO {
            let inceptum = locus.checked_sub(retro * 4)?;
            let quattuor: [u8; 4] = corpus.get(inceptum..inceptum + 4)?.try_into().ok()?;
            if vocatio(u32::from_le_bytes(quattuor)) {
                return None;
            }
            if let Some((0, imm)) = movz_w(u32::from_le_bytes(quattuor)) {
                return Some(imm);
            }
            retro += 1;
        }
        None
    }

    /// How far back [`argumentum_primum`] looks.
    const RETRO: usize = 8;

    /// The `sysconf` request number. 30 on Linux, in musl and glibc alike.
    const SC_PAGESIZE: u32 = 30;

    const SIGSEGV: u32 = 11;
    const SIGBUS: u32 = 7;

    /// The handler, by the tail of its v0-mangled name: it is a private `fn`,
    /// so it has no stable symbol.
    const MANUS: &str = "8liss_abi6custos5manus";

    #[test]
    fn every_blob_is_present_and_is_this_target_s_architecture() {
        for meta in Target::OMNES {
            let r = meta.runtime();
            assert_eq!(r.meta, meta);
            for (nomen, octeti) in [
                ("crt1.o", r.crt1),
                ("crti.o", r.crti),
                ("runtime.o", r.nucleus),
                ("crtn.o", r.crtn),
            ] {
                assert!(
                    !octeti.is_empty(),
                    "{}: {nomen} is empty — a GNU `ld` built for the other \
                     architecture writes exactly this",
                    meta.triple()
                );
                assert_eq!(
                    machina(octeti),
                    meta.machina(),
                    "{}: {nomen} is the wrong architecture",
                    meta.triple(),
                );
            }
        }
    }

    #[test]
    fn the_crt_objects_are_whole() {
        for meta in Target::OMNES {
            let r = meta.runtime();
            let crt1 = object::File::parse(r.crt1).expect("crt1.o is an object file");
            assert!(
                definita(&crt1).iter().any(|d| d == "_start"),
                "{}: crt1.o defines no `_start`",
                meta.triple(),
            );

            for (nomen, octeti) in [("crti.o", r.crti), ("crtn.o", r.crtn)] {
                let fasciculus = object::File::parse(octeti).expect("an object file");
                for sectio in [".init", ".fini"] {
                    assert!(
                        fasciculus.section_by_name(sectio).is_some(),
                        "{}: {nomen} has no {sectio}",
                        meta.triple(),
                    );
                }
            }
        }
    }

    #[test]
    fn the_runtime_defines_the_abi_and_leaves_liss_initium_to_compiled_code() {
        for meta in Target::OMNES {
            let octeti = meta.runtime().nucleus;
            let fasciculus = object::File::parse(octeti).expect("an object file");
            let definita = definita(&fasciculus);

            for nomen in [
                "main",
                "liss_panic",
                "liss_exit",
                "liss_flush",
                "liss_custos_institue",
                "liss_int_new",
                "liss_print_text",
                "liss_live_count",
            ] {
                assert!(
                    definita.iter().any(|d| d == nomen),
                    "{}: the runtime does not define {nomen}",
                    meta.triple(),
                );
            }

            assert!(
                !definita.iter().any(|d| d == "liss_initium"),
                "{}: the runtime defines liss_initium. Compiled code supplies \
                 it; a definition here would be silently linked instead of the \
                 program",
                meta.triple(),
            );
        }
    }

    #[test]
    fn the_prelink_leaves_nothing_undefined_but_the_program_itself() {
        for meta in Target::OMNES {
            let octeti = meta.runtime().nucleus;
            let fasciculus = object::File::parse(octeti).expect("an object file");
            let mut reliqua = desiderata(&fasciculus);
            reliqua.sort();
            reliqua.dedup();
            assert_eq!(
                reliqua,
                vec!["liss_initium".to_owned()],
                "{}: the prelinked runtime must need nothing but liss_initium",
                meta.triple(),
            );
        }
    }

    #[test]
    fn the_prelink_pulled_musl_in() {
        for meta in Target::OMNES {
            let octeti = meta.runtime().nucleus;
            let fasciculus = object::File::parse(octeti).expect("an object file");
            let definita = definita(&fasciculus);
            for nomen in [
                "__libc_start_main",
                "malloc",
                "write",
                "memcpy",
                "sigaltstack",
                "sysconf",
            ] {
                assert!(
                    definita.iter().any(|d| d == nomen),
                    "{}: {nomen} is not defined — libc.a was not prelinked",
                    meta.triple(),
                );
            }
        }
    }

    // D-04
    #[test]
    fn main_installs_the_guard_before_it_calls_the_program() {
        for meta in Target::OMNES {
            let octeti = meta.runtime().nucleus;
            let fasciculus = object::File::parse(octeti).expect("an object file");
            let vocationes = vocationes(&fasciculus);

            let custos = vocationes
                .get("liss_custos_institue")
                .copied()
                .unwrap_or_else(|| {
                    panic!(
                        "{}: `main` never references liss_custos_institue. Deep \
                     recursion is a bare SIGSEGV again and D-04 is reopened — \
                     spec §8, and the module doc on liss-abi's `ingressus`",
                        meta.triple(),
                    )
                });
            let initium = vocationes
                .get("liss_initium")
                .copied()
                .unwrap_or_else(|| panic!("{}: `main` never calls liss_initium", meta.triple()));

            assert!(
                custos < initium,
                "{}: `main` calls liss_initium at +{initium:#x} and installs \
                 the guard only at +{custos:#x}. Everything expensive happens \
                 before the program runs (spec §8.2), and a handler installed \
                 afterwards protects nothing that recursed in between",
                meta.triple(),
            );
        }
    }

    #[test]
    fn the_guard_traps_both_sigsegv_and_sigbus() {
        for meta in Target::OMNES {
            let octeti = meta.runtime().nucleus;
            let fasciculus = object::File::parse(octeti).expect("an object file");
            let sedes = loci(&fasciculus, "liss_custos_institue", "sigaction");
            assert_eq!(
                sedes.len(),
                2,
                "{}: `liss_custos_institue` calls sigaction {} time(s). Spec \
                 §8.2 requires both SIGSEGV and SIGBUS, because on aarch64 a \
                 stack fault may arrive as either, and every test that provokes \
                 one provokes SIGSEGV",
                meta.triple(),
                sedes.len(),
            );

            if meta != Target::Aarch64 {
                continue;
            }
            let corpus = machina_corporis(&fasciculus, "liss_custos_institue");
            let mut signa: Vec<u32> = sedes
                .iter()
                .map(|locus| {
                    argumentum_primum(&corpus, *locus).unwrap_or_else(|| {
                        panic!(
                            "{}: no `movz w0, #imm` within {RETRO} instructions \
                             before the sigaction call at +{locus:#x}",
                            meta.triple(),
                        )
                    })
                })
                .collect();
            signa.sort_unstable();
            assert_eq!(
                signa,
                vec![SIGBUS, SIGSEGV],
                "{}: the guard is installed for signals {signa:?}, not for \
                 SIGBUS ({SIGBUS}) and SIGSEGV ({SIGSEGV})",
                meta.triple(),
            );
        }
    }

    #[test]
    fn the_page_size_is_asked_of_sysconf_and_never_assumed() {
        for meta in Target::OMNES {
            let octeti = meta.runtime().nucleus;
            let fasciculus = object::File::parse(octeti).expect("an object file");
            let sedes = loci(&fasciculus, "liss_custos_institue", "sysconf");
            assert!(
                !sedes.is_empty(),
                "{}: `liss_custos_institue` never calls sysconf — the page size \
                 is being assumed, and spec §8.2 requires it to be asked for, \
                 because aarch64 kernels commonly ship 16 KiB and 64 KiB pages",
                meta.triple(),
            );

            if meta != Target::Aarch64 {
                continue;
            }
            let corpus = machina_corporis(&fasciculus, "liss_custos_institue");
            let quaesita: Vec<Option<u32>> = sedes
                .iter()
                .map(|locus| argumentum_primum(&corpus, *locus))
                .collect();
            assert!(
                quaesita.contains(&Some(SC_PAGESIZE)),
                "{}: no sysconf call site in `liss_custos_institue` is handed \
                 _SC_PAGESIZE ({SC_PAGESIZE}); the requests found at {sedes:?} \
                 were {quaesita:?}",
                meta.triple(),
            );
        }
    }

    #[test]
    fn the_handler_asks_nothing_and_allocates_nothing() {
        for meta in Target::OMNES {
            let octeti = meta.runtime().nucleus;
            let fasciculus = object::File::parse(octeti).expect("an object file");
            let (attingibilia, munera) = clausura(&fasciculus, MANUS);
            println!(
                "{}: the handler is {munera} function(s) and reaches {:?}",
                meta.triple(),
                attingibilia
                    .iter()
                    .map(|(quid, unde)| format!("{quid} (via {unde})"))
                    .collect::<Vec<_>>(),
            );
            assert!(
                munera > 1,
                "{}: the reachable set is {munera} function(s). Either the \
                 handler genuinely calls nothing — in which case `write` could \
                 not be at its boundary — or the traversal is resolving no \
                 landing sites and this check is inert",
                meta.triple(),
            );

            for vetitum in [
                "malloc",
                "calloc",
                "realloc",
                "free",
                "sysconf",
                "getrlimit",
                "open",
                "read",
                "close",
                "sigaction",
                "sigaltstack",
                "exit",
            ] {
                assert!(
                    !attingibilia.contains_key(vetitum),
                    "{}: the signal handler reaches `{vetitum}`, from `{}`. \
                     Everything expensive happens at initialisation, before \
                     `liss_initium` (spec §8.2); by the time the handler runs \
                     there is no stack left and no allocator it can trust",
                    meta.triple(),
                    attingibilia.get(vetitum).map_or("?", String::as_str),
                );
            }

            assert!(
                attingibilia.contains_key("_exit"),
                "{}: the signal handler never reaches `_exit` — it must not \
                 return, and it must not run `atexit` handlers on a corrupted \
                 stack (spec §8.2)",
                meta.triple(),
            );
        }
    }

    #[test]
    fn the_embedded_runtime_costs_what_it_costs() {
        let mut summa = 0;
        for meta in Target::OMNES {
            let r = meta.runtime();
            let mensura = r.mensura();
            println!(
                "{}: runtime.o {} B, crt {} B, total {mensura} B ({:.1} MiB)",
                meta.triple(),
                r.nucleus.len(),
                r.crt1.len() + r.crti.len() + r.crtn.len(),
                mensura as f64 / (1024.0 * 1024.0),
            );
            assert!(
                (1 << 20..6 << 20).contains(&mensura),
                "{}: {mensura} bytes is outside the measured band. Above it \
                 usually means `exonera` stopped stripping the bitcode and the \
                 DWARF, which costs every compiled program four megabytes it \
                 cannot use",
                meta.triple(),
            );
            summa += mensura;
        }
        println!(
            "both targets: {summa} B ({:.1} MiB)",
            summa as f64 / (1024.0 * 1024.0)
        );
        assert!(
            summa < 15 << 20,
            "{summa} bytes is outside spec §9.2's 8–15 MB prediction for both \
             targets"
        );
    }

    #[test]
    fn a_target_can_be_named_by_triple_or_by_architecture() {
        assert_eq!(Target::ex_nomine("x86_64"), Some(Target::X86_64));
        assert_eq!(
            Target::ex_nomine("aarch64-unknown-linux-musl"),
            Some(Target::Aarch64)
        );
        assert_eq!(Target::ex_nomine("riscv64"), None);
        for meta in Target::OMNES {
            assert_eq!(Target::ex_nomine(meta.triple()), Some(meta));
        }
    }

    #[test]
    fn the_link_line_puts_the_crt_around_everything_else() {
        let programma = b"not really an object";
        let artefacta = Target::X86_64.runtime().artefacta(programma);
        let nomina: Vec<_> = artefacta.iter().map(|(n, _)| *n).collect();
        assert_eq!(
            nomina,
            ["crt1.o", "crti.o", "programma.o", "runtime.o", "crtn.o"]
        );
        assert_eq!(artefacta[2].1, programma);
    }
}
