//! `ferro-compare`: runs `ferro check` from two builds on generated
//! programs, and reports the programs whose result changed, so that a change
//! to the checker shows what it accepts or refuses differently
//! (`docs/compatibility-testing.md`, "Comparing two builds").
//!
//! It compares FerroBASIC with itself, not with B4J: whether a change is
//! right is for the recordings to say.

pub mod families;

use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;
use std::fs::{File, OpenOptions};
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::{Duration, Instant};

/// How many examples of each change the report shows by default.
pub const DEFAULT_EXAMPLES: usize = 3;

/// How long one `ferro check` may run before it counts as a crash.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

/// What `ferro check` made of a program.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// No error (exit code 0).
    Accepted,
    /// Compile errors (exit code 3): the `error:` lines, in order.
    Refused(Vec<String>),
    /// Anything else: a crash, a usage error, a timeout. How it ended, and
    /// its stderr.
    Crashed(String),
}

impl Outcome {
    /// Reads `ferro check`'s exit code and stderr (`docs/spec/cli.md`,
    /// "Exit codes" and "Diagnostics").
    pub fn from_check(code: Option<i32>, stderr: &str) -> Outcome {
        match code {
            Some(0) => Outcome::Accepted,
            Some(3) => Outcome::Refused(
                stderr
                    .lines()
                    .filter(|line| line.contains(": error: "))
                    .map(str::to_string)
                    .collect(),
            ),
            Some(code) => Outcome::Crashed(format!("exit code {code}\n{stderr}")),
            None => Outcome::Crashed(format!("killed by a signal\n{stderr}")),
        }
    }

    fn errors(&self) -> usize {
        match self {
            Outcome::Refused(errors) => errors.len(),
            _ => 0,
        }
    }
}

/// How a program's result changed from the old build to the new one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Change {
    /// Only the new build crashed on it: a bug the change brings.
    NewCrash,
    /// Both builds crashed on it: a bug the change did not bring.
    BothCrash,
    /// The old build crashed on it, the new one does not.
    OldCrash,
    /// Refused before, accepted now.
    NewlyAccepted,
    /// Accepted before, refused now.
    NewlyRefused,
    /// Refused by both, with more errors now.
    MoreErrors,
    /// Refused by both, with fewer errors now.
    FewerErrors,
    /// Refused by both, as many errors, other messages or places.
    OtherMessages,
}

impl Change {
    pub const ALL: [Change; 8] = [
        Change::NewCrash,
        Change::BothCrash,
        Change::OldCrash,
        Change::NewlyAccepted,
        Change::NewlyRefused,
        Change::MoreErrors,
        Change::FewerErrors,
        Change::OtherMessages,
    ];

    fn label(self) -> &'static str {
        match self {
            Change::NewCrash => "crashes in the new build only",
            Change::BothCrash => "crashes in both builds",
            Change::OldCrash => "crashed in the old build only",
            Change::NewlyAccepted => "newly accepted",
            Change::NewlyRefused => "newly refused",
            Change::MoreErrors => "more errors",
            Change::FewerErrors => "fewer errors",
            Change::OtherMessages => "same count, other messages",
        }
    }

    /// The change between two outcomes, if any.
    pub fn between(old: &Outcome, new: &Outcome) -> Option<Change> {
        use Outcome::*;
        match (old, new) {
            (Crashed(_), Crashed(_)) => Some(Change::BothCrash),
            (_, Crashed(_)) => Some(Change::NewCrash),
            (Crashed(_), _) => Some(Change::OldCrash),
            (Accepted, Accepted) => None,
            (Refused(_), Accepted) => Some(Change::NewlyAccepted),
            (Accepted, Refused(_)) => Some(Change::NewlyRefused),
            (Refused(a), Refused(b)) if b.len() > a.len() => Some(Change::MoreErrors),
            (Refused(a), Refused(b)) if b.len() < a.len() => Some(Change::FewerErrors),
            (Refused(a), Refused(b)) if a != b => Some(Change::OtherMessages),
            (Refused(_), Refused(_)) => None,
        }
    }
}

/// One generated program, written to disk, with both builds' outcomes.
pub struct Compared {
    pub family: &'static str,
    pub name: String,
    pub path: PathBuf,
    pub source_len: usize,
    pub old: Outcome,
    pub new: Outcome,
}

