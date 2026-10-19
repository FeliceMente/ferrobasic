//! `ferro bundle` end to end (acceptance criterion 5 of `docs/spec/cli.md`):
//! for every conformance test with an expected output, the standalone binary,
//! copied away from the build tree, must print exactly what `ferro run`
//! prints and exit with the same code.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const FERRO: &str = env!("CARGO_BIN_EXE_ferro");

/// A fresh directory under Cargo's per-target scratch area (removed by
/// `cargo clean`, and never on a `noexec` `/tmp`), deleted when dropped, also
/// when the test fails.
struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Scratch {
        let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("bundle-{name}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        Scratch(dir)
    }
}

impl std::ops::Deref for Scratch {
    type Target = Path;
    fn deref(&self) -> &Path {
        &self.0
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// A program's executable name on this platform.
fn exe(name: &str) -> String {
    format!("{name}{}", std::env::consts::EXE_SUFFIX)
}

fn ferro(args: &[&str]) -> Output {
    Command::new(FERRO).args(args).output().unwrap()
}

/// Runs an executable that was just written. Linux refuses to run a file
/// that some process still has open for writing (`ETXTBSY`). Tests run on
/// several threads, and a child process another test is starting can hold a
/// copy of this test's write descriptor for a moment, between its `fork` and
/// its `exec`. So retry briefly instead of failing at random.
fn run_binary(command: &mut Command) -> Output {
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

fn conformance_dir() -> PathBuf {
    ferro_corpus::root().join("level0")
}

/// Builds `source` and returns the binary, moved into its own empty directory.
fn build_and_move(source: &Path, dir: &Path) -> PathBuf {
    let built = dir.join("build").join(exe("prog"));
    fs::create_dir_all(built.parent().unwrap()).unwrap();
    let out = ferro(&[
        "bundle",
        source.to_str().unwrap(),
        "-o",
        built.to_str().unwrap(),
    ]);
    assert!(
        out.status.success(),
        "bundle failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let moved = dir.join("elsewhere").join(exe("prog"));
    fs::create_dir_all(moved.parent().unwrap()).unwrap();
    fs::rename(&built, &moved).unwrap();
    fs::remove_dir_all(dir.join("build")).unwrap();
    moved
}

#[test]
fn standalone_binaries_behave_like_ferro_run() {
    let mut checked = 0;
    let mut failures = Vec::new();
    for program in ferro_corpus::programs() {
        let Some(expected) = program.expected_output() else {
            continue;
        };
        let name = &program.name;
        // Prefixed, so that no conformance test can share a directory with
        // the other tests here.
        let dir = Scratch::new(&format!("conformance-{}", program.file_name()));
        let binary = build_and_move(&program.path, &dir);
        let run = ferro(&["run", program.path.to_str().unwrap()]);
        let standalone = run_binary(Command::new(&binary).current_dir(dir.join("elsewhere")));
        if String::from_utf8_lossy(&standalone.stdout) != expected {
            failures.push(format!("{name}: stdout differs from the expected output"));
        }
        if standalone.stdout != run.stdout {
            failures.push(format!("{name}: stdout differs from ferro run"));
        }
        if standalone.stderr != run.stderr {
            failures.push(format!("{name}: stderr differs from ferro run"));
        }
        if standalone.status.code() != run.status.code() {
            failures.push(format!(
                "{name}: exit code {:?}, ferro run gives {:?}",
                standalone.status.code(),
                run.status.code()
            ));
        }
        checked += 1;
    }
    assert!(checked > 0, "no conformance test with an expected output");
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

#[test]
fn arguments_go_to_app_start() {
    let dir = Scratch::new("args");
    let source = dir.join("echo.bas");
    fs::write(
        &source,
        "Sub AppStart (Args() As String)\n    Log(Args.Length)\n    For Each a As String In Args\n        Log(a)\n    Next\nEnd Sub\n",
    )
    .unwrap();
    let binary = build_and_move(&source, &dir);
    // Arguments that look like ferro's own commands and flags are the program's.
    let out = run_binary(Command::new(&binary).args(["run", "--help", "é"]));
    assert_eq!(String::from_utf8_lossy(&out.stdout), "3\nrun\n--help\né\n");
    assert!(out.status.success());
    // Same through `ferro run`.
    let run = ferro(&["run", source.to_str().unwrap(), "--", "run", "--help", "é"]);
    assert_eq!(run.stdout, out.stdout);
}

#[test]
fn exit_codes_and_corrupt_binaries() {
    let dir = Scratch::new("exit");
    let source = dir.join("exit.bas");
    fs::write(
        &source,
        "Sub AppStart (Args() As String)\n    Log(\"bye\")\n    ExitApplication2(7)\nEnd Sub\n",
    )
    .unwrap();
    let binary = build_and_move(&source, &dir);
    let out = run_binary(&mut Command::new(&binary));
    assert_eq!(out.stdout, b"bye\n");
    assert_eq!(out.status.code(), Some(7));

    // Damage the payload length in the trailer: the binary must refuse to run
    // with exit code 2 rather than run something else or crash. (On macOS the
    // package is not at the end of the file, and any change breaks the
    // signature.)
    #[cfg(not(target_os = "macos"))]
    {
        let mut bytes = fs::read(&binary).unwrap();
        let trailer = bytes.len() - 24;
        bytes[trailer..trailer + 8].copy_from_slice(&u64::MAX.to_le_bytes());
        fs::write(&binary, &bytes).unwrap();
        let out = run_binary(&mut Command::new(&binary));
        assert_eq!(out.status.code(), Some(2));
        assert!(String::from_utf8_lossy(&out.stderr).contains("corrupt"));
    }
}

/// macOS refuses binaries with a broken signature on Apple Silicon:
/// the output must pass `codesign`'s strict verification.
#[cfg(target_os = "macos")]
#[test]
fn binaries_are_validly_signed() {
    let dir = Scratch::new("signed");
    let binary = build_and_move(&conformance_dir().join("prices_map.bas"), &dir);
    let verify = Command::new("codesign")
        .args(["--verify", "--strict", "--verbose=2"])
        .arg(&binary)
        .output()
        .unwrap();
    assert!(
        verify.status.success(),
        "codesign --verify: {}",
        String::from_utf8_lossy(&verify.stderr)
    );
}

#[test]
fn programs_with_errors_are_not_built() {
    let dir = Scratch::new("errors");
    let out_path = dir.join("prog");
    let negative = conformance_dir().join("do_loop_while.bas");
    let out = ferro(&[
        "bundle",
        negative.to_str().unwrap(),
        "-o",
        out_path.to_str().unwrap(),
    ]);
    assert_eq!(out.status.code(), Some(3));
    assert!(!out_path.exists());
}

#[test]
fn default_output_is_the_file_stem() {
    let dir = Scratch::new("default");
    let source = conformance_dir().join("prices_map.bas");
    let out = Command::new(FERRO)
        .args(["bundle", source.to_str().unwrap()])
        .current_dir(&*dir)
        .output()
        .unwrap();
    assert!(out.status.success());
    let binary = dir.join(exe("prices_map"));
    let run = run_binary(&mut Command::new(&binary));
    assert_eq!(run.stdout, b"Big breakfast\nTotal: 2.7\n");
}

/// Reserved commands (`docs/spec/cli.md`, "Commands") say what is missing and
/// when it arrives, and never do something else instead. `build` meant
/// `bundle` in 0.0.1, so it points there.
#[test]
fn reserved_commands_are_not_implemented_yet() {
    // Run in an empty directory: nothing may be written there.
    let dir = Scratch::new("reserved");
    let source = conformance_dir().join("prices_map.bas");
    let source = source.to_str().unwrap();
    for (args, hint) in [
        (vec!["build", source], "ferro bundle"),
        (vec!["build"], "ferro bundle"),
        (vec!["compile", source], "(M3)"),
        (vec!["compile", source, "--target", "js"], "(M3)"),
        (vec!["targets"], "ferro compile"),
    ] {
        let out = Command::new(FERRO)
            .args(&args)
            .current_dir(&*dir)
            .output()
            .unwrap();
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert_eq!(out.status.code(), Some(2), "{args:?}");
        assert!(out.stdout.is_empty(), "{args:?}");
        assert!(
            stderr.contains("not implemented yet") && stderr.contains(hint),
            "{args:?}: {stderr}"
        );
    }
    assert_eq!(fs::read_dir(&*dir).unwrap().count(), 0);
}
