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

    /// The section-relative span of one defined function: `(ab, ad)`.
    fn spatium(fasciculus: &object::File<'_>, nomen: &str) -> (u64, u64) {
        let signum = fasciculus
            .symbol_by_name(nomen)
            .unwrap_or_else(|| panic!("the runtime object defines `{nomen}`"));
        let ab = signum.address();
        (ab, ab + signum.size())
    }

    /// **Every** offset inside `nomen`'s body at which a relocation names
    /// `petitum`, relative to the start of that body.
    ///
    /// [`vocationes`] answers "which symbols, and how early"; this answers "how
    /// many times, and where" — which is the question `sigaction` needs, since
    /// installing one disposition and forgetting the other is exactly the
    /// failure spec §8.2's table warns about and a first-offset map cannot see.
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

    /// aarch64 `MOVZ w<Rd>, #imm16` with `hw == 0` — the instruction that puts a
    /// small constant into an argument register. `0x5280_0000 | imm16 << 5 | Rd`.
    ///
    /// Returns `(Rd, imm16)`.
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
    /// the preceding [`RETRO`] instructions **and no further back than the
    /// previous call**.
    ///
    /// This is how a *call's argument* is read off a stripped object with no
    /// disassembler in the dependency list: on aarch64 every instruction is
    /// four bytes, a relocation against a `bl` sits on the `bl` itself, and the
    /// first argument of a C call is `w0`.
    ///
    /// # Two filters, and both are load-bearing
    ///
    /// **`Rd == 0`.** The `sigaddset` calls that build the handler's signal mask
    /// take their signal number in `w1`, and `movz w1, #11` / `movz w1, #7` sit
    /// inside the same [`RETRO`] window as the `sigaction` calls that follow. A
    /// scan that accepted any destination register would read those and get the
    /// right answer for the wrong reason — and would keep getting it after the
    /// `sigaction` arguments themselves had gone wrong.
    ///
    /// **Stopping at a call.** A `bl` clobbers `w0` — it is both the argument
    /// register and the return register — so a `movz w0` on the far side of one
    /// says nothing about this call. Without this the scan is a heuristic that
    /// happens to be right on today's layout; with it, a value found is a value
    /// this call actually receives.
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

    /// How far back [`argumentum_primum`] looks. Eight instructions; the
    /// shipped code sets `w0` one or two before the call.
    const RETRO: usize = 8;

    /// `_SC_PAGESIZE`, the `sysconf` request number. 30 on Linux, in musl and
    /// glibc alike.
    const SC_PAGESIZE: u32 = 30;

    /// The two signal numbers spec §8.2 requires. Both are in the
    /// architecture-independent half of the Linux signal table, so they are the
    /// same on x86_64 and aarch64 — which is what lets §8 need "no per-target
    /// code beyond the two signal numbers" and then have them be the same two.
    const SIGSEGV: u32 = 11;
    const SIGBUS: u32 = 7;

    /// The handler, by the tail of its v0-mangled name. It is a private `fn`,
    /// so it has no stable symbol — but the path segments are stable and the
    /// only thing varying is the crate-disambiguator hash in the middle.
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

    /// The CRT objects are copied verbatim out of the toolchain and never
    /// inspected by `build.rs`, so this is the only thing that looks at their
    /// *contents*. Architecture and non-emptiness are not enough: a truncated
    /// `crt1.o` of the right machine passes both and fails at the first program
    /// link with an undefined `_start`.
    ///
    /// `crt1.o` is the one with symbols. `crti.o` and `crtn.o` carry only the
    /// prologue and epilogue halves of `.init`/`.fini` and define no global
    /// symbol at all, so the assertion for them is that those sections are
    /// present — which is exactly what would be missing from a truncated copy.
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
    ///
    /// # What this test assumes, for whoever it breaks on
    ///
    /// It reads *positional* order — the byte offset of the relocation, not a
    /// proof about execution — and it requires both calls to be in `main`'s own
    /// body. Three legitimate changes would therefore fail it: inlining
    /// `liss_custos_institue` into `main` (its relocation disappears), moving
    /// the call into a helper that `main` calls (likewise), and any
    /// transformation that reorders the two call sites. All three are fail-safe
    /// — the test goes red, nothing ships broken — but none of them is a bug,
    /// so a failure here is a prompt to re-establish the property some other
    /// way, not to delete the call.
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

    /// **Spec §8.2's `SIGBUS` requirement, asserted against the shipped bytes.**
    /// §8.2 asks for `sigaction` on *both* `SIGSEGV` and `SIGBUS`, "because on
    /// aarch64 a stack fault may arrive as either" — a bullet in that section,
    /// not a row of its table. (§8.2's only table is the four bounds methods,
    /// and this has nothing to do with it.)
    ///
    /// `tests/custos.rs` in `liss-abi` proves the handler *works*, but only for
    /// the signal it can provoke, and only on the host. A `SIGBUS` cannot be
    /// provoked reliably from a Lingua Serpentis program on any architecture,
    /// and on this 4 KiB-page x86_64 host under `qemu-aarch64` it cannot be
    /// provoked at all — so the aarch64 half of the requirement is checked here,
    /// by reading the *installer* out of the runtime this compiler embeds rather
    /// than by arranging a fault.
    ///
    /// Two things are asserted, and the second is the one that matters:
    ///
    /// - `liss_custos_institue` calls `sigaction` **twice**, on both targets.
    ///   One call is what a `custos.rs` that had quietly lost its `SIGBUS` line
    ///   would leave, and every existing test would still pass, because every
    ///   existing test faults with `SIGSEGV`.
    /// - on aarch64, the two signal numbers handed to those calls are
    ///   **7 and 11** — `SIGBUS` and `SIGSEGV` — read out of the `movz w0`
    ///   that precedes each `bl`.
    ///
    /// # What this assumes, for whoever it breaks on
    ///
    /// It reads instruction encodings, so it is a statement about *this* code
    /// generator's output. Three legitimate changes would fail it: setting `w0`
    /// more than [`RETRO`] instructions before the call, materialising a signal
    /// number some way other than a `MOVZ` (a shared constant register, say),
    /// and routing the two calls through a loop over an array of signal numbers
    /// — which would be a perfectly good refactor and would leave one `bl` and
    /// no immediates at all. All three are fail-safe, none is a bug, and a
    /// failure here is a prompt to re-establish the property, never to delete a
    /// disposition.
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

    /// **Spec §8.2's page-size requirement**: the page size comes from
    /// `sysconf(_SC_PAGESIZE)`, "never a hardcoded 4096, because aarch64 kernels
    /// commonly ship 16 KiB and 64 KiB pages". A bullet in §8.2, paraphrased —
    /// that section's only table is the four bounds methods.
    ///
    /// This host reports 4096 and `qemu-aarch64` inherits it, so a runtime that
    /// hardcoded 4096 would behave identically to a correct one everywhere this
    /// suite can reach. The property therefore has to be checked as a property
    /// of the code rather than of a run.
    ///
    /// # What is actually measured, which is narrower than "asked once"
    ///
    /// `liss_custos_institue`'s **own body** references `sysconf` at least once,
    /// and on aarch64 at least one of those call sites is handed `_SC_PAGESIZE`.
    ///
    /// It is *not* "asked once and never again". `liss_custos_institue` calls
    /// `fines`, which calls `pagina()` a second time at run time; that call is
    /// invisible here only because `fines` is outlined into its own symbol. An
    /// earlier draft of this test asserted `== 1` and said "asked for once, at
    /// initialisation, and never in the handler" — which would have gone red on
    /// a perfectly correct build that inlined `fines`, and was describing
    /// something the check could not see. The "never in the handler" half is a
    /// real and separate claim, and
    /// [`the_handler_asks_nothing_and_allocates_nothing`] is where it is checked.
    ///
    /// What none of this establishes: nothing here or anywhere in this suite has
    /// ever executed the handler on a kernel whose pages are 16 or 64 KiB. The
    /// call is proven; the arithmetic downstream of a 16384 is not.
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

    /// **§8.2's other half: "Everything expensive happens at initialisation,
    /// before `liss_initium`."**
    ///
    /// The handler runs with no stack left. §8's prose is explicit about what it
    /// may touch — `write(2)`, `_exit(2)`, atomic loads on already-initialised
    /// statics, and integer comparison — and every one of the things it may not
    /// touch is a named libc symbol in the prelinked runtime, so their *absence*
    /// from the handler's relocations is checkable.
    ///
    /// This is the claim
    /// [`the_page_size_is_asked_of_sysconf_and_never_assumed`] used to make in
    /// its failure message without checking it. Here it is checked, and the
    /// difference is not cosmetic: a `sysconf` or an `open` that migrated from
    /// the installer into the handler is precisely the regression §8.2 is
    /// written to prevent, and no test in the workspace could see it.
    ///
    /// Measured on the shipped objects: the handler's only named relocation, on
    /// both targets, is `_exit`. The denylist below is therefore satisfied with
    /// room, and it is a denylist rather than an exact allowlist because
    /// `liss_scribe_erratum` is `#[unsafe(no_mangle)]` and would appear by name
    /// the moment the optimiser stopped inlining it — which is legal, and not a
    /// defect.
    #[test]
    fn the_handler_asks_nothing_and_allocates_nothing() {
        for meta in Target::OMNES {
            let octeti = meta.runtime().nucleus;
            let fasciculus = object::File::parse(octeti).expect("an object file");
            let manus = fasciculus
                .symbols()
                .filter_map(|s| s.name().ok())
                .find(|n| n.ends_with(MANUS))
                .unwrap_or_else(|| panic!("{}: the runtime object has no `{MANUS}`", meta.triple()))
                .to_owned();

            // Allocation — no stack to allocate on, and the allocator's own
            // locks may be held by the faulting thread. Initialisation-time
            // queries — §8.2 computes the bounds "once here and never in the
            // handler". Installation — re-entering it from a fault is a bug.
            // And `exit`, which runs `atexit` handlers on a corrupted stack;
            // `_exit` is the only one permitted, and is required below.
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
                    loci(&fasciculus, &manus, vetitum).is_empty(),
                    "{}: the signal handler calls `{vetitum}`. Everything \
                     expensive happens at initialisation, before `liss_initium` \
                     (spec §8.2); by the time the handler runs there is no stack \
                     left and no allocator it can trust",
                    meta.triple(),
                );
            }

            assert!(
                !loci(&fasciculus, &manus, "_exit").is_empty(),
                "{}: the signal handler never calls `_exit` — it must not \
                 return, and it must not run `atexit` handlers on a corrupted \
                 stack (spec §8.2)",
                meta.triple(),
            );
        }
    }

    /// Spec §9.2 predicted "roughly 8–15 MB of embedded blobs for two targets".
    /// Measured: **3,396,800 B** and **3,524,880 B**, 6.6 MiB together — inside
    /// the prediction, with room.
    ///
    /// It was not, at first. The unstripped prelink was 18.5 MB and 19.1 MB,
    /// and four fifths of that was two things nothing downstream reads:
    /// `.llvmbc` (7.6 MB on x86_64) for an LTO that never runs, and `.debug_*`
    /// (7.2 MB) for a design that emits no debug information at all (spec §2,
    /// §8.4). `build.rs::exonera` removes both. Neither `--whole-archive` nor
    /// `lto = true` touches either, which is why two earlier attempts at this
    /// number moved it by nothing and 3.6 MB the wrong way respectively.
    ///
    /// **The knock-on is larger than the compiler.** `--gc-sections` cannot
    /// reach debug sections or bitcode — they are not reachability-analysed —
    /// so every byte of it was being carried into every user's binary. A linked
    /// program went from 4,627,048 B to **512,912 B**. The "4.6 MB floor" this
    /// task first reported was not a floor.
    ///
    /// The band is wide on purpose: it is a tripwire for a change of *kind* — a
    /// `--whole-archive` prelink, a strip that silently stopped running, an
    /// empty blob — not a size ratchet. Its **upper** bound is what fails if
    /// `exonera` is removed, and that is the bound that matters now.
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
