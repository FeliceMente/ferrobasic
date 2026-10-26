//! `ferro-corpus-fetch`: downloads the real-world corpus that
//! `tests/corpus/corpus.toml` references into `target/corpus`, checking each
//! file's SHA-256. Files already there and right are kept. Exit code 0 when
//! every file is there, 1 when one could not be fetched.

use std::process::ExitCode;

fn main() -> ExitCode {
    let (mut fetched, mut kept, mut failed) = (0, 0, 0);
    for project in ferro_corpus::projects() {
        for file in &project.files {
            match project.fetch(file) {
                Ok(true) => fetched += 1,
                Ok(false) => kept += 1,
                Err(e) => {
                    eprintln!("ferro-corpus-fetch: {}: {e}", project.name);
                    failed += 1;
                }
            }
        }
    }
    let root = ferro_corpus::fetched_root();
    println!(
        "{fetched} fetched, {kept} already there, {failed} failed: {}",
        root.display()
    );
    if failed == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
