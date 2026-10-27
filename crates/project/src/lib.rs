//! Reader for B4X project files (.b4j/.b4a/.b4i/.b4r) and .b4xlib archives.
//!
//! Today:
//! - the `Key=Value` header that project files (`.b4j`) and module files
//!   (`.bas`) start with, and the Level 0 checks on it
//!   (`docs/spec/project-files.md`, `docs/spec/cli.md`);
//! - project, module and `.b4j.meta` files read so that they write back
//!   byte for byte ([`HeaderFile`], [`MetaFile`]), and a whole project with
//!   the modules it names ([`load`]).

mod file;
mod project;

pub use file::{HeaderFile, Line, MetaFile};
pub use project::{Build, FileEntry, Module, ModuleRef, Project, find_file, load};

/// One `Key=Value` line of a header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry<'a> {
    pub key: &'a str,
    pub value: &'a str,
    /// Byte offset of the line in the whole text.
    pub offset: usize,
}

/// The header lines of `text`, after an optional BOM and up to (not
/// including) the `@EndOfDesignText@` line, or `None` if `text` has no
/// header. Uses the lexer's own rule, so that the two always agree on where
/// the code starts.
pub fn header(text: &str) -> Option<Vec<Entry<'_>>> {
    let bom = if text.starts_with('\u{feff}') {
        '\u{feff}'.len_utf8()
    } else {
        0
    };
    let len = ferro_syntax::lexer::header_len(&text[bom..])?;
    let mut entries = Vec::new();
    // Lines end as the lexer ends them: CRLF, CR or LF.
    let mut offset = bom;
    while offset < bom + len {
        let rest = &text[offset..bom + len];
        let line = &rest[..rest.find(['\r', '\n']).unwrap_or(rest.len())];
        if let Some((key, value)) = line.split_once('=') {
            entries.push(Entry { key, value, offset });
        }
        let newline = if rest[line.len()..].starts_with("\r\n") {
            2
        } else {
            1
        };
        offset += line.len() + newline;
    }
    Some(entries)
}

/// Something in a header that Level 0 cannot honour, at a byte offset of the
/// whole text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Problem {
    pub offset: usize,
    pub message: String,
}

/// Checks a project file (`.b4j`): Level 0 runs only a console program
/// (`AppType=StandardJava`) whose one library is jCore, with no other
/// module and no file of its own. Every problem is reported, each at its
/// header line. Other keys (`Build1`, `Group`, `Version`, the `NumberOf…`
/// counts) only describe the project in the IDE, and are ignored.
pub fn check_project(text: &str) -> Vec<Problem> {
    let Some(entries) = header(text) else {
        return vec![Problem {
            offset: 0,
            message: "not a project file: it has no header ending with `@EndOfDesignText@`".into(),
        }];
    };
    let mut problems = Vec::new();
    let mut problem = |entry: &Entry, message: String| {
        problems.push(Problem {
            offset: entry.offset,
            message,
        })
    };
    let mut app_type = false;
    for entry in &entries {
        let key = entry.key;
        let value = entry.value.trim();
        if key == "AppType" {
            app_type = true;
            match value {
                "StandardJava" => {}
                "JavaFX" => problem(
                    entry,
                    "this is a UI project (`AppType=JavaFX`): only console projects \
                     (`AppType=StandardJava`) are supported at Level 0"
                        .into(),
                ),
                other => problem(
                    entry,
                    format!(
                        "unknown project type `AppType={other}`: only console projects (`AppType=StandardJava`) are supported at Level 0"
                    ),
                ),
            }
        } else if numbered(key, "Library") && !value.eq_ignore_ascii_case("jcore") {
            problem(
                entry,
                format!("the project uses library `{value}`: only jCore is supported at Level 0"),
            );
        } else if numbered(key, "Module") {
            problem(
                entry,
                format!(
                    "the project has module `{value}`: programs with several modules arrive at Level 1"
                ),
            );
        } else if numbered(key, "File") {
            problem(
                entry,
                format!(
                    "the project includes file `{value}`: project files (the Files folder) are not supported at Level 0"
                ),
            );
        }
    }
    if !app_type {
        problems.push(Problem {
            offset: entries.first().map_or(0, |e| e.offset),
            message: "the project has no `AppType`: only console projects \
                      (`AppType=StandardJava`) are supported at Level 0"
                .into(),
        });
    }
    problems
}

/// Checks a module file's header, if it has one: Level 0 runs a code module
/// (`Type=StaticCode`, or no `Type`).
pub fn check_module(text: &str) -> Vec<Problem> {
    let entries = header(text).unwrap_or_default();
    let mut problems = Vec::new();
    for entry in entries.iter().filter(|e| e.key == "Type") {
        let message = match entry.value.trim() {
            "StaticCode" => continue,
            "Class" => "this is a class module (`Type=Class`): classes arrive at Level 1".into(),
            other => format!(
                "module type `{other}` is not supported: Level 0 runs a code module (`Type=StaticCode`)"
            ),
        };
        problems.push(Problem {
            offset: entry.offset,
            message,
        });
    }
    problems
}

