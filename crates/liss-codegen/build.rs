//! Cross-build `liss-abi` for both musl targets and prelink each one into a
//! single relocatable object, so the shipped compiler can produce a static ELF
//! with **no rustc, no `cc` and no linker on the host** (spec §2, §9.2).
//!
//! Everything here runs at *compiler* build time. It is allowed to need a Rust
//! toolchain, and it is allowed to panic: a cross-build that silently produced
//! half a runtime would surface as an undefined symbol fifteen tasks later.
//!
//! # What gets embedded, and why it is more than the staticlib
//!
//! **Measured, in Task 2's probe and again here: rustc does not bundle `libc.a`
//! into `staticlib` output.** The archive is 330 members of Rust codegen units
//! plus `compiler-builtins`, and not one of them defines `malloc`, `write` or
//! `__libc_start_main`. So each target embeds, from
//! `$(rustc --print target-libdir --target=T)/self-contained/`:
//!
//! - `crt1.o`, `crti.o`, `crtn.o`, copied verbatim;
//! - **one prelinked object** — `ld -r` of the staticlib, `libc.a` and
//!   `libunwind.a` — so every program link is three CRT objects plus one
//!   program object plus one runtime object, with no archive-member resolution
//!   on a path that runs once per compiling fixture.
//!
//! `libunwind.a` is **not** in spec §9.2's list and is required anyway. The
//! precompiled musl `libstd` is built with unwinding, so it references
//! `_Unwind_Resume`, `_Unwind_GetIP` and ten more regardless of what `panic`
//! this crate's profile asks for. Without it the prelinked object carries
//! twelve strong undefined `_Unwind_*` symbols and the program link fails.
//! With it, the only strong undefined left is `liss_initium` — which is
//! exactly the one compiled code supplies.
//!
//! # Three things the probe did not encounter
//!
//! 1. **`ld` cannot be GNU `ld`.** The probe prelinked x86_64 on an x86_64
//!    host. binutils is configured for one architecture, and the aarch64
//!    prelink fails with `Relocations in generic ELF (EM: 183)` — a zero-byte
//!    output and exit 1. `rust-lld` is multi-target, ships with the rustc this
//!    script already requires, and does the same job; `LISS_LD` overrides it.
//! 2. **`-u __libc_start_main` is load-bearing.** The prelink names the symbols
//!    it must keep rather than using `--whole-archive` (18.5 MB against
//!    22.1 MB, and the difference is dead Rust codegen units). Selecting only
//!    the `liss_*` entry points leaves `crt1.o`'s own undefined symbols
//!    unresolved, so the undefined set of `crt1.o` is read out of the object
//!    and added to the list rather than written down here.
//! 3. **The blobs are much bigger than §9.2 predicted** — 18.5 MB and 19.1 MB
//!    against a predicted 8–15 MB for *both*. See `src/target.rs`, which
//!    reports the measured figures as a test.
//!
//! # The four traps
//!
//! - **Recursive cargo deadlock.** `liss-abi` is its own workspace root *and*
//!   the inner build gets an `OUT_DIR`-local `--target-dir`. Either alone still
//!   contends on a `.cargo-lock`.
//! - **Inherited environment.** Outer `RUSTFLAGS`, `CARGO_ENCODED_RUSTFLAGS`,
//!   `RUSTC_WRAPPER`, `CARGO_BUILD_TARGET` and the make jobserver variables all
//!   break a cross-build; each is removed explicitly. See [`cargo_mundum`].
//! - **Building outside the nix shell fails**, unavoidably: musl std must
//!   exist. [`bibliotheca`] detects that and says which target and which flake.
//! - **Staleness.** `rerun-if-changed` on both crates' sources, and
//!   `rerun-if-env-changed` on the two variables that change the answer.

use std::collections::BTreeSet;
use std::env;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The two targets, in the order `Target::OMNES` lists them.
const METAE: [&str; 2] = ["x86_64-unknown-linux-musl", "aarch64-unknown-linux-musl"];

