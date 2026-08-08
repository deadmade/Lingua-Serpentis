//! End-to-end behaviour of the binary. Everything here needs a real process:
//! argv, the stdout/stderr split, exit status, and the failure modes that abort
//! rather than return.
//!
//! **The binary is `liss` as of Task 19** (`support::proc::BINARY`), where it
//! was `lingua-serpentis` before. Spec §9.5 kept the interpreting binary "for
//! this milestone" on the grounds that these twenty-three tests were the
//! regression net for the whole pipeline; once they run against the compiler
//! that reason has been spent, and `main.rs` goes.
//!
//! **Four of the twenty-three changed, and the plan predicted two.** The two it
//! named are the recursion pair below, which asserted the interpreter's
//! 64-frame cap. The other two are the usage-string tests: `liss` has its own
//! three-line usage and its own disambiguation sentence, so a test that pins
//! `Usus: lingua-serpentis …` cannot survive pointing at a different binary.
//! Spec §10.1's "twenty-two are untouched and one is updated" was written before
//! the second recursion test and the binary flip were both assigned here.
//!
//! Nineteen are untouched, which is the number that matters: every diagnostic
//! about paths, folders, permissions, encodings, exit statuses and broken pipes
//! reads exactly as it did, because the front end and `program::load_dir` are
//! shared and the runtime reproduces the rest.
mod support;

use std::ffi::OsStr;
use std::time::Duration;
use support::proc::{run_binary, run_closing_stdout_early, temp_dir, Outcome};

const T: Duration = Duration::from_secs(20);

/// `liss`'s usage, byte for byte: three invocation lines and the sentence that
/// disambiguates a folder named `run`.
///
/// Duplicated from `liss_cli.rs`'s `USUS` deliberately rather than shared. The
/// two files drive the same binary by different routes — that one calls it
/// directly, this one through `support::proc` — and a shared constant would let
/// a reworded usage string stay green in both because both read the new
/// wording. Two independent transcriptions is the point.
const USUS: &str = "\
Usus: liss run <via.liss | directorium>
      liss build <via.liss | directorium> [--exi <via>] [--ad <machina>]
      liss <via.liss | directorium>
Primum argumentum mandatum est solum si 'run' vel 'build' exacte est;
directorium ita nominatum per './run' dandum est.
";

fn run(args: &[&str]) -> Outcome {
    let owned: Vec<&OsStr> = args.iter().map(OsStr::new).collect();
    run_binary(&owned, T)
}

fn exited(o: &Outcome) -> (i32, &str, &str) {
    match o {
        Outcome::Exited { code, stdout, stderr } => (*code, stdout.as_str(), stderr.as_str()),
        other => panic!("expected a normal exit, got {other:?}"),
    }
}

#[test]
fn no_arguments_prints_usage_to_stderr() {
    let o = run(&[]);
    let (code, out, err) = exited(&o);
    assert_eq!(code, 1);
    assert_eq!(out, "");
    assert_eq!(err, USUS);
}

#[test]
fn two_arguments_prints_usage_to_stderr() {
    // Two positional paths, which `liss` refuses for the same reason
    // `lingua-serpentis` refused two arguments: there is one program to run.
    let o = run(&["a.liss", "b.liss"]);
    let (code, _, err) = exited(&o);
    assert_eq!(code, 1);
    assert_eq!(err, USUS);
}

#[test]
fn a_missing_file_is_reported_in_latin() {
    let d = temp_dir("liss-cli");
    let missing = d.path().join("nusquam.liss");
    let o = run(&[missing.to_str().unwrap()]);
    let (code, _, err) = exited(&o);
    assert_eq!(code, 1);
    // D-11. Verbatim now: with the errno tail gone the whole message is ours,
    // so the Global Constraints' carve-out for an OS-supplied tail expires.
    assert_eq!(err, format!("Error: via '{}' non inventa est\n", missing.display()));
}

#[test]
fn a_non_liss_extension_is_refused_in_latin() {
    let d = temp_dir("liss-cli");
    let p = d.write("notae.txt", "munus initium reddit ni { redde nullus; }");
    let o = run(&[p.to_str().unwrap()]);
    let (code, _, err) = exited(&o);
    assert_eq!(code, 1);
    assert_eq!(err, "Error: nomen scidae in '.liss' desinere debet\n");
}

#[test]
fn a_nonexistent_path_is_reported_as_missing_not_as_a_bad_extension() {
    // The assertion here inverts. `is_dir()` runs first, so a path that is
    // neither file nor directory used to fall through to the extension branch
    // and be told it needed a `.liss` extension — a message about extensions
    // for a path that simply is not there. The register treats D-11 as two
    // string literals to translate; its own note shows the control flow was
    // wrong too, and translating alone would have kept the misfire in Latin.
    let d = temp_dir("liss-cli");
    let missing = d.path().join("nusquam");
    let o = run(&[missing.to_str().unwrap()]);
    let (code, _, err) = exited(&o);
    assert_eq!(code, 1);
    assert_eq!(err, format!("Error: via '{}' non inventa est\n", missing.display()));
}

