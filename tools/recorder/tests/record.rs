//! The whole recording flow on a small corpus, with a fake builder and a
//! fake Java instead of the original toolchain.
//!
//! The fake tools read instructions from the test's code, in comments:
//! `' OUT: text` prints a line in every locale, `' OUT-EN:` / `' OUT-IT:`
//! only in the reference locale / the machine's, `' THROW: kind` ends with
//! an uncaught error, `' HANG` never ends, and `' NO BUILD` fails the build.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;

use ferro_recorder::{Config, Run, Runner, Verdict, record, select};

struct Fake {
    builder: PathBuf,
}

impl Runner for Fake {
    fn run(&self, program: &Path, args: &[String], dir: &Path, _: Duration) -> io::Result<Run> {
        if program == self.builder {
            assert_eq!(args[0], "-Task=Build");
            let project = args.last().unwrap().strip_prefix("-Project=").unwrap();
            let text = fs::read_to_string(dir.join(project))?;
            if text.contains("' NO BUILD") {
                return Ok(Run {
                    code: Some(1),
                    stdout: b"Error compiling program.\r\n".to_vec(),
                    ..Run::default()
                });
            }
            fs::create_dir_all(dir.join("Objects"))?;
            let jar = project.replace(".b4j", ".jar");
            fs::write(dir.join("Objects").join(jar), text)?;
            return Ok(Run {
                code: Some(0),
                ..Run::default()
            });
        }
        if args.iter().any(|a| a == "-XshowSettings:properties") {
            return Ok(Run {
                code: Some(0),
                stderr: b"Property settings:\r\n    java.version = 19.0.2\r\n    java.vendor = Oracle Corporation\r\n    os.name = Windows 10\r\n    os.version = 10.0\r\n    user.country = IT\r\n    user.language = it\r\n".to_vec(),
                ..Run::default()
            });
        }
        assert!(args.contains(&"-Dstdout.encoding=UTF-8".to_string()));
        let forced = args.iter().any(|a| a == "-Duser.language.format=en");
        let jar = args.last().unwrap();
        let text = fs::read_to_string(jar)?;
        let mut run = Run {
            code: Some(0),
            ..Run::default()
        };
        for line in text.lines() {
            let line = line.trim();
            if let Some(out) = line.strip_prefix("' OUT: ") {
                run.stdout.extend(format!("{out}\r\n").bytes());
            } else if let Some(out) =
                line.strip_prefix(if forced { "' OUT-EN: " } else { "' OUT-IT: " })
            {
                run.stdout.extend(format!("{out}\r\n").bytes());
            } else if let Some(kind) = line.strip_prefix("' THROW: ") {
                run.stderr.extend(
                    format!(
                        "main._appstart (java line: 9)\r\n{kind}: boom\r\n\tat x.y(Z.java:1)\r\n"
                    )
                    .bytes(),
                );
                run.code = Some(1);
            } else if let Some(n) = line.strip_prefix("' TRACE ") {
                for _ in 0..n.parse::<usize>().unwrap() {
                    run.stderr.extend(
                        b"	at java.util.ArrayList.equals(ArrayList.java:529)
",
                    );
                }
            } else if line == "' HANG" {
                return Ok(Run {
                    timed_out: true,
                    ..Run::default()
                });
            }
        }
        Ok(run)
    }
}

const PENDING: &str =
    "status = \"pending-recording\"   # pending-recording | recorded | deviation | negative
level = 0

# Hand-derived output, used until a recording exists. See docs/compatibility-testing.md.
[provisional]
basis = \"source\"

# Filled in by tools/recorder when the expected output is recorded:
# [recording]
# product = \"B4J\"
# version = \"\"
";

/// A fresh copy of a tiny repository: its root.
fn repo(name: &str, tests: &[(&str, &str, &str, Option<&str>)]) -> PathBuf {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("recorder")
        .join(name);
    if root.exists() {
        fs::remove_dir_all(&root).unwrap();
    }
    let level0 = root.join("tests/conformance/level0");
    fs::create_dir_all(&level0).unwrap();
    fs::create_dir_all(root.join("docs")).unwrap();
    fs::write(
        root.join("docs/deviations.md"),
        "| ID |\n| --- |\n| D9 | x |\n",
    )
    .unwrap();
    for (test, code, meta, provisional) in tests {
        fs::write(level0.join(format!("{test}.bas")), code).unwrap();
        fs::write(level0.join(format!("{test}.meta.toml")), meta).unwrap();
        if let Some(p) = provisional {
            fs::write(level0.join(format!("{test}.provisional")), p).unwrap();
        }
    }
    root
}

