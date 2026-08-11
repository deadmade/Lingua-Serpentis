#![allow(dead_code)]

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// The compiling parent: a signalled child arrives at `diag::judge` as a sentence, not a signal.
pub const BINARY: &str = env!("CARGO_BIN_EXE_liss");

#[derive(Debug, Clone, PartialEq)]
pub enum Outcome {
    Exited { code: i32, stdout: String, stderr: String },
    Signalled { signal: i32, stdout: String, stderr: String },
    TimedOut,
}

/// This module's poll interval; `support`'s own `PAUSA` is a different, documented 2 ms.
const PAUSA: Duration = Duration::from_millis(5);

/// Wait for `child`, polling every `pausa`. At `timeout` the child is killed and reaped and
/// `Ok(None)` is returned; each caller renders that in its own idiom.
pub fn wait_or_kill(
    child: &mut std::process::Child,
    timeout: Duration,
    pausa: Duration,
) -> std::io::Result<Option<std::process::ExitStatus>> {
    let deadline = Instant::now() + timeout;
    loop {
        match child.try_wait()? {
            Some(s) => return Ok(Some(s)),
            None if Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                return Ok(None);
            }
            None => std::thread::sleep(pausa),
        }
    }
}

/// Captures both streams to files rather than pipes; some of these inputs would deadlock a pipe.
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

    let status = match wait_or_kill(&mut child, timeout, PAUSA).expect("try_wait") {
        Some(s) => s,
        None => return Outcome::TimedOut,
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

    let status = match wait_or_kill(&mut child, timeout, PAUSA).expect("try_wait") {
        Some(s) => s,
        None => return Outcome::TimedOut,
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

/// A directory removed on drop. A newtype over `tempfile`'s rather than a re-export, because
/// `write` is this harness's own and forty call sites spell it.
pub struct TempDir(tempfile::TempDir);

impl TempDir {
    pub fn path(&self) -> &Path {
        self.0.path()
    }
    /// `rel` is `/`-separated and relative to the temp root; parents are created.
    pub fn write(&self, rel: &str, content: &str) -> PathBuf {
        let p = self.path().join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        let mut f = std::fs::File::create(&p).unwrap();
        f.write_all(content.as_bytes()).unwrap();
        p
    }
}

pub fn temp_dir(prefix: &str) -> TempDir {
    TempDir(tempfile::Builder::new().prefix(prefix).tempdir().expect("a scratch directory"))
}