/// Copied verbatim out of `self-contained/`, in link order.
const CRT: [&str; 3] = ["crt1.o", "crti.o", "crtn.o"];

/// The prelinked runtime object's name, under `$OUT_DIR/<triple>/`.
const RUNTIME: &str = "runtime.o";

/// The archives prelinked into it, in this order. `libc.a` is mandatory (rustc
/// bundles none of it); `libunwind.a` is what `libstd`'s unwinding references
/// need, and is required whatever `panic` setting this crate is built with.
const ARCHIVA: [&str; 2] = ["libc.a", "libunwind.a"];

fn main() {
    println!("cargo::rerun-if-changed=build.rs");
    for crate_dir in ["liss-abi", "liss-runtime"] {
        println!("cargo::rerun-if-changed=../{crate_dir}/src");
        println!("cargo::rerun-if-changed=../{crate_dir}/Cargo.toml");
    }
    println!("cargo::rerun-if-env-changed=LISS_RUNTIME_DIR");
    println!("cargo::rerun-if-env-changed=LISS_LD");

    let horreum = env::var_os("LISS_RUNTIME_DIR").map(PathBuf::from);
    let out = PathBuf::from(opus("OUT_DIR"));

    for meta in METAE {
        let sedes = out.join(meta);
        std::fs::create_dir_all(&sedes).unwrap_or_else(|e| panic!("{}: {e}", sedes.display()));
        match &horreum {
            Some(dir) => ex_horreo(dir, meta, &sedes),
            None => conde(meta, &out, &sedes),
        }
    }
}

/// A variable cargo always sets for a build script. Absent means the script is
/// being run by something that is not cargo, and there is nothing to salvage.
fn opus(nomen: &str) -> OsString {
    env::var_os(nomen).unwrap_or_else(|| panic!("cargo sets {nomen} for build scripts"))
}

/// The escape hatch: a packager or a CI image that has already built the four
/// objects points `LISS_RUNTIME_DIR` at a tree of `<triple>/{crt1,crti,crtn,
/// runtime}.o` and no rustc cross-build happens at all.
///
/// This is the only supported way to build the compiler without musl std
/// installed, and it is deliberately dumb: the files are copied, not checked.
/// `src/target.rs`'s tests are the check, and they run against whatever landed
/// here — a wrong blob fails them exactly as a wrong cross-build would.
fn ex_horreo(horreum: &Path, meta: &str, sedes: &Path) {
    println!("cargo::rerun-if-changed={}", horreum.display());
    for nomen in CRT.iter().chain([RUNTIME].iter()) {
        let fons = horreum.join(meta).join(nomen);
        transfer(&fons, &sedes.join(nomen)).unwrap_or_else(|e| {
            panic!(
                "LISS_RUNTIME_DIR is set, so {} must exist and be readable: {e}",
                fons.display()
            )
        });
    }
}

/// Copy, replacing whatever was there.
///
/// **The destination is removed first, and that is not tidiness.** `fs::copy`
/// preserves the source's permissions, and the CRT objects come out of the nix
/// store at mode 0444; the first build copies a read-only file into `OUT_DIR`
/// and the *second* fails to open it for writing with `Permission denied`. It
/// took one incremental rebuild to find and would have looked like anything but
/// a permissions problem.
fn transfer(fons: &Path, ad: &Path) -> std::io::Result<()> {
    match std::fs::remove_file(ad) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(e),
    }
    std::fs::copy(fons, ad).map(drop)
}

/// Cross-build and prelink one target.
fn conde(meta: &str, out: &Path, sedes: &Path) {
    let libdir = bibliotheca(meta);
    let contentum = libdir.join("self-contained");

    for nomen in CRT {
        let fons = contentum.join(nomen);
        transfer(&fons, &sedes.join(nomen)).unwrap_or_else(|e| panic!("{}: {e}", fons.display()));
    }

    let archivum = aedifica(meta, out);
    praeliga(&archivum, &contentum, &sedes.join(RUNTIME));
}