#[test]
fn a_nonexistent_dot_liss_directory_is_also_reported_as_missing() {
    // The other half of the same misfire: a missing `x.liss/` used to be told
    // it could not be read as a file.
    let d = temp_dir("liss-cli");
    let missing = d.path().join("nusquam.liss");
    let o = run(&[missing.to_str().unwrap()]);
    let (code, _, err) = exited(&o);
    assert_eq!(code, 1);
    assert_eq!(err, format!("Error: via '{}' non inventa est\n", missing.display()));
}

#[test]
fn a_directory_named_dot_liss_runs_as_a_program_folder() {
    // is_dir() is checked before the extension, so this is a folder program.
    let d = temp_dir("liss-cli");
    d.write("scaena.liss/principale.liss",
            "munus initium reddit ni { scribe(\"ex directorio\"); redde nullus; }");
    let o = run(&[d.path().join("scaena.liss").to_str().unwrap()]);
    let (code, out, err) = exited(&o);
    assert_eq!((code, out, err), (0, "ex directorio\n", ""));
}

#[test]
fn warnings_go_to_stderr_and_output_to_stdout_separately() {
    let d = temp_dir("liss-cli");
    let p = d.write("fons.liss",
        "munus initium reddit ni {\n  ni result = °V;\n  scribe(result);\n  redde nullus;\n}");
    let o = run(&[p.to_str().unwrap()]);
    let (code, out, err) = exited(&o);
    assert_eq!(code, 0);
    // The contract: a piped stdout is never contaminated by warnings.
    assert_eq!(out, "V\n");
    assert_eq!(err, "Monitum in linea 2: nomen 'result' fortasse non Latinum est\n");
}

#[test]
fn the_process_exit_status_comes_from_initium() {
    let d = temp_dir("liss-cli");
    let p = d.write("fons.liss", "munus initium reddit ni { redde °III; }");
    assert_eq!(exited(&run(&[p.to_str().unwrap()])).0, 3);
}

#[test]
fn an_out_of_range_exit_status_is_truncated_and_says_so() {
    let d = temp_dir("liss-cli");
    // `°C.C.C` is C*C + C = 10_100, not a million: convert_int_roman splits on
    // '.' and multiplies pairwise, adding an odd trailing term. 10_100 mod 256
    // is 116.
    let p = d.write("fons.liss", "munus initium reddit ni { redde °C.C.C; }");
    let o = run(&[p.to_str().unwrap()]);
    let (code, _, err) = exited(&o);
    // The truncation is specified (packages design §4.3) and stays. D-10 was
    // that it happened in silence — a program's own status could become 0 with
    // nothing said.
    assert_eq!(code, 116);
    assert_eq!(
        err,
        "Monitum: reditus 'initium' extra fines status exitus est; ad CXVI redactus\n"
    );
}

#[test]
fn a_negative_exit_status_is_wrapped_and_says_so() {
    let d = temp_dir("liss-cli");
    let p = d.write("fons.liss", "munus initium reddit ni { redde -°I; }");
    let o = run(&[p.to_str().unwrap()]);
    let (code, _, err) = exited(&o);
    assert_eq!(code, 255);
    assert_eq!(
        err,
        "Monitum: reditus 'initium' extra fines status exitus est; ad CCLV redactus\n"
    );
}

#[test]
fn an_exit_status_that_already_fits_says_nothing() {
    let d = temp_dir("liss-cli");
    let p = d.write("fons.liss", "munus initium reddit ni { redde °III; }");
    let o = run(&[p.to_str().unwrap()]);
    let (code, _, err) = exited(&o);
    assert_eq!(code, 3);
    assert_eq!(err, "");
}

#[test]
fn a_hidden_directory_containing_liss_is_skipped() {
    let d = temp_dir("liss-cli");
    d.write("prog/principale.liss",
            "munus initium reddit ni { scribe(\"salve\"); redde nullus; }");
    d.write("prog/.git/vestigium.liss", "munus x reddit ni { redde °I; }");
    let o = run(&[d.path().join("prog").to_str().unwrap()]);
    let (code, out, err) = exited(&o);
    // D-17. Running a program folder that happens to be a git checkout used to
    // fail with `nomen fasciculi '.git' non validum est` — `.git` does not lex
    // as a package name — even though the program beside it was perfectly
    // good. The failure had nothing to do with the program.
    assert_eq!(code, 0);
    assert_eq!(out, "salve\n");
    assert_eq!(err, "");
}

