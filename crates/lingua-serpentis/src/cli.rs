//! The command-line surface, shared by both binaries (spec §9.5).
//!
//! `write_or_exit` and `finish` live here rather than in `main.rs` so that
//! broken-pipe handling and stream order have exactly one implementation; the
//! `liss` driver lives here beside them.
//!
//! # Why the driver is a library module and not `src/bin/liss.rs`
//!
//! `tests/catalogus.rs::extract_sites` reads `crates/*/src` with a
//! **non-recursive** `read_dir`, so anything under `src/bin/` is invisible to
//! the diagnostic ratchet: every sentence written there could be added,
//! reworded or lost with the snapshot header still reading the same number of
//! sites and nobody the wiser. Keeping the driver here keeps all of it inside
//! the catalogue with no change to the scanner — which is the cheaper half of
//! the choice, since the scanner is shared with three other crates and Task 19
//! extends it again. `src/bin/liss.rs` is a shim that collects argv and calls
//! [`liss`], and says nothing.
//!
//! # Unix only, and deliberately
//!
//! `run` stages an image through `O_EXCL`, mode bits, `unlink` and
//! `/proc/self/fd`, none of which have a portable spelling. Spec §9.4 draws the
//! Windows seam at the linker and does not pretend the answer exists yet, and
//! `liss-codegen` embeds musl blobs for two Linux targets, so this crate is
//! already a Linux compiler. Nothing here is `#[cfg]`-guarded to suggest
//! otherwise.

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
pub fn write_or_exit(w: &mut impl Write, text: &str) {
    match w.write_all(text.as_bytes()).and_then(|()| w.flush()) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::BrokenPipe => exit(0),
        // Anything else is a real failure to write, and saying so on a stream
        // that may itself be broken is not worth a second panic.
        Err(_) => exit(1),
    }
}

/// Render a finished pipeline result to the two streams and exit.
///
/// `liss run` never reaches this on the success path — the child owns stdout
/// there — but every front-end failure does. That is what kept a compiled
/// program's diagnostics byte-identical to an interpreted one's while both
/// existed, and it is now simply the one place a front-end failure is written.
pub fn finish(result: Result<(String, u8, Vec<Warning>), ProgramError>) -> ! {
    let r = render(result);
    // Warnings and errors go to stderr so they never contaminate program output.
    write_or_exit(&mut std::io::stderr(), &r.stderr);
    write_or_exit(&mut std::io::stdout(), &r.stdout);
    exit(r.code);
}

/// What `liss` was asked to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Task {
    Run,
    Build,
}

/// One parsed invocation.
#[derive(Debug)]
struct Invocation {
    task: Task,
    /// The path exactly as the user wrote it: every diagnostic about it names
    /// it back the same way.
    path: String,
    out: Option<PathBuf>,
    machine: Option<String>,
}

/// The usage string, and exit 1.
///
/// Two sentences and therefore two sites, which is also what keeps each
/// literal inside the 300-byte window `catalogus.rs` reads a pattern from.
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

/// Parse argv (without the program name). `None` means "print the usage".
///
/// **The first argument is a subcommand iff it is exactly `run` or `build`.**
/// Anything else — including a folder that happens to be named `run` — is the
/// path, which is why such a folder must be given as `./run`, exactly as `git`
/// and `cargo` require. Matching on the whole word rather than probing the
/// filesystem is what makes that rule statable in one sentence.
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
            // `--exi` and `--ad` belong to `build`. Under `run` they fall
            // through to the unknown-option arm, because an option that is
            // silently accepted and then ignored is worse than one refused.
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

