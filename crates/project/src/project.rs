//! A whole project: its project file, its `.b4j.meta` file and the module
//! files it names, found the way `docs/spec/project-files.md` describes.

use std::ffi::OsStr;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::Problem;
use crate::file::{HeaderFile, MetaFile, compare_numbers, number};

/// A build configuration, from `Build<n>=<name>,<package>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Build<'a> {
    pub name: &'a str,
    /// The package name, when the value has one after the first `,`.
    pub package: Option<&'a str>,
}

/// A file of the project's Files folder, from `File<n>` and its
/// `FileGroup<n>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileEntry<'a> {
    pub name: &'a str,
    pub group: Option<&'a str>,
}

/// Where a `Module<n>` line says a module is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModuleRef {
    /// `Module1=Tools`: `Tools.bas` in the project's folder.
    Local(String),
    /// `Module1=|relative|..\Shared\Tools`: `Tools.bas` at that path from
    /// the project's folder, with `\` or `/` between folders.
    Relative(String),
}

impl ModuleRef {
    pub fn parse(value: &str) -> ModuleRef {
        match value.strip_prefix("|relative|") {
            Some(path) => ModuleRef::Relative(path.to_string()),
            None => ModuleRef::Local(value.to_string()),
        }
    }

    /// The module's name: the last part of its path.
    pub fn name(&self) -> &str {
        match self {
            ModuleRef::Local(name) => name,
            ModuleRef::Relative(path) => path.rsplit(['\\', '/']).next().unwrap_or(path),
        }
    }

    /// The folders from the project's folder to the module, then the
    /// module's file name, as written. An absolute path, or one that ends
    /// with a separator, is an error.
    fn parts(&self) -> Result<(Vec<&str>, String), String> {
        let (folders, name) = match self {
            ModuleRef::Local(name) => (Vec::new(), name.as_str()),
            ModuleRef::Relative(path) => {
                let absolute =
                    path.starts_with(['\\', '/']) || path.as_bytes().get(1) == Some(&b':');
                if absolute {
                    return Err(format!("`|relative|{path}` is not a relative path"));
                }
                let mut parts: Vec<&str> = path.split(['\\', '/']).collect();
                let name = parts.pop().unwrap_or_default();
                let folders = parts
                    .into_iter()
                    .filter(|p| !p.is_empty() && *p != ".")
                    .collect();
                (folders, name)
            }
        };
        if name.is_empty() {
            return Err("the module has no name".into());
        }
        Ok((folders, format!("{name}.bas")))
    }

    /// The module file's path from the project's folder, as written. The
    /// folders and the file may be named in another case: see
    /// [`ModuleRef::find`].
    pub fn path(&self, project_dir: &Path) -> Result<PathBuf, String> {
        let (folders, file) = self.parts()?;
        let mut path = project_dir.to_path_buf();
        path.extend(folders);
        Ok(path.join(file))
    }

    /// The module's file, found as the IDE finds it on Windows, whose file
    /// names ignore letter case: each folder, and the file, is the entry
    /// with that exact name, or else the one entry of its folder whose name
    /// differs only in case (`Module1=JsonWebToken` finds
    /// `JSONWebToken.bas`). Two such entries, and no exact one, is an
    /// error.
    pub fn find(&self, project_dir: &Path) -> Result<PathBuf, String> {
        let (folders, file) = self.parts()?;
        let mut path = project_dir.to_path_buf();
        for folder in folders {
            path = if folder == ".." {
                path.join(folder)
            } else {
                entry(&path, OsStr::new(folder), Kind::Folder)?
            };
        }
        entry(&path, OsStr::new(&file), Kind::File)
    }
}

#[derive(Clone, Copy)]
enum Kind {
    Folder,
    File,
}

impl Kind {
    fn is(self, path: &Path) -> bool {
        match self {
            Kind::Folder => path.is_dir(),
            Kind::File => path.is_file(),
        }
    }
}

