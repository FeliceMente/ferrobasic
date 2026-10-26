//! Differential test runner and compatibility report: runs every program of
//! the conformance corpus on a target and compares what it does with what
//! the original toolchain did (`docs/compatibility-testing.md`).
//!
//! The original toolchain does not run here: its behaviour is the recorded
//! `.expected` output, or a test's status (`negative`, `pending-recording`,
//! `deviation`) as `ferro-recorder` and the docs settled it.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::{Duration, Instant};

use ferro_corpus::{Program, Status};

/// How long one test may take on a target before it fails: generous, as a
/// program that loops forever is all it guards against.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(60);

/// Where a program runs.
pub enum Target {
    /// In process, through the interpreter.
    Interp,
    /// As a standalone binary that `ferro bundle` builds, in `work`.
    Standalone { ferro: PathBuf, work: PathBuf },
}

impl Target {
    pub fn name(&self) -> &'static str {
        match self {
            Target::Interp => "interp",
            Target::Standalone { .. } => "standalone",
        }
    }
}

/// The four states of the report.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum State {
    Pass,
    Fail,
    /// A documented deviation (`docs/deviations.md`).
    Deviation,
    /// Waiting for a recording.
    Pending,
}

impl State {
    pub const ALL: [State; 4] = [State::Pass, State::Fail, State::Deviation, State::Pending];

    pub fn name(self) -> &'static str {
        match self {
            State::Pass => "pass",
            State::Fail => "fail",
            State::Deviation => "deviation",
            State::Pending => "pending",
        }
    }
}

/// What one test gave.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Verdict {
    pub state: State,
    /// A pass against a hand-derived `.provisional` output, not a recording:
    /// counted apart.
    pub provisional: bool,
    /// Why it failed, or what it is waiting for.
    pub detail: String,
}

impl Verdict {
    fn new(state: State, detail: impl Into<String>) -> Verdict {
        Verdict {
            state,
            provisional: false,
            detail: detail.into(),
        }
    }
}

/// What a program did: refused to build, or ran.
enum Run {
    /// Refused to build, with the first reason given.
    Rejected(String),
    Ran {
        stdout: String,
        error: Option<String>,
        exit_code: i32,
    },
    /// The target itself failed (a crash, a missing binary).
    Broken(String),
}

/// Runs one test on `target`, for at most `timeout`, and judges it.
pub fn judge(program: &Program, target: &Target, timeout: Duration) -> Verdict {
    let run = match target {
        Target::Interp => run_interp(program, timeout),
        Target::Standalone { ferro, work } => run_standalone(program, ferro, work, timeout),
    };
    verdict(program, run)
}

fn verdict(program: &Program, run: Run) -> Verdict {
    let (stdout, error, exit_code) = match run {
        Run::Broken(why) => return Verdict::new(State::Fail, why),
        Run::Rejected(why) if !program.rejected => {
            return Verdict::new(State::Fail, format!("rejected, but must build:\n{why}"));
        }
        Run::Rejected(_) => {
            return match program.status {
                Status::Negative => Verdict::new(State::Pass, ""),
                Status::Deviation => Verdict::new(State::Deviation, "rejected on purpose"),
                _ => Verdict::new(State::Pending, "rejected until recorded"),
            };
        }
        Run::Ran {
            stdout,
            error,
            exit_code,
        } => (stdout, error, exit_code),
    };
    if program.rejected {
        return Verdict::new(State::Fail, "builds, but must be rejected");
    }
    let recorded = program.path.with_extension("expected").exists();
    match program.status {
        Status::Recorded if !recorded => {
            return Verdict::new(State::Fail, "recorded, but has no .expected output");
        }
        Status::PendingRecording if recorded => {
            return Verdict::new(
                State::Fail,
                "has a recording: set its status to \"recorded\" in its .meta.toml",
            );
        }
        _ => {}
    }
    let expected = program.expected_output().map(|e| normalize(&e));
    // The uncaught error and the exit code are known for every test that
    // runs, also one whose output there is none to compare with yet.
    let mut why = String::new();
    if let Some(expected) = &expected
        && stdout != *expected
    {
        why += &output_diff(expected, &stdout);
    }
    if error != program.uncaught_error {
        let _ = writeln!(
            why,
            "uncaught error {}, expected {}",
            shown(error.as_deref()),
            shown(program.uncaught_error.as_deref())
        );
    }
    if exit_code != program.exit_code {
        let _ = writeln!(why, "exit code {exit_code}, expected {}", program.exit_code);
    }
    if !why.is_empty() {
        return Verdict::new(State::Fail, why.trim_end());
    }
    match (program.status, expected) {
        (Status::Deviation, None) => Verdict::new(
            State::Deviation,
            "runs; its reference output is described in its .meta.toml",
        ),
        (Status::Deviation, Some(_)) => Verdict::new(State::Deviation, ""),
        (Status::PendingRecording, None) => {
            Verdict::new(State::Pending, "runs; no output to compare yet")
        }
        (_, Some(_)) => Verdict {
            state: State::Pass,
            provisional: !recorded,
            detail: String::new(),
        },
        // A `negative` test that builds was refused above.
        (_, None) => Verdict::new(State::Fail, "no output to compare with"),
    }
}