/// Load the program named on the command line, and say whether it is a
/// **single-source** one.
///
/// The control flow is `main.rs`'s, defect for defect avoided: asking whether
/// the path exists at all *before* asking what kind of path it is, is what
/// keeps a missing directory from being told it needs a `.liss` extension
/// (D-11).
fn load(arg: &str) -> Result<(Program, bool), ProgramError> {
    let path = Path::new(arg);
    if path.is_dir() {
        return crate::program::load_dir(path).map(|p| (p, false));
    }
    if !path.exists() {
        eprintln!("Error: via '{}' non inventa est", arg);
        exit(1);
    }
    if !arg.ends_with(".liss") {
        eprintln!("Error: nomen scidae in '.liss' desinere debet");
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

/// The single-source strip, applied to a failure.
///
/// The deleted `run_file_with_warnings` did exactly this, and the rule is
/// unchanged: a one-file program is checked under a synthetic `fons.liss` and
/// the name is removed afterwards, so the message reads `Error in linea N:`.
/// Structural errors, which have no line of their own, surface at line 1.
/// This is now the only implementation of it.
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

/// `liss`.
pub fn liss(args: &[String]) -> ! {
    let Some(inv) = parse(args) else { usus() };

    let (loaded, single) = match load(&inv.path) {
        Ok(x) => x,
        // A load failure is structural and carries no line: it must not be
        // given one by `strip`, or `scida '...' non inventa est` would acquire
        // a phantom `in linea 1`.
        Err(e) => finish(Err(e)),
    };

    let mut parsed = match crate::parse_loaded(loaded) {
        Ok(p) => p,
        Err(e) => finish(Err(strip(single, e))),
    };

    // Linting before the checker, and attached to a failure rather than
    // dropped with it (D-09) — the same order `run_loaded` uses, for the same
    // reason: a lint that only appears once the program compiles is one nobody
    // reads.
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

    // **Spec §4.1's debt, and this is the only place that can pay it.** The
    // checker has no single-file signal — it is handed an ordinary one-file
    // program under a synthetic path — so whoever builds the `Programma` must
    // clear `Func::file`, exactly as the driver already clears the error's.
    // It cannot be repaired downstream either: the name is baked into the
    // image at codegen time and `run` inherits the child's stderr, so the
    // parent never sees the sentence it would have to rewrite.
    if single {
        for f in &mut ir.funcs {
            f.file = None;
        }
    }

    // **Before the child exists.** The parent writes no stdout at all — the
    // child's runtime owns it — so the warnings-then-output order the corpus
    // records only holds if these are on stderr before the spawn (spec §9.5).
    let rendered = render(Ok::<_, ProgramError>((String::new(), 0, monita)));
    write_or_exit(&mut std::io::stderr(), &rendered.stderr);

    let machina = match &inv.machine {
        Some(n) => match Target::ex_nomine(n) {
            Some(m) => m,
            None => {
                eprintln!("Error: machina '{}' ignota est", n);
                exit(1);
            }
        },
        None => match Target::hospes() {
            Some(m) => m,
            None => {
                eprintln!("Error: machina huius computatri ignota est");
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
        // The back end's own sentence, already `Error: `-prefixed and already
        // Latin. Forwarded through a bare hole so the catalogue does not read
        // it as a site of its own — the literal lives in `liss-codegen`.
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

/// Where `build` puts the image: `--exi` if given, else the `.liss` stem or the
/// program folder's name, in the current directory.
fn destination(inv: &Invocation, single: bool) -> PathBuf {
    if let Some(via) = &inv.out {
        return via.clone();
    }
    let path = Path::new(&inv.path);
    // A folder keeps its whole name (`computatrum`, not `computa` for a folder
    // called `computa.trum`); a file loses only its `.liss`. `canonicalize`
    // is what makes `liss build .` from inside a program folder name the
    // folder rather than the dot.
    let base = if single {
        path.file_stem().map(std::ffi::OsString::from)
    } else {
        path.canonicalize().ok().and_then(|p| p.file_name().map(std::ffi::OsString::from))
    };
    let via = match base {
        Some(b) => PathBuf::from(b),
        None => {
            eprintln!("Error: nomen exitus statui non potest; viam per '--exi' da");
            exit(1);
        }
    };
    // **Refused here rather than at the write**, which is several seconds of
    // code generation and linking later. The one shape that cannot work and is
    // easy to reach: run from the parent of a program folder, `liss build
    // ./computatrum` resolves its default output to the input directory itself.
    if via.is_dir() {
        eprintln!(
            "Error: exitus '{}' directorium est; aliam viam per '--exi' da",
            via.display()
        );
        exit(1);
    }
    via
}

/// Whether opening this handle with `O_TRUNC` destroyed anything.
///
/// **Only a regular file has content for `O_TRUNC` to discard.** On a device,
/// FIFO or socket the flag is a no-op: the open succeeds trivially and nothing
/// is lost. Asked of the open handle rather than by a second `stat` of the
/// path, so it answers about the object actually opened and cannot be raced.
fn truncatur(f: &File) -> bool {
    f.metadata().map(|m| m.is_file()).unwrap_or(false)
}

/// Write the image, distinguishing a failure that **destroyed the
/// destination's contents** from one that did not: `Err(true)` means bytes were
/// already lost.
///
/// **The rule, and it is the whole reason this is a function:** remove only
/// what we truncated, and we truncated only if it was a regular file. This is
/// the one place `build` touches a user-named path destructively, and the two
/// ways to get it wrong are opposite.
///
/// Removing on `Err(false)` would delete a file this process never managed to
/// open — an existing program in a writable directory, say, whose own mode
/// refuses us — turning a refusal to write into a deletion.
///
/// Answering `true` merely because the open succeeded is the other, and is
/// worse. `/dev/full` is exactly the shape that provokes it: the open succeeds,
/// every write fails `ENOSPC`, and nothing was ever truncated — so a caller
/// told `true` would unlink the **device node**. Run as root in a container,
/// that removes `/dev/full` from a devtmpfs which will not repopulate it, and
/// every later process that touches it breaks.
///
/// The same test decides the `chmod`. A device the user named at `--exi` is
/// left alone in every respect: this must not make `/dev/full` mode 0755
/// either, and 0755 means nothing on something that is not a program.
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
        // Mode 0755 twice, and the second is the one that holds: `mode()`
        // applies only when the file is created and is masked by the umask,
        // and `--exi` may well name a file that already exists with some
        // other mode entirely.
        return scriptum
            .and_then(|()| f.set_permissions(std::fs::Permissions::from_mode(0o755)))
            .map_err(|_| true);
    }
    scriptum.map_err(|_| false)
}

/// Write the finished image and exit.
///
/// Mode 0755: a built artefact is a program someone will hand to someone else,
/// unlike `run`'s temporary, which is 0700 and nameless.
fn build(via: &Path, imago: &[u8]) -> ! {
    match write_image(via, imago) {
        Ok(()) => exit(0),
        Err(truncata) => {
            // A truncated 0755 file where a working program used to be is
            // worse than no file at all: it still looks like a program, and
            // fails to load. `create_exclusive` removes its partial write for
            // the same reason.
            if truncata {
                let _ = std::fs::remove_file(via);
            }
            eprintln!("Error: scida '{}' scribi non potest", via.display());
            exit(1);
        }
    }
}

/// Everything that must outlive the exec.
struct Statio {
    /// What to hand `execve`.
    via: PathBuf,
    /// The read-only descriptor holding the unlinked inode alive. Dropping it
    /// before the child has been waited for would be a bug on the primary
    /// path, which is why it is carried rather than let go.
    _custos: Option<File>,
    /// A name still on disk: the fallback path, or a primary path whose unlink
    /// failed.
    nomen: Option<PathBuf>,
}

impl Statio {
    /// What `run` does once `wait` has returned. On the primary path there has
    /// been no name since before the exec and this does nothing at all.
    ///
    /// A method rather than three lines inside `run` so that the fallback's
    /// cleanup is the code a test can call, instead of a second spelling of it
    /// that could agree with a broken original.
    fn purga(&self) {
        if let Some(n) = &self.nomen {
            let _ = std::fs::remove_file(n);
        }
    }
}

/// Create the temporary image `O_EXCL` with mode 0700 and write it. The
/// returned path names a closed file: **no write descriptor survives this
/// function**, and that is required rather than tidy — `execve` refuses with
/// `ETXTBSY` any file that is open for writing anywhere in the system, so
/// reusing the write handle instead of reopening read-only turns every `liss
/// run` into a failure to start.
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
                // `mode()` is masked by the process umask, which can only ever
                // take bits away — and the one bit this file cannot do without
                // is owner-execute. Setting the permissions again afterwards is
                // not subject to the umask, so 0700 holds however the shell
                // that invoked us was configured.
                let scriptum = f
                    .write_all(imago)
                    .and_then(|()| f.flush())
                    // Through the handle, not the name: by-path it would be a
                    // second lookup of a path in a world-writable directory,
                    // and there is no reason to look twice.
                    .and_then(|()| f.set_permissions(std::fs::Permissions::from_mode(0o700)));
                if let Err(e) = scriptum {
                    drop(f);
                    let _ = std::fs::remove_file(&via);
                    return Err(e);
                }
                return Ok(via);
            }
            // **`create_new` is not about collisions.** The name carries this
            // process's own pid, so no second `liss run` can propose it; what
            // `O_CREAT|O_EXCL` buys is that the open refuses an existing file
            // *and refuses to follow a symlink*, which is the whole defence
            // against a name pre-planted in a world-writable `TMPDIR`.
            // Weakening this to `create` would silently reintroduce the
            // clobber. The retry is for the residue of a previous run of this
            // same pid, or anything else already sitting on the name.
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => last = Some(e),
            Err(e) => return Err(e),
        }
    }
    Err(last.unwrap_or_else(|| std::io::ErrorKind::AlreadyExists.into()))
}