/// `name` in `dir`, or the one entry of `dir` named like it in another
/// letter case.
fn entry(dir: &Path, name: &OsStr, kind: Kind) -> Result<PathBuf, String> {
    let exact = dir.join(name);
    if kind.is(&exact) {
        return Ok(exact);
    }
    let entries = fs::read_dir(dir).map_err(|e| format!("{}: {e}", exact.display()))?;
    let mut found: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .filter(|e| same_name(&e.file_name(), name))
        .map(|e| e.path())
        .filter(|p| kind.is(p))
        .collect();
    found.sort();
    match found.len() {
        0 => Err(format!("{} not found", exact.display())),
        1 => Ok(found.pop().unwrap()),
        _ => Err(format!(
            "{} is ambiguous: {}",
            exact.display(),
            found
                .iter()
                .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join(", ")
        )),
    }
}

/// Whether two file names differ at most in letter case, of any letter, as
/// on Windows. Names that are not Unicode match only exactly.
fn same_name(a: &OsStr, b: &OsStr) -> bool {
    match (a.to_str(), b.to_str()) {
        (Some(a), Some(b)) => a == b || a.to_uppercase() == b.to_uppercase(),
        _ => a == b,
    }
}

/// A module file's contents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModuleText {
    /// A header, then the code: what the IDE writes.
    Header(HeaderFile),
    /// A file with no header, as it is.
    Plain(String),
}

impl fmt::Display for ModuleText {
    /// The file, byte for byte as it was read.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ModuleText::Header(file) => file.fmt(f),
            ModuleText::Plain(text) => f.write_str(text),
        }
    }
}

/// A module file of a project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Module {
    /// The `Module<n>` key that names it.
    pub key: String,
    pub reference: ModuleRef,
    pub path: PathBuf,
    pub text: ModuleText,
}

impl Module {
    /// `Type` in its header: `StaticCode` or `Class` in B4J.
    pub fn module_type(&self) -> Option<&str> {
        match &self.text {
            ModuleText::Header(file) => file.get("Type"),
            ModuleText::Plain(_) => None,
        }
    }
}

/// A project, as [`load`] reads it.
#[derive(Debug, Clone)]
pub struct Project {
    pub path: PathBuf,
    pub file: HeaderFile,
    /// `<project>.b4j.meta`, when there is one and it could be read.
    pub meta: Option<MetaFile>,
    /// Every module the project file names, in `Module<n>` order, that was
    /// found and read.
    pub modules: Vec<Module>,
    /// Modules that could not be found or read, each at its line of the
    /// project file, and a `.b4j.meta` file that could not be read, at the
    /// file's start.
    pub problems: Vec<Problem>,
}

impl Project {
    /// The project's folder.
    pub fn dir(&self) -> &Path {
        self.path.parent().unwrap_or(Path::new(""))
    }

    pub fn app_type(&self) -> Option<&str> {
        self.file.get("AppType")
    }

    /// The IDE version that last saved it.
    pub fn version(&self) -> Option<&str> {
        self.file.get("Version")
    }

