//! `ferro-recorder`: see `tools/recorder/README.md`.

use std::path::PathBuf;
use std::process::ExitCode;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use ferro_recorder::{Config, Processes, record};

const USAGE: &str = "\
Records the expected output of conformance tests on the original toolchain.

Usage: ferro-recorder --out <folder> --toolchain-version <version> [options]

  --repo <folder>              a copy of the FerroBASIC repository (default: .)
  --out <folder>               where the recordings and the report go
  --toolchain <folder>         the original toolchain's install folder
                               (default: C:\\Program Files\\Anywhere Software\\B4J)
  --builder <file>             its command-line builder
                               (default: B4JBuilder.exe in --toolchain)
  --java <file>                the java the toolchain uses
                               (default: C:\\Java\\jdk-19.0.2\\bin\\java.exe)
  --toolchain-version <text>   its version, as Help > About shows it
  --only <test>                record this test only (level0/x, or x);
                               can be repeated
  --rerecord                   record tests that already have a recording too
  --keep                       keep each test's generated project (in <out>/work)
  --no-base-folder             do not pass -BaseFolder to the builder
  --build-timeout <seconds>    default 300
  --run-timeout <seconds>      default 30
  --dry-run                    only list the tests it would record
  --help                       show this help
";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match parse(&args) {
        Ok(None) => {
            print!("{USAGE}");
            ExitCode::SUCCESS
        }
        Ok(Some(config)) => match record(&config, &Processes) {
            Ok(summary) => {
                if config.dry_run {
                    return ExitCode::SUCCESS;
                }
                let failures = summary.failures();
                let disagree = summary.entries.iter().filter(|e| e.disagrees).count();
                println!(
                    "\n{} recorded into {}: {failures} failed, {disagree} disagree with the repository.",
                    ferro_recorder::tests(summary.entries.len()),
                    config.out.display()
                );
                println!("See {}", config.out.join("report.md").display());
                if failures > 0 {
                    ExitCode::FAILURE
                } else {
                    ExitCode::SUCCESS
                }
            }
            Err(e) => {
                eprintln!("error: {e}");
                ExitCode::FAILURE
            }
        },
        Err(e) => {
            eprintln!("error: {e}\n\n{USAGE}");
            ExitCode::from(2)
        }
    }
}

/// The configuration, or `None` for `--help`.
fn parse(args: &[String]) -> Result<Option<Config>, String> {
    let mut repo = PathBuf::from(".");
    let mut out = None;
    let mut toolchain = PathBuf::from(r"C:\Program Files\Anywhere Software\B4J");
    let mut builder = None;
    let mut java = PathBuf::from(r"C:\Java\jdk-19.0.2\bin\java.exe");
    let mut version = None;
    let mut only = Vec::new();
    let (mut rerecord, mut keep, mut dry_run, mut base_folder) = (false, false, false, true);
    let mut build_timeout = Duration::from_secs(300);
    let mut run_timeout = Duration::from_secs(30);
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        let mut value = || {
            it.next()
                .cloned()
                .ok_or_else(|| format!("{arg} needs a value"))
        };
        let seconds = |v: String| {
            v.parse::<u64>()
                .ok()
                .filter(|&s| s > 0)
                .map(Duration::from_secs)
                .ok_or_else(|| format!("{arg} needs a number of seconds, not {v}"))
        };
        match arg.as_str() {
            "--help" | "-h" => return Ok(None),
            "--repo" => repo = value()?.into(),
            "--out" => out = Some(PathBuf::from(value()?)),
            "--toolchain" => toolchain = value()?.into(),
            "--builder" => builder = Some(PathBuf::from(value()?)),
            "--java" => java = value()?.into(),
            "--toolchain-version" => version = Some(value()?),
            "--only" => only.push(value()?),
            "--rerecord" => rerecord = true,
            "--keep" => keep = true,
            "--dry-run" => dry_run = true,
            "--no-base-folder" => base_folder = false,
            "--build-timeout" => build_timeout = seconds(value()?)?,
            "--run-timeout" => run_timeout = seconds(value()?)?,
            other => return Err(format!("unknown option {other}")),
        }
    }
    let corpus = repo.join("tests").join("conformance");
    if !corpus.is_dir() || !repo.join("docs").join("deviations.md").is_file() {
        return Err(format!(
            "{} is not a copy of the FerroBASIC repository (no tests/conformance or docs/deviations.md)",
            repo.display()
        ));
    }
    let out = match out {
        Some(out) => out,
        None if dry_run => PathBuf::from("recordings"),
        None => return Err("--out is required".into()),
    };
    let version = match version {
        Some(v) if !v.trim().is_empty() => v.trim().to_string(),
        _ if dry_run => String::new(),
        _ => return Err("--toolchain-version is required (Help > About in the IDE)".into()),
    };
    let builder = builder.unwrap_or_else(|| toolchain.join("B4JBuilder.exe"));
    if !dry_run {
        for (what, path) in [("builder", &builder), ("java", &java)] {
            if !path.is_file() {
                return Err(format!("the {what} {} does not exist", path.display()));
            }
        }
    }
    // Absolute, as the builder runs in each test's folder.
    let absolute = |p: PathBuf| std::path::absolute(&p).unwrap_or(p);
    Ok(Some(Config {
        corpus: absolute(corpus),
        out: absolute(out),
        builder: absolute(builder),
        java: absolute(java),
        version,
        only,
        rerecord,
        keep,
        dry_run,
        base_folder,
        build_timeout,
        run_timeout,
        date: today(),
    }))
}

/// Today's UTC date, `YYYY-MM-DD` (days to civil date, Howard Hinnant's
/// algorithm).
fn today() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    civil(secs / 86_400)
}

fn civil(days: u64) -> String {
    let z = days as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn computes_dates() {
        assert_eq!(civil(0), "1970-01-01");
        assert_eq!(civil(19_723), "2024-01-01");
        assert_eq!(civil(19_782), "2024-02-29");
        assert_eq!(civil(20_751), "2026-10-25");
    }

    #[test]
    fn needs_a_version_and_an_output_folder() {
        let repo = env!("CARGO_MANIFEST_DIR").to_string() + "/../..";
        let args = |a: &[&str]| -> Vec<String> {
            ["--repo", repo.as_str()]
                .iter()
                .chain(a)
                .map(|s| s.to_string())
                .collect()
        };
        assert!(
            parse(&args(&["--toolchain-version", "10"]))
                .unwrap_err()
                .contains("--out")
        );
        assert!(
            parse(&args(&["--out", "x"]))
                .unwrap_err()
                .contains("--toolchain-version")
        );
        assert!(
            parse(&args(&["--out", "x", "--toolchain-version", "10"]))
                .unwrap_err()
                .contains("does not exist")
        );
        assert!(
            parse(&args(&["--bogus"]))
                .unwrap_err()
                .contains("unknown option")
        );
        assert!(
            parse(&args(&["--run-timeout", "0"]))
                .unwrap_err()
                .contains("seconds")
        );
        assert!(parse(&["--help".to_string()]).unwrap().is_none());
        let config = parse(&args(&["--dry-run", "--only", "x"]))
            .unwrap()
            .unwrap();
        assert!(config.dry_run && config.base_folder);
        assert!(config.builder.ends_with("B4JBuilder.exe"));
        assert!(
            parse(&["--repo".into(), "/nonexistent".into()])
                .unwrap_err()
                .contains("not a copy")
        );
    }
}
