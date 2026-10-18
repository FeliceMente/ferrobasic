//! Every conformance test that parses must also type-check without
//! diagnostics, unless it is a negative test.

use std::fs;
use std::path::{Path, PathBuf};

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
fn conformance_corpus_type_checks() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/conformance");
    let mut files = Vec::new();
    bas_files(&root, &mut files);
    files.sort();
    let mut failures = Vec::new();
    for file in files.iter().filter(|f| !is_negative(f)) {
        let text = fs::read_to_string(file).unwrap();
        let parse = ferro_syntax::parse(&text);
        assert!(
            parse.errors().is_empty(),
            "{}: syntax errors",
            file.display()
        );
        let (_, diags) = ferro_hir::check(&parse.syntax());
        for d in diags {
            let line = text[..usize::from(d.range.start())].lines().count();
            failures.push(format!(
                "{}:{line}: {}",
                file.file_name().unwrap().to_string_lossy(),
                d.message
            ));
        }
    }
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}