    pub fn builds(&self) -> Vec<Build<'_>> {
        self.file
            .numbered("Build")
            .into_iter()
            .map(|(_, value)| match value.split_once(',') {
                Some((name, package)) => Build {
                    name,
                    package: Some(package),
                },
                None => Build {
                    name: value,
                    package: None,
                },
            })
            .collect()
    }

    /// The libraries, as written (the IDE writes them in lower case).
    pub fn libraries(&self) -> Vec<&str> {
        self.file
            .numbered("Library")
            .into_iter()
            .map(|(_, v)| v)
            .collect()
    }

    pub fn module_refs(&self) -> Vec<ModuleRef> {
        self.file
            .numbered("Module")
            .into_iter()
            .map(|(_, v)| ModuleRef::parse(v))
            .collect()
    }

    pub fn files(&self) -> Vec<FileEntry<'_>> {
        let groups = self.file.numbered("FileGroup");
        self.file
            .numbered("File")
            .into_iter()
            .map(|(n, name)| FileEntry {
                name,
                group: groups.iter().find(|(g, _)| *g == n).map(|(_, v)| *v),
            })
            .collect()
    }

    /// Where the `NumberOfFiles`, `NumberOfLibraries` and `NumberOfModules`
    /// counts differ from the numbered keys: `(count key, written, found)`.
    pub fn count_mismatches(&self) -> Vec<(&'static str, String, usize)> {
        [
            ("NumberOfFiles", self.files().len()),
            ("NumberOfLibraries", self.libraries().len()),
            ("NumberOfModules", self.module_refs().len()),
        ]
        .into_iter()
        .filter_map(|(key, found)| {
            let written = self.file.get(key)?;
            (written.trim().parse::<usize>().ok() != Some(found))
                .then(|| (key, written.to_string(), found))
        })
        .collect()
    }
}

