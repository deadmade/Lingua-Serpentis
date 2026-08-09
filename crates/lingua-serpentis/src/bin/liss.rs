//! A shim; the driver and every sentence it can say live in `lingua_serpentis::cli`.
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    lingua_serpentis::cli::liss(&args)
}
