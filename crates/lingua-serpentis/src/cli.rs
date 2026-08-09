//! The command-line surface (spec §9.5), Unix only; the driver lives here and not in
//! `src/bin/` because `tests/catalogus.rs` does not recurse there.

use crate::error::{ProgramError, Warning};
use crate::program::{Program, SourceFile};
use crate::render;
use liss_codegen::nexus::{Nexus, WildNexus};
use liss_codegen::target::Target;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::os::fd::AsRawFd;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::{exit, Command, Stdio};

/// Write `text` to `w`, treating a closed pipe as an ordinary end of output (D-06).
/// The flush is load-bearing: `exit` runs no destructors, so nothing else flushes.
pub fn write_or_exit(w: &mut impl Write, text: &str) {
    match w.write_all(text.as_bytes()).and_then(|()| w.flush()) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::BrokenPipe => exit(0),
        Err(_) => exit(1),
    }
}

/// Render a finished pipeline result to the two streams and exit. `liss run` never
/// reaches it on the success path — the child owns stdout — but every failure does.
pub fn finish(result: Result<(String, u8, Vec<Warning>), ProgramError>) -> ! {
    let r = render(result);
    write_or_exit(&mut std::io::stderr(), &r.stderr);
    write_or_exit(&mut std::io::stdout(), &r.stdout);
    exit(r.code);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Task {
    Run,
    Build,
}

#[derive(Debug)]
struct Invocation {
    task: Task,
    /// Exactly as the user wrote it; every diagnostic names it back the same way.
    path: String,
    out: Option<PathBuf>,
    machine: Option<String>,
}

/// The usage string, and exit 1. Two sentences, and therefore two catalogue sites.
fn usus() -> ! {
    eprintln!(
        "Usus: liss run <via.liss | directorium>
      liss build <via.liss | directorium> [--exi <via>] [--ad <machina>]
      liss <via.liss | directorium>"
    );
    eprintln!(
        "Primum argumentum mandatum est solum si 'run' vel 'build' exacte est;
directorium ita nominatum per './run' dandum est."
    );
    exit(1)
}

/// Parse argv (without the program name); `None` means print the usage. The first
/// argument is a subcommand iff it is exactly `run` or `build`.
fn parse(args: &[String]) -> Option<Invocation> {
    let (task, rest) = match args.first().map(String::as_str) {
        Some("run") => (Task::Run, &args[1..]),
        Some("build") => (Task::Build, &args[1..]),
        _ => (Task::Run, args),
    };

    let mut path: Option<String> = None;
    let mut out: Option<PathBuf> = None;
    let mut machine: Option<String> = None;
    let mut i = 0;
    while i < rest.len() {
        let a = rest[i].as_str();
        match a {
            // Under `run` these fall through to the unknown-option arm, deliberately.
            "--exi" | "--ad" if task == Task::Build => {
                let value = rest.get(i + 1)?;
                let taken = if a == "--exi" {
                    out.replace(PathBuf::from(value)).is_some()
                } else {
                    machine.replace(value.clone()).is_some()
                };
                if taken {
                    return None;
                }
                i += 2;
            }
            _ if a.starts_with("--") => return None,
            _ => {
                if path.replace(a.to_string()).is_some() {
                    return None;
                }
                i += 1;
            }
        }
    }
    Some(Invocation { task, path: path?, out, machine })
}

/// Load the program, and say whether it is a single-source one. Whether the path
/// exists is asked before what kind of path it is (D-11).
fn load(arg: &str) -> Result<(Program, bool), ProgramError> {
    let path = Path::new(arg);
    if path.is_dir() {
        return crate::program::load_dir(path).map(|p| (p, false));
    }
    if !path.exists() {
        eprintln!("erratum: via '{}' non inventa est", arg);
        exit(1);
    }
    if !arg.ends_with(".liss") {
        eprintln!("erratum: nomen scidae in '.liss' desinere debet");
        exit(1);
    }
    let src = std::fs::read_to_string(path)
        .map_err(|e| crate::program::scida_legi_non_potest(arg, &e))?;
    Ok((
        Program {
            root: vec![SourceFile { path: "fons.liss".into(), src }],
            packages: std::collections::BTreeMap::new(),
        },
        true,
    ))
}

/// The single-source strip: the synthetic `fons.liss` is removed from a failure, and
/// structural errors, which have no line of their own, surface at line 1.
fn strip(single: bool, mut e: ProgramError) -> ProgramError {
    if single {
        e.file = None;
        e.line = Some(e.line.unwrap_or(1));
        for w in &mut e.warnings {
            w.file = None;
        }
    }
    e
}

pub fn liss(args: &[String]) -> ! {
    let Some(inv) = parse(args) else { usus() };

    let (loaded, single) = match load(&inv.path) {
        Ok(x) => x,
        // Structural, and carries no line: `strip` must not give it one.
        Err(e) => finish(Err(e)),
    };

    let mut parsed = match crate::parse_loaded(loaded) {
        Ok(p) => p,
        Err(e) => finish(Err(strip(single, e))),
    };

    // Before the checker, and attached to a failure rather than dropped with it (D-09).
    let mut monita = crate::latinity::lint_program(&parsed);
    if single {
        for w in &mut monita {
            w.file = None;
        }
    }

    let mut ir = match crate::checker::check_program_typed(&mut parsed) {
        Ok(ir) => ir,
        Err(mut e) => {
            e.warnings = monita;
            finish(Err(strip(single, e)))
        }
    };

    // Spec §4.1's debt, and this is the only place that can pay it: the name is baked
    // into the image at codegen time and cannot be repaired downstream.
    if single {
        for f in &mut ir.funcs {
            f.file = None;
        }
    }

    // Before the child exists: the warnings-then-output order the corpus records
    // only holds if these reach stderr before the spawn (spec §9.5).
    let rendered = render(Ok::<_, ProgramError>((String::new(), 0, monita)));
    write_or_exit(&mut std::io::stderr(), &rendered.stderr);

    let machina = match &inv.machine {
        Some(n) => match Target::ex_nomine(n) {
            Some(m) => m,
            None => {
                eprintln!("erratum: machina '{}' ignota est", n);
                exit(1);
            }
        },
        None => match Target::hospes() {
            Some(m) => m,
            None => {
                eprintln!("erratum: machina huius computatri ignota est");
                exit(1);
            }
        },
    };

    let exitus = match inv.task {
        Task::Run => None,
        Task::Build => Some(destination(&inv, single)),
    };

    let objectum = match liss_codegen::compila(&ir, &machina) {
        Ok(o) => o,
        // A bare hole, so the catalogue does not read this as a site of its own;
        // the literal lives in `liss-codegen`.
        Err(d) => {
            eprintln!("{}", d);
            exit(1);
        }
    };
    let nomen = exitus
        .as_deref()
        .and_then(Path::file_name)
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "imago".to_string());
    let imago = match WildNexus.iunge(&machina.runtime().artefacta(&objectum), &nomen) {
        Ok(i) => i,
        Err(d) => {
            eprintln!("{}", d);
            exit(1);
        }
    };

    match exitus {
        Some(via) => build(&via, &imago),
        None => run(&imago),
    }
}

