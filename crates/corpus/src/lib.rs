//! The conformance corpus in `tests/conformance` (see
//! `docs/compatibility-testing.md`), read the same way by every crate's test
//! harness: every program under it, at any depth, with its status and its
//! expected output.
//!
//! The metadata is checked strictly, so that a typo cannot quietly turn a
//! test into one that is skipped: every `.bas` needs a `.meta.toml` with a
//! known `status`, and an empty corpus is an error.

use std::fs;
use std::path::{Path, PathBuf};

/// `status` in a `.meta.toml`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// Waiting for a recording from the original toolchain.
    PendingRecording,
    /// Its `.expected` output was recorded from the original toolchain.
    Recorded,
    /// FerroBASIC differs on purpose: see `deviation`.
    Deviation,
    /// Invalid: both toolchains must reject it.
    Negative,
}

/// One conformance test.
#[derive(Debug, Clone)]
pub struct Program {
    /// The `.bas` file.
    pub path: PathBuf,
    /// Its path under `tests/conformance`, without extension and with `/`
    /// separators: `level0/prices_map`.
    pub name: String,
    pub status: Status,
    /// FerroBASIC refuses to compile it, on purpose: a `negative` test, or a
    /// pending or deviating one with `ferro_rejects = true` (a construct
    /// FerroBASIC rejects until a recording shows what it does, or by a
    /// documented deviation).
    pub rejected: bool,
    /// For a `deviation`: its entry in `docs/deviations.md`, e.g. `D1`.
    pub deviation: Option<String>,
    /// The kind of the uncaught error that stops the program, if any.
    pub uncaught_error: Option<String>,
    /// The exit code the program ends with: `exit_code` (a program that calls
    /// `ExitApplication2`), else 1 with an uncaught error, else 0.
    pub exit_code: i32,
    /// The language level it belongs to, from `level`.
    pub level: Option<u8>,
    /// What it tests, from `features`: the rows of the compatibility report.
    pub features: Vec<String>,
}

impl Program {
    pub fn source(&self) -> String {
        fs::read_to_string(&self.path).unwrap_or_else(|e| panic!("{}: {e}", self.path.display()))
    }

    /// The output to compare with: the recording if there is one, else the
    /// hand-derived provisional output (a recording always wins). A file that
    /// exists but cannot be read is an error, not a missing output.
    pub fn expected_output(&self) -> Option<String> {
        ["expected", "provisional"].iter().find_map(|ext| {
            let path = self.path.with_extension(ext);
            match fs::read_to_string(&path) {
                Ok(text) => Some(text),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
                Err(e) => panic!("{}: {e}", path.display()),
            }
        })
    }

    /// A name usable as a single path component: `level0-prices_map`.
    pub fn file_name(&self) -> String {
        self.name.replace('/', "-")
    }
}

/// `tests/conformance` in this repository.
pub fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/conformance")
}

/// Every program in the corpus, sorted by name. Panics on an empty corpus or
/// on metadata it cannot read.
pub fn programs() -> Vec<Program> {
    programs_in(&root())
}

/// Every program under `root`, the `tests/conformance` folder of a copy of
/// this repository (its `docs/deviations.md` is read too), as [`programs`].
pub fn programs_in(root: &Path) -> Vec<Program> {
    let mut files = Vec::new();
    bas_files(root, &mut files);
    files.sort();
    assert!(
        !files.is_empty(),
        "no conformance tests in {}",
        root.display()
    );
    let deviations = deviation_ids(
        &fs::read_to_string(root.join("../../docs/deviations.md")).unwrap_or_default(),
    );
    files
        .into_iter()
        .map(|path| {
            let name = path
                .strip_prefix(root)
                .unwrap()
                .with_extension("")
                .components()
                .map(|c| c.as_os_str().to_string_lossy())
                .collect::<Vec<_>>()
                .join("/");
            let meta_path = path.with_extension("meta.toml");
            let meta = fs::read_to_string(&meta_path)
                .unwrap_or_else(|e| panic!("{}: {e}", meta_path.display()));
            let program = from_meta(path, name, &meta)
                .unwrap_or_else(|e| panic!("{}: {e}", meta_path.display()));
            if let Some(id) = &program.deviation {
                assert!(
                    deviations.contains(id),
                    "{}: deviation {id} is not in docs/deviations.md",
                    program.name
                );
            }
            program
        })
        .collect()
}

