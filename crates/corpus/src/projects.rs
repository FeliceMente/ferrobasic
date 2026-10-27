//! The real-world corpus: the projects `tests/corpus/corpus.toml`
//! references. Their files are not in this repository; `fetch` downloads
//! them into [`fetched_root`], and every read checks their SHA-256 against
//! the one recorded, so a test never runs on a file other than the one
//! referenced.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::{sha256, string, value};

/// The licences a project may be referenced under: SPDX identifiers of
/// licences that allow redistribution.
const LICENSES: &[&str] = &[
    "0BSD",
    "Apache-2.0",
    "BSD-2-Clause",
    "BSD-3-Clause",
    "GPL-2.0-only",
    "GPL-2.0-or-later",
    "GPL-3.0-only",
    "GPL-3.0-or-later",
    "ISC",
    "LGPL-2.1-only",
    "LGPL-2.1-or-later",
    "LGPL-3.0-only",
    "LGPL-3.0-or-later",
    "MIT",
    "MPL-2.0",
    "Unlicense",
];

/// The products a project file may be for.
const PRODUCTS: &[&str] = &["b4a", "b4i", "b4j", "b4r"];

/// One project, as its `[[project]]` table describes it. Only
/// [`projects`] makes one, so every field has passed its checks.
#[derive(Debug, Clone)]
pub struct Project {
    /// Its folder in `target/corpus`.
    pub name: String,
    /// Where it was published: in `repository`, at `commit`.
    pub source: String,
    /// `https://github.com/<owner>/<repo>`.
    pub repository: String,
    pub commit: String,
    /// Its folder in the repository, `""` for the root.
    pub path: String,
    /// An SPDX identifier, from [`LICENSES`].
    pub license: String,
    /// The licence's text: in `repository`, at `commit`.
    pub license_url: String,
    /// `b4j`, `b4a`, `b4i` or `b4r`.
    pub product: String,
    /// `AppType` in its project file, as `StandardJava`.
    pub app_type: String,
    pub files: Vec<ProjectFile>,
    /// `<owner>/<repo>`, from `repository`.
    slug: String,
}

/// One file of a project, from a `[[project.file]]` table.
#[derive(Debug, Clone)]
pub struct ProjectFile {
    /// Its name in the project's folder.
    pub name: String,
    /// Its SHA-256, in lower-case hexadecimal.
    pub sha256: String,
}

impl Project {
    /// Where `fetch` puts it.
    pub fn dir(&self) -> PathBuf {
        fetched_root().join(&self.name)
    }

    /// Its project (`.b4j`) and module (`.bas`) files.
    pub fn sources(&self) -> impl Iterator<Item = &ProjectFile> {
        self.files.iter().filter(|f| {
            Path::new(&f.name)
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("b4j") || e.eq_ignore_ascii_case("bas"))
        })
    }

    /// The URL `file` is downloaded from, at the commit referenced.
    pub fn url(&self, file: &ProjectFile) -> String {
        let folder = if self.path.is_empty() {
            String::new()
        } else {
            format!("{}/", self.path)
        };
        // Names hold only letters, digits, `.`, `-`, `_` and spaces.
        format!(
            "https://raw.githubusercontent.com/{}/{}/{folder}{}",
            self.slug, self.commit, file.name
        )
        .replace(' ', "%20")
    }

    /// The fetched `file`. An error when it was not fetched or is not the
    /// file referenced.
    pub fn read(&self, file: &ProjectFile) -> Result<Vec<u8>, String> {
        let path = self.dir().join(&file.name);
        let bytes = fs::read(&path).map_err(|e| {
            format!(
                "{}: {e}: fetch the real-world corpus first, with \
                 `cargo run -p ferro-corpus --bin ferro-corpus-fetch`",
                path.display()
            )
        })?;
        check(&path.display(), &bytes, file)?;
        Ok(bytes)
    }

    /// Downloads `file` with `curl`, unless the right one is already
    /// there. True when it was downloaded.
    pub fn fetch(&self, file: &ProjectFile) -> Result<bool, String> {
        let path = self.dir().join(&file.name);
        if fs::read(&path).is_ok_and(|bytes| sha256::hex(&bytes) == file.sha256) {
            return Ok(false);
        }
        fs::create_dir_all(self.dir()).map_err(|e| format!("{}: {e}", self.dir().display()))?;
        let part = self.dir().join(format!("{}.part", file.name));
        let url = self.url(file);
        let status = Command::new("curl")
            .args(["--fail", "--silent", "--show-error", "--location"])
            .args(["--retry", "3", "--output"])
            .arg(&part)
            .arg(&url)
            .status()
            .map_err(|e| format!("cannot run curl: {e}"))?;
        if !status.success() {
            let _ = fs::remove_file(&part);
            return Err(format!("{url}: curl failed ({status})"));
        }
        let bytes = fs::read(&part).map_err(|e| format!("{}: {e}", part.display()))?;
        if let Err(e) = check(&url, &bytes, file) {
            let _ = fs::remove_file(&part);
            return Err(e);
        }
        let _ = fs::remove_file(&path);
        fs::rename(&part, &path).map_err(|e| format!("{}: {e}", path.display()))?;
        Ok(true)
    }
}

