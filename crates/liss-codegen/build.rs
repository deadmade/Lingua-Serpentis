//! Cross-build `liss-abi` for both musl targets and prelink each one (spec §9.2).

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

/// The archives prelinked into it, in this order.
const ARCHIVA: [&str; 2] = ["libc.a", "libunwind.a"];

fn main() {
    println!("cargo::rerun-if-changed=build.rs");
    for crate_dir in ["liss-abi", "liss-runtime"] {
        println!("cargo::rerun-if-changed=../{crate_dir}/src");
        println!("cargo::rerun-if-changed=../{crate_dir}/Cargo.toml");
    }
    println!("cargo::rerun-if-env-changed=LISS_RUNTIME_DIR");
    println!("cargo::rerun-if-env-changed=LISS_LD");
    println!("cargo::rerun-if-env-changed=LISS_OBJCOPY");
    // `RUSTC` is an input: switching toolchains changes the runtime.
    println!("cargo::rerun-if-env-changed=RUSTC");

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

/// A variable cargo always sets for a build script.
fn opus(nomen: &str) -> OsString {
    env::var_os(nomen).unwrap_or_else(|| panic!("cargo sets {nomen} for build scripts"))
}

/// The escape hatch: `LISS_RUNTIME_DIR` names a prebuilt tree of
/// `<triple>/{crt1,crti,crtn,runtime}.o`. The files are copied, not checked.
///
/// D-29
fn ex_horreo(horreum: &Path, meta: &str, sedes: &Path) {
    for nomen in CRT.iter().chain([RUNTIME].iter()) {
        let fons = horreum.join(meta).join(nomen);
        // Each blob, not the directory: replacing one in place changes no listing.
        println!("cargo::rerun-if-changed={}", fons.display());
        transfer(&fons, &sedes.join(nomen)).unwrap_or_else(|e| {
            panic!(
                "LISS_RUNTIME_DIR is set, so {} must exist and be readable: {e}",
                fons.display()
            )
        });
    }
}

/// Copy, replacing whatever was there — removed first, because `fs::copy`
/// preserves the source's mode, and the CRT objects arrive at 0444.
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

/// The target's `lib` directory, once musl std is established to be there:
/// `--print target-libdir` is a path computation, so the path alone proves nothing.
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

/// The inner cargo build of `liss-abi`, returning the staticlib it produced.
fn aedifica(meta: &str, out: &Path) -> PathBuf {
    let manifestum = PathBuf::from(opus("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("liss-abi")
        .join("Cargo.toml");
    // `OUT_DIR`-local: half of the deadlock fix, the other half being that
    // `liss-abi` is its own workspace root.
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

/// A `cargo` command with the inherited variables that break a cross-build removed.
fn cargo_mundum() -> Command {
    let mut cmd = Command::new(env::var_os("CARGO").unwrap_or_else(|| OsString::from("cargo")));
    for nomen in [
        "RUSTFLAGS",
        "RUSTDOCFLAGS",
        "CARGO_ENCODED_RUSTDOCFLAGS",
        "CARGO_BUILD_RUSTFLAGS",
        "CARGO_BUILD_RUSTDOCFLAGS",
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
    // Set, not removed: rustflags have no command-line form, so this has to beat
    // a `.cargo/config.toml` rather than merely not be inherited.
    cmd.env("CARGO_ENCODED_RUSTFLAGS", "");
    cmd
}

/// `ld -r` of the staticlib, `libc.a` and `libunwind.a` into one object,
/// keeping what `-u` names rather than the whole archive.
fn praeliga(archivum: &Path, contentum: &Path, opus: &Path) {
    let mut nomina = signa_archivi(archivum);
    // Asked before the `extend`: `crt1.o` needs `main` too, and merging it in
    // first would make this vacuous.
    assert!(
        nomina.contains("main"),
        "{}: the archive defines no `main`. The `ingressus` feature adds it and \
         the cross-build must pass `--features ingressus`",
        archivum.display()
    );
    nomina.extend(desiderata_crt(&contentum.join("crt1.o")));

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

    exonera(opus);
}

/// Remove what nothing downstream reads: embedded LLVM bitcode, and debug
/// information. Four fifths of the object.
fn exonera(opus: &Path) {
    let status = Command::new(instrumentum("LISS_OBJCOPY", "rust-objcopy"))
        .arg("--remove-section=.llvmbc")
        .arg("--remove-section=.llvmcmd")
        .arg("--strip-debug")
        .arg(opus)
        .status()
        .unwrap_or_else(|e| panic!("strip {}: {e}", opus.display()));
    assert!(
        status.success(),
        "strip {} failed: {status}",
        opus.display()
    );
}

/// The relocatable linker: `LISS_LD`, else the `rust-lld` beside the toolchain.
fn ligator() -> PathBuf {
    instrumentum("LISS_LD", "rust-lld")
}

/// A cross-capable binutil out of the rustc sysroot, overridable by `clavis`.
fn instrumentum(clavis: &str, nomen: &str) -> PathBuf {
    if let Some(via) = env::var_os(clavis) {
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
        .join(nomen);
    assert!(
        via.is_file(),
        "{} is absent. Set {clavis} to a multi-target replacement — the host \
         binutils will not do, it cannot read the other architecture's \
         relocations",
        via.display(),
    );
    via
}

/// Every global symbol the staticlib defines that a compiled program, or the C
/// runtime, can name: the `liss_*` ABI and `main`.
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
