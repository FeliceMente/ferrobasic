//! `ferro-compare`: compares how two builds of `ferro` check generated
//! programs. Exit code 0 when the new build alone crashes on none, 1 when
//! it crashes on one, 2 on a usage error or when a build fails.

use std::fs::OpenOptions;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::time::Duration;

use ferro_compare::families::{FAMILIES, Family};
use ferro_compare::{
    Compared, DEFAULT_EXAMPLES, DEFAULT_TIMEOUT, Report, check_all, write_programs,
};

const USAGE: &str = "\
usage: ferro-compare [--old <git ref> | --old-ferro <path>] [--new-ferro <path>]
                     [--family <name>]... [--jobs <n>] [--timeout <seconds>]
                     [--examples <n>] [--work <dir>] [--list]

  --old        the git ref to build the old ferro from (default: origin/dev,
               fetched first)
  --old-ferro  an old ferro already built, instead of --old
  --new-ferro  the new ferro, instead of building it from this checkout
  --family     only this family of programs; repeat for several
               (default: every family, see --list)
  --jobs       how many programs are checked at once
               (default: the number of processors)
  --timeout    how long one check may run before it counts as a crash
               (default: 30)
  --examples   how many programs of each change to show (default: 3)
  --work       where the programs and the old build go
               (default: target/compare)
  --list       list the families and stop";

/// The checkout this program was built from: git and cargo run there,
/// wherever it is run from.
const WORKSPACE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");

fn main() -> ExitCode {
    match run(std::env::args().skip(1).collect()) {
        Ok(code) => code,
        Err(message) => {
            eprintln!("ferro-compare: {message}\n\n{USAGE}");
            ExitCode::from(2)
        }
    }
}

fn run(args: Vec<String>) -> Result<ExitCode, String> {
    let (mut old, mut old_ferro, mut new_ferro, mut work) = (None, None, None, None);
    let mut family_names = Vec::new();
    let mut jobs = std::thread::available_parallelism().map_or(1, |n| n.get());
    let mut timeout = DEFAULT_TIMEOUT;
    let mut examples = DEFAULT_EXAMPLES;
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        let mut value = |name: &str| args.next().ok_or(format!("{name} needs a value"));
        match arg.as_str() {
            "--old" => old = Some(value("--old")?),
            "--old-ferro" => old_ferro = Some(PathBuf::from(value("--old-ferro")?)),
            "--new-ferro" => new_ferro = Some(PathBuf::from(value("--new-ferro")?)),
            "--family" => family_names.push(value("--family")?),
            "--jobs" => jobs = number("--jobs", &value("--jobs")?, 1)?,
            "--timeout" => {
                timeout = Duration::from_secs(number("--timeout", &value("--timeout")?, 1)? as u64)
            }
            "--examples" => examples = number("--examples", &value("--examples")?, 0)?,
            "--work" => work = Some(PathBuf::from(value("--work")?)),
            "--list" => {
                for family in FAMILIES {
                    println!(
                        "{}: {} programs\n  {}",
                        family.name,
                        (family.generate)().len(),
                        family.about
                    );
                }
                return Ok(ExitCode::SUCCESS);
            }
            "-h" | "--help" => {
                println!("{USAGE}");
                return Ok(ExitCode::SUCCESS);
            }
            other => return Err(format!("unknown argument {other:?}")),
        }
    }
    let families = pick_families(&family_names)?;
    if old.is_some() && old_ferro.is_some() {
        return Err("--old and --old-ferro are exclusive".into());
    }
    let me = std::env::current_exe().map_err(|e| format!("cannot find this program: {e}"))?;
    // target/<profile>/ferro-compare
    let profile_dir = me.parent().ok_or("cannot find this program's folder")?;
    let target = profile_dir
        .parent()
        .ok_or("cannot find the target folder: pass --work")?;
    let work = work.unwrap_or_else(|| target.join("compare"));
    std::fs::create_dir_all(&work).map_err(|e| format!("cannot create {}: {e}", work.display()))?;
    let _lock = Lock::take(&work)?;

    let new_ferro = match new_ferro {
        Some(path) if path.is_file() => path,
        Some(path) => return Err(format!("{} not found", path.display())),
        None => build_new(profile_dir, target)?,
    };
    let (old_ferro, old_name) = match (old_ferro, old) {
        (Some(path), _) if path.is_file() => {
            let name = path.display().to_string();
            (path, name)
        }
        (Some(path), _) => return Err(format!("{} not found", path.display())),
        (None, Some(reference)) => build_ref(&reference, &work)?,
        (None, None) => {
            if let Err(e) = git(&["fetch", "-q", "origin", "dev"]) {
                eprintln!("ferro-compare: warning: {e}; comparing with origin/dev as it is");
            }
            build_ref("origin/dev", &work)?
        }
    };

    let written = write_programs(&families, &work.join("programs"))
        .map_err(|e| format!("cannot write the programs: {e}"))?;
    let files: Vec<PathBuf> = written.iter().map(|(_, _, path, _)| path.clone()).collect();
    eprintln!(
        "ferro-compare: checking {} programs with each build, {jobs} at a time",
        files.len()
    );
    let old_outcomes = check_all(&old_ferro, &files, jobs, timeout, &work);
    let new_outcomes = check_all(&new_ferro, &files, jobs, timeout, &work);
    let programs = written
        .into_iter()
        .zip(old_outcomes.into_iter().zip(new_outcomes))
        .map(|((family, name, path, source_len), (old, new))| Compared {
            family,
            name,
            path,
            source_len,
            old,
            new,
        })
        .collect();
    let report = Report {
        old_name,
        new_name: new_ferro.display().to_string(),
        programs,
    };
    print!("{}", report.render(examples));
    Ok(if report.new_crashes() == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

fn number(name: &str, text: &str, min: usize) -> Result<usize, String> {
    match text.parse::<usize>() {
        Ok(n) if n >= min => Ok(n),
        _ => Err(format!(
            "{name} must be a number of at least {min}, not {text:?}"
        )),
    }
}

fn pick_families(names: &[String]) -> Result<Vec<&'static Family>, String> {
    if names.is_empty() {
        return Ok(FAMILIES.iter().collect());
    }
    names
        .iter()
        .map(|name| {
            FAMILIES.iter().find(|f| f.name == name).ok_or_else(|| {
                let known: Vec<&str> = FAMILIES.iter().map(|f| f.name).collect();
                format!("unknown family {name:?}: {}", known.join(", "))
            })
        })
        .collect()
}