fn check(at: &dyn std::fmt::Display, bytes: &[u8], file: &ProjectFile) -> Result<(), String> {
    let found = sha256::hex(bytes);
    if found == file.sha256 {
        Ok(())
    } else {
        Err(format!(
            "{at}: SHA-256 {found}, not the {} recorded in corpus.toml",
            file.sha256
        ))
    }
}

/// This repository's root.
fn workspace() -> &'static Path {
    let crates = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    crates.parent().unwrap()
}

/// Where the projects are fetched: `corpus` in Cargo's target directory,
/// `CARGO_TARGET_DIR` when it is set (a relative one is taken from this
/// repository's root), else `target` in this repository.
pub fn fetched_root() -> PathBuf {
    let target = match std::env::var_os("CARGO_TARGET_DIR") {
        Some(dir) if !dir.is_empty() => workspace().join(dir),
        _ => workspace().join("target"),
    };
    target.join("corpus")
}

/// Every project `tests/corpus/corpus.toml` references, in its order. Panics
/// on a table it cannot read.
pub fn projects() -> Vec<Project> {
    let list = workspace().join("tests/corpus/corpus.toml");
    let text = fs::read_to_string(&list).unwrap_or_else(|e| panic!("{}: {e}", list.display()));
    let projects = from_toml(&text).unwrap_or_else(|e| panic!("{}: {e}", list.display()));
    assert!(!projects.is_empty(), "no projects in {}", list.display());
    projects
}

/// A table's text, without its header.
enum Table {
    Project(String),
    File(String),
}

fn from_toml(text: &str) -> Result<Vec<Project>, String> {
    let mut tables = Vec::new();
    for line in text.lines() {
        match line.trim() {
            "[[project]]" => tables.push(Table::Project(String::new())),
            "[[project.file]]" => tables.push(Table::File(String::new())),
            other if other.starts_with('[') => return Err(format!("unknown table {other}")),
            _ => match tables.last_mut() {
                Some(Table::Project(t) | Table::File(t)) => {
                    t.push_str(line);
                    t.push('\n');
                }
                None if line.trim().is_empty() || line.trim().starts_with('#') => {}
                None => return Err(format!("a key outside a table: {line}")),
            },
        }
    }
    let mut projects: Vec<Project> = Vec::new();
    for table in tables {
        match table {
            Table::Project(t) => {
                let project = project(&t)?;
                if projects.iter().any(|p| p.name == project.name) {
                    return Err(format!("{} is listed twice", project.name));
                }
                projects.push(project);
            }
            Table::File(t) => {
                let project = projects
                    .last_mut()
                    .ok_or("a [[project.file]] before any [[project]]")?;
                let file = project_file(&t).map_err(|e| format!("{}: {e}", project.name))?;
                // Fetched onto file systems that may ignore case.
                if project
                    .files
                    .iter()
                    .any(|f| f.name.eq_ignore_ascii_case(&file.name))
                {
                    return Err(format!("{}: {} is listed twice", project.name, file.name));
                }
                project.files.push(file);
            }
        }
    }
    if let Some(p) = projects.iter().find(|p| p.sources().next().is_none()) {
        return Err(format!("{}: no .b4j or .bas file", p.name));
    }
    Ok(projects)
}

