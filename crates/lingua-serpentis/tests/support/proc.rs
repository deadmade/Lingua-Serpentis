//! Spawning the real binary. Needed for everything the library cannot
//! observe: aborts, broken pipes, real process exit status, stream routing
//! and argv handling.
#![allow(dead_code)]

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// The binary every subprocess test drives.
///
/// **`liss`, as of Task 19**, and that is what gives spec §8 its teeth. The
/// robustness and generative harnesses judge whatever this spawns, so pointing
/// it at the compiler is what makes "no compiled program dies by signal" a
/// fuzzed claim rather than a written one — a mutation or a generated program
/// that faults now reaches `judge` instead of reaching a tree-walker that could
/// not fault in the same way.
///
/// It costs a Cranelift pass, a link and a second `execve` per case, which is
/// why `generative.rs` carries an explicit case budget (spec §10.2).
///
/// The signal never arrives as `Outcome::Signalled`, though: the process spawned
/// here is the compiling **parent**, and `liss` maps a signalled child to
/// `Error: programma abrupte cessavit` and exit 1 (§9.5). `diag::judge` has a
/// rule for exactly that sentence, and without it §8 would have no enforcement
/// at all — see the note there.
pub const BINARY: &str = env!("CARGO_BIN_EXE_liss");

/// How a run of the binary ended.
#[derive(Debug, Clone, PartialEq)]
pub enum Outcome {
    /// Exited normally with this status.
    Exited { code: i32, stdout: String, stderr: String },
    /// Killed by a signal — SIGABRT from a stack overflow, SIGSEGV, ...
    Signalled { signal: i32, stdout: String, stderr: String },
    /// Still running when the deadline passed.
    TimedOut,
}

impl Outcome {
    pub fn stderr(&self) -> &str {
        match self {
            Outcome::Exited { stderr, .. } | Outcome::Signalled { stderr, .. } => stderr,
            Outcome::TimedOut => "",
        }
    }
    pub fn stdout(&self) -> &str {
        match self {
            Outcome::Exited { stdout, .. } | Outcome::Signalled { stdout, .. } => stdout,
            Outcome::TimedOut => "",
        }
    }
}

/// Run the binary with `args`, capturing both streams to temp files rather
/// than pipes — a pipe would deadlock as soon as the child fills the buffer,
/// which some of these inputs do on purpose.
pub fn run_binary(args: &[&std::ffi::OsStr], timeout: Duration) -> Outcome {
    let dir = temp_dir("liss-proc");
    let out_path = dir.path().join("stdout");
    let err_path = dir.path().join("stderr");
    let out = std::fs::File::create(&out_path).unwrap();
    let err = std::fs::File::create(&err_path).unwrap();

    let mut child = Command::new(BINARY)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::from(out))
        .stderr(Stdio::from(err))
        .spawn()
        .expect("spawn liss");

    let deadline = Instant::now() + timeout;
    let status = loop {
        match child.try_wait().expect("try_wait") {
            Some(s) => break s,
            None if Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                return Outcome::TimedOut;
            }
            None => std::thread::sleep(Duration::from_millis(5)),
        }
    };

    let stdout = String::from_utf8_lossy(&std::fs::read(&out_path).unwrap()).into_owned();
    let stderr = String::from_utf8_lossy(&std::fs::read(&err_path).unwrap()).into_owned();

    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        if let Some(signal) = status.signal() {
            return Outcome::Signalled { signal, stdout, stderr };
        }
    }
    Outcome::Exited { code: status.code().unwrap_or(-1), stdout, stderr }
}

/// Like `run_binary`, but reads a little of the child's stdout and then closes
/// the pipe while the child is still writing. This is the only way to provoke
/// a broken pipe, which is not observable through a file redirect.
pub fn run_closing_stdout_early(args: &[&std::ffi::OsStr], timeout: Duration) -> Outcome {
    use std::io::Read;

    let dir = temp_dir("liss-pipe");
    let err_path = dir.path().join("stderr");
    let err = std::fs::File::create(&err_path).unwrap();

    let mut child = Command::new(BINARY)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::from(err))
        .spawn()
        .expect("spawn liss");

    {
        let mut out = child.stdout.take().expect("piped stdout");
        let mut buf = [0u8; 64];
        let _ = out.read(&mut buf);
        // `out` drops here, closing the read end while the child writes on.
    }

    let deadline = Instant::now() + timeout;
    let status = loop {
        match child.try_wait().expect("try_wait") {
            Some(s) => break s,
            None if Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                return Outcome::TimedOut;
            }
            None => std::thread::sleep(Duration::from_millis(5)),
        }
    };

    let stderr = String::from_utf8_lossy(&std::fs::read(&err_path).unwrap()).into_owned();
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        if let Some(signal) = status.signal() {
            return Outcome::Signalled { signal, stdout: String::new(), stderr };
        }
    }
    Outcome::Exited { code: status.code().unwrap_or(-1), stdout: String::new(), stderr }
}

/// A directory removed on drop. Hand-rolled because runtime deps stay at zero
/// and `tempfile` is not worth a dev-dep for twenty lines.
pub struct TempDir(PathBuf);

impl TempDir {
    pub fn path(&self) -> &Path {
        &self.0
    }
    /// Write a file, creating parent directories. Path is `/`-separated and
    /// relative to the temp root.
    pub fn write(&self, rel: &str, content: &str) -> PathBuf {
        let p = self.0.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        let mut f = std::fs::File::create(&p).unwrap();
        f.write_all(content.as_bytes()).unwrap();
        p
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

static COUNTER: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

pub fn temp_dir(prefix: &str) -> TempDir {
    let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let p = std::env::temp_dir().join(format!("{prefix}-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).unwrap();
    TempDir(p)
}
