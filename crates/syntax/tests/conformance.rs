//! Parses every conformance test: the CST must print back byte-identical,
//! and every program FerroBASIC accepts must have no syntax errors. (Whether
//! a rejected program is rejected, by the parser or the type checker, is
//! checked in `crates/hir/tests/conformance.rs`.)

#[test]
fn conformance_corpus_parses() {
    let mut failures = Vec::new();
    for program in ferro_corpus::programs() {
        let text = program.source();
        let parse = ferro_syntax::parse(&text);
        if parse.syntax().to_string() != text {
            failures.push(format!("{}: CST does not round-trip", program.name));
        }
        if !program.rejected && !parse.errors().is_empty() {
            failures.push(format!("{}: {:?}", program.name, parse.errors()));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