/// `--exi` if given, else the `.liss` stem or the program folder's name, here.
fn destination(inv: &Invocation, single: bool) -> PathBuf {
    if let Some(via) = &inv.out {
        return via.clone();
    }
    let path = Path::new(&inv.path);
    // A folder keeps its whole name; `canonicalize` makes `liss build .` name it.
    let base = if single {
        path.file_stem().map(std::ffi::OsString::from)
    } else {
        path.canonicalize().ok().and_then(|p| p.file_name().map(std::ffi::OsString::from))
    };
    let via = match base {
        Some(b) => PathBuf::from(b),
        None => {
            eprintln!("erratum: nomen exitus statui non potest; viam per '--exi' da");
            exit(1);
        }
    };
    // Refused here rather than several seconds of codegen and linking later.
    if via.is_dir() {
        eprintln!(
            "erratum: exitus '{}' directorium est; aliam viam per '--exi' da",
            via.display()
        );
        exit(1);
    }
    via
}

/// Whether the `O_TRUNC` open destroyed anything: only a regular file has content
/// to lose. Asked of the open handle, not by a second `stat`, so it cannot be raced.
fn truncatur(f: &File) -> bool {
    f.metadata().map(|m| m.is_file()).unwrap_or(false)
}

/// `Err(true)` means bytes were already lost. The rule, and the whole reason this is
/// a function: remove only what we truncated, and we truncated only a regular file.
fn write_image(via: &Path, imago: &[u8]) -> Result<(), bool> {
    let mut f = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o755)
        .open(via)
        .map_err(|_| false)?;
    let regularis = truncatur(&f);
    let scriptum = f.write_all(imago).and_then(|()| f.flush());
    if regularis {
        // The second 0755 is the one that holds: `mode()` applies only at creation
        // and is masked by the umask.
        return scriptum
            .and_then(|()| f.set_permissions(std::fs::Permissions::from_mode(0o755)))
            .map_err(|_| true);
    }
    scriptum.map_err(|_| false)
}