/// Stage the image for execution.
///
/// # The unlink is the entire cleanup story
///
/// Reopen the written file read-only, **unlink the path**, and exec
/// `/proc/self/fd/<n>`. After the unlink the file has no name at all, and the
/// kernel frees the inode when the last descriptor closes — so cleanup covers
/// normal exit, error exit, panic, `SIGINT`, `SIGTERM` and **`SIGKILL`** alike,
/// none of which a `Drop` guard or a signal handler could. Concurrent runs
/// cannot collide either, because after the unlink there is no name to collide
/// on.
///
/// # `FD_CLOEXEC` is not cleared, and does not need to be
///
/// Spec §9.5 says to clear it. std has no such API and spec §11 keeps this
/// crate free of `libc`, so the step is skipped — and it is skipped safely:
/// `execve` resolves its path argument in `do_open_execat`, at the very top of
/// `do_execveat_common`, long before `begin_new_exec` runs `do_close_on_exec`.
/// A close-on-exec descriptor is therefore still open when `/proc/self/fd/<n>`
/// is looked up. This was measured before it was relied on — a real ELF,
/// created `O_EXCL` 0700, written, closed, reopened read-only (std forces
/// `O_CLOEXEC`), unlinked, then exec'd through `/proc/self/fd/<n>`: exit 0,
/// correct output, no name left behind. It is also the mechanism behind
/// `fexecve(3)`, which glibc and musl both implement as exactly this path and
/// which exists to be used with `memfd_create(MFD_CLOEXEC)`. **Do not "fix"
/// this back by adding a dependency.**
///
/// # The fallback, and what it honestly does not cover
///
/// Where `/proc` is not mounted there is no way to name an unlinked file, so
/// the name is kept and removed after `wait` returns. Spec §9.5 calls that path
/// best-effort — a `SIGKILL`ed parent leaves a file behind — and here
/// `SIGINT` and `SIGTERM` join it, because the parent-side handlers the spec
/// asks for need `sigaction` and this crate has no `libc` (§11). That is a
/// widening of an already-documented gap on a path that requires `/proc` to be
/// absent to be reached at all, not a new one.
fn stage(imago: &[u8]) -> std::io::Result<Statio> {
    stage_in(imago, Path::new("/proc/self/fd"))
}