/// Reads the project file at `path`, its `.b4j.meta` file if there is one,
/// and every module it names. An unreadable project file, or one without a
/// header, is an error. A module that cannot be found or read is a
/// [`Problem`] at its line of the project file; a `.b4j.meta` file that
/// cannot be read is one too, and is left out, as it only holds the IDE's
/// state.
pub fn load(path: &Path) -> Result<Project, String> {
    let text = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let file = HeaderFile::parse(&text).ok_or_else(|| {
        format!(
            "{}: not a project file: it has no header ending with `@EndOfDesignText@`",
            path.display()
        )
    })?;
    let (mut modules, mut problems) = (Vec::new(), Vec::new());
    let mut meta_path = path.as_os_str().to_owned();
    meta_path.push(".meta");
    let meta_path = PathBuf::from(meta_path);
    let meta = match fs::read_to_string(&meta_path) {
        Ok(text) => Some(MetaFile::parse(&text)),
        Err(e) if e.kind() == io::ErrorKind::NotFound => None,
        Err(e) => {
            problems.push(Problem {
                offset: 0,
                message: format!(
                    "{}: {e}: left out, as it only holds the IDE's state",
                    meta_path.display()
                ),
            });
            None
        }
    };
    let dir = path.parent().unwrap_or(Path::new("")).to_path_buf();
    for line in &file.header.lines {
        let Some((key, value)) = line.key_value() else {
            continue;
        };
        if number(key, "Module").is_none() {
            continue;
        }
        let reference = ModuleRef::parse(value);
        let read = reference.find(&dir).and_then(|found| {
            fs::read_to_string(&found)
                .map(|text| (found.clone(), text))
                .map_err(|e| format!("{}: {e}", found.display()))
        });
        match read {
            Ok((found, text)) => modules.push(Module {
                key: key.to_string(),
                text: match HeaderFile::parse(&text) {
                    Some(file) => ModuleText::Header(file),
                    None => ModuleText::Plain(text),
                },
                reference,
                path: found,
            }),
            Err(message) => problems.push(Problem {
                offset: line.offset,
                message: format!("module `{}`: {message}", reference.name()),
            }),
        }
    }
    modules.sort_by(|a, b| {
        let n = |m: &Module| number(&m.key, "Module").unwrap_or_default().to_string();
        compare_numbers(&n(a), &n(b))
    });
    Ok(Project {
        path: path.to_path_buf(),
        file,
        meta,
        modules,
        problems,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fresh folder of its own under the system's temporary folder.
    fn folder(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("ferro-project-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    const MODULE: &str = "B4J=true\r\nGroup=Default Group\r\nModulesStructureVersion=1\r\nType=StaticCode\r\nVersion=10\r\n@EndOfDesignText@\r\nSub Hello\r\nEnd Sub\r\n";

    /// Whether `dir` tells file names apart by letter case.
    fn case_sensitive(dir: &Path) -> bool {
        fs::write(dir.join("probe"), "").unwrap();
        let sensitive = !dir.join("PROBE").exists();
        fs::remove_file(dir.join("probe")).unwrap();
        sensitive
    }

    #[test]
    fn module_refs() {
        let p = Path::new("p");
        let local = ModuleRef::parse("Tools");
        assert_eq!(local, ModuleRef::Local("Tools".into()));
        assert_eq!(local.path(p).unwrap(), p.join("Tools.bas"));
        let shared = ModuleRef::parse("|relative|..\\Shared\\B4XMainPage");
        assert_eq!(shared.name(), "B4XMainPage");
        assert_eq!(
            shared.path(p).unwrap(),
            p.join("..").join("Shared").join("B4XMainPage.bas")
        );
        let slashes = ModuleRef::parse("|relative|.//lib/Util");
        assert_eq!(slashes.path(p).unwrap(), p.join("lib").join("Util.bas"));
        for bad in [
            "|relative|..\\Shared\\",
            "|relative|/opt/lib/Util",
            "|relative|\\\\server\\Util",
            "|relative|C:\\lib\\Util",
            "",
        ] {
            let e = ModuleRef::parse(bad).path(p).unwrap_err();
            assert!(
                e.contains("relative path") || e.contains("no name"),
                "{bad}: {e}"
            );
        }
    }

    #[test]
    fn names_differ_in_case_of_any_letter() {
        assert!(same_name(
            OsStr::new("JSONWebToken.bas"),
            OsStr::new("JsonWebToken.bas")
        ));
        assert!(same_name(
            OsStr::new("üBERBLICK.bas"),
            OsStr::new("Überblick.bas")
        ));
        assert!(same_name(OsStr::new("ärger.bas"), OsStr::new("Ärger.bas")));
        assert!(!same_name(OsStr::new("Tools.bas"), OsStr::new("Tool.bas")));
    }

    #[test]
    fn loads_a_project_its_modules_and_its_meta_file() {
        let root = folder("load");
        let dir = root.join("App");
        fs::create_dir_all(dir.join("Files")).unwrap();
        // The folder is named in another case than in the project file.
        fs::create_dir_all(root.join("Shared")).unwrap();
        let project = "\u{feff}AppType=StandardJava\r\nBuild1=Default,b4j.example\r\nBuild2=Test\r\nFile1=data.txt\r\nFileGroup1=Default Group\r\nGroup=Default Group\r\nLibrary1=jcore\r\nLibrary2=json\r\nModule1=JsonWebToken\r\nModule2=|relative|..\\shared\\Util\r\nModule10=Überblick\r\nNumberOfFiles=1\r\nNumberOfLibraries=2\r\nNumberOfModules=4\r\nVersion=10.5\r\n@EndOfDesignText@\r\nSub Process_Globals\r\nEnd Sub\r\n";
        let path = dir.join("App.b4j");
        fs::write(&path, project).unwrap();
        fs::write(
            dir.join("App.b4j.meta"),
            "SelectedBuild=0\r\nVisibleModules=1,2\r\n",
        )
        .unwrap();
        // Named in another case than in the project file, as on Windows.
        fs::write(dir.join("JSONWebToken.bas"), MODULE).unwrap();
        fs::write(dir.join("üBERBLICK.bas"), "Sub Plain\nEnd Sub\n").unwrap();
        fs::write(
            root.join("Shared").join("Util.bas"),
            MODULE.replace("StaticCode", "Class"),
        )
        .unwrap();

        let p = load(&path).unwrap();
        assert!(p.problems.is_empty(), "{:?}", p.problems);
        assert_eq!(p.file.to_string(), project);
        assert_eq!(p.app_type(), Some("StandardJava"));
        assert_eq!(p.version(), Some("10.5"));
        assert_eq!(
            p.builds(),
            [
                Build {
                    name: "Default",
                    package: Some("b4j.example")
                },
                Build {
                    name: "Test",
                    package: None
                }
            ]
        );
        assert_eq!(p.libraries(), ["jcore", "json"]);
        assert_eq!(
            p.files(),
            [FileEntry {
                name: "data.txt",
                group: Some("Default Group")
            }]
        );
        assert_eq!(p.meta.as_ref().unwrap().get("VisibleModules"), Some("1,2"));
        let keys: Vec<&str> = p.modules.iter().map(|m| m.key.as_str()).collect();
        assert_eq!(keys, ["Module1", "Module2", "Module10"]);
        assert_eq!(p.modules[0].module_type(), Some("StaticCode"));
        assert_eq!(p.modules[0].text.to_string(), MODULE);
        assert_eq!(p.modules[1].reference.name(), "Util");
        assert_eq!(p.modules[1].module_type(), Some("Class"));
        assert_eq!(p.modules[2].module_type(), None);
        assert_eq!(p.modules[2].text.to_string(), "Sub Plain\nEnd Sub\n");
        if case_sensitive(&dir) {
            assert_eq!(p.modules[0].path, dir.join("JSONWebToken.bas"));
            assert_eq!(
                p.modules[1].path,
                dir.join("..").join("Shared").join("Util.bas")
            );
        }
        // NumberOfModules says 4 where 3 are named.
        assert_eq!(
            p.count_mismatches(),
            [("NumberOfModules", "4".to_string(), 3)]
        );
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn a_missing_or_ambiguous_module_is_a_problem_at_its_line() {
        let dir = folder("missing");
        let project = "AppType=StandardJava\nModule1=Gone\nModule2=Twice\nModule99999999999=Huge\nModule3=|relative|..\\\n@EndOfDesignText@\n";
        let path = dir.join("P.b4j");
        fs::write(&path, project).unwrap();
        let sensitive = case_sensitive(&dir);
        fs::write(dir.join("twice.bas"), MODULE).unwrap();
        fs::write(dir.join("TWICE.bas"), MODULE).unwrap();
        let p = load(&path).unwrap();
        let at: Vec<&str> = p
            .problems
            .iter()
            .map(|pr| project[pr.offset..].lines().next().unwrap())
            .collect();
        // A file system that ignores case holds only one of the two.
        if sensitive {
            assert_eq!(
                at,
                [
                    "Module1=Gone",
                    "Module2=Twice",
                    "Module99999999999=Huge",
                    "Module3=|relative|..\\"
                ]
            );
            assert!(
                p.problems[1].message.contains("ambiguous"),
                "{:?}",
                p.problems
            );
        } else {
            assert_eq!(
                at,
                [
                    "Module1=Gone",
                    "Module99999999999=Huge",
                    "Module3=|relative|..\\"
                ]
            );
        }
        assert!(p.problems[0].message.contains("module `Gone`"));
        assert!(p.problems[0].message.contains("not found"));
        assert!(p.problems.last().unwrap().message.contains("no name"));
        assert!(p.meta.is_none());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn an_unreadable_meta_file_is_left_out() {
        let dir = folder("meta");
        let path = dir.join("P.b4j");
        fs::write(&path, "AppType=StandardJava\n@EndOfDesignText@\n").unwrap();
        // Windows-1252, not UTF-8.
        fs::write(dir.join("P.b4j.meta"), b"ModuleBookmarks0=caf\xe9\n").unwrap();
        let p = load(&path).unwrap();
        assert!(p.meta.is_none());
        assert_eq!(p.problems.len(), 1);
        assert!(
            p.problems[0].message.contains("IDE's state"),
            "{:?}",
            p.problems
        );
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_project_file_needs_a_header() {
        let dir = folder("noheader");
        let path = dir.join("P.b4j");
        fs::write(&path, "Sub Process_Globals\nEnd Sub\n").unwrap();
        assert!(load(&path).unwrap_err().contains("no header"));
        assert!(load(&dir.join("absent.b4j")).is_err());
        fs::remove_dir_all(&dir).unwrap();
    }
}