/// The target's `lib` directory, once it is established that musl std is
/// actually installed there.
///
/// `rustc --print target-libdir` answers for a target it has never seen — it is
/// a path computation, not a lookup — so the path alone proves nothing. What
/// proves it is `libstd-*.rlib` beside `self-contained/crt1.o`.
fn bibliotheca(meta: &str) -> PathBuf {
    let rustc = env::var_os("RUSTC").unwrap_or_else(|| OsString::from("rustc"));
    let sermo = Command::new(&rustc)
        .args(["--print", "target-libdir", "--target", meta])
        .output()
        .unwrap_or_else(|e| panic!("{rustc:?} --print target-libdir: {e}"));
    let libdir = PathBuf::from(
        String::from_utf8(sermo.stdout)
            .unwrap_or_else(|e| panic!("rustc printed a non-UTF-8 path: {e}"))
            .trim(),
    );

    let stdlib = std::fs::read_dir(&libdir).ok().is_some_and(|mut d| {
        d.any(|e| {
            e.is_ok_and(|e| {
                let n = e.file_name();
                let n = n.to_string_lossy();
                n.starts_with("libstd-") && n.ends_with(".rlib")
            })
        })
    });
    assert!(
        stdlib && libdir.join("self-contained").join("crt1.o").is_file(),
        "the Rust standard library for {meta} is not installed.\n\
         \n\
         liss-codegen cross-builds the runtime for both musl targets and embeds \
         the result, so this target is required even on a host that will never \
         run it.\n\
         \n\
         Inside this repository:   nix develop -c cargo build\n\
         With rustup:              rustup target add {meta}\n\
         Prebuilt, no toolchain:   set LISS_RUNTIME_DIR to a tree of \
         <triple>/{{crt1,crti,crtn,runtime}}.o\n\
         \n\
         The flake at ./flake.nix pins the toolchain and both targets; a build \
         outside `nix develop` gets neither.\n\
         (looked in {})",
        libdir.display(),
    );
    libdir
}