fn project(table: &str) -> Result<Project, String> {
    let get = |key: &str| string(table, key)?.ok_or(format!("a project has no {key}"));
    let name = get("name")?;
    let in_project = |e: String| format!("{name}: {e}");
    if name.is_empty()
        || !name
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
    {
        return Err(format!(
            "project name {name:?}: only a-z, 0-9 and - are allowed"
        ));
    }
    let repository = get("repository").map_err(in_project)?;
    let slug = repository
        .strip_prefix("https://github.com/")
        .filter(|slug| {
            let parts: Vec<&str> = slug.split('/').collect();
            parts.len() == 2
                && parts.iter().all(|p| {
                    !p.is_empty()
                        && !p.starts_with('.')
                        && p.bytes()
                            .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
                })
                && !parts[1].ends_with(".git")
        })
        .ok_or_else(|| {
            in_project(format!(
                "repository is not https://github.com/<owner>/<repo>: {repository}"
            ))
        })?
        .to_string();
    let commit = get("commit").map_err(in_project)?;
    if !is_hex(&commit, 40) {
        return Err(in_project(format!("commit is not a full SHA-1: {commit}")));
    }
    // Both point into the repository, at the commit.
    let blob = format!("{repository}/blob/{commit}/");
    let tree = format!("{repository}/tree/{commit}");
    let source = get("source").map_err(in_project)?;
    let in_tree = source
        .strip_prefix(&tree)
        .is_some_and(|rest| rest.is_empty() || rest.starts_with('/'));
    if !source.starts_with(&blob) && !in_tree {
        return Err(in_project(format!(
            "source is not in {repository} at {commit}: {source}"
        )));
    }
    let license_url = get("license_url").map_err(in_project)?;
    if !license_url.starts_with(&blob) {
        return Err(in_project(format!(
            "license_url is not a file of {repository} at {commit}: {license_url}"
        )));
    }
    let license = get("license").map_err(in_project)?;
    if !LICENSES.contains(&license.as_str()) {
        return Err(in_project(format!(
            "license {license:?} is not one of the SPDX identifiers {LICENSES:?}"
        )));
    }
    let path = get("path").map_err(in_project)?;
    if !path.is_empty() {
        if let Some(bad) = path.split('/').find(|segment| !is_safe_name(segment)) {
            return Err(in_project(format!(
                "path {path:?}: {bad:?} is not a folder name ({SAFE_NAME})"
            )));
        }
    }
    let product = get("product").map_err(in_project)?;
    if !PRODUCTS.contains(&product.as_str()) {
        return Err(in_project(format!(
            "product {product:?} is not one of {PRODUCTS:?}"
        )));
    }
    let app_type = get("app_type").map_err(in_project)?;
    if app_type.is_empty() || !app_type.bytes().all(|b| b.is_ascii_alphanumeric()) {
        return Err(in_project(format!("app_type {app_type:?} is not a name")));
    }
    Ok(Project {
        name,
        source,
        repository,
        commit,
        path,
        license,
        license_url,
        product,
        app_type,
        files: Vec::new(),
        slug,
    })
}

/// What [`is_safe_name`] accepts, for messages.
const SAFE_NAME: &str = "letters, digits, ., -, _ and spaces, not . or .., \
                         no . or space at the end, no Windows device name";

/// A file or folder name that means the same in a URL and on every file
/// system the tests run on.
fn is_safe_name(name: &str) -> bool {
    const DEVICES: &[&str] = &["con", "prn", "aux", "nul"];
    let stem = name.split('.').next().unwrap_or_default().trim_end();
    let numbered = |prefix: &str| {
        stem.len() == 4
            && stem[..3].eq_ignore_ascii_case(prefix)
            && stem.as_bytes()[3].is_ascii_digit()
    };
    !name.is_empty()
        && name != "."
        && name != ".."
        && !name.ends_with(['.', ' '])
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._- ".contains(&b))
        && !DEVICES.iter().any(|d| stem.eq_ignore_ascii_case(d))
        && !numbered("com")
        && !numbered("lpt")
}