fn build(via: &Path, imago: &[u8]) -> ! {
    match write_image(via, imago) {
        Ok(()) => exit(0),
        Err(truncata) => {
            // A truncated 0755 file is worse than none: it still looks like a program.
            if truncata {
                let _ = std::fs::remove_file(via);
            }
            eprintln!("erratum: scida '{}' scribi non potest", via.display());
            exit(1);
        }
    }
}

/// Everything that must outlive the exec.
struct Statio {
    /// What to hand `execve`.
    via: PathBuf,
    /// Holds the unlinked inode alive; it must outlive the wait.
    _custos: Option<File>,
    /// A name still on disk: the fallback, or a primary path whose unlink failed.
    nomen: Option<PathBuf>,
}

impl Statio {
    /// What `run` does once `wait` has returned; a method so a test can call it.
    fn purga(&self) {
        if let Some(n) = &self.nomen {
            let _ = std::fs::remove_file(n);
        }
    }
}

/// Create the temporary image `O_EXCL` with mode 0700 and write it. The returned path
/// names a *closed* file: `execve` refuses `ETXTBSY` on anything open for writing.
fn create_exclusive(imago: &[u8]) -> std::io::Result<PathBuf> {
    let dir = std::env::temp_dir();
    let mut last: Option<std::io::Error> = None;
    for n in 0..64u32 {
        let nunc = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos())
            .unwrap_or(0);
        let via = dir.join(format!("liss-{}-{}-{}", std::process::id(), nunc, n));
        match OpenOptions::new().write(true).create_new(true).mode(0o700).open(&via) {
            Ok(mut f) => {
                // `mode()` is masked by the umask, so 0700 is set again below —
                // through the handle, not by a second lookup of the path.
                let scriptum = f
                    .write_all(imago)
                    .and_then(|()| f.flush())
                    .and_then(|()| f.set_permissions(std::fs::Permissions::from_mode(0o700)));
                if let Err(e) = scriptum {
                    drop(f);
                    let _ = std::fs::remove_file(&via);
                    return Err(e);
                }
                return Ok(via);
            }
            // `create_new` is not about collisions: `O_CREAT|O_EXCL` refuses an
            // existing file and refuses to follow a symlink in a shared `TMPDIR`.
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => last = Some(e),
            Err(e) => return Err(e),
        }
    }
    Err(last.unwrap_or_else(|| std::io::ErrorKind::AlreadyExists.into()))
}

/// Reopen read-only, unlink the path, and exec `/proc/self/fd/<n>`: the unlink is the
/// entire cleanup story, and `FD_CLOEXEC` is deliberately not cleared (spec §9.5, §11).
fn stage(imago: &[u8]) -> std::io::Result<Statio> {
    stage_in(imago, Path::new("/proc/self/fd"))
}