/// Writes the programs of `families` into `dir`, one folder per family.
pub fn write_programs(
    families: &[&families::Family],
    dir: &Path,
) -> std::io::Result<Vec<(&'static str, String, PathBuf, usize)>> {
    let mut written = Vec::new();
    for family in families {
        let folder = dir.join(family.name);
        if folder.exists() {
            std::fs::remove_dir_all(&folder)?;
        }
        std::fs::create_dir_all(&folder)?;
        for program in (family.generate)() {
            let path = folder.join(format!("{}.bas", program.name));
            std::fs::write(&path, &program.source)?;
            written.push((family.name, program.name, path, program.source.len()));
        }
    }
    Ok(written)
}

/// Runs `ferro check` on every file with `jobs` threads, in the files'
/// order. Each check may take `timeout`; each thread keeps the stderr of
/// its current check in a file of its own in `scratch`.
pub fn check_all(
    ferro: &Path,
    files: &[PathBuf],
    jobs: usize,
    timeout: Duration,
    scratch: &Path,
) -> Vec<Outcome> {
    let next = AtomicUsize::new(0);
    let watchdog = Watchdog::default();
    let mut results: Vec<Option<Outcome>> = vec![None; files.len()];
    let chunks: Vec<Vec<(usize, Outcome)>> = std::thread::scope(|scope| {
        scope.spawn(|| watchdog.run());
        // Stops the watchdog when the workers are done, or when one panics:
        // the scope waits for every thread before it ends.
        let _stop = watchdog.stopper();
        let workers: Vec<_> = (0..jobs.max(1))
            .map(|worker| {
                let (next, watchdog) = (&next, &watchdog);
                scope.spawn(move || {
                    let stderr = scratch.join(format!("stderr-{worker}"));
                    let mut done = Vec::new();
                    loop {
                        let i = next.fetch_add(1, Ordering::Relaxed);
                        let Some(file) = files.get(i) else {
                            return done;
                        };
                        done.push((i, check_watched(ferro, file, timeout, &stderr, watchdog)));
                    }
                })
            })
            .collect();
        workers
            .into_iter()
            .map(|w| w.join().expect("a worker panicked"))
            .collect()
    });
    for (i, outcome) in chunks.into_iter().flatten() {
        results[i] = Some(outcome);
    }
    results
        .into_iter()
        .map(|r| r.expect("every file is checked"))
        .collect()
}

/// `ferro check <file>`, stopped after `timeout`. Its stderr goes through
/// the file `stderr`, so a long one cannot block it.
pub fn check(ferro: &Path, file: &Path, timeout: Duration, stderr: &Path) -> Outcome {
    let watchdog = Watchdog::default();
    std::thread::scope(|scope| {
        scope.spawn(|| watchdog.run());
        let _stop = watchdog.stopper();
        check_watched(ferro, file, timeout, stderr, &watchdog)
    })
}

/// Kills the checks that run past their deadline, so that each check can
/// simply wait for its process instead of polling it.
#[derive(Default)]
struct Watchdog {
    /// The running checks: process id and deadline.
    running: Mutex<HashMap<u32, Instant>>,
    /// The processes it killed, until their check reads it.
    killed: Mutex<HashSet<u32>>,
    stop: AtomicBool,
}

impl Watchdog {
    /// How often it looks for checks past their deadline: a timeout is
    /// reported this much late at most.
    const TICK: Duration = Duration::from_millis(50);

    fn run(&self) {
        while !self.stop.load(Ordering::Relaxed) {
            std::thread::sleep(Self::TICK);
            let now = Instant::now();
            let mut running = self.running.lock().unwrap();
            let overdue: Vec<u32> = running
                .iter()
                .filter(|(_, deadline)| **deadline <= now)
                .map(|(pid, _)| *pid)
                .collect();
            for pid in overdue {
                // Marked killed while the check is still registered, so that
                // the check, which deregisters under the same lock, sees it.
                running.remove(&pid);
                self.killed.lock().unwrap().insert(pid);
                kill(pid);
            }
        }
    }

    /// Stops the watchdog when dropped, even while unwinding.
    fn stopper(&self) -> Stopper<'_> {
        Stopper(&self.stop)
    }

    /// Deregisters a check whose process ended; whether it was killed.
    fn finished(&self, pid: u32) -> bool {
        let mut running = self.running.lock().unwrap();
        running.remove(&pid);
        self.killed.lock().unwrap().remove(&pid)
    }
}

struct Stopper<'a>(&'a AtomicBool);

impl Drop for Stopper<'_> {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Relaxed);
    }
}

fn kill(pid: u32) {
    let pid = pid.to_string();
    let mut command = if cfg!(windows) {
        let mut command = Command::new("taskkill");
        command.args(["/F", "/T", "/PID", &pid]);
        command
    } else {
        let mut command = Command::new("kill");
        command.args(["-KILL", &pid]);
        command
    };
    let _ = command.stdout(Stdio::null()).stderr(Stdio::null()).status();
}

