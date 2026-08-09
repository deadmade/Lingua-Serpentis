//! Environment assertions, not unit tests: they fail loudly in a shell that cannot build the runtime.
use std::process::Command;

fn target_libdir(triple: &str) -> Option<String> {
    let out = Command::new("rustc")
        .args(["--print", "target-libdir", "--target", triple])
        .output().ok()?;
    out.status.success().then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
}

#[test]
fn musl_std_exists_for_both_targets() {
    for t in ["x86_64-unknown-linux-musl", "aarch64-unknown-linux-musl"] {
        let dir = target_libdir(t).unwrap_or_else(|| panic!("rustc knows no target-libdir for {t}"));
        assert!(std::path::Path::new(&dir).join("libstd.rlib").exists()
             || std::fs::read_dir(&dir).map(|mut d| d.any(|e| e.map(|e|
                    e.file_name().to_string_lossy().starts_with("libstd-")).unwrap_or(false)))
                .unwrap_or(false),
            "no libstd in {dir} for {t}");
    }
}

/// Asserted here rather than skipped in `liss-codegen/tests/aemulatio.rs`, which runs under it.
#[test]
fn qemu_aarch64_is_on_the_path() {
    let out = Command::new("qemu-aarch64").arg("--version").output();
    let out = out.unwrap_or_else(|e| {
        panic!(
            "qemu-aarch64 did not run ({e}). It is `pkgs.qemu` in flake.nix; \
             without it the aarch64 target can be built but never executed"
        )
    });
    assert!(
        out.status.success(),
        "qemu-aarch64 --version exited {:?}",
        out.status.code(),
    );
}

/// `aemulatio.rs`'s `acervus_exhaustus_sub_aemulatore` inherits this limit; see docs/internals/testing.md.
#[test]
fn the_stack_limit_is_finite_and_large_enough_for_the_guard_test() {
    const MINIMUM: u64 = 8 * 1024 * 1024;
    let limits = std::fs::read_to_string("/proc/self/limits")
        .expect("/proc is mounted; the guard tests need it too");
    let linea = limits
        .lines()
        .find(|l| l.starts_with("Max stack size"))
        .expect("/proc/self/limits names the stack limit");
    let mollis = linea
        .split_whitespace()
        .nth(3)
        .expect("a soft limit column");

    assert_ne!(
        mollis, "unlimited",
        "RLIMIT_STACK is unlimited. The runtime then has no floor to compare \
         against (spec §8.2 treats RLIM_INFINITY as bounds-unknown), the \
         handler reports exhaustion unconditionally, and \
         `acervus_exhaustus_sub_aemulatore` passes without ever exercising the \
         discriminator. Run with a finite `ulimit -s`",
    );
    let mollis: u64 = mollis
        .parse()
        .unwrap_or_else(|e| panic!("cannot read the soft stack limit {mollis:?}: {e}"));
    assert!(
        mollis >= MINIMUM,
        "RLIMIT_STACK is {mollis} bytes; `acervus_exhaustus_sub_aemulatore` \
         needs at least {MINIMUM}. Below it, qemu-user still gives the guest an \
         8 MiB stack, so the floor the runtime derives lands above the real one \
         and a correct build reports `memoria violata est: signum` — a failure that \
         reads as a §8 regression and is not one",
    );
}

#[test]
fn the_self_contained_crt_objects_are_present() {
    for t in ["x86_64-unknown-linux-musl", "aarch64-unknown-linux-musl"] {
        let dir = target_libdir(t).unwrap();
        let sc = std::path::Path::new(&dir).join("self-contained");
        for o in ["crt1.o", "crti.o", "crtn.o"] {
            assert!(sc.join(o).exists(), "missing {o} in {}", sc.display());
        }
    }
}
