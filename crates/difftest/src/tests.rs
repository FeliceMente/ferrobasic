use super::*;

/// A test whose `.bas` and outputs live in a fresh directory, removed when
/// the test ends.
struct Fixture {
    program: Program,
    dir: PathBuf,
}

impl std::ops::Deref for Fixture {
    type Target = Program;
    fn deref(&self) -> &Program {
        &self.program
    }
}

impl std::ops::DerefMut for Fixture {
    fn deref_mut(&mut self) -> &mut Program {
        &mut self.program
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

fn program(name: &str, status: Status, rejected: bool, expected: Option<(&str, &str)>) -> Fixture {
    let dir = std::env::temp_dir().join(format!("difftest-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("t.bas");
    fs::write(&path, "").unwrap();
    if let Some((ext, text)) = expected {
        fs::write(path.with_extension(ext), text).unwrap();
    }
    let program = Program {
        path,
        name: name.into(),
        status,
        rejected,
        deviation: (status == Status::Deviation).then(|| "D1".into()),
        uncaught_error: None,
        exit_code: 0,
        level: Some(0),
        features: vec!["f".into()],
    };
    Fixture { program, dir }
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
    // Only the recording's line endings are normalized: what FerroBASIC
    // prints is compared as it is.
    assert_eq!(verdict(&recorded, ran("1\r\n")).state, State::Fail);
    let crlf = program("crlf", Status::Recorded, false, Some(("expected", "1\r\n")));
    assert_eq!(verdict(&crlf, ran("1\n")).state, State::Pass);
    let wrong = verdict(&recorded, ran("2\n"));
    assert_eq!(wrong.state, State::Fail);
    assert_eq!(
        wrong.detail,
        "stdout differs at line 1: expected \"1\", got \"2\""
    );
    assert_eq!(
        verdict(&recorded, Run::Rejected("why".into())).state,
        State::Fail
    );
    let provisional = program(
        "prov",
        Status::PendingRecording,
        false,
        Some(("provisional", "")),
    );
    let v = verdict(&provisional, ran(""));
    assert_eq!((v.state, v.provisional), (State::Pass, true));

    let negative = program("neg", Status::Negative, true, None);
    assert_eq!(
        verdict(&negative, Run::Rejected("why".into())).state,
        State::Pass
    );
    assert_eq!(verdict(&negative, ran("")).state, State::Fail);
    let pending = program("pend", Status::PendingRecording, true, None);
    assert_eq!(
        verdict(&pending, Run::Rejected("why".into())).state,
        State::Pending
    );
    assert_eq!(verdict(&pending, ran("")).state, State::Fail);
    let open = program("open", Status::PendingRecording, false, None);
    assert_eq!(verdict(&open, ran("x\n")).state, State::Pending);
    assert_eq!(
        verdict(&open, Run::Rejected("why".into())).state,
        State::Fail
    );
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
fn a_recorded_test_needs_its_recording() {
    // A `.provisional` next to it does not stand in for the recording.
    let p = program(
        "recprov",
        Status::Recorded,
        false,
        Some(("provisional", "1\n")),
    );
    assert_eq!(
        verdict(&p, ran("1\n")).detail,
        "recorded, but has no .expected output"
    );
}

#[test]
fn a_refused_program_says_why() {
    let p = program("must", Status::Recorded, false, Some(("expected", "")));
    assert_eq!(
        verdict(&p, Run::Rejected("`x` is not declared".into())).detail,
        "rejected, but must build:\n`x` is not declared"
    );
    assert_eq!(
        first_error("t.bas:3:5: error: `x` is not declared\n      x\n1 error(s)\n"),
        "`x` is not declared"
    );
}

/// What `ferro run` does with a program, in process.
fn interpreted(body: &str) -> Run {
    run_text(&format!(
        "Sub Process_Globals\nEnd Sub\n\nSub AppStart (Args() As String)\n{body}\nEnd Sub\n"
    ))
}

fn run_text(source: &str) -> Run {
    interpret(source, &Arc::new(AtomicBool::new(false)))
}

#[test]
fn interprets_as_ferro_run_does() {
    // The module header is checked as `ferro run` checks it.
    let class = format!(
        "Type=Class\nVersion=10\n@EndOfDesignText@\n{}",
        "Sub Class_Globals\nEnd Sub\n"
    );
    assert!(matches!(run_text(&class), Run::Rejected(why) if why.contains("class module")));
    assert!(matches!(interpreted("Log(Missing)"), Run::Rejected(why) if why.contains("`Missing`")));
    // The exit code is the process's: its low 8 bits.
    assert!(matches!(
        interpreted("ExitApplication2(256 + 7)"),
        Run::Ran { exit_code: 7, .. }
    ));
}

#[test]
fn a_program_that_does_not_end_fails() {
    let p = program("loop", Status::Recorded, false, Some(("expected", "")));
    fs::write(
        &p.path,
        "Sub Process_Globals\nEnd Sub\n\nSub AppStart (Args() As String)\n    \
         Do While 1 = 1\n    Loop\nEnd Sub\n",
    )
    .unwrap();
    // The thread left behind spins until the test process ends; its
    // output, if any, is discarded.
    let v = judge(&p, &Target::Interp, Duration::from_millis(100));
    assert_eq!(
        (v.state, v.detail.as_str()),
        (State::Fail, "still running after 100ms: stopped")
    );
}

#[test]
fn a_timeout_too_long_for_a_deadline_is_none() {
    let p = program("forever", Status::Recorded, false, Some(("expected", "")));
    fs::write(
        &p.path,
        "Sub Process_Globals\nEnd Sub\n\nSub AppStart (Args() As String)\nEnd Sub\n",
    )
    .unwrap();
    assert_eq!(judge(&p, &Target::Interp, Duration::MAX).state, State::Pass);
    let out = output(Command::new(env!("CARGO")).arg("--version"), Duration::MAX);
    assert_eq!(out.unwrap().unwrap().code, Some(0));
}

#[test]
fn a_pending_test_with_a_recording_is_flagged() {
    let p = program(
        "stale",
        Status::PendingRecording,
        false,
        Some(("expected", "1\n")),
    );
    assert_eq!(
        verdict(&p, ran("1\n")).detail,
        "has a recording: set its status to \"recorded\" in its .meta.toml"
    );
}

#[test]
fn carriage_returns_show_in_the_diff() {
    assert_eq!(
        output_diff("1\n", "1\r\n"),
        "stdout differs at line 1: expected \"1\", got \"1\\r\"\n"
    );
}

#[test]
fn a_crash_is_not_a_program_ending() {
    let finished = |code, stderr: &str| Finished {
        code,
        stdout: Vec::new(),
        stderr: stderr.as_bytes().to_vec(),
        overflowed: false,
    };
    assert!(crashed(&finished(None, "")));
    // Windows: a stack overflow (0xC00000FD) ends the process with an
    // exception's code.
    assert!(crashed(&finished(Some(0xC00000FDu32 as i32), "")));
    // A program that only prints the words is not a panic.
    assert!(!crashed(&finished(Some(0), "the test panicked at noon\n")));
    assert!(crashed(&finished(
        Some(101),
        "thread 'main' panicked at src/x.rs:1:1:\nboom\n"
    )));
    // `ExitApplication2(101)` is a program's ending, not a crash.
    assert!(!crashed(&finished(Some(101), "")));
    assert!(!crashed(&finished(
        Some(1),
        "Error occurred on line: 5 (Main)\njava.lang.RuntimeException: x\n"
    )));
    assert_eq!(tail("1\n2\n3\n4\n5\n6\n7\n"), "3\n4\n5\n6\n7");
}

#[test]
fn a_missing_final_line_break_is_named() {
    assert_eq!(
        output_diff("a\n", "a"),
        "stdout differs at its end: a missing final line break\n"
    );
    assert_eq!(
        output_diff("a", "a\n"),
        "stdout differs at its end: an extra final line break\n"
    );
}

#[test]
fn output_past_the_limit_is_dropped() {
    let stop = Arc::new(AtomicBool::new(false));
    let mut out = Capture::new(&stop);
    std::io::Write::write_all(&mut out, &vec![b'x'; OUTPUT_LIMIT]).unwrap();
    assert!(!out.overflowed);
    std::io::Write::write_all(&mut out, b"y").unwrap();
    assert!(out.overflowed);
    assert_eq!(out.bytes.len(), OUTPUT_LIMIT);
}

#[test]
fn a_test_that_cannot_be_read_fails_alone() {
    let mut missing = program("missing", Status::Recorded, false, Some(("expected", "")));
    missing.path = missing.dir.join("nowhere.bas");
    let fine = program("fine", Status::Recorded, false, Some(("expected", "")));
    fs::write(
        &fine.path,
        "Sub Process_Globals\nEnd Sub\n\nSub AppStart (Args() As String)\nEnd Sub\n",
    )
    .unwrap();
    let programs = [missing.program.clone(), fine.program.clone()];
    let report = Report::run(&programs, &Target::Interp, DEFAULT_TIMEOUT);
    let states: Vec<_> = report.entries.iter().map(|e| e.verdict.state).collect();
    assert_eq!(states, [State::Fail, State::Pass]);
    assert!(
        report.entries[0]
            .verdict
            .detail
            .starts_with("could not be judged: "),
        "{}",
        report.entries[0].verdict.detail
    );
}

#[test]
fn output_must_be_utf8() {
    assert_eq!(text(b"ok\n".to_vec()).ok().as_deref(), Some("ok\n"));
    assert!(
        matches!(text(vec![0xff]), Err(Run::Broken(why)) if why == "stdout is not valid UTF-8")
    );
}

#[test]
fn output_left_behind_is_discarded() {
    let stop = Arc::new(AtomicBool::new(false));
    let mut out = Capture::new(&stop);
    std::io::Write::write_all(&mut out, b"kept").unwrap();
    stop.store(true, Ordering::Relaxed);
    std::io::Write::write_all(&mut out, b"dropped").unwrap();
    assert_eq!(out.bytes, b"kept");
}

#[test]
fn every_test_that_runs_has_its_error_and_exit_code_checked() {
    for status in [Status::Deviation, Status::PendingRecording] {
        let mut p = program(&format!("{status:?}"), status, false, None);
        p.uncaught_error = Some("java.lang.RuntimeException".into());
        p.exit_code = 1;
        let v = verdict(&p, ran("x\n"));
        assert_eq!(v.state, State::Fail, "{status:?}");
        assert!(
            v.detail
                .starts_with("uncaught error none, expected java.lang.RuntimeException"),
            "{status:?}: {}",
            v.detail
        );
        let crashed = Run::Ran {
            stdout: String::new(),
            error: Some("java.lang.RuntimeException".into()),
            exit_code: 1,
        };
        assert_ne!(verdict(&p, crashed).state, State::Fail, "{status:?}");
    }
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
    let report = Report::run(&ferro_corpus::programs(), &Target::Interp, DEFAULT_TIMEOUT);
    assert_eq!(report.failed(), 0, "\n{}", report.render(false, false));
    assert!(
        report
            .entries
            .iter()
            .any(|e| e.verdict.state == State::Pass)
    );
}