/// [`stage`], with the descriptor directory named rather than hardcoded.
///
/// The parameter exists **only** so the fallback has automated coverage: both
/// branches are otherwise unreachable in a test suite, since every host that
/// runs these tests mounts `/proc`, and a refactor that stopped keeping the
/// name or stopped removing it would leak one 0700 image per `liss run` on
/// every host without `/proc` — containers, chroots, some sandboxes — with the
/// whole suite green. Passing a directory that does not exist forces the
/// fallback exactly as an unmounted `/proc` would.
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
    // **A failed unlink degrades to the fallback's contract rather than to a
    // failure.** The descriptor is open and the image is perfectly runnable, so
    // refusing to start the program because a directory entry could not be
    // removed would be the worst of the three outcomes — and returning `Err`
    // here would leave the name behind, which is the one thing the two error
    // paths on either side of this both take care not to do. Remembering the
    // name instead means `run` removes it after `wait`, exactly as it does
    // where `/proc` is absent.
    //
    // Spelled as a `match` rather than as `Result::err` composed with `map`,
    // because a dotted call to `err` is one of `catalogus.rs`'s diagnostic
    // markers: the shorter form adds a phantom UNPARSED row to the ratchet for
    // a call that raises nothing. (Nor may this comment spell it, for the same
    // reason — the scanner reads comments too.)
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

/// Spawn the image with **inherited** stdio and exit with its status.
///
/// Inheritance is essential rather than convenient: the child's own runtime
/// receives the closed pipe and handles it (spec §6), so D-06 survives at the
/// source and `liss run prog | head` needs no re-buffering.
fn run(imago: &[u8]) -> ! {
    let statio = match stage(imago) {
        Ok(s) => s,
        Err(_) => {
            eprintln!("Error: scida temporaria creari non potest");
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
            eprintln!("Error: programma incipi non potest");
            exit(1);
        }
        Ok(s) => match finis(s) {
            Finis::Status(c) => exit(c),
            Finis::Signum => {
                eprintln!("Error: programma abrupte cessavit");
                exit(1);
            }
        },
    }
}

/// How a child ended, as far as `liss run` is concerned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Finis {
    /// It exited on its own; this status passes through unchanged.
    Status(i32),
    /// A signal ended it.
    Signum,
}