fn check_watched(
    ferro: &Path,
    file: &Path,
    timeout: Duration,
    stderr: &Path,
    watchdog: &Watchdog,
) -> Outcome {
    let sink = match File::create(stderr) {
        Ok(sink) => sink,
        Err(e) => return Outcome::Crashed(format!("cannot create {}: {e}", stderr.display())),
    };
    let child = Command::new(ferro)
        .arg("check")
        .arg(file)
        .stdout(Stdio::null())
        .stderr(sink)
        .spawn();
    let mut child = match child {
        Ok(child) => child,
        Err(e) => return Outcome::Crashed(format!("could not start {}: {e}", ferro.display())),
    };
    let pid = child.id();
    watchdog
        .running
        .lock()
        .unwrap()
        .insert(pid, Instant::now() + timeout);
    let status = child.wait();
    if watchdog.finished(pid) {
        return Outcome::Crashed(format!("timed out after {} s", timeout.as_secs()));
    }
    let status = match status {
        Ok(status) => status,
        Err(e) => return Outcome::Crashed(format!("cannot wait for ferro: {e}")),
    };
    let text = std::fs::read(stderr).unwrap_or_default();
    Outcome::from_check(status.code(), &String::from_utf8_lossy(&text))
}

/// Keeps a second run out of a work folder while one uses it: two would
/// remove each other's worktree and rewrite each other's programs. The
/// file holds the run's process id, so a lock left by a run that was
/// stopped (Ctrl-C) is taken over.
pub struct Lock(PathBuf);

impl Lock {
    /// How long a lock file may stay empty before it counts as left behind.
    pub const EMPTY_FOR: Duration = Duration::from_secs(5);

    pub fn take(work: &Path) -> Result<Lock, String> {
        let path = work.join("lock");
        for _ in 0..2 {
            match OpenOptions::new().write(true).create_new(true).open(&path) {
                Ok(mut file) => {
                    let _ = write!(file, "{}", std::process::id());
                    return Ok(Lock(path));
                }
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                    let holder = std::fs::read_to_string(&path).unwrap_or_default();
                    let holder = holder.trim();
                    let stale = match holder.parse::<u32>() {
                        Ok(pid) => !process_runs(pid),
                        // A run writes its id right after creating the file:
                        // one still empty after a while was left by a run
                        // stopped in between.
                        Err(_) => holder.is_empty() && age(&path) > Lock::EMPTY_FOR,
                    };
                    if !stale {
                        let who = if holder.is_empty() {
                            String::new()
                        } else {
                            format!(" (process {holder})")
                        };
                        return Err(format!(
                            "another ferro-compare{who} is using {}: wait for it, or pass \
                             another --work",
                            work.display()
                        ));
                    }
                    let _ = std::fs::remove_file(&path);
                }
                Err(e) => return Err(format!("cannot create {}: {e}", path.display())),
            }
        }
        Err(format!("cannot take {}", path.display()))
    }
}

/// How long ago a file was last written; zero when that is unknown.
fn age(path: &Path) -> Duration {
    std::fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|modified| modified.elapsed().ok())
        .unwrap_or_default()
}

impl Drop for Lock {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

/// Whether a process runs; when it cannot tell, it says yes, so that a lock
/// is never taken from a run that may be going on.
fn process_runs(pid: u32) -> bool {
    let pid = pid.to_string();
    if cfg!(windows) {
        let filter = format!("PID eq {pid}");
        return match Command::new("tasklist")
            .args(["/FI", &filter, "/NH"])
            .output()
        {
            Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout).contains(&pid),
            _ => true,
        };
    }
    // `kill -0` only asks whether the process exists; any failure but "No
    // such process" (no permission, say) leaves the answer unknown.
    let exists = match Command::new("kill")
        .args(["-0", &pid])
        .env("LC_ALL", "C")
        .stdout(Stdio::null())
        .output()
    {
        Ok(out) if out.status.success() => true,
        Ok(out) => !String::from_utf8_lossy(&out.stderr).contains("No such process"),
        Err(_) => true,
    };
    // A zombie exists but has ended and only waits to be reaped: `ps`
    // shows its state as `Z`. Any other answer, or none, means it runs.
    exists
        && !Command::new("ps")
            .args(["-o", "stat=", "-p", &pid])
            .stderr(Stdio::null())
            .output()
            .is_ok_and(|out| String::from_utf8_lossy(&out.stdout).trim().starts_with('Z'))
}

