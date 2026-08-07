//! The two targets, and the runtime embedded for each (spec §9.2).
//!
//! `build.rs` cross-builds `liss-abi` for both musl triples, prelinks each with
//! `libc.a`, `libunwind.a` and copies the three CRT objects out of the
//! toolchain's `self-contained/` directory. This module is where those bytes
//! surface: a program link is [`Runtime::artefacta`] with codegen's own object
//! spliced in, and nothing else — no archives, no member resolution, no
//! toolchain on the machine.

/// A target `liss build` can emit for. Linux only, and exactly two: spec §2
/// names `x86_64` and `aarch64`, and §9.4 draws the Windows seam somewhere
/// else entirely.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Target {
    X86_64,
    Aarch64,
}

impl Target {
    /// Both, in the order `build.rs` builds them.
    pub const OMNES: [Target; 2] = [Target::X86_64, Target::Aarch64];

    /// The Rust target triple. Always musl: spec §2 requires a static ELF that
    /// runs on any Linux of its architecture with nothing else installed, and
    /// glibc cannot be linked statically and mean that.
    pub const fn triple(self) -> &'static str {
        match self {
            Target::X86_64 => "x86_64-unknown-linux-musl",
            Target::Aarch64 => "aarch64-unknown-linux-musl",
        }
    }

    /// The ELF `e_machine` of every object this target's runtime carries:
    /// `EM_X86_64` and `EM_AARCH64`. The one number that says an embedded blob
    /// is not the *other* target's blob.
    pub const fn machina(self) -> u16 {
        match self {
            Target::X86_64 => 62,
            Target::Aarch64 => 183,
        }
    }

    /// The linker's `-m` emulation name, for the argv Task 11 hands libwild.
    /// It lives here so that the two facts that must agree — which triple was
    /// cross-built, and which emulation links it — are one `match` apart.
    pub const fn emulatio(self) -> &'static str {
        match self {
            Target::X86_64 => "elf_x86_64",
            Target::Aarch64 => "aarch64linux",
        }
    }

    /// The target this compiler is itself running on, if it is one of the two.
    /// `None` on anything else, which is a compiler that can cross-compile but
    /// cannot `liss run`.
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

    /// The embedded runtime for this target.
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
///
/// Four blobs, all `'static`: they are in the compiler's `.rodata`, put there
/// at *its* build time by `build.rs`. Reading a field costs nothing and copies
/// nothing.
#[derive(Debug, Clone, Copy)]
pub struct Runtime {
    /// Which target these bytes are for. Carried so a `Runtime` that has been
    /// passed around still knows its own `emulatio` and `machina`.
    pub meta: Target,
    /// The C runtime's entry stub: `_start`, which calls `__libc_start_main`,
    /// which calls `main`.
    pub crt1: &'static [u8],
    /// The prologue half of `.init`/`.fini`.
    pub crti: &'static [u8],
    /// **The runtime.** `liss-abi` cross-built for musl and prelinked (`ld -r`)
    /// with `libc.a` and `libunwind.a`, because rustc bundles neither into a
    /// `staticlib` — measured, spec §9.2. One strong undefined symbol survives
    /// in it, `liss_initium`, and compiled code is what supplies it.
    pub nucleus: &'static [u8],
    /// The epilogue half. Must come last.
    pub crtn: &'static [u8],
}

impl Runtime {
    /// The link line, in order, with `programma` — the object codegen emitted —
    /// in its place.
    ///
    /// The order is not decoration. `crt1.o` must precede `crti.o`, both must
    /// precede everything with code in it, and `crtn.o` must come last or the
    /// `.init`/`.fini` sections are left without their epilogue. The names are
    /// what the in-memory `FileSystem` of spec §9.3 will answer to, so they must
    /// be distinct and must look like filenames.
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
    use std::collections::BTreeMap;

    /// The ELF `e_machine` of a blob, read from the header rather than inferred
    /// from `object`'s `Architecture` enum: the assertion is about the number in
    /// the file.
    fn machina(octeti: &[u8]) -> u16 {
        let caput = object::elf::FileHeader64::<object::Endianness>::parse(octeti)
            .expect("an ELF64 object");
        let ordo = caput.endian().expect("a known endianness");
        caput.e_machine(ordo).0
    }

    /// The two literals in [`Target::machina`] are `EM_X86_64` and
    /// `EM_AARCH64`, and this is what says so — the public constant carries the
    /// number, and the number has to be the right one.
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
    ///
    /// This is the check the whole task turns on, and it is deliberately
    /// stronger than the Task 9 tripwire it replaces. That one asked whether a
    /// file containing `fn main(` also contained `liss_custos_institue(`
    /// somewhere, which is satisfied by a call in an unrelated function, a call
    /// placed after `liss_initium`, or a comment. This asks the *shipped
    /// object* which symbols `main`'s own bytes reference and at what offset —
    /// a question no comment can answer.
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
        // In a relocatable object a symbol's `address` is its offset within its
        // own section, and a section's relocation offsets are section-relative
        // too — so the two are directly comparable.
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

    /// The prelink is complete, and the proof is a subtraction: after `ld -r`
    /// there is **exactly one** strong undefined symbol left, and it is the one
    /// the program provides.
    ///
    /// This is also how `libc.a`'s necessity was re-established rather than
    /// taken on trust. Measured on this build, x86_64, prelinking the staticlib
    /// alone: **165** strong undefined symbols, `malloc`, `memcpy`, `write`,
    /// `sigaltstack` and `sysconf` among them. Adding `libc.a`: 12, all
    /// `_Unwind_*`. Adding `libunwind.a` too: none. (`nm --defined-only` over
    /// the 331-member staticlib finds no musl symbol at all — spec §9.2's
    /// measurement, reproduced.) Each of those three configurations fails here
    /// before it can fail at a program link fifteen tasks later.
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

    /// rustc bundles no `libc.a` into `staticlib` output (spec §9.2, measured
    /// twice). These six are the ones the probe named; if the separate
    /// `libc.a` ever stops being embedded they go missing here first.
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

    /// **The hard requirement carried from Task 9.**
    ///
    /// `liss_custos_institue` had no caller, and a `main` that called
    /// `liss_initium` and forgot it would revert stack exhaustion to signal
    /// death with an empty stderr — D-04, reopened with no test failing. Every
    /// Task 9 test still passes without the call, because each installs the
    /// handler explicitly.
    ///
    /// So the assertion is made against the object, not the source: `main`'s
    /// own relocations must name `liss_custos_institue`, and must name it at a
    /// **lower offset** than `liss_initium` — the installer is called, from
    /// `main`, first.
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

    /// Spec §9.2 predicted "roughly 8–15 MB of embedded blobs for two targets".
    /// It is **two and a half times that**, and the number should be true
    /// rather than reassuring, so it is asserted rather than described.
    ///
    /// The band is wide on purpose: it is a tripwire for a change of *kind* —
    /// a `--whole-archive` prelink that stopped selecting symbols, a debug
    /// runtime, an empty blob — not a size ratchet. A real reduction should
    /// change the numbers below in the same commit that earns it.
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
                (8 << 20..48 << 20).contains(&mensura),
                "{}: {mensura} bytes is outside the measured band",
                meta.triple(),
            );
            summa += mensura;
        }
        println!(
            "both targets: {summa} B ({:.1} MiB)",
            summa as f64 / (1024.0 * 1024.0)
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