/// `Library1`, `Module12`…: `prefix` then a number.
fn numbered(key: &str, prefix: &str) -> bool {
    file::number(key, prefix).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The header of a console project as the IDE writes it: BOM, CRLF,
    /// keys in alphabetical order.
    const CONSOLE: &str = "\u{feff}AppType=StandardJava\r\nBuild1=Default,b4j.example\r\nGroup=Default Group\r\nLibrary1=jcore\r\nNumberOfFiles=0\r\nNumberOfLibraries=1\r\nNumberOfModules=0\r\nVersion=10\r\n@EndOfDesignText@\r\nSub AppStart (Args() As String)\r\nEnd Sub\r\n";

    fn messages(problems: &[Problem]) -> Vec<&str> {
        problems.iter().map(|p| p.message.as_str()).collect()
    }

    #[test]
    fn reads_the_header_lines() {
        // CRLF, CR and LF line ends, mixed.
        let mixed = header("A=1\r\nB=2\rC=3\n@EndOfDesignText@\r\n").unwrap();
        let read: Vec<_> = mixed.iter().map(|e| (e.key, e.value, e.offset)).collect();
        assert_eq!(read, [("A", "1", 0), ("B", "2", 5), ("C", "3", 9)]);
        let entries = header(CONSOLE).unwrap();
        assert_eq!(entries.len(), 8);
        assert_eq!(
            (entries[0].key, entries[0].value, entries[0].offset),
            ("AppType", "StandardJava", 3)
        );
        assert_eq!(entries[3].value, "jcore");
        assert_eq!(
            &CONSOLE[entries[3].offset..entries[3].offset + 8],
            "Library1"
        );
        assert_eq!(header("Sub AppStart (Args() As String)\nEnd Sub\n"), None);
    }

    #[test]
    fn a_console_project_passes() {
        assert_eq!(check_project(CONSOLE), []);
        // Older versions write the keys in another order, and LF line ends.
        let old = "Version=5.82\nAppType=StandardJava\nNumberOfModules=0\nBuild1=Default,b4j.example\nNumberOfFiles=0\nNumberOfLibraries=1\nLibrary1=jCore\n@EndOfDesignText@\n";
        assert_eq!(check_project(old), []);
    }

    #[test]
    fn a_ui_project_with_modules_and_files_is_refused_line_by_line() {
        let text = "AppType=JavaFX\r\nBuild1=Default,b4j.example\r\nFile1=1.bjl\r\nFileGroup1=New Group\r\nGroup=Default Group\r\nLibrary1=b4xpages\r\nLibrary2=jcore\r\nLibrary3=jfx\r\nModule1=|relative|..\\B4XMainPage\r\nNumberOfFiles=1\r\nNumberOfLibraries=3\r\nNumberOfModules=1\r\nVersion=8.9\r\n@EndOfDesignText@\r\n";
        let problems = check_project(text);
        let lines: Vec<&str> = problems
            .iter()
            .map(|p| text[p.offset..].split("\r\n").next().unwrap())
            .collect();
        assert_eq!(
            lines,
            [
                "AppType=JavaFX",
                "File1=1.bjl",
                "Library1=b4xpages",
                "Library3=jfx",
                "Module1=|relative|..\\B4XMainPage"
            ]
        );
        assert!(messages(&problems)[0].contains("UI project"));
        assert!(messages(&problems)[3].contains("library `jfx`"));
    }

    #[test]
    fn a_project_needs_a_header_and_an_app_type() {
        assert!(
            messages(&check_project("Sub AppStart (Args() As String)\nEnd Sub\n"))[0]
                .contains("no header")
        );
        let problems = check_project("Library1=jcore\n@EndOfDesignText@\n");
        assert!(messages(&problems)[0].contains("no `AppType`"));
        let problems = check_project("AppType=Applet\n@EndOfDesignText@\n");
        assert!(messages(&problems)[0].contains("`AppType=Applet`"));
    }

    #[test]
    fn module_headers_must_be_code_modules() {
        assert_eq!(
            check_module(
                "B4J=true\r\nGroup=Default Group\r\nModulesStructureVersion=1\r\nType=StaticCode\r\nVersion=10\r\n@EndOfDesignText@\r\n"
            ),
            []
        );
        assert_eq!(
            check_module("Sub AppStart (Args() As String)\nEnd Sub\n"),
            []
        );
        assert_eq!(check_module("B4J=true\n@EndOfDesignText@\n"), []);
        let text = "B4J=true\nType=Class\n@EndOfDesignText@\n";
        let problems = check_module(text);
        assert_eq!(problems[0].offset, 9);
        assert!(problems[0].message.contains("classes arrive at Level 1"));
        assert!(
            check_module("B4A=true\nType=Service\n@EndOfDesignText@\n")[0]
                .message
                .contains("`Service`")
        );
        // Lines may also end with a lone CR, as the lexer allows.
        let problems = check_module("B4J=true\rType=Class\r@EndOfDesignText@\rSub X\rEnd Sub\r");
        assert_eq!(problems.len(), 1);
        assert_eq!(problems[0].offset, 9);
    }

    #[test]
    fn numbered_keys() {
        assert!(numbered("Library12", "Library"));
        assert!(!numbered("Library", "Library"));
        assert!(!numbered("LibraryX", "Library"));
        assert!(!numbered("FileGroup1", "File"));
    }
}
