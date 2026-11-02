//! `ferro-compare`: compares how two builds of `ferro` check generated
//! programs. Exit code 0 when the new build crashes on none, 1 when it
//! crashes on one, 2 on a usage error or when a build fails.

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use ferro_compare::families::{FAMILIES, Family};
use ferro_compare::{Compared, DEFAULT_EXAMPLES, Report, check_all, write_programs};

const USAGE: &str = "\
usage: ferro-compare [--old <git ref> | --old-ferro <path>] [--new-ferro <path>]
                     [--family <name>]... [--jobs <n>] [--examples <n>]
                     [--work <dir>] [--list]

  --old        the git ref to build the old ferro from (default: origin/dev)
  --old-ferro  an old ferro already built, instead of --old
  --new-ferro  the new ferro (default: ferro next to this program)
  --family     only this family of programs; repeat for several
               (default: every family, see --list)
  --jobs       how many programs are checked at once
               (default: the number of processors)
  --examples   how many programs of each change to show (default: 3)
  --work       where the programs and the old build go
               (default: target/compare)
  --list       list the families and stop";

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
    let me = std::env::current_exe().map_err(|e| format!("cannot find this program: {e}"))?;
    let work = match work {
        Some(work) => work,
        // target/debug/ferro-compare -> target/compare
        None => me
            .parent()
            .and_then(Path::parent)
            .map(|target| target.join("compare"))
            .ok_or("cannot find the target folder: pass --work")?,
    };
    std::fs::create_dir_all(&work).map_err(|e| format!("cannot create {}: {e}", work.display()))?;
    let new_ferro = new_ferro
        .unwrap_or_else(|| me.with_file_name(format!("ferro{}", std::env::consts::EXE_SUFFIX)));
    if !new_ferro.is_file() {
        return Err(format!(
            "{} not found: build it with `cargo build -p ferro-cli`, or pass --new-ferro",
            new_ferro.display()
        ));
    }
    let (old_ferro, old_name) = match (old_ferro, old) {
        (Some(_), Some(_)) => return Err("--old and --old-ferro are exclusive".into()),
        (Some(path), None) if path.is_file() => {
            let name = path.display().to_string();
            (path, name)
        }
        (Some(path), None) => return Err(format!("{} not found", path.display())),
        (None, reference) => {
            let reference = reference.unwrap_or_else(|| "origin/dev".to_string());
            build_ref(&reference, &work)?
        }
    };
    let written = write_programs(&families, &work.join("programs"))
        .map_err(|e| format!("cannot write the programs: {e}"))?;
    let files: Vec<PathBuf> = written.iter().map(|(_, _, path, _)| path.clone()).collect();
    eprintln!(
        "ferro-compare: checking {} programs with each build, {jobs} at a time",
        files.len()
    );
    let old_outcomes = check_all(&old_ferro, &files, jobs);
    let new_outcomes = check_all(&new_ferro, &files, jobs);
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

/// Builds `ferro` from a git ref in a worktree of its own, removed after,
/// and returns the binary, copied into `work`, with a name for the report.
fn build_ref(reference: &str, work: &Path) -> Result<(PathBuf, String), String> {
    let commit = git(&["rev-parse", "--verify", &format!("{reference}^{{commit}}")])?;
    let short = &commit[..commit.len().min(9)];
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

fn git(args: &[&str]) -> Result<String, String> {
    let output = Command::new("git")
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
