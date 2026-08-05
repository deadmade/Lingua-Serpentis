use lingua_serpentis::{render, run_dir_with_warnings, run_file_with_warnings, ProgramError, Warning};
use std::path::Path;
use std::process::exit;

// Concrete now that both entry points return the same error: `render` needs the
// failure's own partial output and warnings, which a bare `Display` bound
// cannot reach.
fn finish(result: Result<(String, u8, Vec<Warning>), ProgramError>) -> ! {
    let r = render(result);
    // Warnings and errors go to stderr so they never contaminate program output.
    eprint!("{}", r.stderr);
    print!("{}", r.stdout);
    exit(r.code);
}

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