/// [`stage`], with the descriptor directory named only so the `/proc`-less fallback
/// has automated coverage; both branches are otherwise unreachable in a test suite.
fn stage_in(imago: &[u8], proc_fd: &Path) -> std::io::Result<Statio> {
    let via = create_exclusive(imago)?;

    if !proc_fd.is_dir() {
        return Ok(Statio { via: via.clone(), _custos: None, nomen: Some(via) });
    }

    let custos = match File::open(&via) {
        Ok(f) => f,
        Err(e) => {
            let _ = std::fs::remove_file(&via);
            return Err(e);
        }
    };
    // A failed unlink degrades to the fallback's contract, not to a failure. A `match`
    // and not a dotted call, which `catalogus.rs` would read as a phantom site.
    let nomen = match std::fs::remove_file(&via) {
        Ok(()) => None,
        Err(_) => Some(via),
    };
    Ok(Statio {
        via: proc_fd.join(custos.as_raw_fd().to_string()),
        _custos: Some(custos),
        nomen,
    })
}

/// Spawn with inherited stdio and exit with its status. Inheritance is what lets the
/// child's own runtime receive the closed pipe (spec §6), so D-06 survives at source.
fn run(imago: &[u8]) -> ! {
    let statio = match stage(imago) {
        Ok(s) => s,
        Err(_) => {
            eprintln!("erratum: scida temporaria creari non potest");
            exit(1);
        }
    };

    let effectus = Command::new(&statio.via)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status();

    statio.purga();

    match effectus {
        Err(_) => {
            eprintln!("erratum: programma incipi non potest");
            exit(1);
        }
        Ok(s) => match finis(s) {
            Finis::Status(c) => exit(c),
            Finis::Signum => {
                eprintln!("erratum: programma abrupte cessavit");
                exit(1);
            }
        },
    }
}

/// How a child ended, as far as `liss run` is concerned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Finis {
    Status(i32),
    /// A signal ended it.
    Signum,
}