// The three D-22 sites, all reached through `program::load_dir` rather than
// main.rs. Each needs a file mode or a byte sequence the corpus cannot carry
// in git, which is why none of them is fixture-shaped and all three live here.

#[test]
#[cfg(unix)]
fn an_unreadable_file_inside_a_program_folder_is_reported_in_latin() {
    use std::os::unix::fs::PermissionsExt;
    let d = temp_dir("liss-cli");
    let p = d.write("prog/principale.liss", "munus initium reddit ni { redde nullus; }");
    std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o000)).unwrap();
    let o = run(&[d.path().join("prog").to_str().unwrap()]);
    let (code, _, err) = exited(&o);
    assert_eq!(code, 1);
    // Relative to the program root, not an absolute path — which is the other
    // half of D-22 and the part that was leaking a filesystem path.
    assert_eq!(err, "Error: scida 'principale.liss' legi non potest\n");
    std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o644)).unwrap();
}

#[test]
#[cfg(unix)]
fn an_unreadable_subdirectory_is_reported_by_its_relative_name() {
    use std::os::unix::fs::PermissionsExt;
    let d = temp_dir("liss-cli");
    d.write("prog/principale.liss", "munus initium reddit ni { redde nullus; }");
    let sub = d.write("prog/mathematica/summa.liss", "munus s reddit ni { redde °I; }");
    let subdir = sub.parent().unwrap().to_path_buf();
    std::fs::set_permissions(&subdir, std::fs::Permissions::from_mode(0o000)).unwrap();
    let o = run(&[d.path().join("prog").to_str().unwrap()]);
    let (code, _, err) = exited(&o);
    assert_eq!(code, 1);
    // This one used to print an absolute path, alone among every diagnostic in
    // the language.
    assert_eq!(err, "Error: directorium 'mathematica' legi non potest\n");
    std::fs::set_permissions(&subdir, std::fs::Permissions::from_mode(0o755)).unwrap();
}

#[test]
fn a_non_utf8_file_inside_a_program_folder_is_reported_in_latin() {
    let d = temp_dir("liss-cli");
    d.write("prog/principale.liss", "munus initium reddit ni { redde nullus; }");
    let p = d.path().join("prog").join("alia.liss");
    std::fs::write(&p, [0xff, 0xfe, 0x00, 0x62]).unwrap();
    let o = run(&[d.path().join("prog").to_str().unwrap()]);
    let (code, _, err) = exited(&o);
    assert_eq!(code, 1);
    assert_eq!(err, "Error: scida 'alia.liss' signis incognitis scripta est\n");
}

#[test]
fn non_liss_files_in_a_program_folder_are_ignored() {
    let d = temp_dir("liss-cli");
    d.write("prog/principale.liss",
            "munus initium reddit ni { scribe(\"salve\"); redde nullus; }");
    d.write("prog/notae.txt", "non est programma");
    d.write("prog/vacua/intus/.keep", "");
    let o = run(&[d.path().join("prog").to_str().unwrap()]);
    assert_eq!(exited(&o).0, 0);
    assert_eq!(exited(&o).1, "salve\n");
}

#[test]
fn an_unreadable_file_is_reported_not_panicked() {
    let d = temp_dir("liss-cli");
    let p = d.write("fons.liss", "munus initium reddit ni { redde nullus; }");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o000)).unwrap();
    }
    let o = run(&[p.to_str().unwrap()]);
    let (code, _, err) = exited(&o);
    assert_eq!(code, 1);
    // Verbatim. D-22 classified the failure instead of forwarding the OS
    // string, so there is no longer an unportable errno tail to allow for —
    // the Global Constraints' carve-out for this site expires with it.
    assert_eq!(err, format!("Error: scida '{}' legi non potest\n", p.display()));
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o644)).unwrap();
    }
}

#[test]
fn a_non_utf8_source_file_is_reported_not_panicked() {
    let d = temp_dir("liss-cli");
    let p = d.path().join("fons.liss");
    std::fs::write(&p, [0xff, 0xfe, 0x00, 0x62]).unwrap();
    let o = run(&[p.to_str().unwrap()]);
    let (code, _, err) = exited(&o);
    assert_eq!(code, 1);
    // "not valid UTF-8" cannot be said in this language: UTF-8 carries a digit
    // and no message may. `signis incognitis scripta est` says the same thing
    // and chimes with the lexer's own `signum ignotum`.
    assert_eq!(
        err,
        format!("Error: scida '{}' signis incognitis scripta est\n", p.display())
    );
}

