//! `ferro`, `ferro help` and `ferro version` (`docs/spec/cli.md`, "Commands").

use std::process::{Command, Output};

const FERRO: &str = env!("CARGO_BIN_EXE_ferro");

fn ferro(args: &[&str]) -> Output {
    Command::new(FERRO).args(args).output().unwrap()
}

/// The platform name `ferro` must report for the machine running the tests.
fn expected_platform() -> String {
    let arch = match std::env::consts::ARCH {
        "x86_64" => "x64",
        "aarch64" => "arm64",
        other => other,
    };
    format!("{}-{arch}", std::env::consts::OS)
}

#[test]
fn help_starts_with_version_and_platform() {
    let header = format!(
        "FerroBASIC {} ({})\nA multi-target BASIC compiler and interpreter\n\n",
        env!("CARGO_PKG_VERSION"),
        expected_platform()
    );
    for args in [&[][..], &["help"], &["--help"], &["-h"]] {
        let out = ferro(args);
        assert!(out.status.success(), "{args:?}");
        let stdout = String::from_utf8(out.stdout).unwrap();
        assert!(stdout.starts_with(&header), "{args:?}: {stdout}");
        for command in ["check", "run", "bundle", "version", "help", "compile"] {
            assert!(stdout.contains(&format!("\n  {command} ")), "{command}");
        }
    }
}

#[test]
fn version_names_the_platform() {
    for args in [["version"], ["--version"], ["-V"]] {
        let out = ferro(&args);
        assert!(out.status.success());
        assert_eq!(
            String::from_utf8(out.stdout).unwrap(),
            format!(
                "ferro {} ({})\n",
                env!("CARGO_PKG_VERSION"),
                expected_platform()
            )
        );
    }
}

/// `AGENTS.md`: the product names never appear in user-facing messages.
#[test]
fn messages_do_not_use_product_names() {
    for args in [
        &["help"][..],
        &["version"],
        &["frob"],
        &["build"],
        &["compile"],
    ] {
        let out = ferro(args);
        let text = String::from_utf8_lossy(&out.stdout) + String::from_utf8_lossy(&out.stderr);
        for name in ["B4X", "B4J", "B4A", "B4i", "B4R"] {
            assert!(!text.contains(name), "{args:?} mentions {name}: {text}");
        }
    }
}
