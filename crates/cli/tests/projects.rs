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
const UI: &str = "AppType=JavaFX\r\nBuild1=Default,b4j.example\r\nFile1=1.bjl\r\nFileGroup1=New Group\r\nGroup=Default Group\r\nLibrary1=b4xpages\r\nLibrary2=jcore\r\nLibrary3=jfx\r\nModule1=|relative|..\\B4XMainPage\r\nNumberOfFiles=1\r\nNumberOfLibraries=3\r\nNumberOfModules=1\r\nVersion=8.9\r\n@EndOfDesignText@\r\nSub Process_Globals\r\nEnd Sub\r\nSub AppStart (Args() As String)\r\nEnd Sub\r\n";

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

/// Bundles `input` from `cwd` with no `-o`, then runs the program expected at
/// `expected` (relative to the scratch directory, without `.exe`).
fn bundle_lands_at(dir: &Scratch, cwd: &str, input: &str, expected: &str) {
    let out = ferro(&dir.0.join(cwd), &["bundle", input]);
    assert_eq!(out.status.code(), Some(0), "{input}: {}", text(&out.stderr));
    let exe = dir
        .0
        .join(format!("{expected}{}", std::env::consts::EXE_SUFFIX));
    let out = run_written(Command::new(&exe).arg("x"));
    assert_eq!(text(&out.stdout), "Hello 1\n", "{input}");
    fs::remove_file(&exe).unwrap();
}

/// A project's program goes in the project's folder, named after the
/// project, wherever `ferro` runs from: in the IDE's layout the folder has
/// the project's name, so `./MyApp` would be the folder itself.
#[test]
fn a_bundled_project_lands_in_its_folder() {
    let dir = Scratch::new("bundle");
    dir.file("MyApp/MyApp.b4j", CONSOLE);
    // The IDE's layout: a `Files` and an `Objects` folder beside the project.
    dir.file("MyApp/Files/notes.txt", "x");
    dir.file("MyApp/Objects/MyApp.jar", "x");
    bundle_lands_at(&dir, "", "MyApp", "MyApp/MyApp");
    bundle_lands_at(&dir, "", "MyApp/", "MyApp/MyApp");
    bundle_lands_at(&dir, "", "MyApp/MyApp.b4j", "MyApp/MyApp");
    bundle_lands_at(&dir, "MyApp", ".", "MyApp/MyApp");
    bundle_lands_at(&dir, "MyApp", "MyApp.b4j", "MyApp/MyApp");
    // The folder's name does not matter: also when it differs from the
    // project's only in case, which macOS and Windows ignore.
    dir.file("hello/Hello.b4j", CONSOLE);
    bundle_lands_at(&dir, "", "hello", "hello/Hello");
    dir.file("app/Hello.b4j", CONSOLE);
    bundle_lands_at(&dir, "", "app", "app/Hello");
}

/// A project named like one of the IDE's folders: its default output on
/// Linux and macOS, `Objects/Objects`, is that folder. Bundling refuses it
/// and says to use `-o`; on Windows the output is `Objects.exe`.
#[test]
fn a_project_named_like_its_folder_needs_o() {
    let dir = Scratch::new("bundle-objects");
    dir.file("Objects/Objects.b4j", CONSOLE);
    dir.file("Objects/Objects/Objects.jar", "x");
    let out = ferro(&dir.0, &["bundle", "Objects"]);
    if std::env::consts::EXE_SUFFIX.is_empty() {
        assert_eq!(out.status.code(), Some(2));
        assert!(
            text(&out.stderr).contains(
                "cannot write Objects/Objects: it is a directory; choose another output with -o"
            ),
            "{}",
            text(&out.stderr)
        );
        assert!(dir.0.join("Objects/Objects/Objects.jar").is_file());
        let out = ferro(&dir.0, &["bundle", "Objects", "-o", "app"]);
        assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
        let out = run_written(&mut Command::new(dir.0.join("app")));
        assert_eq!(text(&out.stdout), "Hello 0\n");
    } else {
        assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
    }
}

#[test]
fn a_bundled_project_goes_where_o_says() {
    let dir = Scratch::new("bundle-o");
    dir.file("MyApp/MyApp.b4j", CONSOLE);
    fs::create_dir(dir.0.join("build")).unwrap();
    // Windows runs `build/app` as `build/app.exe`: name it so.
    let app = format!("build/app{}", std::env::consts::EXE_SUFFIX);
    let out = ferro(&dir.0, &["bundle", "MyApp", "-o", &app]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
    let out = run_written(&mut Command::new(dir.0.join(&app)));
    assert_eq!(text(&out.stdout), "Hello 0\n");
    assert!(
        !dir.0
            .join(format!("MyApp/MyApp{}", std::env::consts::EXE_SUFFIX))
            .exists()
    );
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
        "Sub Process_Globals\nEnd Sub\n\nSub AppStart (Args() As String)\nEnd Sub\n",
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
    let code = "B4J=true\r\nGroup=Default Group\r\nModulesStructureVersion=1\r\nType=StaticCode\r\nVersion=10\r\n@EndOfDesignText@\r\nSub Process_Globals\r\nEnd Sub\r\nSub AppStart (Args() As String)\r\n\tLog(\"code\")\r\nEnd Sub\r\n";
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
    dir.file(
        "Bare.b4j",
        "Sub Process_Globals\nEnd Sub\n\nSub AppStart (Args() As String)\nEnd Sub\n",
    );
    let out = ferro(&dir.0, &["check", "Bare.b4j"]);
    assert_eq!(out.status.code(), Some(3));
    assert!(text(&out.stderr).contains("not a project file"));
}
