//! `ferro-difftest --target standalone` end to end: with this `ferro`, every
//! conformance test built by `ferro bundle` gives what the corpus expects.

use ferro_difftest::{DEFAULT_TIMEOUT, Report, State, Target};

#[test]
fn the_corpus_passes_as_standalone_binaries() {
    let work = std::path::Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("difftest-{}", std::process::id()));
    let target = Target::Standalone {
        ferro: env!("CARGO_BIN_EXE_ferro").into(),
        work: work.clone(),
    };
    let programs = ferro_corpus::programs();
    let report = Report::run(&programs, &target, DEFAULT_TIMEOUT);
    let _ = std::fs::remove_dir_all(&work);
    assert_eq!(report.failed(), 0, "\n{}", report.render(false, false));
    // Each kind of test went through the binary: built and run, or refused.
    // Pending ones only while the corpus has some: a recording run may
    // answer them all.
    let ran = |state| report.entries.iter().any(|e| e.verdict.state == state);
    for state in [State::Pass, State::Deviation] {
        assert!(ran(state), "{state:?}");
    }
    if programs
        .iter()
        .any(|p| p.status == ferro_corpus::Status::PendingRecording)
    {
        assert!(ran(State::Pending), "Pending");
    }
}

/// A program that must build but is refused fails with `ferro bundle`'s
/// first diagnostic.
#[test]
fn a_refused_program_reports_the_diagnostic() {
    let work = std::path::Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("difftest-refused-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&work);
    std::fs::create_dir_all(&work).unwrap();
    let path = work.join("t.bas");
    std::fs::write(
        &path,
        "Sub Process_Globals\nEnd Sub\n\nSub AppStart (Args() As String)\n    Log(Missing)\nEnd Sub\n",
    )
    .unwrap();
    std::fs::write(path.with_extension("expected"), "").unwrap();
    let program = ferro_corpus::Program {
        path,
        name: "refused".into(),
        status: ferro_corpus::Status::Recorded,
        rejected: false,
        deviation: None,
        uncaught_error: None,
        exit_code: 0,
        level: Some(0),
        features: Vec::new(),
    };
    let target = Target::Standalone {
        ferro: env!("CARGO_BIN_EXE_ferro").into(),
        work: work.join("build"),
    };
    let verdict = ferro_difftest::judge(&program, &target, DEFAULT_TIMEOUT);
    let _ = std::fs::remove_dir_all(&work);
    assert_eq!(verdict.state, State::Fail);
    assert_eq!(
        verdict.detail,
        "rejected, but must build:\n`Missing` is not declared"
    );
}

/// A `--work` given as a relative path: the binary still runs from inside
/// its own folder.
#[test]
fn a_relative_work_folder_works() {
    let work = std::path::PathBuf::from(format!("difftest-relative-{}", std::process::id()));
    let target = Target::Standalone {
        ferro: env!("CARGO_BIN_EXE_ferro").into(),
        work: work.clone(),
    };
    let programs: Vec<_> = ferro_corpus::programs()
        .into_iter()
        .filter(|p| p.name == "level0/prices_map")
        .collect();
    let report = Report::run(&programs, &target, DEFAULT_TIMEOUT);
    let _ = std::fs::remove_dir_all(&work);
    assert_eq!(report.failed(), 0, "\n{}", report.render(false, false));
}
