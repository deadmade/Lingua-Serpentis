//! End-to-end behaviour of the `liss` binary — the compiling surface.
//!
//! Everything here needs a real process, and most of it needs *two*: `liss run`
//! compiles, links, and then execs a child whose stdio it inherits. That
//! inheritance is why several of these tests merge the two streams into one
//! file rather than capturing them separately — the parent's Latinity warnings
//! and the child's output land on different file descriptors, and the only way
//! to observe that the first come *before* the second is to give both
//! descriptors the same open file description.
mod support;

use std::io::Read;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use support::proc::{temp_dir, TempDir};

const LISS: &str = env!("CARGO_BIN_EXE_liss");

/// The three usage lines, and the sentence that disambiguates a folder named
/// `run`. Byte for byte what the binary must print.
const USUS: &str = "\
Usus: liss run <via.liss | directorium>
      liss build <via.liss | directorium> [--exi <via>] [--ad <machina>]
      liss <via.liss | directorium>
Primum argumentum mandatum est solum si 'run' vel 'build' exacte est;
directorium ita nominatum per './run' dandum est.
";

#[derive(Debug)]
struct Res {
    code: Option<i32>,
    stdout: String,
    stderr: String,
}

/// Run `liss` with both streams captured separately.
fn liss(args: &[&str]) -> Res {
    liss_full(args, None, None)
}

/// Run `liss` from `cwd`.
fn liss_at(cwd: &Path, args: &[&str]) -> Res {
    liss_full(args, Some(cwd), None)
}

/// Run `liss` with `TMPDIR` pointed at `tmp`, so the temporary image — if one
/// survives — is somewhere this test can see it.
fn liss_tmp(tmp: &Path, args: &[&str]) -> Res {
    liss_full(args, None, Some(tmp))
}

fn liss_full(args: &[&str], cwd: Option<&Path>, tmp: Option<&Path>) -> Res {
    let mut c = Command::new(LISS);
    c.args(args).stdin(Stdio::null());
    if let Some(d) = cwd {
        c.current_dir(d);
    }
    if let Some(d) = tmp {
        c.env("TMPDIR", d);
    }
    let out = c.output().expect("spawn liss");
    Res {
        code: out.status.code(),
        stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
    }
}

/// Run `liss` with stdout and stderr sharing **one** open file description, so
/// the order the two processes wrote in is recoverable.
fn liss_merged(args: &[&str]) -> (Option<i32>, String) {
    let d = temp_dir("liss-merged");
    let p = d.path().join("ambo");
    let f = std::fs::File::create(&p).expect("a capture file");
    let g = f.try_clone().expect("dup the capture file");
    let status = Command::new(LISS)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::from(f))
        .stderr(Stdio::from(g))
        .status()
        .expect("spawn liss");
    let text = std::fs::read_to_string(&p).expect("read the capture file");
    (status.code(), text)
}

/// A single-source program on disk.
fn source(d: &TempDir, name: &str, src: &str) -> PathBuf {
    d.write(name, src)
}

/// A program *folder* with one root file.
fn folder(d: &TempDir, name: &str, src: &str) -> PathBuf {
    d.write(&format!("{name}/principale.liss"), src);
    d.path().join(name)
}

const SALVE: &str = "munus initium reddit ni {\n  scribe(\"salve\");\n  redde nullus;\n}";
/// Line 2 names a variable no Latin dictionary knows, which is one Latinity
/// warning from the *parent* and one line of output from the child.
const MONITUM: &str =
    "munus initium reddit ni {\n  ni counter = \u{b0}I;\n  scribe(counter);\n  redde nullus;\n}";
/// Fails at run time, on line 3.
const DIVISIO: &str = "munus initium reddit ni {\n  ni z = \u{b0}I - \u{b0}I;\n  \
                       scribe(\u{b0}V / z);\n  redde nullus;\n}";
/// Fails the checker, on line 2.
const MALUS: &str = "munus initium reddit ni {\n  ni x = \"non numerus\" + \u{b0}I;\n  \
                     redde nullus;\n}";

// ---------------------------------------------------------------- arity

#[test]
fn no_arguments_prints_usage_to_stderr() {
    let r = liss(&[]);
    assert_eq!(r.code, Some(1));
    assert_eq!(r.stdout, "");
    assert_eq!(r.stderr, USUS);
}

#[test]
fn a_subcommand_without_a_path_prints_usage() {
    for sub in ["run", "build"] {
        let r = liss(&[sub]);
        assert_eq!(r.code, Some(1), "{sub}");
        assert_eq!(r.stderr, USUS, "{sub}");
    }
}

