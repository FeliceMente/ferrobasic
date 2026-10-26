//! The real-world corpus: the projects `tests/corpus/corpus.toml`
//! references. Their files are not in this repository; `fetch` downloads
//! them into `target/corpus`, and every read checks their SHA-256 against
//! the one recorded, so a test never runs on a file other than the one
//! referenced.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::{sha256, string, value};

/// One project, as its `[[project]]` table describes it.
#[derive(Debug, Clone)]
pub struct Project {
    /// Its folder in `target/corpus`.
    pub name: String,
    /// Where it was published, at the commit referenced.
    pub source: String,
    pub repository: String,
    pub commit: String,
    /// Its folder in the repository, `""` for the root.
    pub path: String,
    pub license: String,
    pub license_url: String,
    pub files: Vec<ProjectFile>,
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
                .is_some_and(|e| e == "b4j" || e == "bas")
        })
    }

    /// The URL `file` is downloaded from, at the commit referenced.
    pub fn url(&self, file: &ProjectFile) -> String {
        let repo = self
            .repository
            .strip_prefix("https://github.com/")
            .expect("checked when read");
        let folder = if self.path.is_empty() {
            String::new()
        } else {
            format!("{}/", self.path)
        };
        format!(
            "https://raw.githubusercontent.com/{repo}/{}/{folder}{}",
            self.commit, file.name
        )
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

/// `target/corpus` in this repository: where the projects are fetched.
pub fn fetched_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/corpus")
}

/// Every project `tests/corpus/corpus.toml` references, in its order. Panics
/// on a table it cannot read.
pub fn projects() -> Vec<Project> {
    let list = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/corpus/corpus.toml");
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
                if project.files.iter().any(|f| f.name == file.name) {
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
    let url = |key: &str| {
        let url = get(key).map_err(in_project)?;
        if url.starts_with("https://github.com/") {
            Ok(url)
        } else {
            Err(in_project(format!("{key} is not a GitHub URL: {url}")))
        }
    };
    let (source, repository, license_url) =
        (url("source")?, url("repository")?, url("license_url")?);
    let commit = get("commit").map_err(in_project)?;
    if !is_hex(&commit, 40) {
        return Err(in_project(format!("commit is not a full SHA-1: {commit}")));
    }
    let path = get("path").map_err(in_project)?;
    if path.starts_with('/') || path.ends_with('/') || path.split('/').any(|p| p == "..") {
        return Err(in_project(format!(
            "path {path:?} must be relative, with no trailing /"
        )));
    }
    for key in ["product", "app_type"] {
        get(key).map_err(in_project)?;
    }
    Ok(Project {
        license: get("license").map_err(in_project)?,
        name,
        source,
        repository,
        commit,
        path,
        license_url,
        files: Vec::new(),
    })
}

fn project_file(table: &str) -> Result<ProjectFile, String> {
    let get = |key: &str| string(table, key)?.ok_or(format!("a file has no {key}"));
    let name = get("name")?;
    if name.is_empty() || name.contains(['/', '\\']) || name == ".." || name.ends_with(".part") {
        return Err(format!(
            "file name {name:?}: a name in the project's folder"
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
    }

    #[test]
    fn rejects_what_it_cannot_trust() {
        for (from, to) in [
            ("name = \"a\"", "name = \"../a\""),
            ("name = \"a\"", "name = \"A\""),
            ("https://github.com/o/r\"", "https://example.com/o/r\""),
            (
                "commit = \"0123456789012345678901234567890123456789\"",
                "commit = \"main\"",
            ),
            ("path = \"src\"", "path = \"../src\""),
            ("path = \"src\"", "path = \"/src\""),
            ("license = \"MIT\"\n", ""),
            ("name = \"A.b4j\"", "name = \"../A.b4j\""),
            ("name = \"A.b4j.meta\"", "name = \"A.b4j\""),
            ("name = \"A.b4j\"", "name = \"A.txt\""),
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
        assert!(projects.iter().all(|p| p.license_url.contains(&p.commit)));
        assert!(projects.iter().all(|p| p.source.contains(&p.commit)));
    }
}
