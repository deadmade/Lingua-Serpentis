//! The toolchain the compiler backend needs. These are environment assertions,
//! not unit tests: they fail loudly in a shell that cannot build the runtime.
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