/// The IDs in the first column of the table in `docs/deviations.md`.
fn deviation_ids(markdown: &str) -> Vec<String> {
    markdown
        .lines()
        .filter_map(|line| line.trim().strip_prefix('|')?.split('|').next())
        .map(str::trim)
        .filter(|id| {
            id.len() > 1 && id.starts_with('D') && id[1..].bytes().all(|b| b.is_ascii_digit())
        })
        .map(String::from)
        .collect()
}

fn bas_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display()));
    for entry in entries {
        let path = entry.unwrap().path();
        if path.is_dir() {
            bas_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "bas") {
            out.push(path);
        }
    }
}

fn from_meta(path: PathBuf, name: String, meta: &str) -> Result<Program, String> {
    let status = match string(meta, "status")?.as_deref() {
        Some("pending-recording") => Status::PendingRecording,
        Some("recorded") => Status::Recorded,
        Some("deviation") => Status::Deviation,
        Some("negative") => Status::Negative,
        Some(other) => return Err(format!("unknown status {other:?}")),
        None => return Err("no status".into()),
    };
    let ferro_rejects = match value(meta, "ferro_rejects")?.as_deref() {
        None | Some("false") => false,
        Some("true") => true,
        Some(other) => return Err(format!("ferro_rejects must be true or false, not {other}")),
    };
    if ferro_rejects && !matches!(status, Status::PendingRecording | Status::Deviation) {
        return Err("ferro_rejects is only for pending-recording and deviation tests".into());
    }
    let deviation = string(meta, "deviation")?;
    if (status == Status::Deviation) != deviation.is_some() {
        return Err("a deviation test, and only one, names its entry in `deviation`".into());
    }
    let uncaught_error = string(meta, "uncaught_error")?;
    let rejected = ferro_rejects || status == Status::Negative;
    let exit_code = match value(meta, "exit_code")? {
        None => i32::from(uncaught_error.is_some()),
        Some(_) if uncaught_error.is_some() => {
            return Err("exit_code and uncaught_error exclude each other".into());
        }
        Some(_) if rejected => return Err("a program FerroBASIC rejects has no exit code".into()),
        Some(v) => match v.parse::<i32>() {
            Ok(code) if (0..=255).contains(&code) => code,
            _ => return Err(format!("exit_code must be a number from 0 to 255, not {v}")),
        },
    };
    let level = match value(meta, "level")? {
        None => None,
        Some(v) => Some(
            v.parse::<u8>()
                .map_err(|_| format!("level must be a number, not {v}"))?,
        ),
    };
    let features = match value(meta, "features")? {
        None => Vec::new(),
        Some(v) => {
            strings(&v).ok_or_else(|| format!("features must be a list of strings, not {v}"))?
        }
    };
    let program = Program {
        level,
        features,
        rejected,
        uncaught_error,
        exit_code,
        path,
        name,
        status,
        deviation,
    };
    if program.rejected && program.expected_output().is_some() {
        return Err("a program FerroBASIC rejects has no output to compare".into());
    }
    Ok(program)
}

/// The raw value of top-level `key = value` (before the first `[table]`),
/// without its comment.
fn value(meta: &str, key: &str) -> Result<Option<String>, String> {
    let mut found = None;
    for line in meta.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            break;
        }
        let Some((k, v)) = line.split_once('=') else {
            continue;
        };
        if k.trim() != key {
            continue;
        }
        if found.is_some() {
            return Err(format!("{key} is set twice"));
        }
        found = Some(strip_comment(v).trim().to_string());
    }
    Ok(found)
}

/// The value of top-level `key = "text"`.
fn string(meta: &str, key: &str) -> Result<Option<String>, String> {
    match value(meta, key)? {
        None => Ok(None),
        Some(v) => match v.strip_prefix('"').and_then(|v| v.strip_suffix('"')) {
            Some(text) if !text.contains('"') => Ok(Some(text.to_string())),
            _ => Err(format!("{key} must be a string, not {v}")),
        },
    }
}

/// The strings of a one-line TOML array of basic strings without escapes:
/// `["a", "b, c"]`. A comma inside quotes belongs to its string.
fn strings(value: &str) -> Option<Vec<String>> {
    let mut rest = value.strip_prefix('[')?.strip_suffix(']')?.trim();
    let mut items = Vec::new();
    while !rest.is_empty() {
        let after = rest.strip_prefix('"')?;
        let end = after.find('"')?;
        items.push(after[..end].to_string());
        rest = after[end + 1..].trim_start();
        match rest.strip_prefix(',') {
            // A trailing comma is not accepted: one more string must follow.
            Some(next) => {
                rest = next.trim_start();
                if rest.is_empty() {
                    return None;
                }
            }
            None if rest.is_empty() => {}
            None => return None,
        }
    }
    Some(items)
}

