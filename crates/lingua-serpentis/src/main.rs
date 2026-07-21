use lingua_serpentis::run;
use std::process::exit;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("Usus: lingua-serpentis <path.liss>");
        exit(1);
    }
    let path = &args[1];
    if !path.ends_with(".liss") {
        eprintln!("Error: file must have .liss extension");
        exit(1);
    }
    let src = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => { eprintln!("Error reading file: {e}"); exit(1); }
    };
    match run(&src) {
        Ok(output) => print!("{output}"),
        Err(e) => { eprintln!("{e}"); exit(1); }
    }
}
