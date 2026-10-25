//! Running a program with a time limit, capturing what it prints.

use std::io::{self, Read};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

/// How a program ended.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Run {
    /// The exit code; `None` if it was stopped (time limit) or killed.
    pub code: Option<i32>,
    pub timed_out: bool,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

/// Runs the builder and Java. The real one starts processes; tests use a
/// fake.
pub trait Runner {
    fn run(&self, program: &Path, args: &[String], dir: &Path, limit: Duration) -> io::Result<Run>;
}

/// Starts real processes.
pub struct Processes;

impl Runner for Processes {
    fn run(&self, program: &Path, args: &[String], dir: &Path, limit: Duration) -> io::Result<Run> {
        let mut child = Command::new(program)
            .args(args)
            .current_dir(dir)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| io::Error::new(e.kind(), format!("{}: {e}", program.display())))?;
        // Read both pipes while waiting, so a chatty program cannot block.
        let stdout = drain(child.stdout.take());
        let stderr = drain(child.stderr.take());
        let (code, timed_out) = wait(&mut child, limit)?;
        Ok(Run {
            code,
            timed_out,
            stdout: stdout.join().unwrap_or_default(),
            stderr: stderr.join().unwrap_or_default(),
        })
    }
}

fn drain(pipe: Option<impl Read + Send + 'static>) -> thread::JoinHandle<Vec<u8>> {
    thread::spawn(move || {
        let mut out = Vec::new();
        if let Some(mut pipe) = pipe {
            let _ = pipe.read_to_end(&mut out);
        }
        out
    })
}

fn wait(child: &mut Child, limit: Duration) -> io::Result<(Option<i32>, bool)> {
    let start = Instant::now();
    loop {
        if let Some(status) = child.try_wait()? {
            return Ok((status.code(), false));
        }
        if start.elapsed() >= limit {
            // It may have ended just now; either way it is gone afterwards.
            let _ = child.kill();
            child.wait()?;
            return Ok((None, true));
        }
        thread::sleep(Duration::from_millis(20));
    }
}
