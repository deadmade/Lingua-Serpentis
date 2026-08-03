use lingua_serpentis::{render, run_dir_with_warnings, run_file_with_warnings, Warning};
use std::path::Path;
use std::process::exit;

fn finish<E: std::fmt::Display>(result: Result<(String, u8, Vec<Warning>), E>) -> ! {
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
    if !args[1].ends_with(".liss") {
        eprintln!("Error: file must have .liss extension");
        exit(1);
    }
    let src = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => { eprintln!("Error reading file: {e}"); exit(1); }
    };
    finish(run_file_with_warnings(&src));
}
