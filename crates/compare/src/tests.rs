use std::collections::HashSet;

use crate::families::FAMILIES;
use crate::{Change, Compared, Outcome, Report};

fn family(name: &str) -> &'static crate::families::Family {
    FAMILIES.iter().find(|f| f.name == name).unwrap()
}

#[test]
fn families_have_their_sizes_and_unique_names() {
    // The sizes of the comparisons run by hand for #67 and #68.
    for (name, size) in [("const", 65_160), ("sub-named-loops", 1_687)] {
        let programs = (family(name).generate)();
        assert_eq!(programs.len(), size, "{name}");
        let names: HashSet<&str> = programs.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names.len(), size, "{name}: names are unique");
    }
}

#[test]
fn every_generated_program_parses() {
    // Every 20th program of the big family is enough to meet every
    // declaration, use and variant, and keeps this test quick.
    for family in FAMILIES {
        let step = if family.name == "const" { 20 } else { 1 };
        for program in (family.generate)().into_iter().step_by(step) {
            let parse = ferro_syntax::parse(&program.source);
            assert!(
                parse.errors().is_empty(),
                "{}/{}: {:?}\n{}",
                family.name,
                program.name,
                parse.errors(),
                program.source
            );
        }
    }
}

#[test]
fn generated_programs_are_the_expected_shapes() {
    let programs = (family("const").generate)();
    let find = |name: &str| programs.iter().find(|p| p.name == name).unwrap();
    assert!(
        find("Ci_Di__log")
            .source
            .starts_with("Sub Process_Globals\n")
    );
    assert!(find("first__Ci_Di__log").source.starts_with("Sub AppStart"));
    assert!(find("otherfirst__Ci__asg").source.starts_with("Sub Other"));
    assert!(find("midasg__Ci_Di__log").source.contains(
        "    Private Const Size As Int = 10\n    Size = 4\n    Private Size As Int = 3\n"
    ));
    assert!(
        find("inif__Ci_Di__log")
            .source
            .contains("    If True Then\n        Private Size As Int = 3\n    End If\n")
    );
    let loops = (family("sub-named-loops").generate)();
    let nested = loops.iter().find(|p| p.name == "TC_nest_sub").unwrap();
    assert!(nested.source.contains(
        "    For Each Bump As Int In Array As Int(1)\n        For Bump = 1 To 2\n        Next\n    Next\n"
    ));
}

#[test]
fn outcomes_follow_the_exit_codes() {
    assert_eq!(Outcome::from_check(Some(0), ""), Outcome::Accepted);
    let stderr = "a.bas:3:5: error: one\n    x = 1\n    ^\na.bas:4:1: error: two\n2 error(s)\n";
    assert_eq!(
        Outcome::from_check(Some(3), stderr),
        Outcome::Refused(vec![
            "a.bas:3:5: error: one".into(),
            "a.bas:4:1: error: two".into()
        ])
    );
    assert!(matches!(
        Outcome::from_check(Some(101), "panicked"),
        Outcome::Crashed(_)
    ));
    assert!(matches!(Outcome::from_check(None, ""), Outcome::Crashed(_)));
}

#[test]
fn changes_between_outcomes() {
    let refused =
        |errors: &[&str]| Outcome::Refused(errors.iter().map(|e| e.to_string()).collect());
    let crashed = Outcome::Crashed("exit code 101".into());
    let cases = [
        (Outcome::Accepted, Outcome::Accepted, None),
        (refused(&["a"]), refused(&["a"]), None),
        (
            refused(&["a"]),
            Outcome::Accepted,
            Some(Change::NewlyAccepted),
        ),
        (
            Outcome::Accepted,
            refused(&["a"]),
            Some(Change::NewlyRefused),
        ),
        (
            refused(&["a"]),
            refused(&["a", "b"]),
            Some(Change::MoreErrors),
        ),
        (
            refused(&["a", "b"]),
            refused(&["a"]),
            Some(Change::FewerErrors),
        ),
        (
            refused(&["a"]),
            refused(&["b"]),
            Some(Change::OtherMessages),
        ),
        (Outcome::Accepted, crashed.clone(), Some(Change::NewCrash)),
        (crashed.clone(), crashed.clone(), Some(Change::BothCrash)),
        (crashed.clone(), Outcome::Accepted, Some(Change::OldCrash)),
    ];
    for (old, new, change) in cases {
        assert_eq!(Change::between(&old, &new), change, "{old:?} -> {new:?}");
    }
}