#[test]
fn two_paths_print_usage() {
    let r = liss(&["run", "a.liss", "b.liss"]);
    assert_eq!(r.code, Some(1));
    assert_eq!(r.stderr, USUS);
}

#[test]
fn an_option_without_its_value_prints_usage() {
    for a in [["build", "a.liss", "--exi"], ["build", "a.liss", "--ad"]] {
        let r = liss(&a);
        assert_eq!(r.code, Some(1), "{a:?}");
        assert_eq!(r.stderr, USUS, "{a:?}");
    }
}

#[test]
fn the_build_options_are_not_accepted_by_run() {
    let r = liss(&["run", "a.liss", "--exi", "b"]);
    assert_eq!(r.code, Some(1));
    assert_eq!(r.stderr, USUS);
}

// ------------------------------------------------- the `./run` rule

#[test]
fn a_folder_named_run_must_be_given_as_dot_slash_run() {
    let d = temp_dir("liss-run-folder");
    d.write("run/principale.liss", SALVE);

    // Bare `run` is the subcommand, always — so this is `liss run` with no
    // path at all, and the answer is the usage string.
    let bare = liss_at(d.path(), &["run"]);
    assert_eq!(bare.code, Some(1));
    assert_eq!(bare.stderr, USUS);

    // `./run` is a path, and names the folder.
    let dotted = liss_at(d.path(), &["./run"]);
    assert_eq!(dotted.code, Some(0), "stderr: {}", dotted.stderr);
    assert_eq!(dotted.stdout, "salve\n");

    // And so does `run ./run`.
    let both = liss_at(d.path(), &["run", "./run"]);
    assert_eq!(both.code, Some(0), "stderr: {}", both.stderr);
    assert_eq!(both.stdout, "salve\n");
}

// ------------------------------------------------------------ paths

#[test]
fn a_missing_path_is_reported_in_latin() {
    let d = temp_dir("liss-missing");
    let p = d.path().join("nusquam.liss");
    let r = liss(&["run", p.to_str().unwrap()]);
    assert_eq!(r.code, Some(1));
    assert_eq!(r.stderr, format!("Error: via '{}' non inventa est\n", p.display()));
}

#[test]
fn a_non_liss_extension_is_refused_in_latin() {
    let d = temp_dir("liss-ext");
    let p = source(&d, "notae.txt", SALVE);
    let r = liss(&[p.to_str().unwrap()]);
    assert_eq!(r.code, Some(1));
    assert_eq!(r.stderr, "Error: nomen scidae in '.liss' desinere debet\n");
}

// -------------------------------------------------------------- run

#[test]
fn the_bare_positional_form_runs() {
    let d = temp_dir("liss-bare");
    let p = source(&d, "salve.liss", SALVE);
    let r = liss(&[p.to_str().unwrap()]);
    assert_eq!(r.code, Some(0), "stderr: {}", r.stderr);
    assert_eq!(r.stdout, "salve\n");
    assert_eq!(r.stderr, "");
}

#[test]
fn run_propagates_the_programs_exit_status() {
    let d = temp_dir("liss-status");
    let p = source(&d, "tres.liss", "munus initium reddit ni {\n  redde \u{b0}III;\n}");
    let r = liss(&["run", p.to_str().unwrap()]);
    assert_eq!(r.code, Some(3));
}

/// **The `Func::file` proof.** A single-source program's diagnostics carry no
/// file name, and the name is baked into the image at codegen time — the parent
/// inherits the child's stderr and cannot rewrite it afterwards (spec §4.1).
#[test]
fn a_single_file_diagnostic_names_no_file() {
    let d = temp_dir("liss-solum");
    let p = source(&d, "fons.liss", DIVISIO);
    let r = liss(&["run", p.to_str().unwrap()]);
    assert_eq!(r.code, Some(1));
    assert_eq!(r.stderr, "Error in linea 3: divisio per nullum\n");
}

#[test]
fn a_folder_diagnostic_names_its_file() {
    let d = temp_dir("liss-scrinium");
    let p = folder(&d, "programma", DIVISIO);
    let r = liss(&["run", p.to_str().unwrap()]);
    assert_eq!(r.code, Some(1));
    assert_eq!(r.stderr, "Error in 'principale.liss', linea 3: divisio per nullum\n");
}

// ------------------------------------------------ the parent's warnings

