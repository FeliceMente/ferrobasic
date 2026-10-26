//! `ferro-difftest`: runs the conformance corpus on a target and prints the
//! compatibility report. Exit code 0 when no test fails, 1 when one does,
//! 2 on a usage error.

use std::path::PathBuf;
use std::process::ExitCode;

use ferro_difftest::{Report, Target};

const USAGE: &str = "\
usage: ferro-difftest [--target interp|standalone] [--ferro <path>] [--work <dir>]
                      [--filter <text>] [--verbose] [--by-feature]

  --target      where the programs run (default: interp)
  --ferro       the ferro binary that builds standalone programs
                (default: ferro next to this program)
  --work        where standalone programs are built (default: a folder
                next to ferro, emptied before and after)
  --filter      only the tests whose name contains <text>
  --verbose     also list the deviation and pending tests
  --by-feature  also count the results by feature";

fn main() -> ExitCode {
    match run(std::env::args().skip(1).collect()) {
        Ok(code) => code,
        Err(message) => {
            eprintln!("ferro-difftest: {message}\n\n{USAGE}");
            ExitCode::from(2)
        }
    }
}

fn run(args: Vec<String>) -> Result<ExitCode, String> {
    let mut target = "interp".to_string();
    let (mut ferro, mut work, mut filter) = (None, None, None);
    let (mut verbose, mut by_feature) = (false, false);
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        let mut value = |name: &str| args.next().ok_or(format!("{name} needs a value"));
        match arg.as_str() {
            "--target" => target = value("--target")?,
            "--ferro" => ferro = Some(PathBuf::from(value("--ferro")?)),
            "--work" => work = Some(PathBuf::from(value("--work")?)),
            "--filter" => filter = Some(value("--filter")?),
            "--verbose" => verbose = true,
            "--by-feature" => by_feature = true,
            "-h" | "--help" => {
                println!("{USAGE}");
                return Ok(ExitCode::SUCCESS);
            }
            other => return Err(format!("unknown argument {other:?}")),
        }
    }
    let target = match target.as_str() {
        "interp" => Target::Interp,
        "standalone" => {
            let ferro = match ferro {
                Some(path) => path,
                None => default_ferro()?,
            };
            if !ferro.is_file() {
                return Err(format!(
                    "{} not found: build it with `cargo build -p ferro-cli`, or pass --ferro",
                    ferro.display()
                ));
            }
            let work = work.unwrap_or_else(|| ferro.with_file_name("difftest-work"));
            Target::Standalone { ferro, work }
        }
        other => return Err(format!("unknown target {other:?}: interp or standalone")),
    };
    let mut programs = ferro_corpus::programs();
    if let Some(text) = &filter {
        programs.retain(|p| p.name.contains(text.as_str()));
        if programs.is_empty() {
            return Err(format!("no test name contains {text:?}"));
        }
    }
    let report = Report::run(&programs, &target);
    if let Target::Standalone { work, .. } = &target {
        let _ = std::fs::remove_dir_all(work);
    }
    print!("{}", report.render(verbose, by_feature));
    Ok(if report.failed() == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

/// `ferro` next to this program: both are in Cargo's target directory.
fn default_ferro() -> Result<PathBuf, String> {
    let me = std::env::current_exe().map_err(|e| format!("cannot find this program: {e}"))?;
    Ok(me.with_file_name(format!("ferro{}", std::env::consts::EXE_SUFFIX)))
}