fn shown(error: Option<&str>) -> &str {
    error.unwrap_or("none")
}

/// What a recorded output is compared as: line endings are not behaviour
/// (`docs/compatibility-testing.md`, "Normalization"), and number formatting
/// and `Map` order are, so nothing else is normalized. What FerroBASIC
/// prints is compared as it is.
pub fn normalize(output: &str) -> String {
    output.replace("\r\n", "\n")
}

/// The first line where two outputs differ, and how many lines each has.
/// Lines end at `\n` only, so that a `\r` shows in the line it ends.
fn output_diff(expected: &str, actual: &str) -> String {
    let lines =
        |text: &str| -> Vec<String> { text.split_terminator('\n').map(String::from).collect() };
    let (e, a) = (lines(expected), lines(actual));
    let line = e
        .iter()
        .zip(&a)
        .position(|(x, y)| x != y)
        .unwrap_or(e.len().min(a.len()));
    let at = |lines: &[String]| match lines.get(line) {
        Some(text) => format!("{text:?}"),
        None => "end of output".to_string(),
    };
    let mut out = format!(
        "stdout differs at line {}: expected {}, got {}",
        line + 1,
        at(&e),
        at(&a)
    );
    if e.len() != a.len() || expected.ends_with('\n') != actual.ends_with('\n') {
        let _ = write!(out, " ({} lines expected, {} printed)", e.len(), a.len());
    }
    out.push('\n');
    out
}

/// Runs a program the way `ferro run` does, through the same steps
/// (`ferro_cli`), on `ferro`'s stack. A program still running after
/// `timeout` is left behind on its thread, its output from then on
/// discarded: a thread cannot be stopped, so it keeps a core busy until it
/// ends or the process does.
fn run_interp(program: &Program, timeout: Duration) -> Run {
    let source = program.source();
    let (send, receive) = mpsc::channel();
    let stop = Arc::new(AtomicBool::new(false));
    let stopped = Arc::clone(&stop);
    let started = thread::Builder::new()
        .name(program.name.clone())
        .stack_size(ferro_cli::STACK_SIZE)
        .spawn(move || {
            let _ = send.send(interpret(&source, &stopped));
        });
    if let Err(e) = started {
        return Run::Broken(format!("cannot start the interpreter: {e}"));
    }
    match receive.recv_timeout(timeout) {
        Ok(run) => run,
        Err(mpsc::RecvTimeoutError::Timeout) => {
            stop.store(true, Ordering::Relaxed);
            Run::Broken(timed_out(timeout))
        }
        // The thread ended without an answer: it panicked.
        Err(mpsc::RecvTimeoutError::Disconnected) => Run::Broken("the interpreter panicked".into()),
    }
}

