//! End to end: every conformance test with an expected output is parsed,
//! checked, lowered and run, and its stdout must match byte for byte
//! (`.expected` if recorded, else `.provisional`), with the uncaught error
//! named in `.meta.toml` when there is one.

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

/// The string value of `key = "value"` in a `.meta.toml`, if present.
fn meta_value(meta: &str, key: &str) -> Option<String> {
    meta.lines().find_map(|line| {
        let line = line.split('#').next().unwrap().trim();
        let (k, v) = line.split_once('=')?;
        (k.trim() == key).then(|| v.trim().trim_matches('"').to_string())
    })
}

#[test]
fn conformance_programs_produce_their_expected_output() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/conformance");
    let mut files = Vec::new();
    bas_files(&root, &mut files);
    files.sort();
    let mut ran = 0;
    let mut failures = Vec::new();
    for file in &files {
        let meta = fs::read_to_string(file.with_extension("meta.toml")).unwrap_or_default();
        if meta_value(&meta, "status").as_deref() == Some("negative") {
            continue;
        }
        // A recording always wins over a provisional output.
        let expected = fs::read_to_string(file.with_extension("expected"))
            .or_else(|_| fs::read_to_string(file.with_extension("provisional")));
        let Ok(expected) = expected else { continue };
        let name = file.file_stem().unwrap().to_string_lossy().into_owned();
        let text = fs::read_to_string(file).unwrap();
        let parse = ferro_syntax::parse(&text);
        assert!(parse.errors().is_empty(), "{name}: syntax errors");
        let (module, diags) = ferro_hir::check(&parse.syntax());
        assert!(diags.is_empty(), "{name}: {diags:?}");
        let program = ferro_mir::lower(&module);
        let (mut stdout, mut stderr) = (Vec::new(), Vec::new());
        let outcome = ferro_interp::run(&program, &[], &mut stdout, &mut stderr);
        ran += 1;
        let stdout = String::from_utf8(stdout).unwrap();
        if stdout != expected {
            failures.push(format!(
                "{name}: stdout\n--- expected\n{expected}--- actual\n{stdout}"
            ));
        }
        let error = outcome.error.as_ref().map(|e| e.kind.as_str());
        let want = meta_value(&meta, "uncaught_error");
        if error != want.as_deref() {
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
