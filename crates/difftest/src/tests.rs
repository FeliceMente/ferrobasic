use super::*;

/// A test whose `.bas` and outputs live in a fresh directory.
fn program(name: &str, status: Status, rejected: bool, expected: Option<(&str, &str)>) -> Program {
    let dir = std::env::temp_dir().join(format!("difftest-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("t.bas");
    fs::write(&path, "").unwrap();
    if let Some((ext, text)) = expected {
        fs::write(path.with_extension(ext), text).unwrap();
    }
    Program {
        path,
        name: name.into(),
        status,
        rejected,
        deviation: (status == Status::Deviation).then(|| "D1".into()),
        uncaught_error: None,
        exit_code: 0,
        level: Some(0),
        features: vec!["f".into()],
    }
}

fn ran(stdout: &str) -> Run {
    Run::Ran {
        stdout: stdout.into(),
        error: None,
        exit_code: 0,
    }
}

#[test]
fn judges_each_kind_of_test() {
    let recorded = program("rec", Status::Recorded, false, Some(("expected", "1\n")));
    assert_eq!(verdict(&recorded, ran("1\n")).state, State::Pass);
    assert!(!verdict(&recorded, ran("1\n")).provisional);
    assert_eq!(verdict(&recorded, ran("1\r\n")).state, State::Pass);
    let wrong = verdict(&recorded, ran("2\n"));
    assert_eq!(wrong.state, State::Fail);
    assert_eq!(
        wrong.detail,
        "stdout differs at line 1: expected \"1\", got \"2\""
    );
    assert_eq!(verdict(&recorded, Run::Rejected).state, State::Fail);
    let provisional = program(
        "prov",
        Status::PendingRecording,
        false,
        Some(("provisional", "")),
    );
    let v = verdict(&provisional, ran(""));
    assert_eq!((v.state, v.provisional), (State::Pass, true));

    let negative = program("neg", Status::Negative, true, None);
    assert_eq!(verdict(&negative, Run::Rejected).state, State::Pass);
    assert_eq!(verdict(&negative, ran("")).state, State::Fail);
    let pending = program("pend", Status::PendingRecording, true, None);
    assert_eq!(verdict(&pending, Run::Rejected).state, State::Pending);
    assert_eq!(verdict(&pending, ran("")).state, State::Fail);
    let open = program("open", Status::PendingRecording, false, None);
    assert_eq!(verdict(&open, ran("x\n")).state, State::Pending);
    assert_eq!(verdict(&open, Run::Rejected).state, State::Fail);
    let deviation = program("dev", Status::Deviation, false, None);
    assert_eq!(verdict(&deviation, ran("x\n")).state, State::Deviation);
    let no_output = program("none", Status::Recorded, false, None);
    assert_eq!(verdict(&no_output, ran("")).state, State::Fail);
    assert_eq!(
        verdict(&recorded, Run::Broken("boom".into())).detail,
        "boom"
    );
}

#[test]
fn compares_the_error_and_the_exit_code() {
    let mut p = program("err", Status::Recorded, false, Some(("expected", "a\n")));
    p.uncaught_error = Some("java.lang.ArithmeticException".into());
    p.exit_code = 1;
    let run = |error: Option<&str>, exit_code| Run::Ran {
        stdout: "a\n".into(),
        error: error.map(String::from),
        exit_code,
    };
    assert_eq!(
        verdict(&p, run(Some("java.lang.ArithmeticException"), 1)).state,
        State::Pass
    );
    assert_eq!(
        verdict(&p, run(None, 0)).detail,
        "uncaught error none, expected java.lang.ArithmeticException\nexit code 0, expected 1"
    );
}

#[test]
fn diffs_name_the_first_differing_line() {
    assert_eq!(
        output_diff("a\nb\n", "a\n"),
        "stdout differs at line 2: expected \"b\", got end of output (2 lines expected, 1 printed)\n"
    );
    assert_eq!(
        output_diff("a\n", "a"),
        "stdout differs at line 2: expected end of output, got end of output (1 lines expected, 1 printed)\n"
    );
}

#[test]
fn reads_the_uncaught_error_kind() {
    let stderr = "Error occurred on line: 7 (Main)\njava.lang.ArithmeticException: / by zero\n";
    assert_eq!(
        uncaught_error(stderr).as_deref(),
        Some("java.lang.ArithmeticException")
    );
    assert_eq!(uncaught_error("nothing here\n"), None);
}

#[test]
fn the_report_counts_by_state_and_group() {
    let entry = |name: &str, state, features: &[&str]| Entry {
        name: name.into(),
        level: Some(0),
        features: features.iter().map(|f| f.to_string()).collect(),
        verdict: Verdict::new(state, if state == State::Fail { "why" } else { "" }),
    };
    let report = Report {
        target: "interp",
        entries: vec![
            entry("a", State::Pass, &["for"]),
            entry("b", State::Fail, &["for", "map"]),
            entry("c", State::Pending, &["map"]),
        ],
    };
    assert_eq!(report.failed(), 1);
    let text = report.render(false, true);
    assert!(text.starts_with("FAIL b\n    why\n\n"), "{text}");
    assert!(text.contains("| level 0 | 1 | 1 | 0 | 1 |"), "{text}");
    assert!(text.contains("| for | 1 | 1 | 0 | 0 |"), "{text}");
    assert!(text.contains("| map | 0 | 1 | 0 | 1 |"), "{text}");
    assert!(text.ends_with("interp: 3 tests, 1 pass (0 against a provisional output), 1 fail, 0 deviation, 1 pending\n"), "{text}");
    assert!(report.render(true, false).contains("PENDING c\n"));
}

/// The whole corpus on the interpreter: nothing fails.
#[test]
fn the_corpus_passes_on_the_interpreter() {
    let report = Report::run(&ferro_corpus::programs(), &Target::Interp);
    assert_eq!(report.failed(), 0, "\n{}", report.render(false, false));
    assert!(
        report
            .entries
            .iter()
            .any(|e| e.verdict.state == State::Pass)
    );
}
