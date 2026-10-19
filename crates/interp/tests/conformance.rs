//! End to end: every conformance test with an expected output is parsed,
//! checked, lowered and run, and its stdout must match byte for byte
//! (`.expected` if recorded, else `.provisional`), with the uncaught error
//! named in `.meta.toml` when there is one.

#[test]
fn conformance_programs_produce_their_expected_output() {
    let mut ran = 0;
    let mut failures = Vec::new();
    for program in ferro_corpus::programs() {
        let Some(expected) = program.expected_output() else {
            continue;
        };
        let name = &program.name;
        let parse = ferro_syntax::parse(&program.source());
        assert!(parse.errors().is_empty(), "{name}: syntax errors");
        let (module, diags) = ferro_hir::check(&parse.syntax());
        assert!(diags.is_empty(), "{name}: {diags:?}");
        let lowered = ferro_mir::lower(&module);
        let (mut stdout, mut stderr) = (Vec::new(), Vec::new());
        let outcome = ferro_interp::run(&lowered, &[], &mut stdout, &mut stderr);
        ran += 1;
        let stdout = String::from_utf8(stdout).unwrap();
        if stdout != expected {
            failures.push(format!(
                "{name}: stdout\n--- expected\n{expected}--- actual\n{stdout}"
            ));
        }
        let error = outcome.error.as_ref().map(|e| e.kind.as_str());
        let want = program.uncaught_error.as_deref();
        if error != want {
            failures.push(format!(
                "{name}: uncaught error {error:?}, expected {want:?}"
            ));
        }
        let want_code = if want.is_some() { 1 } else { 0 };
        if outcome.exit_code != want_code {
            failures.push(format!(
                "{name}: exit code {}, expected {want_code}",
                outcome.exit_code
            ));
        }
    }
    assert!(ran > 0, "no conformance test with an expected output");
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}
