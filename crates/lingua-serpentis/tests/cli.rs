//! End-to-end behaviour of the `lingua-serpentis` binary. Everything here
//! needs a real process: argv, the stdout/stderr split, exit status, and the
//! failure modes that abort rather than return.
mod support;

use std::ffi::OsStr;
use std::time::Duration;
use support::proc::{run_binary, run_closing_stdout_early, temp_dir, Outcome};

const T: Duration = Duration::from_secs(20);

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
    assert_eq!(err, "Usus: lingua-serpentis <via.liss | directorium>\n");
}

#[test]
fn two_arguments_prints_usage_to_stderr() {
    let o = run(&["a.liss", "b.liss"]);
    let (code, _, err) = exited(&o);
    assert_eq!(code, 1);
    assert_eq!(err, "Usus: lingua-serpentis <via.liss | directorium>\n");
}

#[test]
fn a_missing_file_reports_in_english_defect_d11() {
    let d = temp_dir("liss-cli");
    let missing = d.path().join("nusquam.liss");
    let o = run(&[missing.to_str().unwrap()]);
    let (code, _, err) = exited(&o);
    assert_eq!(code, 1);
    // D-11: English, in a language whose every other diagnostic is Latin.
    // Prefix, not verbatim: the errno tail is OS-supplied. The prefix itself
    // is the defect, and it is fully ours.
    assert!(err.starts_with("Error reading file: "), "got: {err:?}");
}

#[test]
fn a_non_liss_extension_is_refused_in_english_defect_d11() {
    let d = temp_dir("liss-cli");
    let p = d.write("notae.txt", "munus initium reddit ni { redde nullus; }");
    let o = run(&[p.to_str().unwrap()]);
    let (code, _, err) = exited(&o);
    assert_eq!(code, 1);
    assert_eq!(err, "Error: file must have .liss extension\n");
}

#[test]
fn a_nonexistent_directory_gets_the_extension_error_defect_d11() {
    // The is_dir() check runs first, so a path that is neither file nor
    // directory falls through to the extension branch and gets a message
    // about extensions rather than about the missing path.
    let d = temp_dir("liss-cli");
    let missing = d.path().join("nusquam");
    let o = run(&[missing.to_str().unwrap()]);
    let (code, _, err) = exited(&o);
    assert_eq!(code, 1);
    assert_eq!(err, "Error: file must have .liss extension\n");
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
fn a_hidden_directory_containing_liss_kills_the_program_defect_d17() {
    let d = temp_dir("liss-cli");
    d.write("prog/principale.liss", "munus initium reddit ni { redde nullus; }");
    d.write("prog/.git/vestigium.liss", "munus x reddit ni { redde °I; }");
    let o = run(&[d.path().join("prog").to_str().unwrap()]);
    let (code, _, err) = exited(&o);
    // D-17: hidden directories are not skipped by load_dir.
    assert_eq!(code, 1);
    assert_eq!(err, "Error: nomen fasciculi '.git' non validum est\n");
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
    // Prefix, not verbatim: the OS supplies the errno tail ("Permission denied
    // (os error 13)"), which is not portable. The only carve-out the Global
    // Constraints allow.
    assert!(err.starts_with("Error reading file: "), "got: {err:?}");
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
    assert_eq!(err, "Error reading file: stream did not contain valid UTF-8\n");
}

#[test]
#[ignore = "D-06: Rust ignores SIGPIPE and print! panics; exit 101 with a backtrace note"]
fn a_closed_pipe_is_not_a_panic() {
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
    // Verbatim now that the message exists — the Global Constraints' carve-out
    // for `contains` inside an ignored test expires with the ignore.
    assert_eq!(err, "Error in linea 1: vocatio nimis alte implicata\n");
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

#[test]
fn recursion_inside_the_limit_still_runs() {
    // The limit must not be so tight that ordinary recursion stops working.
    let d = temp_dir("liss-cli");
    let p = d.write("fons.liss",
        "munus fac capit (ni n) reddit ni { si (n minor_est °II) { redde °I; } redde n * fac(n - °I); }\n\
         munus initium reddit ni { scribe(fac(°X)); redde nullus; }");
    let o = run(&[p.to_str().unwrap()]);
    let (code, out, err) = exited(&o);
    assert_eq!(code, 0);
    assert_eq!(err, "");
    // fac(X) is 3_628_800, printed M̅M̅M̅D̅C̅X̅X̅V̅MMMDCCC — each vinculum is a
    // combining overline on the glyph before it, multiplying it by a thousand.
    assert_eq!(
        out,
        "M\u{305}M\u{305}M\u{305}D\u{305}C\u{305}X\u{305}X\u{305}V\u{305}MMMDCCC\n"
    );
}
