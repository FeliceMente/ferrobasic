//! Parses every source file of the real-world corpus: the CST must print
//! back byte-identical. Syntax errors are expected there, in constructs
//! beyond Level 0, so they are not checked.
//!
//! The files are not in this repository: fetch them first with
//! `cargo run -p ferro-corpus --bin ferro-corpus-fetch`, then run
//! `cargo test -p ferro-syntax --test projects -- --ignored`. Without them
//! the test fails, naming the command.

#[test]
#[ignore = "needs the fetched real-world corpus: cargo run -p ferro-corpus --bin ferro-corpus-fetch"]
fn real_world_corpus_round_trips() {
    let mut failures = Vec::new();
    for project in ferro_corpus::projects() {
        for file in project.sources() {
            let bytes = project.read(file).unwrap_or_else(|e| panic!("{e}"));
            let name = format!("{}/{}", project.name, file.name);
            let Ok(text) = String::from_utf8(bytes) else {
                failures.push(format!("{name}: not UTF-8"));
                continue;
            };
            if ferro_syntax::parse(&text).syntax().to_string() != text {
                failures.push(format!("{name}: CST does not round-trip"));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
