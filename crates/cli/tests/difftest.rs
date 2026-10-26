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
    let report = Report::run(&ferro_corpus::programs(), &target, DEFAULT_TIMEOUT);
    let _ = std::fs::remove_dir_all(&work);
    assert_eq!(report.failed(), 0, "\n{}", report.render(false, false));
    // Each kind of test went through the binary: built and run, or refused.
    for state in [State::Pass, State::Deviation, State::Pending] {
        assert!(
            report.entries.iter().any(|e| e.verdict.state == state),
            "{state:?}"
        );
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
