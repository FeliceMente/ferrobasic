//! `ferro licenses` and `THIRD-PARTY-NOTICES.txt` (`docs/spec/cli.md`,
//! "Commands"; `tools/licenses/README.md`).

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

const FERRO: &str = env!("CARGO_BIN_EXE_ferro");

/// Packages in `Cargo.lock` that are never linked into `ferro`, so the
/// notices leave them out: build-script helpers, and optional dependencies
/// of features FerroBASIC does not enable.
const NOT_LINKED: &[&str] = &["autocfg", "version_check", "zerocopy-derive", "zlib-rs"];

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(name: &str) -> String {
    fs::read_to_string(root().join(name)).unwrap()
}

#[test]
fn licenses_prints_the_licence_then_the_notices() {
    let out = Command::new(FERRO).arg("licenses").output().unwrap();
    assert_eq!(out.status.code(), Some(0));
    assert!(out.stderr.is_empty());
    let expected = format!("{}\n{}", read("LICENSE"), read("THIRD-PARTY-NOTICES.txt"));
    assert!(String::from_utf8(out.stdout).unwrap() == expected);

    let out = Command::new(FERRO)
        .args(["licenses", "x"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert!(out.stdout.is_empty());
}

/// The `name version` pairs of a crate list: indented lines up to the first
/// blank one.
fn listed(notices: &str) -> BTreeSet<(String, String)> {
    let list = notices.split_once("\nRust crates:\n").unwrap().1;
    list.lines()
        .take_while(|line| !line.is_empty())
        .map(|line| {
            let mut words = line.split_whitespace();
            let name = words.next().unwrap().to_string();
            (name, words.next().unwrap().to_string())
        })
        .collect()
}

/// The third-party packages in `Cargo.lock`.
fn locked() -> BTreeSet<(String, String)> {
    let lock = read("Cargo.lock");
    lock.split("[[package]]")
        .skip(1)
        .filter_map(|package| {
            let field = |key: &str| {
                package.lines().find_map(|line| {
                    let value = line.strip_prefix(key)?.strip_prefix(" = \"")?;
                    Some(value.strip_suffix('"')?.to_string())
                })
            };
            Some((field("name")?, field("version")?))
        })
        .filter(|(name, _)| !name.starts_with("ferro-"))
        .collect()
}

/// A dependency added, removed or updated without regenerating the notices
/// fails here.
#[test]
fn notices_match_cargo_lock() {
    let listed = listed(&read("THIRD-PARTY-NOTICES.txt"));
    let locked = locked();
    let stale: Vec<_> = listed.difference(&locked).collect();
    let missing: Vec<_> = locked
        .difference(&listed)
        .filter(|(name, _)| !NOT_LINKED.contains(&name.as_str()))
        .collect();
    assert!(
        stale.is_empty() && missing.is_empty(),
        "THIRD-PARTY-NOTICES.txt is out of date: run tools/licenses/generate.sh \
         (tools/licenses/README.md)\nlisted but not in Cargo.lock: {stale:?}\n\
         in Cargo.lock but not listed: {missing:?}"
    );
}

/// Each licence text keeps its copyright lines: `cargo-about` falls back to a
/// generic text with placeholders when it finds no licence file of its own.
#[test]
fn notices_have_no_placeholder_texts() {
    let notices = read("THIRD-PARTY-NOTICES.txt");
    for placeholder in ["<year>", "<copyright holders>", "&quot;", "&lt;", "&amp;"] {
        assert!(!notices.contains(placeholder), "{placeholder}");
    }
}
