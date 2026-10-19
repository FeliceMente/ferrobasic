//! `ferro` must never destroy files or crash on unusual inputs: outputs that
//! would overwrite the input or `ferro` itself, non-UTF-8 arguments, closed
//! pipes, files with many errors (`docs/spec/cli.md`).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

#[cfg(unix)]
use std::os::unix::fs::FileTypeExt;

const FERRO: &str = env!("CARGO_BIN_EXE_ferro");

const PROGRAM: &str = "Sub AppStart (Args() As String)\n    For Each a As String In Args\n        Log(a)\n    Next\nEnd Sub\n";

/// A fresh directory under Cargo's per-target scratch area, removed when
/// dropped, also when the test fails.
struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Self {
        let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("safety-{name}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        Scratch(dir)
    }

    fn file(&self, name: &str, text: &str) -> PathBuf {
        let path = self.0.join(name);
        fs::write(&path, text).unwrap();
        path
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn ferro_in(dir: &Path, args: &[&std::ffi::OsStr]) -> Output {
    Command::new(FERRO)
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap()
}

/// Runs an executable this test process just wrote, retrying on "text file
/// busy": another test thread starting a process may briefly hold a copy of
/// our write descriptor (see `run_binary` in `tests/bundle.rs`).
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

fn os(s: &str) -> &std::ffi::OsStr {
    s.as_ref()
}

fn assert_refused(out: &Output, reason: &str) {
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(2), "{stderr}");
    assert!(stderr.contains(reason), "{stderr}");
}

#[test]
fn bundle_never_overwrites_its_input() {
    let dir = Scratch::new("input");
    dir.file("prog.bas", PROGRAM);
    let out = ferro_in(
        &dir.0,
        &[os("bundle"), os("prog.bas"), os("-o"), os("prog.bas")],
    );
    assert_refused(&out, "it is the input file");
    assert_eq!(fs::read_to_string(dir.0.join("prog.bas")).unwrap(), PROGRAM);

    // The default output is the file stem plus the platform's executable
    // suffix: `hello` on Unix, `hello.exe` on Windows, the input itself.
    let same_name = format!("hello{}", std::env::consts::EXE_SUFFIX);
    dir.file(&same_name, PROGRAM);
    let out = ferro_in(&dir.0, &[os("bundle"), os(&same_name)]);
    assert_refused(&out, "it is the input file");
    assert_eq!(fs::read_to_string(dir.0.join(&same_name)).unwrap(), PROGRAM);

    // The same file under another name: a hard link (every OS).
    fs::hard_link(dir.0.join("prog.bas"), dir.0.join("hard")).unwrap();
    let out = ferro_in(
        &dir.0,
        &[os("bundle"), os("prog.bas"), os("-o"), os("hard")],
    );
    assert_refused(&out, "it is the input file");
    assert_eq!(fs::read_to_string(dir.0.join("prog.bas")).unwrap(), PROGRAM);

    #[cfg(unix)]
    {
        // A symlink to the input, as the input or as the output: replacing
        // the link would leave the name the user gave without its source.
        std::os::unix::fs::symlink("prog.bas", dir.0.join("alias.bas")).unwrap();
        for (input, output) in [
            ("alias.bas", "alias.bas"),
            ("prog.bas", "alias.bas"),
            ("alias.bas", "hard"),
        ] {
            let out = ferro_in(&dir.0, &[os("bundle"), os(input), os("-o"), os(output)]);
            assert_refused(&out, "it is the input file");
        }
        assert_eq!(
            fs::read_to_string(dir.0.join("alias.bas")).unwrap(),
            PROGRAM
        );
        assert!(
            fs::symlink_metadata(dir.0.join("alias.bas"))
                .unwrap()
                .file_type()
                .is_symlink()
        );

        // A symlink to anything else is replaced by the binary, not followed.
        dir.file("other", "untouched");
        std::os::unix::fs::symlink("other", dir.0.join("link-out")).unwrap();
        let out = ferro_in(
            &dir.0,
            &[os("bundle"), os("prog.bas"), os("-o"), os("link-out")],
        );
        assert_eq!(
            out.status.code(),
            Some(0),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert_eq!(
            fs::read_to_string(dir.0.join("other")).unwrap(),
            "untouched"
        );
        assert!(
            fs::symlink_metadata(dir.0.join("link-out"))
                .unwrap()
                .is_file()
        );
    }
}

#[test]
fn bundle_never_overwrites_ferro_itself() {
    let dir = Scratch::new("self");
    dir.file("prog.bas", PROGRAM);
    let copy = dir
        .0
        .join(format!("ferro-copy{}", std::env::consts::EXE_SUFFIX));
    fs::copy(FERRO, &copy).unwrap();
    let out = run_written(
        Command::new(&copy)
            .args([os("bundle"), os("prog.bas"), os("-o"), copy.as_os_str()])
            .current_dir(&dir.0),
    );
    assert_refused(&out, "it is the running ferro executable");
    let version = run_written(Command::new(&copy).arg("version"));
    assert!(String::from_utf8_lossy(&version.stdout).starts_with("ferro "));
}

#[test]
fn bundle_refuses_outputs_that_are_not_regular_files() {
    let dir = Scratch::new("special");
    dir.file("prog.bas", PROGRAM);
    fs::create_dir(dir.0.join("out")).unwrap();
    fs::write(dir.0.join("out/keep"), "x").unwrap();
    let out = ferro_in(&dir.0, &[os("bundle"), os("prog.bas"), os("-o"), os("out")]);
    assert_refused(&out, "not a regular file");
    assert!(dir.0.join("out/keep").exists());
    // No staged file is left behind.
    let names: Vec<_> = fs::read_dir(&dir.0)
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(names.len(), 2, "{names:?}");
}

/// A FIFO with no writer blocks whoever opens it: `bundle` must refuse it,
/// and replace a symlink to it, without ever opening it.
#[cfg(unix)]
#[test]
fn bundle_never_opens_a_fifo_output() {
    let dir = Scratch::new("fifo");
    dir.file("prog.bas", PROGRAM);
    let made = Command::new("mkfifo")
        .arg(dir.0.join("fifo"))
        .status()
        .unwrap();
    assert!(made.success());
    std::os::unix::fs::symlink("fifo", dir.0.join("to-fifo")).unwrap();

    let bundle = |output: &str| {
        let mut child = Command::new(FERRO)
            .args(["bundle", "prog.bas", "-o", output])
            .current_dir(&dir.0)
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let start = std::time::Instant::now();
        loop {
            if let Some(status) = child.try_wait().unwrap() {
                return status.code();
            }
            if start.elapsed() > std::time::Duration::from_secs(10) {
                let _ = child.kill();
                panic!("ferro bundle -o {output} blocked for 10 s");
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
    };
    assert_eq!(bundle("fifo"), Some(2));
    assert!(
        fs::symlink_metadata(dir.0.join("fifo"))
            .unwrap()
            .file_type()
            .is_fifo()
    );
    assert_eq!(bundle("to-fifo"), Some(0));
    assert!(
        fs::symlink_metadata(dir.0.join("to-fifo"))
            .unwrap()
            .is_file()
    );
}

/// An input read through a symlink to a FIFO is still the input: `bundle`
/// must not replace that symlink when it is also the output.
#[cfg(unix)]
#[test]
fn bundle_protects_an_input_read_from_a_fifo() {
    let dir = Scratch::new("fifo-input");
    let made = Command::new("mkfifo")
        .arg(dir.0.join("source"))
        .status()
        .unwrap();
    assert!(made.success());
    std::os::unix::fs::symlink("source", dir.0.join("alias.bas")).unwrap();
    let fifo = dir.0.join("source");
    // Feeds the program once `ferro` opens the FIFO for reading.
    let writer = std::thread::spawn(move || fs::write(fifo, PROGRAM));
    let mut child = Command::new(FERRO)
        .args(["bundle", "alias.bas", "-o", "alias.bas"])
        .current_dir(&dir.0)
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let start = std::time::Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if start.elapsed() > std::time::Duration::from_secs(10) {
            let _ = child.kill();
            panic!("ferro bundle alias.bas -o alias.bas blocked for 10 s");
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    };
    writer.join().unwrap().unwrap();
    assert_eq!(status.code(), Some(2));
    assert!(
        fs::symlink_metadata(dir.0.join("alias.bas"))
            .unwrap()
            .file_type()
            .is_symlink()
    );
}

/// Arguments and file names are bytes, not necessarily UTF-8. `AppStart`
/// receives undecodable bytes as U+FFFD, like on the JVM.
#[cfg(unix)]
#[test]
fn non_utf8_arguments_and_file_names_work() {
    use std::os::unix::ffi::OsStrExt;
    let dir = Scratch::new("bytes");
    let name = std::ffi::OsStr::from_bytes(b"pr\xffog.bas");
    fs::write(dir.0.join(name), PROGRAM).unwrap();
    let arg = std::ffi::OsStr::from_bytes(b"a\xffb");

    let out = ferro_in(&dir.0, &[os("run"), name, os("--"), arg]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&out.stdout), "a\u{fffd}b\n");

    let out = ferro_in(&dir.0, &[os("bundle"), name, os("-o"), os("prog")]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let out = Command::new(dir.0.join("prog")).arg(arg).output().unwrap();
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(String::from_utf8_lossy(&out.stdout), "a\u{fffd}b\n");

    // A non-UTF-8 command is an unknown command, not a crash.
    let out = ferro_in(&dir.0, &[arg]);
    assert_eq!(out.status.code(), Some(2));
}

/// Spawns `ferro` with stdout or stderr connected to a pipe whose reading end
/// is closed at once, so its writes fail with a broken pipe.
fn with_closed_pipe(args: &[&str], close_stdout: bool) -> Output {
    let mut command = Command::new(FERRO);
    command.args(args);
    if close_stdout {
        command.stdout(Stdio::piped()).stderr(Stdio::piped());
    } else {
        command.stdout(Stdio::null()).stderr(Stdio::piped());
    }
    let mut child = command.spawn().unwrap();
    if close_stdout {
        drop(child.stdout.take());
        child.wait_with_output().unwrap()
    } else {
        drop(child.stderr.take());
        let status = child.wait().unwrap();
        Output {
            status,
            stdout: Vec::new(),
            stderr: Vec::new(),
        }
    }
}

#[test]
fn closed_pipes_do_not_change_the_exit_code() {
    for args in [&["help"][..], &["version"]] {
        let out = with_closed_pipe(args, true);
        assert_eq!(out.status.code(), Some(0), "{args:?}");
        assert!(!String::from_utf8_lossy(&out.stderr).contains("panicked"));
    }
    let dir = Scratch::new("pipes");
    let bad = dir.file(
        "bad.bas",
        "Sub AppStart (Args() As String)\n    x = = 1\nEnd Sub\n",
    );
    let out = with_closed_pipe(&["check", bad.to_str().unwrap()], false);
    assert_eq!(out.status.code(), Some(3));
    let crash = dir.file(
        "crash.bas",
        "Sub AppStart (Args() As String)\n    Dim l As List\n    l.Add(1)\nEnd Sub\n",
    );
    let out = with_closed_pipe(&["run", crash.to_str().unwrap()], false);
    assert_eq!(out.status.code(), Some(1));
}

#[test]
fn many_errors_are_counted_but_not_all_printed() {
    let dir = Scratch::new("errors");
    let mut text = String::from("Sub AppStart (Args() As String)\n");
    for _ in 0..150 {
        text.push_str("    Log(undeclared)\n");
    }
    text.push_str("End Sub\n");
    let file = dir.file("many.bas", &text);
    let out = ferro_in(&dir.0, &[os("check"), file.as_os_str()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(3));
    assert_eq!(stderr.matches(": error: ").count(), 100);
    assert!(
        stderr.ends_with("... and 50 more error(s) not shown\n150 error(s)\n"),
        "{stderr}"
    );
}
