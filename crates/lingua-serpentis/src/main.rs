//! The interpreting binary, unchanged. Spec §9.5 keeps it beside `liss` for
//! this milestone because its twenty-three tests are the regression net for the
//! whole pipeline, and losing them in the milestone that swaps the backend
//! would be exactly the wrong time.
//!
//! `write_or_exit` and `finish` moved to `lingua_serpentis::cli`, so that
//! broken-pipe handling and stream order have one implementation shared with
//! `liss`. Nothing else here changed, and nothing about what this binary does.
use lingua_serpentis::cli::finish;
use lingua_serpentis::{run_dir_with_warnings, run_file_with_warnings};
use std::path::Path;
use std::process::exit;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("Usus: lingua-serpentis <via.liss | directorium>");
        exit(1);
    }
    let path = Path::new(&args[1]);
    if path.is_dir() {
        finish(run_dir_with_warnings(path));
    }
    // Defectus D-11 was two English strings here — but translating them would
    // have left the real bug in place, in Latin. The control flow was wrong:
    // a directory that does not exist fails `is_dir()`, fell through to the
    // extension check, and was told it needed a `.liss` extension; a missing
    // `x.liss/` directory was told it could not be read as a file. Both
    // misfired. Asking whether the path exists at all, before asking what kind
    // of path it is, is what fixes them.
    if !path.exists() {
        eprintln!("Error: via '{}' non inventa est", args[1]);
        exit(1);
    }
    if !args[1].ends_with(".liss") {
        eprintln!("Error: nomen scidae in '.liss' desinere debet");
        exit(1);
    }
    let src = match std::fs::read_to_string(path) {
        Ok(s) => s,
        // The same classification the library uses, so a file that cannot be
        // read reads the same whether it was named on the command line or
        // found inside a program folder.
        Err(e) => {
            eprintln!("{}", lingua_serpentis::program::scida_legi_non_potest(&args[1], &e));
            exit(1);
        }
    };
    finish(run_file_with_warnings(&src));
}
