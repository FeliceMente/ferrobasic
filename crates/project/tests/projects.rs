//! Reads the real-world corpus: every project, module and `.b4j.meta` file
//! must write back byte for byte, and every project must find the modules it
//! names, as the IDE did.
//!
//! The files are not in this repository: fetch them first with
//! `cargo run -p ferro-corpus --bin ferro-corpus-fetch`, then run
//! `cargo test -p ferro-project --test projects -- --ignored`. Without them
//! the test fails, naming the command.

use std::path::Path;

use ferro_project::{HeaderFile, MetaFile};

#[test]
#[ignore = "needs the fetched real-world corpus: cargo run -p ferro-corpus --bin ferro-corpus-fetch"]
fn real_world_projects_round_trip_and_find_their_modules() {
    let mut failures = Vec::new();
    for project in ferro_corpus::projects() {
        for file in &project.files {
            let bytes = project.read(file).unwrap_or_else(|e| panic!("{e}"));
            let name = format!("{}/{}", project.name, file.name);
            let Ok(text) = String::from_utf8(bytes) else {
                failures.push(format!("{name}: not UTF-8"));
                continue;
            };
            let lower = file.name.to_ascii_lowercase();
            let written = if lower.ends_with(".meta") {
                MetaFile::parse(&text).to_string()
            } else {
                match HeaderFile::parse(&text) {
                    Some(header) => header.to_string(),
                    None => {
                        failures.push(format!("{name}: no header"));
                        continue;
                    }
                }
            };
            if written != text {
                failures.push(format!("{name}: does not write back byte for byte"));
            }
        }
        for file in project.files.iter().filter(|f| {
            Path::new(&f.name)
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("b4j"))
        }) {
            let name = format!("{}/{}", project.name, file.name);
            let loaded = match ferro_project::load(&project.dir().join(&file.name)) {
                Ok(loaded) => loaded,
                Err(e) => {
                    failures.push(format!("{name}: {e}"));
                    continue;
                }
            };
            for problem in &loaded.problems {
                failures.push(format!("{name}: {}", problem.message));
            }
            if loaded.modules.len() != loaded.module_refs().len() {
                failures.push(format!("{name}: not every module was read"));
            }
            if loaded.app_type() != Some(project.app_type.as_str()) {
                failures.push(format!(
                    "{name}: AppType {:?}, corpus.toml says {}",
                    loaded.app_type(),
                    project.app_type
                ));
            }
            for (key, written, found) in loaded.count_mismatches() {
                failures.push(format!("{name}: {key}={written}, but {found} found"));
            }
            for module in &loaded.modules {
                if module.module_type().is_none() {
                    failures.push(format!("{name}: {} has no Type", module.path.display()));
                }
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