fn interpret(source: &str, stop: &Arc<AtomicBool>) -> Run {
    let header_errors = ferro_cli::header_errors(source, false);
    let (module, errors) = ferro_cli::check(source, header_errors);
    let Some(module) = module else {
        let first = errors.into_iter().next().map(|(_, message)| message);
        return Run::Rejected(first.unwrap_or_default());
    };
    let mut stdout = Capture::new(stop);
    let mut stderr = Capture::new(stop);
    let outcome = ferro_cli::run(&module, &[], &mut stdout, &mut stderr);
    match text(stdout.bytes) {
        Ok(stdout) => Run::Ran {
            stdout,
            error: outcome.error.map(|e| e.kind),
            exit_code: i32::from(ferro_cli::exit_status(outcome.exit_code)),
        },
        Err(broken) => broken,
    }
}

/// A program's stdout as text: bytes that are not UTF-8 are a failure, not
/// something to replace and compare.
fn text(stdout: Vec<u8>) -> Result<String, Run> {
    String::from_utf8(stdout).map_err(|_| Run::Broken("stdout is not valid UTF-8".into()))
}

/// An output kept in memory until `stop` is set, then discarded: a program
/// left running after its timeout must not fill the memory.
struct Capture {
    bytes: Vec<u8>,
    stop: Arc<AtomicBool>,
}

impl Capture {
    fn new(stop: &Arc<AtomicBool>) -> Capture {
        Capture {
            bytes: Vec::new(),
            stop: Arc::clone(stop),
        }
    }
}

