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

/// A directory removed on drop.
pub struct TempDir(PathBuf);

impl TempDir {
    pub fn path(&self) -> &Path {
        &self.0
    }
    /// `rel` is `/`-separated and relative to the temp root; parents are created.
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