/// `cargo build --release --features ingressus --target <meta>` on `liss-abi`,
/// returning the staticlib it produced.
///
/// **Release always**, whatever profile the compiler itself is being built in.
/// The alternative makes "debug build" a property of *which* prebuilt musl
/// archive shipped, which doubles the artefact matrix for no benefit — the
/// runtime is not what anyone debugs.
///
/// **`--features ingressus`** is what adds `main`. It is off by default because
/// this crate is also built as an rlib for its own tests, and a test binary
/// already has a `main`.
fn aedifica(meta: &str, out: &Path) -> PathBuf {
    let manifestum = PathBuf::from(opus("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("liss-abi")
        .join("Cargo.toml");
    // **`OUT_DIR`-local, and this is half of the deadlock fix.** `liss-abi`
    // being its own workspace root stops it contending for the *outer*
    // `.cargo-lock`; a private target directory stops it contending for the one
    // in the shared `target/`. Either alone still blocks.
    let aedificium = out.join("aedificium");

    let status = cargo_mundum()
        .arg("build")
        .arg("--manifest-path")
        .arg(&manifestum)
        .arg("--release")
        .arg("--features")
        .arg("ingressus")
        .arg("--target")
        .arg(meta)
        .arg("--target-dir")
        .arg(&aedificium)
        .status()
        .unwrap_or_else(|e| panic!("cargo build for {meta}: {e}"));
    assert!(status.success(), "cargo build for {meta} failed: {status}");

    let archivum = aedificium.join(meta).join("release").join("libliss_abi.a");
    assert!(
        archivum.is_file(),
        "cargo reported success but produced no {}",
        archivum.display()
    );
    archivum
}

/// A `cargo` command with every inherited variable that breaks a cross-build
/// removed.
///
/// Each of these was chosen for a reason:
///
/// - `RUSTFLAGS` / `CARGO_ENCODED_RUSTFLAGS` — the nix shell and CI both set
///   host-specific flags (`-C target-cpu`, `-C link-arg=…`) that are wrong or
///   fatal for a musl cross-build.
/// - `RUSTC_WRAPPER` / `RUSTC_WORKSPACE_WRAPPER` — `sccache` and friends key on
///   an environment this inner build does not share.
/// - `CARGO_BUILD_TARGET` — would silently override the `--target` below.
/// - `CARGO_TARGET_DIR` / `CARGO_BUILD_TARGET_DIR` — would silently override
///   `--target-dir`, putting the inner build back in the contended directory.
/// - `CARGO_MAKEFLAGS` / `MAKEFLAGS` / `MFLAGS` — the jobserver. The file
///   descriptors they name belong to the *outer* cargo, and an inner build that
///   inherits them either hangs waiting for tokens it will never be given or
///   warns on every invocation.
/// - `RUSTDOCFLAGS` / `CARGO_ENCODED_RUSTDOCFLAGS` — same reasoning as
///   `RUSTFLAGS`, for completeness rather than from an observed failure.
///
/// `RUSTC` is deliberately **kept**: it names the toolchain the outer build is
/// using, and the runtime should be built by the same one.
fn cargo_mundum() -> Command {
    let mut cmd = Command::new(env::var_os("CARGO").unwrap_or_else(|| OsString::from("cargo")));
    for nomen in [
        "RUSTFLAGS",
        "CARGO_ENCODED_RUSTFLAGS",
        "RUSTDOCFLAGS",
        "CARGO_ENCODED_RUSTDOCFLAGS",
        "RUSTC_WRAPPER",
        "RUSTC_WORKSPACE_WRAPPER",
        "CARGO_BUILD_TARGET",
        "CARGO_TARGET_DIR",
        "CARGO_BUILD_TARGET_DIR",
        "CARGO_MAKEFLAGS",
        "MAKEFLAGS",
        "MFLAGS",
    ] {
        cmd.env_remove(nomen);
    }
    cmd
}

/// `ld -r` of the staticlib, `libc.a` and `libunwind.a` into one object.
///
/// The exact invocation is
///
/// ```text
/// rust-lld -flavor gnu -r -o <runtime.o> \
///     -u main -u __libc_start_main -u liss_int_add … (39 in all) \
///     <libliss_abi.a> <libc.a> <libunwind.a>
/// ```
///
/// **`-u` rather than `--whole-archive`.** `ld -r` pulls an archive member only
/// to satisfy an undefined symbol, and at prelink time there are none — so
/// without either flag the output is empty. `--whole-archive` on the staticlib
/// works and costs 3.6 MB per target in Rust codegen units nothing will ever
/// call. Naming the symbols keeps only what is reachable from an entry point,
/// and the entry points are not a list anyone has to maintain: they are every
/// `liss_*` symbol the archive defines, plus `main`, plus whatever `crt1.o`
/// says it needs of us.
fn praeliga(archivum: &Path, contentum: &Path, opus: &Path) {
    let mut nomina = signa_archivi(archivum);
    nomina.extend(desiderata_crt(&contentum.join("crt1.o")));
    assert!(
        nomina.contains("main"),
        "{}: no `main`. The `ingressus` feature adds it and the cross-build \
         must pass `--features ingressus`",
        archivum.display()
    );

    let mut cmd = Command::new(ligator());
    cmd.args(["-flavor", "gnu", "-r", "-o"]).arg(opus);
    for nomen in &nomina {
        cmd.arg("-u").arg(nomen);
    }
    cmd.arg(archivum);
    for nomen in ARCHIVA {
        cmd.arg(contentum.join(nomen));
    }

    let status = cmd
        .status()
        .unwrap_or_else(|e| panic!("prelink {}: {e}", opus.display()));
    assert!(
        status.success(),
        "prelink {} failed: {status}",
        opus.display()
    );
    assert!(
        std::fs::metadata(opus).is_ok_and(|m| m.len() > 0),
        "the linker reported success and wrote nothing to {}. A GNU `ld` built \
         for one architecture does exactly this on the other; set LISS_LD to a \
         multi-target linker",
        opus.display()
    );
}

/// The relocatable linker: `LISS_LD`, else the `rust-lld` beside the toolchain.
///
/// **Not the host `ld`.** binutils is configured for a single architecture, and
/// the aarch64 prelink on an x86_64 host fails with `Relocations in generic ELF
/// (EM: 183)` after writing a zero-byte output — measured here, and outside the
/// range of what the probe could have seen on one architecture. `rust-lld` is
/// LLVM's linker, is multi-target, and is installed by every rustc that can
/// cross-compile at all, which this script already requires.
fn ligator() -> PathBuf {
    if let Some(via) = env::var_os("LISS_LD") {
        return PathBuf::from(via);
    }
    let rustc = env::var_os("RUSTC").unwrap_or_else(|| OsString::from("rustc"));
    let sermo = Command::new(&rustc)
        .args(["--print", "sysroot"])
        .output()
        .unwrap_or_else(|e| panic!("{rustc:?} --print sysroot: {e}"));
    let sysroot = PathBuf::from(
        String::from_utf8(sermo.stdout)
            .unwrap_or_else(|e| panic!("rustc printed a non-UTF-8 sysroot: {e}"))
            .trim(),
    );
    let via = sysroot
        .join("lib")
        .join("rustlib")
        .join(String::from_utf8_lossy(opus("HOST").as_encoded_bytes()).as_ref())
        .join("bin")
        .join("rust-lld");
    assert!(
        via.is_file(),
        "no relocatable linker: {} is absent. Set LISS_LD to a multi-target \
         `ld` — the host binutils will not do, it cannot read the other \
         architecture's relocations",
        via.display(),
    );
    via
}

/// Every global symbol the staticlib defines that a compiled program, or the C
/// runtime, can name: the `liss_*` ABI and `main`.
///
/// Read out of the archive rather than written down, so that adding an entry
/// point to `liss-abi` needs no edit here — and so that *removing* one cannot
/// leave a stale `-u` that fails the prelink with an undefined symbol.
fn signa_archivi(archivum: &Path) -> BTreeSet<String> {
    use object::{Object, ObjectSymbol};

    let octeti = std::fs::read(archivum).unwrap_or_else(|e| panic!("{}: {e}", archivum.display()));
    let archive = object::read::archive::ArchiveFile::parse(&*octeti)
        .unwrap_or_else(|e| panic!("{} is not an archive: {e}", archivum.display()));

    let mut nomina = BTreeSet::new();
    for membrum in archive.members() {
        let membrum =
            membrum.unwrap_or_else(|e| panic!("{}: unreadable member: {e}", archivum.display()));
        let Ok(data) = membrum.data(&*octeti) else {
            continue;
        };
        let Ok(obiectum) = object::File::parse(data) else {
            continue;
        };
        for signum in obiectum.symbols() {
            if !signum.is_global() || !signum.is_definition() {
                continue;
            }
            if let Ok(nomen) = signum.name()
                && (nomen == "main" || nomen.starts_with("liss_"))
            {
                nomina.insert(nomen.to_owned());
            }
        }
    }
    nomina
}

/// What `crt1.o` demands of the runtime object: `__libc_start_main`, and `main`
/// again.
///
/// Weak undefined symbols are skipped — `_DYNAMIC`, `_init`, `_fini` all
/// resolve to zero or come from `crti.o` — as is the linker-synthesised
/// `_GLOBAL_OFFSET_TABLE_`, which no archive defines and `-u` would only fail
/// to find.
fn desiderata_crt(crt1: &Path) -> BTreeSet<String> {
    use object::{Object, ObjectSymbol};

    let octeti = std::fs::read(crt1).unwrap_or_else(|e| panic!("{}: {e}", crt1.display()));
    let obiectum = object::File::parse(&*octeti)
        .unwrap_or_else(|e| panic!("{} is not an object file: {e}", crt1.display()));

    obiectum
        .symbols()
        .filter(|s| s.is_undefined() && !s.is_weak())
        .filter_map(|s| s.name().ok())
        .filter(|n| !n.is_empty() && *n != "_GLOBAL_OFFSET_TABLE_")
        .map(str::to_owned)
        .collect()
}
