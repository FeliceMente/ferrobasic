//! Project files (`.b4j`) and directories as input, and the header checks
//! (`docs/spec/cli.md`, "Inputs"; `docs/spec/project-files.md`).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const FERRO: &str = env!("CARGO_BIN_EXE_ferro");

/// A console project as the IDE writes it: BOM, CRLF, the template's
/// project attributes.
const CONSOLE: &str = "\u{feff}AppType=StandardJava\r\nBuild1=Default,b4j.example\r\nGroup=Default Group\r\nLibrary1=jcore\r\nNumberOfFiles=0\r\nNumberOfLibraries=1\r\nNumberOfModules=0\r\nVersion=10\r\n@EndOfDesignText@\r\n'Non-UI application (console / server application)\r\n#Region Project Attributes \r\n\t#CommandLineArgs:\r\n\t#MergeLibraries: True \r\n#End Region\r\n\r\nSub Process_Globals\r\n\t\r\nEnd Sub\r\n\r\nSub AppStart (Args() As String)\r\n\tLog(\"Hello \" & Args.Length)\r\nEnd Sub\r\n";

/// A UI project with a module, a file and other libraries.
const UI: &str = "AppType=JavaFX\r\nBuild1=Default,b4j.example\r\nFile1=1.bjl\r\nFileGroup1=New Group\r\nGroup=Default Group\r\nLibrary1=b4xpages\r\nLibrary2=jcore\r\nLibrary3=jfx\r\nModule1=|relative|..\\B4XMainPage\r\nNumberOfFiles=1\r\nNumberOfLibraries=3\r\nNumberOfModules=1\r\nVersion=8.9\r\n@EndOfDesignText@\r\nSub AppStart (Args() As String)\r\nEnd Sub\r\n";

/// A fresh directory under Cargo's per-target scratch area, removed when
/// dropped, also when the test fails.
struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Self {
        let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
            .join(format!("projects-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        Scratch(dir)
    }

    fn file(&self, name: &str, text: &str) -> PathBuf {
        let path = self.0.join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, text).unwrap();
        path
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn ferro(dir: &Path, args: &[&str]) -> Output {
    Command::new(FERRO)
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap()
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// Runs an executable this test process just wrote, retrying on "text file
/// busy" (see `run_binary` in `tests/bundle.rs`).
fn run_written(command: &mut Command) -> Output {
    for _ in 0..200 {
        match command.output() {
            Err(e) if e.kind() == std::io::ErrorKind::ExecutableFileBusy => {
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            result => return result.unwrap(),
        }
    }
    panic!("{command:?}: still \"text file busy\" after 2 s");
}

#[test]
fn a_console_project_runs_from_its_file_or_its_directory() {
    let dir = Scratch::new("console");
    dir.file("hello/Hello.b4j", CONSOLE);
    // Other files in the directory do not count.
    dir.file("hello/Notes.bas", "Sub X\nEnd Sub\n");
    for input in ["hello/Hello.b4j", "hello", "hello/"] {
        let out = ferro(&dir.0, &["run", input, "--", "a", "b"]);
        assert_eq!(out.status.code(), Some(0), "{input}: {}", text(&out.stderr));
        assert_eq!(text(&out.stdout), "Hello 2\n", "{input}");
    }
    let out = ferro(&dir.0, &["check", "hello"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
}

#[test]
fn a_bundled_directory_is_named_after_its_project() {
    let dir = Scratch::new("bundle");
    // The folder's name differs from the project's: on a file system that
    // ignores case (macOS, Windows), `Hello` would be the folder `hello`
    // itself, which bundle refuses to overwrite.
    dir.file("app/Hello.b4j", CONSOLE);
    let out = ferro(&dir.0, &["bundle", "app"]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
    let exe = dir.0.join(format!("Hello{}", std::env::consts::EXE_SUFFIX));
    let out = run_written(Command::new(&exe).arg("x"));
    assert_eq!(text(&out.stdout), "Hello 1\n");
}

#[test]
fn an_unsupported_project_is_refused_line_by_line() {
    let dir = Scratch::new("ui");
    dir.file("ui/App.b4j", UI);
    for command in ["check", "run", "bundle"] {
        let out = ferro(&dir.0, &[command, "ui/App.b4j"]);
        let stderr = text(&out.stderr);
        assert_eq!(out.status.code(), Some(3), "{command}: {stderr}");
        assert!(out.stdout.is_empty());
        for expected in [
            "App.b4j:1:1: error: this is a UI project (`AppType=JavaFX`)",
            "App.b4j:3:1: error: the project includes file `1.bjl`",
            "App.b4j:6:1: error: the project uses library `b4xpages`",
            "App.b4j:8:1: error: the project uses library `jfx`",
            "App.b4j:9:1: error: the project has module `|relative|..\\B4XMainPage`",
            "5 error(s)",
        ] {
            assert!(stderr.contains(expected), "{command}: {expected}\n{stderr}");
        }
    }
    assert!(
        !dir.0
            .join(format!("App{}", std::env::consts::EXE_SUFFIX))
            .exists()
    );
}

#[test]
fn a_directory_needs_exactly_one_project() {
    let dir = Scratch::new("dirs");
    dir.file(
        "none/Main.bas",
        "Sub AppStart (Args() As String)\nEnd Sub\n",
    );
    dir.file("two/A.b4j", CONSOLE);
    dir.file("two/B.B4J", CONSOLE);
    let out = ferro(&dir.0, &["run", "none"]);
    assert_eq!(out.status.code(), Some(2));
    assert!(text(&out.stderr).contains("contains no project file (.b4j)"));
    let out = ferro(&dir.0, &["check", "two"]);
    assert_eq!(out.status.code(), Some(2));
    assert!(
        text(&out.stderr).contains("contains 2 project files (A.b4j, B.B4J): name the one to use"),
        "{}",
        text(&out.stderr)
    );
}

#[test]
fn a_module_header_must_be_a_code_module() {
    let dir = Scratch::new("modules");
    let code = "B4J=true\r\nGroup=Default Group\r\nModulesStructureVersion=1\r\nType=StaticCode\r\nVersion=10\r\n@EndOfDesignText@\r\nSub AppStart (Args() As String)\r\n\tLog(\"code\")\r\nEnd Sub\r\n";
    dir.file("Code.bas", code);
    let out = ferro(&dir.0, &["run", "Code.bas"]);
    assert_eq!(text(&out.stdout), "code\n", "{}", text(&out.stderr));
    dir.file("Thing.bas", &code.replace("StaticCode", "Class"));
    let out = ferro(&dir.0, &["check", "Thing.bas"]);
    assert_eq!(out.status.code(), Some(3));
    assert!(
        text(&out.stderr).contains("Thing.bas:4:1: error: this is a class module (`Type=Class`)"),
        "{}",
        text(&out.stderr)
    );
    // A project file without a header is not a project.
    dir.file("Bare.b4j", "Sub AppStart (Args() As String)\nEnd Sub\n");
    let out = ferro(&dir.0, &["check", "Bare.b4j"]);
    assert_eq!(out.status.code(), Some(3));
    assert!(text(&out.stderr).contains("not a project file"));
}
