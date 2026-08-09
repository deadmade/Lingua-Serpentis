//! What the fixture format cannot express: argv, the stream split, exit status, aborts.
mod support;

use std::ffi::OsStr;
use std::time::Duration;
use support::proc::{run_binary, run_closing_stdout_early, temp_dir, Outcome};

const T: Duration = Duration::from_secs(20);

/// Duplicated from `liss_cli.rs`'s `USUS` deliberately: two independent transcriptions.
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
    // D-11
    assert_eq!(err, format!("erratum: via '{}' non inventa est\n", missing.display()));
}

#[test]
fn a_non_liss_extension_is_refused_in_latin() {
    let d = temp_dir("liss-cli");
    let p = d.write("notae.txt", "munus initium reddit ni { redde nullus; }");
    let o = run(&[p.to_str().unwrap()]);
    let (code, _, err) = exited(&o);
    assert_eq!(code, 1);
    assert_eq!(err, "erratum: nomen scidae in '.liss' desinere debet\n");
}

#[test]
fn a_nonexistent_path_is_reported_as_missing_not_as_a_bad_extension() {
    // D-11
    let d = temp_dir("liss-cli");
    let missing = d.path().join("nusquam");
    let o = run(&[missing.to_str().unwrap()]);
    let (code, _, err) = exited(&o);
    assert_eq!(code, 1);
    assert_eq!(err, format!("erratum: via '{}' non inventa est\n", missing.display()));
}

#[test]
fn a_nonexistent_dot_liss_directory_is_also_reported_as_missing() {
    // D-11
    let d = temp_dir("liss-cli");
    let missing = d.path().join("nusquam.liss");
    let o = run(&[missing.to_str().unwrap()]);
    let (code, _, err) = exited(&o);
    assert_eq!(code, 1);
    assert_eq!(err, format!("erratum: via '{}' non inventa est\n", missing.display()));
}

#[test]
fn a_directory_named_dot_liss_runs_as_a_program_folder() {
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
    assert_eq!(out, "V\n");
    assert_eq!(err, "monitum: nomen 'result' fortasse non Latinum est\n  in linea II\n");
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
    // `°C.C.C` is C×C + C = 10_100; 10_100 mod 256 is 116.
    let p = d.write("fons.liss", "munus initium reddit ni { redde °C.C.C; }");
    let o = run(&[p.to_str().unwrap()]);
    let (code, _, err) = exited(&o);
    // D-10
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
    // D-17
    assert_eq!(code, 0);
    assert_eq!(out, "salve\n");
    assert_eq!(err, "");
}

// The three D-22 sites: each needs a file mode or byte sequence the corpus cannot carry in git.

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
    // Relative to the program root, not an absolute path.
    assert_eq!(err, "erratum: scida 'principale.liss' legi non potest\n");
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
    assert_eq!(err, "erratum: directorium 'mathematica' legi non potest\n");
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
    assert_eq!(err, "erratum: scida 'alia.liss' signis incognitis scripta est\n");
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
    // D-22
    assert_eq!(err, format!("erratum: scida '{}' legi non potest\n", p.display()));
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
    assert_eq!(
        err,
        format!("erratum: scida '{}' signis incognitis scripta est\n", p.display())
    );
}

#[test]
fn a_closed_pipe_is_not_a_panic() {
    // D-06
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
    // D-04. `exited` rather than a signal check is the claim: the child dies by SIGSEGV
    // and `liss` must turn that into a status.
    let (code, out, err) = exited(&o);
    assert_eq!(code, 1);
    assert_eq!(err, "Error: acervus vocationum exhaustus est\n");
    assert_eq!(out, "");
}

#[test]
fn deep_expression_nesting_is_a_clean_latin_error() {
    // D-05
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
    assert_eq!(err, "erratum: expressio nimis alte implicata\n  in linea I\n");
}

/// The guard on §8's guard; `conformance/currunt/recursio-profunda` makes the same point at 5,000.
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
    // `°X.M` is X × M = 10,000, which prints as X with a vinculum.
    assert_eq!(out, "X\u{305}\n");
}

/// The deadline arm of `wait_or_kill` had no direct coverage while it was three copied loops.
#[test]
fn wait_or_kill_returns_the_status_of_a_child_that_finishes() {
    let mut child = std::process::Command::new("true").spawn().expect("spawn true");
    let status =
        support::proc::wait_or_kill(&mut child, Duration::from_secs(10), Duration::from_millis(1))
            .expect("try_wait");
    assert!(status.expect("the child exited before the deadline").success());
}

#[test]
fn wait_or_kill_reaps_a_child_that_outlives_its_deadline() {
    let mut child =
        std::process::Command::new("sleep").arg("30").spawn().expect("spawn sleep");
    let status = support::proc::wait_or_kill(
        &mut child,
        Duration::from_millis(50),
        Duration::from_millis(1),
    )
    .expect("try_wait");
    assert!(status.is_none(), "the deadline must be reported as `None`");
    // A `wait` on a child that was not reaped would block until it exited on its own.
    assert!(child.try_wait().expect("try_wait").is_some(), "the child was not reaped");
}