fn config(root: &Path) -> Config {
    Config {
        corpus: root.join("tests/conformance"),
        out: root.join("out"),
        builder: root.join("Builder.exe"),
        java: root.join("java.exe"),
        version: "10.30".into(),
        only: Vec::new(),
        rerecord: false,
        keep: false,
        dry_run: false,
        base_folder: true,
        build_timeout: Duration::from_secs(5),
        run_timeout: Duration::from_secs(5),
        date: "2026-10-25".into(),
    }
}

fn fake(root: &Path) -> Fake {
    Fake {
        builder: root.join("Builder.exe"),
    }
}

fn read(path: PathBuf) -> String {
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// Every file under `dir`, with its contents.
fn snapshot(dir: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    let mut out = Vec::new();
    for e in fs::read_dir(dir).unwrap() {
        let path = e.unwrap().path();
        if path.is_dir() {
            out.extend(snapshot(&path));
        } else {
            out.push((path.clone(), fs::read(&path).unwrap()));
        }
    }
    out.sort();
    out
}

#[test]
fn records_every_kind_of_test() {
    let uncaught = PENDING.replace(
        "level = 0\n",
        "level = 0\nuncaught_error = \"java.lang.RuntimeException\"\n",
    );
    let root = repo(
        "every_kind",
        &[
            ("same", "' OUT: 1\n' OUT: 2\n", PENDING, Some("1\n2\n")),
            (
                "differs",
                "' OUT: 1\n' OUT: three\n",
                PENDING,
                Some("1\n3\n"),
            ),
            ("open", "' OUT-EN: 1.5\n' OUT-IT: 1,5\n", PENDING, None),
            (
                "throws",
                "' OUT: a\n' THROW: java.lang.RuntimeException\n",
                &uncaught,
                Some("a\n"),
            ),
            (
                "exits",
                "' THROW: java.lang.ArithmeticException\n",
                PENDING,
                Some(""),
            ),
            (
                "rejected",
                "' NO BUILD\n",
                "status = \"negative\"\nlevel = 0\n",
                None,
            ),
            (
                "accepted",
                "' OUT: x\n",
                "status = \"negative\"\nlevel = 0\n",
                None,
            ),
            (
                "deviation",
                "' OUT: d\n",
                "status = \"deviation\"\ndeviation = \"D9\"\nlevel = 0\n",
                None,
            ),
        ],
    );
    let before = snapshot(&root.join("tests"));
    let summary = record(&config(&root), &fake(&root)).unwrap();
    // The repository is never changed.
    assert_eq!(snapshot(&root.join("tests")), before);

    let verdict = |name: &str| {
        let e = summary
            .entries
            .iter()
            .find(|e| e.name == format!("level0/{name}"))
            .unwrap();
        (e.verdict, e.disagrees)
    };
    assert_eq!(verdict("same"), (Verdict::Recorded, false));
    assert_eq!(verdict("differs"), (Verdict::Recorded, true));
    assert_eq!(verdict("open"), (Verdict::Recorded, true));
    assert_eq!(verdict("throws"), (Verdict::Recorded, false));
    assert_eq!(verdict("exits"), (Verdict::Recorded, true));
    assert_eq!(verdict("rejected"), (Verdict::Rejected, false));
    assert_eq!(verdict("accepted"), (Verdict::NegativeAccepted, true));
    assert_eq!(verdict("deviation"), (Verdict::DeviationOutput, false));
    assert_eq!(summary.failures(), 1);

    let out = root.join("out");
    let conf = out.join("tests/conformance/level0");
    assert_eq!(read(conf.join("same.expected")), "1\n2\n");
    let meta = read(conf.join("same.meta.toml"));
    assert!(meta.starts_with("status = \"recorded\""));
    assert!(!meta.contains("provisional"));
    assert!(meta.contains("version = \"10.30\"\n"));
    assert!(meta.contains("java = \"19.0.2 (Oracle Corporation)\"\n"));
    assert!(meta.contains("platform = \"Windows 10 10.0\"\n"));
    assert!(meta.contains("machine_locale = \"it-IT\"\nmachine_locale_output = \"same\"\n"));
    // The output in the machine's locale is kept when it differs.
    assert_eq!(read(conf.join("open.expected")), "1.5\n");
    assert_eq!(read(conf.join("open.it-IT.expected")), "1,5\n");
    assert!(read(conf.join("open.meta.toml")).contains("machine_locale_output = \"differs\""));
    assert!(!conf.join("same.it-IT.expected").exists());
    // How a program ends goes in the metadata when it was wrong there.
    assert_eq!(read(conf.join("throws.expected")), "a\n");
    assert!(
        read(conf.join("throws.meta.toml"))
            .contains("uncaught_error = \"java.lang.RuntimeException\"")
    );
    assert!(
        read(conf.join("exits.meta.toml"))
            .contains("uncaught_error = \"java.lang.ArithmeticException\"")
    );
    assert!(read(conf.join("rejected.meta.toml")).contains("rejected = true"));
    assert!(!conf.join("accepted.meta.toml").exists());
    assert!(!conf.join("deviation.expected").exists());
    assert_eq!(
        read(out.join("deviation-outputs/level0/deviation.txt")),
        "d\n"
    );
    assert_eq!(
        read(out.join("remove.txt")),
        "tests/conformance/level0/differs.provisional\n\
         tests/conformance/level0/exits.provisional\n\
         tests/conformance/level0/same.provisional\n\
         tests/conformance/level0/throws.provisional\n"
    );
    let report = read(out.join("report.md"));
    assert!(report.contains("8 tests: 1 failed, 4 disagree"));
    assert!(report.contains("- 3\n+ three\n"));
    assert!(report.contains("In the machine's locale (it-IT) the output differs"));
    assert!(report.contains("**The metadata said the program ends with exit code 0; it ends with an uncaught `java.lang.ArithmeticException`.**"));
    assert!(report.contains("main._appstart (java line: 9)"));
    // The generated projects are gone.
    assert!(!out.join("work").exists());
}

#[test]
fn reports_builds_that_fail_or_hang() {
    let root = repo(
        "failures",
        &[
            ("broken", "' NO BUILD\n", PENDING, Some("x\n")),
            ("hangs", "' HANG\n", PENDING, Some("x\n")),
            (
                "overflows",
                "' THROW: java.lang.StackOverflowError\n' TRACE 100\n",
                PENDING,
                Some(""),
            ),
        ],
    );
    let mut config = config(&root);
    config.keep = true;
    let summary = record(&config, &fake(&root)).unwrap();
    let verdicts: Vec<_> = summary.entries.iter().map(|e| e.verdict).collect();
    assert_eq!(
        verdicts,
        [Verdict::BuildFailed, Verdict::TimedOut, Verdict::Recorded]
    );
    assert_eq!(summary.failures(), 2);
    let out = root.join("out");
    let report = read(out.join("report.md"));
    assert!(report.contains("Error compiling program."));
    assert!(report.contains("3 tests: 2 failed"));
    // A long trace is cut.
    assert_eq!(report.matches("ArrayList.equals").count(), 37);
    assert!(report.contains("… 63 more lines\n"));
    // --keep leaves the generated project, a console project with the code.
    let project = read(out.join("work/level0-broken/broken.b4j"));
    assert!(project.starts_with("AppType=StandardJava\n"));
    assert!(project.ends_with("#End Region\n\n' NO BUILD\n"));
}

#[test]
fn selects_tests_by_name() {
    let recorded = PENDING.replace("pending-recording\"", "recorded\"");
    let root = repo(
        "select",
        &[("a", "", PENDING, None), ("b", "", &recorded, None)],
    );
    let mut config = config(&root);
    let names =
        |c: &Config| -> Vec<String> { select(c).unwrap().into_iter().map(|p| p.name).collect() };
    assert_eq!(names(&config), ["level0/a"]);
    config.rerecord = true;
    assert_eq!(names(&config), ["level0/a", "level0/b"]);
    config.only = vec!["b".into(), "level0/a.bas".into(), "a".into()];
    assert_eq!(names(&config), ["level0/a", "level0/b"]);
    config.only = vec!["c".into()];
    assert_eq!(
        select(&config).unwrap_err(),
        "no conformance test is called c"
    );
    // A dry run writes nothing.
    config.only = Vec::new();
    config.dry_run = true;
    record(&config, &fake(&root)).unwrap();
    assert!(!root.join("out").exists());
}