impl std::io::Write for Capture {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if !self.stop.load(Ordering::Relaxed) {
            self.bytes.extend_from_slice(buf);
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn timed_out(timeout: Duration) -> String {
    format!("still running after {timeout:?}: stopped")
}

fn run_standalone(program: &Program, ferro: &Path, work: &Path, timeout: Duration) -> Run {
    // Numbered, so that no two tests share one, whatever their names; and
    // absolute, as the binary runs from inside it.
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let dir = match std::path::absolute(work.join(format!("{n}-{}", program.file_name()))) {
        Ok(dir) => dir,
        Err(e) => return Run::Broken(format!("{}: {e}", work.display())),
    };
    let _ = fs::remove_dir_all(&dir);
    if let Err(e) = fs::create_dir_all(&dir) {
        return Run::Broken(format!("{}: {e}", dir.display()));
    }
    let binary = dir.join(format!("prog{}", std::env::consts::EXE_SUFFIX));
    let mut bundle = Command::new(ferro);
    bundle
        .arg("bundle")
        .arg(&program.path)
        .arg("-o")
        .arg(&binary);
    let run = match output(&mut bundle, timeout) {
        Err(e) => Run::Broken(format!("{}: {e}", ferro.display())),
        Ok(None) => Run::Broken(format!("ferro bundle {}", timed_out(timeout))),
        Ok(Some(built)) => match built.code {
            Some(0) => match output(Command::new(&binary).current_dir(&dir), timeout) {
                Ok(Some(out)) if crashed(&out) => Run::Broken(format!(
                    "the binary crashed ({}): {}",
                    out.code
                        .map_or("killed by a signal".into(), |c| format!("exit code {c}")),
                    tail(&String::from_utf8_lossy(&out.stderr))
                )),
                Ok(Some(out)) => match text(out.stdout) {
                    Ok(stdout) => Run::Ran {
                        stdout,
                        error: uncaught_error(&String::from_utf8_lossy(&out.stderr)),
                        exit_code: out.code.unwrap_or(-1),
                    },
                    Err(broken) => broken,
                },
                Ok(None) => Run::Broken(timed_out(timeout)),
                Err(e) => Run::Broken(format!("{}: {e}", binary.display())),
            },
            // `ferro bundle` exits with 3 on a program it rejects, after its
            // diagnostics.
            Some(code) if code == i32::from(ferro_cli::EXIT_COMPILE_ERRORS) => {
                Run::Rejected(first_error(&String::from_utf8_lossy(&built.stderr)))
            }
            code => Run::Broken(format!(
                "ferro bundle exited with {code:?}: {}",
                String::from_utf8_lossy(&built.stderr).trim()
            )),
        },
    };
    let _ = fs::remove_dir_all(&dir);
    run
}

/// The first diagnostic `ferro` printed: the message of its first line,
/// `<path>:<line>:<column>: error: <message>`.
fn first_error(stderr: &str) -> String {
    let line = stderr.lines().next().unwrap_or_default();
    line.split_once("error: ")
        .map_or(line, |(_, message)| message)
        .to_string()
}

/// Whether a binary stopped on a crash rather than at its program's end:
/// killed by a signal, or a Rust panic (whose exit code 101 a program may
/// also give, so its message decides).
fn crashed(out: &Finished) -> bool {
    out.code.is_none() || String::from_utf8_lossy(&out.stderr).contains("panicked at")
}

/// The last lines of a crash's stderr.
fn tail(stderr: &str) -> String {
    let lines: Vec<&str> = stderr.trim_end().lines().collect();
    lines[lines.len().saturating_sub(5)..].join("\n")
}

/// What a finished process gave.
struct Finished {
    code: Option<i32>,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

/// [`Command::output`] on a binary that was just written: retried while
/// Linux still sees it open for writing (see [`retry_busy`]).
pub fn output_written(command: &mut Command) -> std::io::Result<std::process::Output> {
    retry_busy(|| command.output())
}

/// Starts a binary that was just written, for up to 2 s. Linux refuses to
/// run a file that some process still has open for writing (`ETXTBSY`). With
/// several threads, a child process another thread is starting can hold a
/// copy of this thread's write descriptor for a moment, between its `fork`
/// and its `exec`; so retry briefly instead of failing at random.
fn retry_busy<T>(mut start: impl FnMut() -> std::io::Result<T>) -> std::io::Result<T> {
    for _ in 0..200 {
        match start() {
            Err(e) if e.kind() == std::io::ErrorKind::ExecutableFileBusy => {
                thread::sleep(Duration::from_millis(10));
            }
            result => return result,
        }
    }
    start()
}

/// Runs `command` to its end, or kills it after `timeout` (`None`), started
/// as a binary that was just written (see [`retry_busy`]).
fn output(command: &mut Command, timeout: Duration) -> std::io::Result<Option<Finished>> {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = retry_busy(|| command.spawn())?;
    // Read both pipes while waiting, so that a full pipe cannot stall it.
    let drain = |pipe: Option<Box<dyn Read + Send>>| {
        thread::spawn(move || {
            let mut bytes = Vec::new();
            if let Some(mut pipe) = pipe {
                let _ = pipe.read_to_end(&mut bytes);
            }
            bytes
        })
    };
    let stdout = drain(
        child
            .stdout
            .take()
            .map(|p| Box::new(p) as Box<dyn Read + Send>),
    );
    let stderr = drain(
        child
            .stderr
            .take()
            .map(|p| Box::new(p) as Box<dyn Read + Send>),
    );
    // A timeout too long to add to now is no deadline at all.
    let deadline = Instant::now().checked_add(timeout);
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break Some(status);
        }
        if deadline.is_some_and(|d| Instant::now() >= d) {
            let _ = child.kill();
            let _ = child.wait();
            break None;
        }
        thread::sleep(Duration::from_millis(5));
    };
    let (stdout, stderr) = (
        stdout.join().unwrap_or_default(),
        stderr.join().unwrap_or_default(),
    );
    Ok(status.map(|status| Finished {
        code: status.code(),
        stdout,
        stderr,
    }))
}

/// The kind of the uncaught error a program printed to stderr, in the shape
/// B4J uses (`docs/spec/cli.md`): the line after `Error occurred on line:`,
/// up to its `: `.
pub fn uncaught_error(stderr: &str) -> Option<String> {
    let mut lines = stderr.lines();
    lines.find(|l| l.starts_with("Error occurred on line:"))?;
    let line = lines.next()?;
    Some(
        line.split_once(": ")
            .map_or(line, |(kind, _)| kind)
            .to_string(),
    )
}

/// One test's result in the report.
pub struct Entry {
    pub name: String,
    pub level: Option<u8>,
    pub features: Vec<String>,
    pub verdict: Verdict,
}

/// The results of a run on one target.
pub struct Report {
    pub target: &'static str,
    pub entries: Vec<Entry>,
}

impl Report {
    /// Judges every program, several at a time, each for at most `timeout`.
    pub fn run(programs: &[Program], target: &Target, timeout: Duration) -> Report {
        let workers = thread::available_parallelism().map_or(1, |n| n.get().min(8));
        let next = std::sync::atomic::AtomicUsize::new(0);
        let mut verdicts: Vec<Option<Verdict>> = vec![None; programs.len()];
        let done = Mutex::new(&mut verdicts);
        thread::scope(|scope| {
            for _ in 0..workers {
                scope.spawn(|| {
                    loop {
                        let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        let Some(program) = programs.get(i) else {
                            break;
                        };
                        let verdict = judge(program, target, timeout);
                        done.lock().unwrap()[i] = Some(verdict);
                    }
                });
            }
        });
        let entries = programs
            .iter()
            .zip(verdicts)
            .map(|(p, verdict)| Entry {
                name: p.name.clone(),
                level: p.level,
                features: p.features.clone(),
                verdict: verdict.expect("every program is judged"),
            })
            .collect();
        Report {
            target: target.name(),
            entries,
        }
    }