/// Classify a finished child.
///
/// `ExitStatus::code()` is `None` **exactly** when a signal ended the process,
/// and this is where that becomes a Latin sentence and exit 1 instead of the
/// shell's bare `139`. With §8's handler installed and `SIGPIPE` ignored, no
/// ordinary failure reaches the `Signum` arm: it is a genuine external kill, or
/// a defect in this compiler.
///
/// **What this branch buys, stated accurately.** Spec §9.5 justifies it by
/// saying that without it a signalled child "exits 139 and the robustness
/// harness silently accepts it". The second half is not true and the reasoning
/// should not be repeated: `judge` reaches `Outcome::Signalled` only when the
/// process *it* spawned was signalled, which under `liss run` is the parent,
/// never the child. A parent exiting 139 takes `judge`'s `code != 1` arm and is
/// `Acceptable`; a parent exiting 1 with this sentence parses as a well-formed
/// structural diagnostic and is *also* `Acceptable`. Both are accepted, so the
/// mapping closes no hole in the harness.
///
/// What it does buy is the person holding the terminal: a Latin diagnostic
/// saying the program was killed, rather than a bare number they must decode.
/// Enforcing §8's "never a signal" guarantee needs a `judge` rule that maps this
/// sentence to `no abort`, and that belongs to the task that points the harness
/// at this binary.
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
        // The disambiguation rule, stated as a test: a folder named `run` is
        // the subcommand when written bare, and a path when written `./run`.
        assert!(parse(&args(&["run"])).is_none(), "`run` alone has no path");
        let dotted = parse(&args(&["./run"])).expect("`./run` is a path");
        assert_eq!(dotted.task, Task::Run);
        assert_eq!(dotted.path, "./run");
        // And nothing else is a subcommand, however close.
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

    /// **The signalled child.** `Error: programma abrupte cessavit` had no test
    /// at all: collapsing the whole match to `exit(code().unwrap_or(139))` left
    /// the suite green. The decision is lifted out of `run` so it can be made
    /// against real `ExitStatus` values rather than against a re-encoding of
    /// them — `from_raw` takes a `wait(2)` status, where an ordinary exit is
    /// `code << 8` and a signal death is the signal number itself.
    #[test]
    fn a_signalled_child_is_told_apart_from_one_that_exited() {
        use std::os::unix::process::ExitStatusExt;
        use std::process::ExitStatus;

        assert_eq!(finis(ExitStatus::from_raw(0)), Finis::Status(0));
        assert_eq!(finis(ExitStatus::from_raw(3 << 8)), Finis::Status(3));
        assert_eq!(finis(ExitStatus::from_raw(1 << 8)), Finis::Status(1));
        // SIGSEGV — the case that would otherwise reach a shell as 139, and
        // the shape a compiler defect or a missing §8 guard would produce.
        assert_eq!(finis(ExitStatus::from_raw(11)), Finis::Signum);
        // SIGKILL, and SIGSEGV with the core-dump bit set.
        assert_eq!(finis(ExitStatus::from_raw(9)), Finis::Signum);
        assert_eq!(finis(ExitStatus::from_raw(11 | 0x80)), Finis::Signum);
    }

    /// **The `/proc`-less fallback**, which no host running this suite can
    /// otherwise reach. A directory that does not exist stands in for an
    /// unmounted `/proc`, and the two things the fallback owes are asserted:
    /// it execs the *name*, and the name is still there to be removed after
    /// `wait`.
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
        // `run`'s own cleanup, called rather than re-spelled.
        statio.purga();
        assert!(!nomen.exists(), "the fallback left {} behind", nomen.display());
    }

    /// And the primary path, asserted where it is actually visible: the magic
    /// link resolves to a path the kernel marks `(deleted)`, which is the
    /// inode saying it has no name left. Nothing else in the suite can see
    /// this — from outside the process there is only an absence.
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

    /// `O_TRUNC` destroys content only where there is content to destroy.
    ///
    /// The distinction `build` unlinks on. A device node opened with
    /// `O_CREAT|O_TRUNC` loses nothing, so there is nothing to clean up — and
    /// answering otherwise would have `build` unlink the device.
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

    /// A failure that already destroyed the destination must be told apart from
    /// one that did not. Get the first wrong and a refusal to write becomes a
    /// deletion; get the second wrong and `build` unlinks a device node.
    #[test]
    fn a_destructive_failure_is_told_apart_from_a_harmless_one() {
        // `/dev/full` opens and then refuses every write with ENOSPC — the
        // shape of a disk filling up mid-build — but it is a device, so
        // `O_TRUNC` destroyed nothing and there is nothing to remove.
        assert_eq!(write_image(Path::new("/dev/full"), b"imago"), Err(false));
        // A path whose directory does not exist never gets opened at all.
        assert_eq!(write_image(Path::new("/nusquam/omnino/imago"), b"imago"), Err(false));
    }
}