#[test]
fn the_report_counts_and_shows_the_smallest_example() {
    let program = |name: &str, len, old, new| Compared {
        family: "const",
        name: name.to_string(),
        path: format!("work/{name}.bas").into(),
        source_len: len,
        old,
        new,
    };
    let refused = Outcome::Refused(vec!["work/x.bas:2:19: error: open".into()]);
    let report = Report {
        old_name: "origin/dev (abc)".into(),
        new_name: "target/debug/ferro".into(),
        programs: vec![
            program("big", 200, refused.clone(), Outcome::Accepted),
            program("small", 100, refused.clone(), Outcome::Accepted),
            program("same", 50, Outcome::Accepted, Outcome::Accepted),
        ],
    };
    let text = report.render(1);
    assert!(
        text.starts_with("3 programs (const 3): old origin/dev (abc), new target/debug/ferro\n")
    );
    let count = |label: &str| {
        text.lines()
            .find(|line| line.trim_start().starts_with(label))
            .and_then(|line| line.split_whitespace().last())
            .map(str::to_string)
    };
    assert_eq!(count("newly accepted").as_deref(), Some("2"), "{text}");
    assert_eq!(count("newly refused").as_deref(), Some("0"), "{text}");
    assert!(
        text.contains("\nnewly accepted (2):\n\n  const/small  work/small.bas\n"),
        "{text}"
    );
    assert!(!text.contains("const/big"), "{text}");
    assert!(
        text.contains("    old: 1 error(s)\n      2:19: open\n    new: accepted\n"),
        "{text}"
    );
    assert!(
        text.ends_with("accepted: old 1, new 3; errors: old 2, new 0\n"),
        "{text}"
    );
    assert_eq!(report.new_crashes(), 0);
}

/// A fake `ferro`: a shell script running `body`.
#[cfg(unix)]
fn fake_ferro(dir: &std::path::Path, body: &str) -> std::path::PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let path = dir.join("ferro");
    std::fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    path
}

#[cfg(unix)]
#[test]
fn a_check_that_hangs_times_out_as_a_crash() {
    let dir = std::env::temp_dir().join(format!("ferro-compare-test-hang-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let ferro = fake_ferro(&dir, "sleep 30");
    let started = std::time::Instant::now();
    let outcome = crate::check(
        &ferro,
        &dir.join("x.bas"),
        std::time::Duration::from_secs(1),
        &dir.join("stderr"),
    );
    assert!(started.elapsed() < std::time::Duration::from_secs(10));
    assert_eq!(outcome, Outcome::Crashed("timed out after 1 s".into()));
    std::fs::remove_dir_all(&dir).unwrap();
}

#[cfg(unix)]
#[test]
fn a_check_reads_the_errors_through_its_stderr_file() {
    let dir =
        std::env::temp_dir().join(format!("ferro-compare-test-errors-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let ferro = fake_ferro(
        &dir,
        "echo 'x.bas:1:1: error: one' >&2\necho '    caret' >&2\nexit 3",
    );
    let outcome = crate::check(
        &ferro,
        &dir.join("x.bas"),
        crate::DEFAULT_TIMEOUT,
        &dir.join("stderr"),
    );
    assert_eq!(
        outcome,
        Outcome::Refused(vec!["x.bas:1:1: error: one".into()])
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn the_lock_keeps_a_second_run_out_until_released() {
    let dir = std::env::temp_dir().join(format!("ferro-compare-test-lock-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let first = crate::Lock::take(&dir).unwrap();
    let refused = crate::Lock::take(&dir).err().unwrap();
    assert!(refused.contains("another ferro-compare"), "{refused}");
    assert!(
        refused.contains(&std::process::id().to_string()),
        "{refused}"
    );
    drop(first);
    assert!(!dir.join("lock").exists());
    drop(crate::Lock::take(&dir).unwrap());
    std::fs::remove_dir_all(&dir).unwrap();
}

#[cfg(unix)]
#[test]
fn a_lock_left_by_a_stopped_run_is_taken_over() {
    let dir = std::env::temp_dir().join(format!("ferro-compare-test-stale-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    // The id of a process that has ended, as after Ctrl-C.
    let mut ended = std::process::Command::new("true").spawn().unwrap();
    let pid = ended.id();
    ended.wait().unwrap();
    std::fs::write(dir.join("lock"), pid.to_string()).unwrap();
    let lock = crate::Lock::take(&dir).unwrap();
    let holder = std::fs::read_to_string(dir.join("lock")).unwrap();
    assert_eq!(holder, std::process::id().to_string());
    drop(lock);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn profile_folders_map_to_cargo_arguments() {
    use crate::profile_args;
    assert_eq!(profile_args("debug", true), Some(vec![]));
    assert_eq!(
        profile_args("release", true),
        Some(vec!["--release".to_string()])
    );
    assert_eq!(
        profile_args("fast", true),
        Some(vec!["--profile".to_string(), "fast".to_string()])
    );
    // ~/.cargo/bin after `cargo install`: not a profile folder.
    assert_eq!(profile_args("bin", false), None);
}
