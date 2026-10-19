//! `ferro bundle` end to end (acceptance criterion 5 of `docs/spec/cli.md`):
//! for every conformance test with an expected output, the standalone binary,
//! copied away from the build tree, must print exactly what `ferro run`
//! prints and exit with the same code.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const FERRO: &str = env!("CARGO_BIN_EXE_ferro");

/// A fresh, empty directory for one test.
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("ferro-bundle-test-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn ferro(args: &[&str]) -> Output {
    Command::new(FERRO).args(args).output().unwrap()
}

fn conformance_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/conformance/level0")
}

fn is_negative(bas: &Path) -> bool {
    fs::read_to_string(bas.with_extension("meta.toml"))
        .unwrap_or_default()
        .lines()
        .any(|l| l.split('#').next().unwrap().replace(' ', "") == "status=\"negative\"")
}

/// Builds `source` and returns the binary, moved into its own empty directory.
fn build_and_move(source: &Path, dir: &Path) -> PathBuf {
    let built = dir.join("build").join("prog");
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
    let moved = dir.join("elsewhere").join("prog");
    fs::create_dir_all(moved.parent().unwrap()).unwrap();
    fs::rename(&built, &moved).unwrap();
    fs::remove_dir_all(dir.join("build")).unwrap();
    moved
}

#[test]
fn standalone_binaries_behave_like_ferro_run() {
    let mut files: Vec<PathBuf> = fs::read_dir(conformance_dir())
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "bas") && !is_negative(p))
        .collect();
    files.sort();
    let mut checked = 0;
    for file in files {
        let expected = fs::read_to_string(file.with_extension("expected"))
            .or_else(|_| fs::read_to_string(file.with_extension("provisional")));
        let Ok(expected) = expected else { continue };
        let name = file.file_stem().unwrap().to_string_lossy().into_owned();
        let dir = scratch(&name);
        let binary = build_and_move(&file, &dir);
        let run = ferro(&["run", file.to_str().unwrap()]);
        let standalone = Command::new(&binary)
            .current_dir(dir.join("elsewhere"))
            .output()
            .unwrap();
        assert_eq!(
            String::from_utf8_lossy(&standalone.stdout),
            expected,
            "{name}: stdout"
        );
        assert_eq!(
            standalone.stdout, run.stdout,
            "{name}: stdout differs from ferro run"
        );
        assert_eq!(
            standalone.stderr, run.stderr,
            "{name}: stderr differs from ferro run"
        );
        assert_eq!(
            standalone.status.code(),
            run.status.code(),
            "{name}: exit code"
        );
        fs::remove_dir_all(&dir).unwrap();
        checked += 1;
    }
    assert!(checked >= 12, "only {checked} programs checked");
}

#[test]
fn arguments_go_to_app_start() {
    let dir = scratch("args");
    let source = dir.join("echo.bas");
    fs::write(
        &source,
        "Sub AppStart (Args() As String)\n    Log(Args.Length)\n    For Each a As String In Args\n        Log(a)\n    Next\nEnd Sub\n",
    )
    .unwrap();
    let binary = build_and_move(&source, &dir);
    // Arguments that look like ferro's own commands and flags are the program's.
    let out = Command::new(&binary)
        .args(["run", "--help", "é"])
        .output()
        .unwrap();
    assert_eq!(String::from_utf8_lossy(&out.stdout), "3\nrun\n--help\né\n");
    assert!(out.status.success());
    // Same through `ferro run`.
    let run = ferro(&["run", source.to_str().unwrap(), "--", "run", "--help", "é"]);
    assert_eq!(run.stdout, out.stdout);
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn exit_codes_and_corrupt_binaries() {
    let dir = scratch("exit");
    let source = dir.join("exit.bas");
    fs::write(
        &source,
        "Sub AppStart (Args() As String)\n    Log(\"bye\")\n    ExitApplication2(7)\nEnd Sub\n",
    )
    .unwrap();
    let binary = build_and_move(&source, &dir);
    let out = Command::new(&binary).output().unwrap();
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
        let out = Command::new(&binary).output().unwrap();
        assert_eq!(out.status.code(), Some(2));
        assert!(String::from_utf8_lossy(&out.stderr).contains("corrupt"));
    }
    fs::remove_dir_all(&dir).unwrap();
}

/// macOS refuses binaries with a broken signature on Apple Silicon:
/// the output must pass `codesign`'s strict verification.
#[cfg(target_os = "macos")]
#[test]
fn binaries_are_validly_signed() {
    let dir = scratch("signed");
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
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn programs_with_errors_are_not_built() {
    let dir = scratch("errors");
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
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn default_output_is_the_file_stem() {
    let dir = scratch("default");
    let source = conformance_dir().join("prices_map.bas");
    let out = Command::new(FERRO)
        .args(["bundle", source.to_str().unwrap()])
        .current_dir(&dir)
        .output()
        .unwrap();
    assert!(out.status.success());
    let binary = dir.join(format!("prices_map{}", std::env::consts::EXE_SUFFIX));
    let run = Command::new(&binary).output().unwrap();
    assert_eq!(run.stdout, b"Big breakfast\nTotal: 2.7\n");
    fs::remove_dir_all(&dir).unwrap();
}

/// Reserved commands (`docs/spec/cli.md`, "Commands") say what is missing and
/// when it arrives, and never do something else instead. `build` meant
/// `bundle` in 0.0.1, so it points there.
#[test]
fn reserved_commands_are_not_implemented_yet() {
    let source = conformance_dir().join("prices_map.bas");
    let source = source.to_str().unwrap();
    for (args, hint) in [
        (vec!["build", source], "ferro bundle"),
        (vec!["build"], "ferro bundle"),
        (vec!["compile", source], "(M3)"),
        (vec!["compile", source, "--target", "js"], "(M3)"),
        (vec!["targets"], "ferro compile"),
    ] {
        let out = ferro(&args);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert_eq!(out.status.code(), Some(2), "{args:?}");
        assert!(out.stdout.is_empty(), "{args:?}");
        assert!(
            stderr.contains("not implemented yet") && stderr.contains(hint),
            "{args:?}: {stderr}"
        );
    }
    assert!(!Path::new("prices_map").exists());
}
