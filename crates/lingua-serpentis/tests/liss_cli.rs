//! End-to-end behaviour of the `liss` binary — the compiling surface.
mod support;

use std::io::Read;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use support::proc::{temp_dir, TempDir};

const LISS: &str = env!("CARGO_BIN_EXE_liss");

/// Transcribed independently of `cli.rs`'s `USUS`, deliberately.
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

fn liss(args: &[&str]) -> Res {
    liss_full(args, None, None)
}

fn liss_at(cwd: &Path, args: &[&str]) -> Res {
    liss_full(args, Some(cwd), None)
}

/// `TMPDIR` points at `tmp`, so a surviving temporary image is somewhere this test can see it.
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

/// stdout and stderr share one open file description, so the write order is recoverable.
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

fn source(d: &TempDir, name: &str, src: &str) -> PathBuf {
    d.write(name, src)
}

fn folder(d: &TempDir, name: &str, src: &str) -> PathBuf {
    d.write(&format!("{name}/principale.liss"), src);
    d.path().join(name)
}

const SALVE: &str = "munus initium reddit ni {\n  scribe(\"salve\");\n  redde nullus;\n}";
/// One Latinity warning from the parent, one line of output from the child.
const MONITUM: &str =
    "munus initium reddit ni {\n  ni counter = \u{b0}I;\n  scribe(counter);\n  redde nullus;\n}";
/// What `MONITUM` renders to, below its location row: the gutter and the caret under
/// `counter`. Written out rather than derived, so a renderer that stopped drawing
/// snippets would fail here rather than agree with itself.
const MONITUM_PICTURA: &str = "    |\n II |   ni counter = \u{b0}I;\n    |      ^^^^^^^\n";
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

    // Bare `run` is the subcommand, always.
    let bare = liss_at(d.path(), &["run"]);
    assert_eq!(bare.code, Some(1));
    assert_eq!(bare.stderr, USUS);

    let dotted = liss_at(d.path(), &["./run"]);
    assert_eq!(dotted.code, Some(0), "stderr: {}", dotted.stderr);
    assert_eq!(dotted.stdout, "salve\n");

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
    assert_eq!(r.stderr, format!("erratum: via '{}' non inventa est\n", p.display()));
}