    pub fn failed(&self) -> usize {
        self.count(State::Fail)
    }

    fn count(&self, state: State) -> usize {
        self.entries
            .iter()
            .filter(|e| e.verdict.state == state)
            .count()
    }

    /// The report as text: failures with their reason, the tests waiting
    /// for a recording if `verbose`, then a table by level (and by feature
    /// if `by_feature`).
    pub fn render(&self, verbose: bool, by_feature: bool) -> String {
        let mut out = String::new();
        for e in &self.entries {
            let show = match e.verdict.state {
                State::Fail => true,
                State::Pending | State::Deviation => verbose,
                State::Pass => false,
            };
            if show {
                let _ = writeln!(out, "{} {}", e.verdict.state.name().to_uppercase(), e.name);
                for line in e.verdict.detail.lines() {
                    let _ = writeln!(out, "    {line}");
                }
            }
        }
        if !out.is_empty() {
            out.push('\n');
        }
        let by_level = self.table(|e| {
            vec![
                e.level
                    .map_or("level ?".to_string(), |l| format!("level {l}")),
            ]
        });
        out += &render_table("Level", &by_level);
        if by_feature {
            out.push('\n');
            out += &render_table("Feature", &self.table(|e| e.features.clone()));
        }
        let provisional = self
            .entries
            .iter()
            .filter(|e| e.verdict.provisional && e.verdict.state == State::Pass)
            .count();
        let _ = writeln!(
            out,
            "\n{}: {} tests, {} pass ({provisional} against a provisional output), {} fail, \
             {} deviation, {} pending",
            self.target,
            self.entries.len(),
            self.count(State::Pass),
            self.failed(),
            self.count(State::Deviation),
            self.count(State::Pending),
        );
        out
    }

    /// Counts by state for each group a test belongs to.
    fn table(&self, groups: impl Fn(&Entry) -> Vec<String>) -> BTreeMap<String, [usize; 4]> {
        let mut table: BTreeMap<String, [usize; 4]> = BTreeMap::new();
        for e in &self.entries {
            let column = State::ALL
                .iter()
                .position(|&s| s == e.verdict.state)
                .unwrap();
            for group in groups(e) {
                table.entry(group).or_default()[column] += 1;
            }
        }
        table
    }
}

fn render_table(title: &str, rows: &BTreeMap<String, [usize; 4]>) -> String {
    let mut out = format!("| {title} |");
    for state in State::ALL {
        let _ = write!(out, " {} |", state.name());
    }
    out += "\n| --- |";
    out += &" ---: |".repeat(State::ALL.len());
    out.push('\n');
    for (group, counts) in rows {
        let _ = write!(out, "| {group} |");
        for n in counts {
            let _ = write!(out, " {n} |");
        }
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests;