/// The `cargo build` arguments that build into the profile folder `name`
/// of a target folder (`debug`, `release`, or a custom profile's own
/// name); `None` when `name` is not a profile folder.
pub fn profile_args(name: &str, is_profile_folder: bool) -> Option<Vec<String>> {
    if !is_profile_folder {
        return None;
    }
    Some(match name {
        "debug" => vec![],
        "release" => vec!["--release".to_string()],
        other => vec!["--profile".to_string(), other.to_string()],
    })
}

/// The comparison of two builds.
pub struct Report {
    pub old_name: String,
    pub new_name: String,
    pub programs: Vec<Compared>,
}

impl Report {
    /// The programs with a change, by change, each list smallest first.
    pub fn changes(&self) -> Vec<(Change, Vec<&Compared>)> {
        let mut by_change: Vec<(Change, Vec<&Compared>)> =
            Change::ALL.iter().map(|&c| (c, Vec::new())).collect();
        for program in &self.programs {
            if let Some(change) = Change::between(&program.old, &program.new) {
                let slot = Change::ALL.iter().position(|&c| c == change).unwrap();
                by_change[slot].1.push(program);
            }
        }
        for (_, programs) in &mut by_change {
            programs.sort_by_key(|p| (p.source_len, p.name.clone()));
        }
        by_change
    }

    pub fn new_crashes(&self) -> usize {
        self.programs
            .iter()
            .filter(|p| Change::between(&p.old, &p.new) == Some(Change::NewCrash))
            .count()
    }

    /// The report: counts by family and change, then the smallest
    /// `examples` programs of each change with both builds' errors.
    pub fn render(&self, examples: usize) -> String {
        let mut out = String::new();
        let mut families: Vec<&str> = self.programs.iter().map(|p| p.family).collect();
        families.dedup();
        let counts: Vec<String> = families
            .iter()
            .map(|f| {
                let n = self.programs.iter().filter(|p| p.family == *f).count();
                format!("{f} {n}")
            })
            .collect();
        let _ = writeln!(
            out,
            "{} programs ({}): old {}, new {}",
            self.programs.len(),
            counts.join(", "),
            self.old_name,
            self.new_name
        );
        let changes = self.changes();
        let width = Change::ALL
            .iter()
            .map(|c| c.label().len())
            .max()
            .unwrap_or(0);
        out.push('\n');
        for (change, programs) in &changes {
            let _ = writeln!(out, "  {:<width$}  {}", change.label(), programs.len());
        }
        for (change, programs) in &changes {
            if programs.is_empty() || examples == 0 {
                continue;
            }
            let _ = writeln!(out, "\n{} ({}):", change.label(), programs.len());
            for program in programs.iter().take(examples) {
                let _ = writeln!(
                    out,
                    "\n  {}/{}  {}",
                    program.family,
                    program.name,
                    program.path.display()
                );
                let _ = writeln!(out, "    old: {}", describe(&program.old));
                let _ = writeln!(out, "    new: {}", describe(&program.new));
            }
        }
        let accepted = |outcome: &Outcome| matches!(outcome, Outcome::Accepted);
        let total_errors = |pick: fn(&Compared) -> &Outcome| -> usize {
            self.programs.iter().map(|p| pick(p).errors()).sum()
        };
        let _ = writeln!(
            out,
            "\naccepted: old {}, new {}; errors: old {}, new {}",
            self.programs.iter().filter(|p| accepted(&p.old)).count(),
            self.programs.iter().filter(|p| accepted(&p.new)).count(),
            total_errors(|p| &p.old),
            total_errors(|p| &p.new),
        );
        out
    }
}

/// One outcome on a few lines: each error at its line and column, without
/// the file's path.
fn describe(outcome: &Outcome) -> String {
    match outcome {
        Outcome::Accepted => "accepted".to_string(),
        Outcome::Refused(errors) => {
            let shown: Vec<String> = errors.iter().map(|e| short_error(e)).collect();
            format!(
                "{} error(s)\n      {}",
                errors.len(),
                shown.join("\n      ")
            )
        }
        Outcome::Crashed(how) => {
            let first = how.lines().next().unwrap_or_default();
            format!("crashed ({first})")
        }
    }
}

/// `path/x.bas:3:5: error: message` as `3:5: message`.
fn short_error(line: &str) -> String {
    match line.split_once(": error: ") {
        Some((at, message)) => {
            let place = at.rsplit_once(".bas:").map_or(at, |(_, place)| place);
            format!("{place}: {message}")
        }
        None => line.to_string(),
    }
}

#[cfg(test)]
mod tests;