#[test]
fn a_closed_pipe_is_not_a_panic() {
    // D-06. Was #[ignore]d and aspirational: `prog | head` produced a Rust
    // panic message, a backtrace note and exit 101 — for the most ordinary
    // idiom a shell has.
    // A real reproduction, not a placeholder: read a little of the child's
    // stdout, close the pipe, and let it keep writing. Every other Unix tool
    // exits quietly here.
    let d = temp_dir("liss-cli");
    let p = d.write(
        "fons.liss",
        "munus initium reddit ni {\n  ni i = nullus;\n  \
         dum (i = nullus; i minor_est °M.M.M.M.M; i = i + °I) {\n    \
         scribe(\"linea satis longa ad tubum implendum\");\n  }\n  redde nullus;\n}",
    );
    let o = run_closing_stdout_early(&[OsStr::new(p.to_str().unwrap())], T);
    match o {
        Outcome::Exited { code, stderr, .. } => {
            assert_ne!(code, 101, "panicked on a closed pipe; stderr: {stderr:?}");
            assert!(!stderr.contains("panicked"), "stderr: {stderr:?}");
        }
        Outcome::Signalled { signal, .. } => assert_eq!(signal, 13, "expected SIGPIPE or a clean exit"),
        Outcome::TimedOut => panic!("hung on a closed pipe"),
    }
}

#[test]
fn unbounded_recursion_is_a_clean_latin_error() {
    let d = temp_dir("liss-cli");
    let p = d.write("fons.liss",
        "munus recurre capit (ni n) reddit ni { redde recurre(n + °I); }\n\
         munus initium reddit ni { scribe(recurre(°I)); redde nullus; }");
    let o = run(&[p.to_str().unwrap()]);
    // Was #[ignore]d and aspirational: the process used to be Signalled, so
    // `exited` panicked before reaching any assertion. D-04.
    let (code, out, err) = exited(&o);
    assert_eq!(code, 1);
    // **The sentence changed with the backend, and this is the only test in
    // this file that says so.** The interpreter refused the 65th frame and said
    // `vocatio nimis alte implicata` at the call's line; compiled code reaches
    // the guard page and §8's handler answers with no line at all (§8.4),
    // because a faulting thread cannot know which call was the last.
    //
    // `exited` rather than a signal check is the whole claim: the child dies by
    // SIGSEGV and `liss` must turn that into a status, never let it through.
    assert_eq!(err, "Error: acervus vocationum exhaustus est\n");
    assert_eq!(out, "");
}

#[test]
fn deep_expression_nesting_is_a_clean_latin_error() {
    // D-05's half. Twenty thousand parentheses used to abort the parser the
    // same way; the register cites only this route, but nested `si` blocks and
    // stacked unary operators each overflowed independently and are guarded by
    // the same counter.
    let d = temp_dir("liss-cli");
    let src = format!(
        "munus initium reddit ni {{ scribe({}°I{}); redde nullus; }}",
        "(".repeat(20_000),
        ")".repeat(20_000)
    );
    let p = d.write("fons.liss", &src);
    let o = run(&[p.to_str().unwrap()]);
    let (code, _, err) = exited(&o);
    assert_eq!(code, 1);
    assert_eq!(err, "Error in linea 1: expressio nimis alte implicata\n");
}

/// **There is no limit, and ten thousand levels is how that is stated.**
///
/// Renamed from `recursion_inside_the_limit_still_runs`, which asked whether
/// the interpreter's ceiling was set too low — a question about
/// `VOCATIONES_MAXIMAE = 64` that stopped meaning anything when spec §3.3
/// removed it. It ran `fac(°X)`: ten frames, comfortably inside a cap of
/// sixty-four, and equally comfortably inside a cap of eleven.
///
/// Ten thousand is chosen to be unmistakably past the old ceiling — two orders
/// of magnitude beyond it, and well past the ~2,000 the interpreter managed
/// even before the cap — while staying inside the 40,000–80,000 a default 8 MiB
/// stack allows (§3.3). So this is the capability gain, asserted: a program the
/// language could not run yesterday.
///
/// It is also the guard on §8's guard. `currunt/recursio-profunda` makes the
/// same point in the corpus at 5,000; a handler that fired early would fail
/// both, and only these two would notice.
#[test]
fn deep_recursion_runs_because_there_is_no_limit() {
    let d = temp_dir("liss-cli");
    let p = d.write("fons.liss",
        "munus numera capit (ni n) reddit ni { si (n minor_est °I) { redde nullus; } redde °I + numera(n - °I); }\n\
         munus initium reddit ni { scribe(numera(°X.M)); redde nullus; }");
    let o = run(&[p.to_str().unwrap()]);
    let (code, out, err) = exited(&o);
    assert_eq!(code, 0);
    assert_eq!(err, "");
    // `°X.M` is X × M = 10,000 — `convert_int_roman` splits on '.' and
    // multiplies pairwise. Ten thousand prints X̅, a vinculum over the X
    // multiplying it by a thousand.
    assert_eq!(out, "X\u{305}\n");
}
