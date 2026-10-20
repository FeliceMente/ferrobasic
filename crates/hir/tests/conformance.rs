//! The front end on every conformance test: a program FerroBASIC accepts
//! must parse and type-check without diagnostics, and one it rejects (a
//! negative test, or `ferro_rejects = true`) must get at least one error.

#[test]
fn conformance_corpus_type_checks() {
    let mut failures = Vec::new();
    for program in ferro_corpus::programs() {
        let text = program.source();
        let parse = ferro_syntax::parse(&text);
        let mut errors: Vec<String> = parse.errors().iter().map(|e| e.message.clone()).collect();
        if errors.is_empty() {
            let (_, diags) = ferro_hir::check(&parse.syntax());
            errors.extend(diags.into_iter().map(|d| {
                let line = text[..usize::from(d.range.start())].lines().count();
                format!("line {line}: {}", d.message)
            }));
        }
        match (program.rejected, errors.is_empty()) {
            (false, false) => failures.push(format!("{}: {}", program.name, errors.join("; "))),
            (true, true) => {
                failures.push(format!("{}: accepted, but must be rejected", program.name))
            }
            _ => {}
        }
    }
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}
