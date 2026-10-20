//! `ferro licenses` and `THIRD-PARTY-NOTICES.txt` (`docs/spec/cli.md`,
//! "Commands"; `tools/licenses/README.md`).

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

const FERRO: &str = env!("CARGO_BIN_EXE_ferro");

/// Packages in `Cargo.lock` that are never linked into `ferro`, so the
/// notices leave them out: build-script helpers, and optional dependencies
/// of features FerroBASIC does not enable. With versions, so that another
/// version of the same crate, such as zerocopy-derive 0.7, is still checked.
const NOT_LINKED: &[(&str, &str)] = &[
    ("autocfg", "1.5.1"),
    ("version_check", "0.9.5"),
    ("zerocopy-derive", "0.8.63"),
    ("zlib-rs", "0.6.8"),
];

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

/// The `name version` pairs of the crate list: the lines after
/// `Rust crates:`, up to the first blank one. `lines` also takes CRLF, which
/// a checkout with `core.autocrlf` produces.
fn listed(notices: &str) -> BTreeSet<(String, String)> {
    notices
        .lines()
        .skip_while(|line| *line != "Rust crates:")
        .skip(1)
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
        .filter(|(name, version)| !NOT_LINKED.contains(&(name.as_str(), version.as_str())))
        .collect();
    assert!(
        !listed.is_empty(),
        "no crate list in THIRD-PARTY-NOTICES.txt"
    );
    assert!(
        stale.is_empty() && missing.is_empty(),
        "THIRD-PARTY-NOTICES.txt is out of date: run tools/licenses/generate.sh \
         (tools/licenses/README.md), or list a package that is never linked in \
         NOT_LINKED\nlisted but not in Cargo.lock: {stale:?}\n\
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