/// Cuts a `#` comment, outside quotes.
fn strip_comment(text: &str) -> &str {
    let mut quoted = false;
    for (i, c) in text.char_indices() {
        match c {
            '"' => quoted = !quoted,
            '#' if !quoted => return &text[..i],
            _ => {}
        }
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    fn meta(text: &str) -> Result<Program, String> {
        from_meta(PathBuf::from("/nonexistent/x.bas"), "x".into(), text)
    }

    #[test]
    fn reads_the_status_by_value() {
        let p = meta("# \"negative\" in a comment\nstatus = \"pending-recording\" # or negative\n")
            .unwrap();
        assert_eq!(p.status, Status::PendingRecording);
        assert!(!p.rejected);
        assert!(meta("status = \"negative\"\n").unwrap().rejected);
        // Values after the first table are not top-level keys.
        let p = meta("status = \"recorded\"\n[provisional]\nstatus = \"negative\"\n").unwrap();
        assert_eq!(p.status, Status::Recorded);
    }

    #[test]
    fn rejects_unreadable_metadata() {
        for bad in [
            "",
            "status = negative\n",
            "status = \"negatve\"\n",
            "status = \"recorded\"\nstatus = \"negative\"\n",
            "status = \"recorded\"\nferro_rejects = true\n",
            "status = \"pending-recording\"\nferro_rejects = yes\n",
            "status = \"deviation\"\n",
            "status = \"pending-recording\"\ndeviation = \"D1\"\n",
        ] {
            assert!(meta(bad).is_err(), "{bad:?}");
        }
        let p = meta("status = \"deviation\"\ndeviation = \"D1\"\nferro_rejects = true\n").unwrap();
        assert!(p.rejected);
        assert_eq!(p.deviation.as_deref(), Some("D1"));
        let p =
            meta("status = \"pending-recording\"\nuncaught_error = \"a.B#C\"  # kind\n").unwrap();
        assert_eq!(p.uncaught_error.as_deref(), Some("a.B#C"));
        assert_eq!(p.exit_code, 1);
    }

    #[test]
    fn reads_the_exit_code() {
        assert_eq!(meta("status = \"recorded\"\n").unwrap().exit_code, 0);
        assert_eq!(
            meta("status = \"recorded\"\nexit_code = 3  # ExitApplication2\n")
                .unwrap()
                .exit_code,
            3
        );
        for bad in [
            "status = \"recorded\"\nexit_code = 256\n",
            "status = \"recorded\"\nexit_code = -1\n",
            "status = \"recorded\"\nexit_code = \"3\"\n",
            "status = \"recorded\"\nexit_code = 3\nuncaught_error = \"a.B\"\n",
            "status = \"negative\"\nexit_code = 3\n",
        ] {
            assert!(meta(bad).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn reads_the_level_and_features() {
        let p = meta("status = \"recorded\"\nlevel = 0\nfeatures = [\"for\", \"scope\"]  # x\n")
            .unwrap();
        assert_eq!(p.level, Some(0));
        assert_eq!(p.features, ["for", "scope"]);
        let p = meta("status = \"recorded\"\nfeatures = [\"a, b\",\"c\"]\n").unwrap();
        assert_eq!(p.features, ["a, b", "c"]);
        let p = meta("status = \"recorded\"\nfeatures = []\n").unwrap();
        assert_eq!(p.level, None);
        assert!(p.features.is_empty());
        for bad in [
            "status = \"recorded\"\nlevel = one\n",
            "status = \"recorded\"\nfeatures = for\n",
            "status = \"recorded\"\nfeatures = [for]\n",
            "status = \"recorded\"\nfeatures = [\"for\",]\n",
            "status = \"recorded\"\nfeatures = [\"for\" \"scope\"]\n",
            "status = \"recorded\"\nfeatures = [\"for]\n",
        ] {
            assert!(meta(bad).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn deviation_ids_come_from_the_id_column() {
        let table = "| ID | Area | Target(s) |\n| --- | --- | --- |\n| D1 | Syntax | all |\n| D12 | Runtime | all |\n";
        assert_eq!(deviation_ids(table), ["D1", "D12"]);
    }

    #[test]
    fn the_corpus_is_found() {
        let programs = programs();
        assert!(
            programs
                .iter()
                .all(|p| p.level.is_some() && !p.features.is_empty())
        );
        assert!(programs.iter().any(|p| p.name == "level0/prices_map"));
        assert!(programs.iter().any(|p| p.expected_output().is_some()));
    }
}