/// **The only test of parent-side Latinity warnings anywhere.** Task 15's
/// codegen helper subtracts the checker's warnings from every expectation,
/// because a bare image has no parent to emit them; once that is the only
/// place they were asserted, this is what keeps `liss run` emitting them at
/// all — and emitting them *before* the child writes.
#[test]
fn run_emits_the_parents_latinity_warnings_before_the_childs_output() {
    let d = temp_dir("liss-monitum");
    let p = source(&d, "fons.liss", MONITUM);
    let (code, merged) = liss_merged(&["run", p.to_str().unwrap()]);
    assert_eq!(code, Some(0));
    assert_eq!(
        merged,
        "Monitum in linea 2: nomen 'counter' fortasse non Latinum est\nI\n"
    );
}

#[test]
fn a_folder_programs_warnings_name_their_file() {
    let d = temp_dir("liss-monitum-scrinium");
    let p = folder(&d, "programma", MONITUM);
    let (code, merged) = liss_merged(&["run", p.to_str().unwrap()]);
    assert_eq!(code, Some(0));
    assert_eq!(
        merged,
        "Monitum in 'principale.liss', linea 2: nomen 'counter' fortasse non \
         Latinum est\nI\n"
    );
}

#[test]
fn build_emits_the_parents_latinity_warnings_too() {
    let d = temp_dir("liss-monitum-build");
    let p = source(&d, "fons.liss", MONITUM);
    let r = liss_at(d.path(), &["build", p.to_str().unwrap()]);
    assert_eq!(r.code, Some(0), "stderr: {}", r.stderr);
    assert_eq!(
        r.stderr,
        "Monitum in linea 2: nomen 'counter' fortasse non Latinum est\n"
    );
}

// ------------------------------------------- the temp file leaves nothing

fn entries(p: &Path) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(p)
        .expect("read the temp directory")
        .map(|e| e.expect("an entry").file_name().to_string_lossy().into_owned())
        .collect();
    v.sort();
    v
}

#[test]
fn run_leaves_nothing_behind() {
    let d = temp_dir("liss-vestigium");
    let tmp = temp_dir("liss-vestigium-tmp");
    let p = source(&d, "salve.liss", SALVE);
    let r = liss_tmp(tmp.path(), &["run", p.to_str().unwrap()]);
    assert_eq!(r.code, Some(0), "stderr: {}", r.stderr);
    assert_eq!(entries(tmp.path()), Vec::<String>::new());
}

/// The unlink is the *whole* cleanup story, and this is the case no `Drop`
/// guard and no signal handler could cover: the parent is `SIGKILL`ed while
/// the child is still running, and there is still no name in `TMPDIR` —
/// because there has been none since before the exec.
#[test]
fn run_leaves_nothing_behind_when_the_parent_is_killed() {
    let d = temp_dir("liss-occisus");
    let tmp = temp_dir("liss-occisus-tmp");
    // Prints far more than a pipe buffer holds, so the child blocks on a
    // stdout nobody is draining and stays alive as long as we need it.
    let src = "munus initium reddit ni {\n  \
               dum (ni i = nullus; i minor_est \u{b0}M; i = i + \u{b0}I) {\n    \
               scribe(\"vox in deserto clamantis parate viam domini rectas\");\n  }\n  \
               redde nullus;\n}";
    let p = source(&d, "longum.liss", src);

    let mut parent = Command::new(LISS)
        .args(["run", p.to_str().unwrap()])
        .env("TMPDIR", tmp.path())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn liss");

    // One byte on stdout can only have come from the child, and the child only
    // exists once the image has been written, unlinked and exec'd.
    let mut out = parent.stdout.take().expect("piped stdout");
    let mut byte = [0u8; 1];
    out.read_exact(&mut byte).expect("the child writes");

    parent.kill().expect("SIGKILL the parent");
    parent.wait().expect("reap the parent");

    assert_eq!(
        entries(tmp.path()),
        Vec::<String>::new(),
        "a SIGKILLed parent left a temporary image behind"
    );

    // Closing the pipe ends the orphan: its runtime treats EPIPE as an
    // ordinary end of output (spec §6).
    drop(out);
}

// ------------------------------------------------------------ build

#[test]
fn build_writes_an_executable_next_to_nothing_and_it_runs() {
    let d = temp_dir("liss-aedifica");
    let p = source(&d, "salve.liss", SALVE);
    let r = liss_at(d.path(), &["build", p.to_str().unwrap()]);
    assert_eq!(r.code, Some(0), "stderr: {}", r.stderr);
    assert_eq!(r.stdout, "");
    assert_eq!(r.stderr, "");

    // The `.liss` stem, in the current directory.
    let imago = d.path().join("salve");
    let meta = std::fs::metadata(&imago).expect("the image exists");
    assert_eq!(meta.permissions().mode() & 0o7777, 0o755);

    let out = Command::new(&imago).output().expect("run the image");
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(String::from_utf8_lossy(&out.stdout), "salve\n");
}

