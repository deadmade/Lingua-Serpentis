use lingua_serpentis::{render, run_dir_with_warnings, run_file_with_warnings, ProgramError, Warning};
use std::io::Write;
use std::path::Path;
use std::process::exit;

/// Write `text` to `w`, treating a closed pipe as an ordinary end of output.
///
/// Defectus D-06. Rust's runtime sets SIGPIPE to SIG_IGN, so the `print!` this
/// replaces panicked on `prog | head` — a Rust panic message, a backtrace note
/// and exit 101, for the most ordinary idiom a shell has. Every Unix tool
/// terminates quietly instead, and `head` closing its end of the pipe is not
/// an error the user needs telling about.
///
/// Zero runtime dependencies rules out resetting the signal handler through
/// libc, so the write error is handled where it surfaces. `exit` does not run
/// destructors, which means nothing else would flush these handles: the flush
/// here is load-bearing rather than tidy.
fn write_or_exit(w: &mut impl Write, text: &str) {
    match w.write_all(text.as_bytes()).and_then(|()| w.flush()) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::BrokenPipe => exit(0),
        // Anything else is a real failure to write, and saying so on a stream
        // that may itself be broken is not worth a second panic.
        Err(_) => exit(1),
    }
}

fn finish(result: Result<(String, u8, Vec<Warning>), ProgramError>) -> ! {
    let r = render(result);
    // Warnings and errors go to stderr so they never contaminate program output.
    write_or_exit(&mut std::io::stderr(), &r.stderr);
    write_or_exit(&mut std::io::stdout(), &r.stdout);
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
