//! Two properties of the embedded runtime that come from *how* `build.rs`
//! invokes cargo, not from what `abi` says about itself. Both went silent when
//! `abi` joined this workspace: `docs/internals/abi.md`.

use codegen::target::Target;
use object::{Object, ObjectSection};

/// `panic = "abort"` reached the inner build.
///
/// A workspace member's own `[profile.release]` is ignored — cargo honours
/// profiles at the workspace root only — so `build.rs` passes `--config`
/// instead. Without it every `Result`-returning function in `abi` and
/// `runtime` regains a landing pad, in a runtime whose panic never unwinds.
/// `libstd` and `libunwind` keep theirs either way; these are ours.
#[test]
fn nucleus_nostra_sine_pontibus() {
    for meta in Target::OMNES {
        let obiectum = object::File::parse(meta.runtime().nucleus)
            .unwrap_or_else(|e| panic!("{}: the runtime is not an object file: {e}", meta.triple()));

        let nostrae: Vec<String> = obiectum
            .sections()
            .filter_map(|s| s.name().ok().map(str::to_owned))
            .filter(|n| n.starts_with(".gcc_except_table"))
            .filter(|n| n.contains("3abi") || n.contains("7runtime"))
            .collect();

        assert!(
            nostrae.is_empty(),
            "{}: {} landing pads in our own code. `build.rs` must pass \
             `--config profile.release.panic=\"abort\"` to the inner build; \
             `abi`'s own `[profile.release]` is ignored now that it is a \
             member of this workspace.\nfirst: {}",
            meta.triple(),
            nostrae.len(),
            nostrae[0],
        );
    }
}

/// No path from the machine that built the compiler.
///
/// `runtime` is a path dependency of `abi`, and `#[track_caller]` locations
/// reach the shipped bytes. While `abi` was its own workspace, cargo named
/// that dependency absolutely and every compiled program carried the build
/// machine's home directory; one workspace makes the same names relative.
#[test]
fn nucleus_sine_via_aedificatoris() {
    let radix = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .expect("crates/codegen sits two below the workspace root")
        .to_owned();
    let radix = radix.to_str().expect("a UTF-8 workspace root").as_bytes();

    for meta in Target::OMNES {
        let nucleus = meta.runtime().nucleus;
        assert!(
            !nucleus.windows(radix.len()).any(|f| f == radix),
            "{}: the runtime carries {}. The inner cargo build must see \
             `runtime` as a workspace-relative path dependency.",
            meta.triple(),
            String::from_utf8_lossy(radix),
        );
    }
}