fn project_file(table: &str) -> Result<ProjectFile, String> {
    let get = |key: &str| string(table, key)?.ok_or(format!("a file has no {key}"));
    let name = get("name")?;
    if !is_safe_name(&name) || name.ends_with(".part") {
        return Err(format!(
            "file name {name:?}: a name in the project's folder ({SAFE_NAME}, not .part)"
        ));
    }
    let sha256 = get("sha256")?;
    if !is_hex(&sha256, 64) {
        return Err(format!("{name}: sha256 is not 64 lower-case hex digits"));
    }
    if value(table, "path")?.is_some() {
        return Err(format!("{name}: a file has a name, not a path"));
    }
    Ok(ProjectFile { name, sha256 })
}

fn is_hex(text: &str, len: usize) -> bool {
    text.len() == len
        && text
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

#[cfg(test)]
mod tests {
    use super::*;

    const ONE: &str = "\
# a comment
[[project]]
name = \"a\"
source = \"https://github.com/o/r/blob/0123456789012345678901234567890123456789/A.b4j\"
repository = \"https://github.com/o/r\"
commit = \"0123456789012345678901234567890123456789\"
path = \"src\"
license = \"MIT\"
license_url = \"https://github.com/o/r/blob/0123456789012345678901234567890123456789/LICENSE\"
product = \"b4j\"
app_type = \"StandardJava\"

[[project.file]]
name = \"A.b4j\"
sha256 = \"e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855\"

[[project.file]]
name = \"A.b4j.meta\"
sha256 = \"e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855\"
";

    #[test]
    fn reads_projects_and_their_files() {
        let projects = from_toml(ONE).unwrap();
        let [p] = projects.as_slice() else {
            panic!("{projects:?}")
        };
        assert_eq!(p.name, "a");
        assert_eq!(p.files.len(), 2);
        assert_eq!(
            p.sources().map(|f| f.name.as_str()).collect::<Vec<_>>(),
            ["A.b4j"]
        );
        assert_eq!(
            p.url(&p.files[0]),
            "https://raw.githubusercontent.com/o/r/0123456789012345678901234567890123456789/src/A.b4j"
        );
        let mut root = p.clone();
        root.path = String::new();
        assert!(root.url(&p.files[0]).ends_with("0123456789/A.b4j"));
        // Spaces are percent-encoded; a source in the repository's tree is
        // accepted; extensions in any case are sources.
        let spaced = ONE
            .replacen("path = \"src\"", "path = \"My Project/src\"", 1)
            .replacen("name = \"A.b4j\"", "name = \"Main.B4J\"", 1)
            .replacen(
                "blob/0123456789012345678901234567890123456789/A.b4j",
                "tree/0123456789012345678901234567890123456789",
                1,
            );
        let p = &from_toml(&spaced).unwrap()[0];
        assert_eq!(
            p.url(&p.files[0]),
            "https://raw.githubusercontent.com/o/r/0123456789012345678901234567890123456789/My%20Project/src/Main.B4J"
        );
        assert_eq!(p.sources().count(), 1);
        assert_eq!(
            (p.product.as_str(), p.app_type.as_str()),
            ("b4j", "StandardJava")
        );
    }

    #[test]
    fn rejects_what_it_cannot_trust() {
        for (from, to) in [
            ("name = \"a\"", "name = \"../a\""),
            ("name = \"a\"", "name = \"A\""),
            ("https://github.com/o/r\"", "https://example.com/o/r\""),
            ("https://github.com/o/r\"", "https://github.com/o/r.git\""),
            ("https://github.com/o/r\"", "https://github.com/o/r/\""),
            (
                "https://github.com/o/r\"",
                "https://github.com/o/r/tree/main\"",
            ),
            ("https://github.com/o/r\"", "https://github.com/o\""),
            (
                "https://github.com/o/r/blob/",
                "https://github.com/o/x/blob/",
            ),
            (
                "r/blob/0123456789012345678901234567890123456789/LICENSE",
                "r/blob/main/LICENSE",
            ),
            (
                "r/blob/0123456789012345678901234567890123456789/LICENSE",
                "x/blob/0123456789012345678901234567890123456789/LICENSE",
            ),
            ("license = \"MIT\"", "license = \"GPL-3.0\""),
            ("product = \"b4j\"", "product = \"b4x\""),
            ("app_type = \"StandardJava\"", "app_type = \"\""),
            (
                "commit = \"0123456789012345678901234567890123456789\"",
                "commit = \"main\"",
            ),
            ("path = \"src\"", "path = \"../src\""),
            ("path = \"src\"", "path = \"/src\""),
            ("path = \"src\"", "path = \"src/\""),
            ("path = \"src\"", "path = \"a//b\""),
            ("path = \"src\"", "path = \"./src\""),
            ("path = \"src\"", "path = \"src\\\\..\\\\x\""),
            ("path = \"src\"", "path = \"a#b\""),
            ("license = \"MIT\"\n", ""),
            ("name = \"A.b4j\"", "name = \"../A.b4j\""),
            ("name = \"A.b4j.meta\"", "name = \"A.b4j\""),
            ("name = \"A.b4j\"", "name = \"A.txt\""),
            ("name = \"A.b4j.meta\"", "name = \"a.B4J\""),
            ("name = \"A.b4j\"", "name = \".\""),
            ("name = \"A.b4j\"", "name = \"Calc#2.bas\""),
            ("name = \"A.b4j\"", "name = \"a?b.bas\""),
            ("name = \"A.b4j\"", "name = \"a%20.bas\""),
            ("name = \"A.b4j\"", "name = \"a:b.bas\""),
            ("name = \"A.b4j\"", "name = \"aux.bas\""),
            ("name = \"A.b4j\"", "name = \"COM1.b4j\""),
            ("name = \"A.b4j\"", "name = \"A.b4j.\""),
            ("name = \"A.b4j\"", "name = \"A.b4j \""),
            (
                "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855\"\n\n",
                "E3B0\"\n\n",
            ),
            (
                "[[project.file]]\nname = \"A.b4j\"",
                "[[project.files]]\nname = \"A.b4j\"",
            ),
            ("# a comment\n", "name = \"x\"\n"),
        ] {
            assert!(ONE.contains(from), "{from}");
            let text = ONE.replacen(from, to, 1);
            assert!(from_toml(&text).is_err(), "accepted with {to:?}");
        }
        let twice = format!("{ONE}{}", &ONE[ONE.find("[[project]]").unwrap()..]);
        assert!(from_toml(&twice).is_err());
    }

    #[test]
    fn safe_names() {
        for good in [
            "A.b4j",
            "My Project",
            "a-b_c.d.bas",
            "console",
            "com10.bas",
            "auxiliary.bas",
        ] {
            assert!(is_safe_name(good), "{good}");
        }
        for bad in [
            "", ".", "..", "a.", "a ", "a/b", "a\\b", "nul", "NUL.txt", "lpt9.bas", "é.bas",
        ] {
            assert!(!is_safe_name(bad), "{bad}");
        }
    }

    #[test]
    fn fetches_into_the_target_directory() {
        let root = fetched_root();
        assert!(root.ends_with("corpus"));
        assert!(
            !root
                .components()
                .any(|c| c == std::path::Component::ParentDir)
        );
    }

    #[test]
    fn a_read_checks_the_hash() {
        let file = ProjectFile {
            name: "x".into(),
            sha256: sha256::hex(b"abc"),
        };
        assert!(check(&"x", b"abc", &file).is_ok());
        let e = check(&"x", b"abd", &file).unwrap_err();
        assert!(e.contains("not the ba7816bf"), "{e}");
    }

    #[test]
    fn the_real_world_corpus_is_found() {
        let projects = projects();
        assert!(projects.iter().any(|p| p.name == "juros"));
    }
}