/// Only one run at a time uses a work folder: two would remove each
/// other's worktree and rewrite each other's programs.
struct Lock(PathBuf);

impl Lock {
    fn take(work: &Path) -> Result<Lock, String> {
        let path = work.join("lock");
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(_) => Ok(Lock(path)),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => Err(format!(
                "another ferro-compare is using {}: wait for it, or pass another --work \
                 (if none is running, remove {})",
                work.display(),
                path.display()
            )),
            Err(e) => Err(format!("cannot create {}: {e}", path.display())),
        }
    }
}

impl Drop for Lock {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

/// Builds `ferro` from this checkout, as it is, into the target folder this
/// program is in, so that a stale `ferro` is never compared.
fn build_new(profile_dir: &Path, target: &Path) -> Result<PathBuf, String> {
    let mut args = vec!["build", "-q", "-p", "ferro-cli"];
    let profile = profile_dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("debug");
    match profile {
        "debug" => {}
        "release" => args.push("--release"),
        other => args.extend(["--profile", other]),
    }
    let workspace = Path::new(WORKSPACE);
    let shown = workspace
        .canonicalize()
        .unwrap_or_else(|_| workspace.to_path_buf());
    eprintln!("ferro-compare: building ferro from {}", shown.display());
    let status = Command::new("cargo")
        .args(&args)
        .arg("--target-dir")
        .arg(target)
        .current_dir(WORKSPACE)
        .status()
        .map_err(|e| format!("cannot run cargo: {e}"))?;
    if !status.success() {
        return Err("building ferro from this checkout failed".into());
    }
    Ok(profile_dir.join(format!("ferro{}", std::env::consts::EXE_SUFFIX)))
}

/// Builds `ferro` from a git ref in a worktree of its own, removed after,
/// and returns the binary, copied into `work`, with a name for the report.
/// Warns when the checkout does not contain the ref: what changed there
/// since then would show as the checkout's own changes.
fn build_ref(reference: &str, work: &Path) -> Result<(PathBuf, String), String> {
    let commit = git(&["rev-parse", "--verify", &format!("{reference}^{{commit}}")])?;
    let short = &commit[..commit.len().min(9)];
    if git(&["merge-base", "--is-ancestor", &commit, "HEAD"]).is_err() {
        eprintln!(
            "ferro-compare: warning: this checkout does not contain {reference} ({short}): \
             what changed there since shows as this checkout's changes"
        );
    }
    let source = work.join("old-source");
    remove_worktree(&source);
    git(&[
        "worktree",
        "add",
        "--detach",
        &source.display().to_string(),
        &commit,
    ])?;
    eprintln!("ferro-compare: building ferro at {reference} ({short})");
    // Its own target folder, kept between runs, so later builds are quick.
    let target = work.join("old-target");
    let status = Command::new("cargo")
        .args(["build", "-q", "-p", "ferro-cli", "--target-dir"])
        .arg(&target)
        .current_dir(&source)
        .status();
    remove_worktree(&source);
    match status {
        Ok(status) if status.success() => {}
        Ok(_) => return Err(format!("building ferro at {reference} failed")),
        Err(e) => return Err(format!("cannot run cargo: {e}")),
    }
    let exe = format!("ferro{}", std::env::consts::EXE_SUFFIX);
    let built = target.join("debug").join(&exe);
    let kept = work.join(format!("ferro-{short}{}", std::env::consts::EXE_SUFFIX));
    std::fs::copy(&built, &kept).map_err(|e| format!("cannot copy {}: {e}", built.display()))?;
    Ok((kept, format!("{reference} ({short})")))
}

fn remove_worktree(source: &Path) {
    if source.exists() {
        let _ = git(&[
            "worktree",
            "remove",
            "--force",
            &source.display().to_string(),
        ]);
        let _ = std::fs::remove_dir_all(source);
    }
    let _ = git(&["worktree", "prune"]);
}

/// Runs git in this program's checkout.
fn git(args: &[&str]) -> Result<String, String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(WORKSPACE)
        .args(args)
        .output()
        .map_err(|e| format!("cannot run git: {e}"))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err(format!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}