/// `ExitStatus::code()` is `None` exactly when a signal ended the process; here that
/// becomes a Latin sentence and exit 1 rather than the shell's bare `139`.
fn finis(status: std::process::ExitStatus) -> Finis {
    match status.code() {
        Some(c) => Finis::Status(c),
        None => Finis::Signum,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| (*s).to_string()).collect()
    }

    #[test]
    fn the_bare_positional_form_means_run() {
        let inv = parse(&args(&["salve.liss"])).expect("a path alone is a run");
        assert_eq!(inv.task, Task::Run);
        assert_eq!(inv.path, "salve.liss");
    }

    #[test]
    fn a_subcommand_is_matched_on_the_whole_word_only() {
        assert!(parse(&args(&["run"])).is_none(), "`run` alone has no path");
        let dotted = parse(&args(&["./run"])).expect("`./run` is a path");
        assert_eq!(dotted.task, Task::Run);
        assert_eq!(dotted.path, "./run");
        for near in ["Run", "runs", "builds", "--run"] {
            let inv = parse(&args(&[near]));
            match near.starts_with("--") {
                true => assert!(inv.is_none(), "{near}"),
                false => assert_eq!(inv.expect(near).path, near),
            }
        }
    }

    #[test]
    fn build_takes_its_two_options_in_either_order() {
        let inv = parse(&args(&["build", "--ad", "aarch64", "a.liss", "--exi", "b"]))
            .expect("options may precede the path");
        assert_eq!(inv.task, Task::Build);
        assert_eq!(inv.path, "a.liss");
        assert_eq!(inv.out, Some(PathBuf::from("b")));
        assert_eq!(inv.machine.as_deref(), Some("aarch64"));
    }

    #[test]
    fn the_build_options_are_refused_under_run() {
        assert!(parse(&args(&["run", "a.liss", "--exi", "b"])).is_none());
        assert!(parse(&args(&["a.liss", "--ad", "aarch64"])).is_none());
    }

    #[test]
    fn an_option_without_a_value_and_a_repeated_one_are_both_refused() {
        assert!(parse(&args(&["build", "a.liss", "--exi"])).is_none());
        assert!(parse(&args(&["build", "a.liss", "--exi", "b", "--exi", "c"])).is_none());
        assert!(parse(&args(&["build", "a.liss", "b.liss"])).is_none());
    }

    /// `from_raw` takes a `wait(2)` status: an ordinary exit is `code << 8`, a signal
    /// death is the signal number itself.
    #[test]
    fn a_signalled_child_is_told_apart_from_one_that_exited() {
        use std::os::unix::process::ExitStatusExt;
        use std::process::ExitStatus;

        assert_eq!(finis(ExitStatus::from_raw(0)), Finis::Status(0));
        assert_eq!(finis(ExitStatus::from_raw(3 << 8)), Finis::Status(3));
        assert_eq!(finis(ExitStatus::from_raw(1 << 8)), Finis::Status(1));
        // SIGSEGV, SIGKILL, and SIGSEGV with the core-dump bit set.
        assert_eq!(finis(ExitStatus::from_raw(11)), Finis::Signum);
        assert_eq!(finis(ExitStatus::from_raw(9)), Finis::Signum);
        assert_eq!(finis(ExitStatus::from_raw(11 | 0x80)), Finis::Signum);
    }

    /// A directory that does not exist stands in for an unmounted `/proc`.
    #[test]
    fn without_proc_the_name_is_kept_and_is_removable_afterwards() {
        let statio = stage_in(b"non vera imago", Path::new("/nusquam/self/fd"))
            .expect("the image is staged");
        let nomen = statio.nomen.clone().expect("the fallback keeps the name");
        assert_eq!(statio.via, nomen, "the fallback execs the path, not a descriptor");
        assert!(nomen.is_file(), "{} is gone", nomen.display());
        assert_eq!(
            std::fs::metadata(&nomen).expect("stat").permissions().mode() & 0o7777,
            0o700,
        );
        statio.purga();
        assert!(!nomen.exists(), "the fallback left {} behind", nomen.display());
    }

    /// The magic link resolves to a path the kernel marks `(deleted)`, which is the
    /// only place the primary path's unlink is visible.
    #[test]
    fn with_proc_the_name_is_gone_before_anything_is_executed() {
        let statio =
            stage_in(b"non vera imago", Path::new("/proc/self/fd")).expect("the image is staged");
        assert!(statio.nomen.is_none(), "nothing is left for `run` to remove");
        assert!(statio.via.starts_with("/proc/self/fd/"), "{:?}", statio.via);
        let target = std::fs::read_link(&statio.via).expect("the magic link resolves");
        assert!(
            target.to_string_lossy().ends_with(" (deleted)"),
            "the inode still has a name: {target:?}",
        );
    }

    /// The distinction `build` unlinks on: a device node opened `O_CREAT|O_TRUNC`
    /// loses nothing, and answering otherwise would have `build` unlink the device.
    #[test]
    fn only_a_regular_file_has_content_for_o_trunc_to_destroy() {
        let via = std::env::temp_dir().join(format!("liss-probatio-{}", std::process::id()));
        let _ = std::fs::remove_file(&via);
        let f = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&via)
            .expect("a regular file");
        assert!(truncatur(&f), "a regular file is truncated by the open");
        drop(f);
        let _ = std::fs::remove_file(&via);

        let dev = OpenOptions::new().write(true).open("/dev/full").expect("/dev/full opens");
        assert!(!truncatur(&dev), "O_TRUNC destroys nothing on a device node");
    }

    /// Get one wrong and a refusal to write becomes a deletion; the other, and `build`
    /// unlinks a device node.
    #[test]
    fn a_destructive_failure_is_told_apart_from_a_harmless_one() {
        assert_eq!(write_image(Path::new("/dev/full"), b"imago"), Err(false));
        assert_eq!(write_image(Path::new("/nusquam/omnino/imago"), b"imago"), Err(false));
    }
}