#[test]
fn build_of_a_folder_uses_the_folders_name() {
    let d = temp_dir("liss-aedifica-scrinium");
    let p = folder(&d, "computatorium", SALVE);
    let out_dir = temp_dir("liss-aedifica-exitus");
    let r = liss_at(out_dir.path(), &["build", p.to_str().unwrap()]);
    assert_eq!(r.code, Some(0), "stderr: {}", r.stderr);
    assert!(out_dir.path().join("computatorium").is_file());
}

#[test]
fn build_honours_exi() {
    let d = temp_dir("liss-exi");
    let p = source(&d, "salve.liss", SALVE);
    let target = d.path().join("alibi/machinamentum");
    std::fs::create_dir_all(target.parent().expect("a parent")).expect("a directory");
    let r = liss(&["build", p.to_str().unwrap(), "--exi", target.to_str().unwrap()]);
    assert_eq!(r.code, Some(0), "stderr: {}", r.stderr);
    let out = Command::new(&target).output().expect("run the image");
    assert_eq!(String::from_utf8_lossy(&out.stdout), "salve\n");
}

/// `liss build ./computatrum` run from the parent of a program folder resolves
/// its default output to the input directory itself. Writing a file there
/// cannot work, and silently doing something else would be worse.
#[test]
fn build_refuses_a_directory_as_its_output() {
    let d = temp_dir("liss-exitus-directorium");
    folder(&d, "programma", SALVE);
    let r = liss_at(d.path(), &["build", "./programma"]);
    assert_eq!(r.code, Some(1));
    assert_eq!(
        r.stderr,
        "Error: exitus 'programma' directorium est; aliam viam per '--exi' da\n"
    );
}

#[test]
fn build_into_an_unwritable_place_is_a_latin_error() {
    let d = temp_dir("liss-clausum");
    let p = source(&d, "salve.liss", SALVE);
    let clausum = d.path().join("clausum");
    std::fs::create_dir_all(&clausum).expect("a directory");
    std::fs::set_permissions(&clausum, std::fs::Permissions::from_mode(0o500))
        .expect("make it unwritable");
    let target = clausum.join("machinamentum");

    let r = liss(&["build", p.to_str().unwrap(), "--exi", target.to_str().unwrap()]);

    // Restore the mode before any assertion, so the TempDir can still be removed.
    let _ = std::fs::set_permissions(&clausum, std::fs::Permissions::from_mode(0o700));

    assert_eq!(r.code, Some(1));
    assert_eq!(
        r.stderr,
        format!("Error: scida '{}' scribi non potest\n", target.display())
    );
}

#[test]
fn build_on_a_program_that_fails_to_check_writes_nothing() {
    let d = temp_dir("liss-malus");
    let p = source(&d, "malus.liss", MALUS);
    let r = liss_at(d.path(), &["build", p.to_str().unwrap()]);
    assert_eq!(r.code, Some(1));
    assert_eq!(r.stdout, "");
    assert!(r.stderr.starts_with("Error in linea 2: "), "stderr: {}", r.stderr);
    assert!(!d.path().join("malus").exists(), "an image was written anyway");
}

#[test]
fn build_cross_compiles_for_the_other_machine() {
    let d = temp_dir("liss-ad");
    let p = source(&d, "salve.liss", SALVE);
    let target = d.path().join("aliena");
    let r = liss(&[
        "build",
        p.to_str().unwrap(),
        "--exi",
        target.to_str().unwrap(),
        "--ad",
        "aarch64",
    ]);
    assert_eq!(r.code, Some(0), "stderr: {}", r.stderr);
    let bytes = std::fs::read(&target).expect("the image exists");
    // ELF `e_machine`, little-endian at offset 18: EM_AARCH64 is 183.
    assert_eq!(u16::from_le_bytes([bytes[18], bytes[19]]), 183);
}

#[test]
fn an_unknown_machine_is_refused_in_latin() {
    let d = temp_dir("liss-machina");
    let p = source(&d, "salve.liss", SALVE);
    let r = liss(&["build", p.to_str().unwrap(), "--ad", "sparc"]);
    assert_eq!(r.code, Some(1));
    assert_eq!(r.stderr, "Error: machina 'sparc' ignota est\n");
}
