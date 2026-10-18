//! Parses every conformance test: the CST must print back byte-identical,
//! non-negative tests must have no syntax errors, and negative tests must
//! have some (when the rejection is syntactic; see `SEMANTIC_NEGATIVES`).

use std::fs;
use std::path::{Path, PathBuf};

/// Negative tests rejected by later stages, not by the parser.
const SEMANTIC_NEGATIVES: &[&str] = &[];

fn bas_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            bas_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "bas") {
            out.push(path);
        }
    }
}

fn is_negative(bas: &Path) -> bool {
    let meta = fs::read_to_string(bas.with_extension("meta.toml")).unwrap_or_default();
    meta.lines()
        .any(|line| line.split('#').next().unwrap().replace(' ', "") == "status=\"negative\"")
}

#[test]
fn conformance_corpus_parses() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/conformance");
    let mut files = Vec::new();
    bas_files(&root, &mut files);
    files.sort();
    assert!(
        !files.is_empty(),
        "no conformance tests found in {}",
        root.display()
    );

    let mut failures = Vec::new();
    for file in &files {
        let text = fs::read_to_string(file).unwrap();
        let parse = ferro_syntax::parse(&text);
        let name = file.file_stem().unwrap().to_string_lossy().into_owned();
        if parse.syntax().to_string() != text {
            failures.push(format!("{name}: CST does not round-trip"));
        }
        let has_errors = !parse.errors().is_empty();
        if is_negative(file) {
            if !has_errors && !SEMANTIC_NEGATIVES.contains(&name.as_str()) {
                failures.push(format!("{name}: negative test parsed without errors"));
            }
        } else if has_errors {
            failures.push(format!("{name}: {:?}", parse.errors()));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