#[test]
fn a_non_liss_extension_is_refused_in_latin() {
    let d = temp_dir("liss-ext");
    let p = source(&d, "notae.txt", SALVE);
    let r = liss(&[p.to_str().unwrap()]);
    assert_eq!(r.code, Some(1));
    assert_eq!(r.stderr, "erratum: nomen scidae in '.liss' desinere debet\n");
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

/// The name is baked into the image at codegen time; the parent cannot rewrite the child's stderr.
#[test]
fn a_single_file_diagnostic_names_no_file() {
    let d = temp_dir("liss-solum");
    let p = source(&d, "fons.liss", DIVISIO);
    let r = liss(&["run", p.to_str().unwrap()]);
    assert_eq!(r.code, Some(1));
    assert_eq!(r.stderr, "erratum: divisio per nullum\n  in linea III\n");
}

#[test]
fn a_folder_diagnostic_names_its_file() {
    let d = temp_dir("liss-scrinium");
    let p = folder(&d, "programma", DIVISIO);
    let r = liss(&["run", p.to_str().unwrap()]);
    assert_eq!(r.code, Some(1));
    assert_eq!(r.stderr, "erratum: divisio per nullum\n  in 'principale.liss', linea III\n");
}

// ------------------------------------------------ the parent's warnings

/// The only test of parent-side Latinity warnings anywhere.
#[test]
fn run_emits_the_parents_latinity_warnings_before_the_childs_output() {
    let d = temp_dir("liss-monitum");
    let p = source(&d, "fons.liss", MONITUM);
    let (code, merged) = liss_merged(&["run", p.to_str().unwrap()]);
    assert_eq!(code, Some(0));
    assert_eq!(
        merged,
        format!("monitum: nomen 'counter' fortasse non Latinum est\n  in linea II\n{MONITUM_PICTURA}I\n")
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
        format!(
            "monitum: nomen 'counter' fortasse non Latinum est\n  \
             in 'principale.liss', linea II\n{MONITUM_PICTURA}I\n"
        )
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
        format!("monitum: nomen 'counter' fortasse non Latinum est\n  in linea II\n{MONITUM_PICTURA}")
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

/// No `Drop` guard or signal handler could cover this: the image has had no name since
/// before the exec.
#[test]
fn run_leaves_nothing_behind_when_the_parent_is_killed() {
    let d = temp_dir("liss-occisus");
    let tmp = temp_dir("liss-occisus-tmp");
    // Prints more than a pipe buffer holds, so the child blocks and stays alive.
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

    // One byte on stdout can only have come from the child, hence after the unlink and exec.
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

    // Closing the pipe ends the orphan: its runtime treats EPIPE as an ordinary end of output.
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

#[test]
fn build_refuses_a_directory_as_its_output() {
    let d = temp_dir("liss-exitus-directorium");
    folder(&d, "programma", SALVE);
    let r = liss_at(d.path(), &["build", "./programma"]);
    assert_eq!(r.code, Some(1));
    assert_eq!(
        r.stderr,
        "erratum: exitus 'programma' directorium est; aliam viam per '--exi' da\n"
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
        format!("erratum: scida '{}' scribi non potest\n", target.display())
    );
}

#[test]
fn build_on_a_program_that_fails_to_check_writes_nothing() {
    let d = temp_dir("liss-malus");
    let p = source(&d, "malus.liss", MALUS);
    let r = liss_at(d.path(), &["build", p.to_str().unwrap()]);
    assert_eq!(r.code, Some(1));
    assert_eq!(r.stdout, "");
    assert!(r.stderr.starts_with("erratum: ") && r.stderr.contains("\n  in linea II\n"), "stderr: {}", r.stderr);
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
fn a_build_that_cannot_write_its_output_says_so() {
    // `/dev/full` opens and then refuses every write with ENOSPC.
    let d = temp_dir("liss-plenum");
    let p = source(&d, "salve.liss", SALVE);
    let r = liss(&["build", p.to_str().unwrap(), "--exi", "/dev/full"]);
    assert_eq!(r.code, Some(1));
    assert_eq!(r.stderr, "erratum: scida '/dev/full' scribi non potest\n");
    assert!(Path::new("/dev/full").exists(), "the build removed /dev/full");
}

/// `open` follows the link to `/dev/full`, but `remove_file` acts on the link, which is ours —
/// so the deletion the `/dev/full` case can only threaten actually happens here.
#[test]
fn a_failed_write_through_a_symlink_removes_neither_end() {
    let d = temp_dir("liss-nexus");
    let p = source(&d, "salve.liss", SALVE);
    let nexus = d.path().join("plenum");
    std::os::unix::fs::symlink("/dev/full", &nexus).expect("a symlink to the device");

    let r = liss(&["build", p.to_str().unwrap(), "--exi", nexus.to_str().unwrap()]);

    assert_eq!(r.code, Some(1));
    assert_eq!(r.stderr, format!("erratum: scida '{}' scribi non potest\n", nexus.display()));
    assert!(
        nexus.symlink_metadata().is_ok(),
        "the build removed the symlink it wrote through"
    );
    assert!(Path::new("/dev/full").exists());
}

/// The device must come back out with its mode unchanged, not 0755.
#[test]
fn building_to_dev_null_succeeds_and_changes_nothing_about_it() {
    let ante = std::fs::metadata("/dev/null").expect("stat /dev/null").permissions().mode();
    let d = temp_dir("liss-nihil");
    let p = source(&d, "salve.liss", SALVE);
    let r = liss(&["build", p.to_str().unwrap(), "--exi", "/dev/null"]);
    assert_eq!(r.code, Some(0), "stderr: {}", r.stderr);
    assert_eq!(
        std::fs::metadata("/dev/null").expect("stat /dev/null").permissions().mode(),
        ante,
        "the build changed the mode of /dev/null",
    );
}

// ------------------------------------------------- the signalled child

/// `None` until a child exists.
fn child_of(pid: u32) -> Option<u32> {
    for task in std::fs::read_dir(format!("/proc/{pid}/task")).ok()?.flatten() {
        let Ok(s) = std::fs::read_to_string(task.path().join("children")) else { continue };
        if let Some(first) = s.split_whitespace().next() {
            return first.parse().ok();
        }
    }
    None
}

/// Nothing an ordinary program does reaches this branch, so the child is killed from outside.
#[test]
fn a_child_killed_by_a_signal_is_reported_in_latin() {
    let d = temp_dir("liss-signum");
    // Several seconds of arithmetic and no output, so the child is still alive to be signalled.
    let src = "munus initium reddit ni {\n  ni s = nullus;\n  \
               dum (ni i = nullus; i minor_est \u{b0}MM; i = i + \u{b0}I) {\n    \
               dum (ni j = nullus; j minor_est \u{b0}M; j = j + \u{b0}I) {\n      \
               s = s + \u{b0}I;\n    }\n  }\n  scribe(s);\n  redde nullus;\n}";
    let p = source(&d, "longum.liss", src);

    let parent = Command::new(LISS)
        .args(["run", p.to_str().unwrap()])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn liss");

    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
    let kid = loop {
        if let Some(k) = child_of(parent.id()) {
            break k;
        }
        assert!(std::time::Instant::now() < deadline, "liss never spawned an image");
        std::thread::sleep(std::time::Duration::from_millis(10));
    };

    let killed = Command::new("sh")
        .arg("-c")
        .arg(format!("kill -9 {kid}"))
        .status()
        .expect("run sh");
    assert!(killed.success(), "could not signal the child");

    let out = parent.wait_with_output().expect("reap liss");
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(
        String::from_utf8_lossy(&out.stderr),
        "erratum: programma abrupte cessavit\n"
    );
}

#[test]
fn an_unknown_machine_is_refused_in_latin() {
    let d = temp_dir("liss-machina");
    let p = source(&d, "salve.liss", SALVE);
    let r = liss(&["build", p.to_str().unwrap(), "--ad", "sparc"]);
    assert_eq!(r.code, Some(1));
    assert_eq!(r.stderr, "erratum: machina 'sparc' ignota est\n");
}
