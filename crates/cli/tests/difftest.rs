//! `ferro-difftest --target standalone` end to end: with this `ferro`, every
//! conformance test built by `ferro bundle` gives what the corpus expects.

use ferro_difftest::{Report, State, Target};

#[test]
fn the_corpus_passes_as_standalone_binaries() {
    let work = std::path::Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("difftest-{}", std::process::id()));
    let target = Target::Standalone {
        ferro: env!("CARGO_BIN_EXE_ferro").into(),
        work: work.clone(),
    };
    let report = Report::run(&ferro_corpus::programs(), &target);
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
