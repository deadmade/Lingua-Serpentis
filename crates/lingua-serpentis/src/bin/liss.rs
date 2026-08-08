//! `liss` — compile and run, or compile and keep.
//!
//! A shim, and deliberately silent: every sentence this binary can say lives in
//! `lingua_serpentis::cli`, because `tests/catalogus.rs` scans `crates/*/src`
//! without recursing and would never see one written here.
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    lingua_serpentis::cli::liss(&args)
}
