//! `ferro`: the FerroBASIC command-line tool.
//!
//! `check`, `run` and `bundle` work for Level 0 modules; `build`, `compile`,
//! `targets`, `repl`, `test` and `fmt` are reserved. See `docs/spec/cli.md`.

mod standalone;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// Everything after the header of `ferro help`.
const COMMANDS: &str = "\
Usage: ferro <command> [args]

Commands:
  check <file>                    Parse and type-check a module
  run <file> [-- <args>...]       Run with the reference interpreter
  bundle <file> [-o <output>]     Bundle the interpreter and the program into
                                  a standalone executable for this machine
  version                         Print the version and platform
  help                            Print this help

Planned (not implemented yet):
  compile <file> [--target <t>]   Compile for this machine or another target
  targets                         List the compilation targets
  build [<project>]               Build a project from its project file
  repl                            Start the interactive REPL
  test                            Run conformance tests
  fmt <file>                      Format source files
";

/// `ferro help`: version, platform, description, then the commands.
fn usage() -> String {
    format!(
        "FerroBASIC {} ({})\nA multi-target BASIC compiler and interpreter\n\n{COMMANDS}",
        env!("CARGO_PKG_VERSION"),
        platform()
    )
}

/// The platform this `ferro` was built for, named like the `--target` values
/// planned in `docs/spec/cli.md`: `linux-x64`, `macos-arm64`, `windows-x64`…
/// Unknown operating systems and architectures keep Rust's names.
fn platform() -> String {
    let arch = match std::env::consts::ARCH {
        "x86_64" => "x64",
        "aarch64" => "arm64",
        other => other,
    };
    format!("{}-{arch}", std::env::consts::OS)
}

/// Exit codes from `docs/spec/cli.md`.
const EXIT_USAGE: u8 = 2;
const EXIT_COMPILE_ERRORS: u8 = 3;

/// Name of the program inside a standalone binary's payload.
const MAIN_ENTRY: &str = "main.bas";

fn main() -> ExitCode {
    // A standalone binary runs its embedded program, with every argument
    // passed to `AppStart`; only a plain `ferro` parses its own CLI.
    if let Some(code) = run_embedded() {
        return code;
    }
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("version" | "--version" | "-V") => {
            println!("ferro {} ({})", env!("CARGO_PKG_VERSION"), platform());
            ExitCode::SUCCESS
        }
        Some("check") => match &args[1..] {
            [file] => check(Path::new(file)),
            _ => usage_error("ferro check: expected exactly one file"),
        },
        // Debugging aid, not part of the documented interface.
        Some("dump-cst") => match &args[1..] {
            [file] => dump_cst(Path::new(file)),
            _ => usage_error("ferro dump-cst: expected exactly one file"),
        },
        Some("dump-mir") => match &args[1..] {
            [file] => match compile(Path::new(file)) {
                Ok((_, module)) => {
                    print!("{}", ferro_mir::dump(&ferro_mir::lower(&module)));
                    ExitCode::SUCCESS
                }
                Err(code) => code,
            },
            _ => usage_error("ferro dump-mir: expected exactly one file"),
        },
        Some("run") => match &args[1..] {
            [file] => run(Path::new(file), &[]),
            [file, sep, program_args @ ..] if sep == "--" => run(Path::new(file), program_args),
            _ => usage_error("ferro run: expected `ferro run <file> [-- <args>...]`"),
        },
        Some("bundle") => match &args[1..] {
            [file] => bundle(Path::new(file), None),
            [file, flag, out] if flag == "-o" => bundle(Path::new(file), Some(Path::new(out))),
            _ => usage_error("ferro bundle: expected `ferro bundle <file> [-o <output>]`"),
        },
        // `build` meant `bundle` in 0.0.1: point there.
        Some("build") => not_implemented(
            "build",
            "building projects from their project files arrives with the JVM backend (M3); \
             to make a standalone executable from a module, use `ferro bundle <file>`",
        ),
        Some("compile") => not_implemented(
            "compile",
            "compilation arrives with the backends: JVM (M3), native and WebAssembly (M4), \
             microcontrollers (M5); until then, use `ferro run` or `ferro bundle`",
        ),
        Some("targets") => not_implemented("targets", "it arrives with `ferro compile`"),
        Some(cmd @ ("repl" | "test" | "fmt")) => not_implemented(cmd, "planned for milestone M1"),
        Some("help" | "--help" | "-h") | None => {
            print!("{}", usage());
            ExitCode::SUCCESS
        }
        Some(other) => usage_error(&format!("ferro: unknown command '{other}'")),
    }
}

/// A reserved command (`docs/spec/cli.md`): says what is missing and when it
/// arrives.
fn not_implemented(command: &str, detail: &str) -> ExitCode {
    eprintln!("ferro {command}: not implemented yet: {detail}");
    ExitCode::from(EXIT_USAGE)
}

fn usage_error(message: &str) -> ExitCode {
    eprintln!("{message}\n");
    eprint!("{}", usage());
    ExitCode::from(EXIT_USAGE)
}

fn read_source(path: &Path) -> Result<String, ExitCode> {
    std::fs::read_to_string(path).map_err(|err| {
        eprintln!("ferro: cannot read {}: {err}", path.display());
        ExitCode::from(EXIT_USAGE)
    })
}

/// Parses and type-checks a module, printing every error. `Err` holds the
/// exit code to stop with.
fn compile(path: &Path) -> Result<(String, ferro_hir::Module), ExitCode> {
    let text = read_source(path)?;
    let module = compile_text(path, &text)?;
    Ok((text, module))
}

/// [`compile`] on source already in memory; `path` names it in diagnostics.
fn compile_text(path: &Path, text: &str) -> Result<ferro_hir::Module, ExitCode> {
    let parse = ferro_syntax::parse(text);
    // Type checking runs only on a tree without syntax errors.
    let (module, errors): (_, Vec<(ferro_syntax::TextRange, String)>) = if parse.errors().is_empty()
    {
        let (module, diags) = ferro_hir::check(&parse.syntax());
        let errors = diags.into_iter().map(|d| (d.range, d.message)).collect();
        (Some(module), errors)
    } else {
        let errors = parse
            .errors()
            .iter()
            .map(|e| (e.range, e.message.clone()))
            .collect();
        (None, errors)
    };
    for (range, message) in &errors {
        eprint!("{}", render_error(path, text, *range, message));
    }
    match module {
        Some(module) if errors.is_empty() => Ok(module),
        _ => {
            eprintln!("{} error(s)", errors.len());
            Err(ExitCode::from(EXIT_COMPILE_ERRORS))
        }
    }
}

fn check(path: &Path) -> ExitCode {
    match compile(path) {
        Ok(_) => ExitCode::SUCCESS,
        Err(code) => code,
    }
}

/// `ferro run`: compile, then interpret. An uncaught runtime error is printed
/// the way B4J prints it (`docs/spec/cli.md`); stdout written before it is kept.
fn run(path: &Path, args: &[String]) -> ExitCode {
    match compile(path) {
        Ok((text, module)) => execute(&text, &module, args),
        Err(code) => code,
    }
}

/// Lowers and interprets a checked module; shared by `ferro run` and
/// standalone binaries, so both behave identically.
fn execute(text: &str, module: &ferro_hir::Module, args: &[String]) -> ExitCode {
    let program = ferro_mir::lower(module);
    let stdout = std::io::stdout();
    let stderr = std::io::stderr();
    let outcome = ferro_interp::run(&program, args, &mut stdout.lock(), &mut stderr.lock());
    if let Some(error) = &outcome.error {
        let (line, _) = line_and_column(text, usize::from(error.range.start()));
        eprintln!("Error occurred on line: {line} (Main)");
        if error.message.is_empty() {
            eprintln!("{}", error.kind);
        } else {
            eprintln!("{}: {}", error.kind, error.message);
        }
    }
    ExitCode::from(exit_status(outcome.exit_code))
}

/// Runs the program embedded in this executable, if it has one. `None` only
/// once its own file has been read and holds no program: an executable that
/// cannot be inspected must not fall back to the CLI in place of its program.
fn run_embedded() -> Option<ExitCode> {
    let fail = |what: &dyn std::fmt::Display| {
        eprintln!("{what}");
        Some(ExitCode::from(EXIT_USAGE))
    };
    let exe = match std::env::current_exe() {
        Ok(exe) => exe,
        Err(error) => return fail(&format_args!("cannot locate own executable: {error}")),
    };
    let entries = match standalone::embedded_program(&exe) {
        Ok(Ok(entries)) => entries?,
        Ok(Err(error)) => return fail(&format_args!("{}: {error}", exe.display())),
        Err(error) => {
            return fail(&format_args!(
                "{}: cannot read own executable: {error}",
                exe.display()
            ));
        }
    };
    let source = entries
        .iter()
        .find(|e| e.name == MAIN_ENTRY)
        .and_then(|e| String::from_utf8(e.bytes.clone()).ok());
    let Some(text) = source else {
        eprintln!(
            "{}: embedded program is corrupt: no {MAIN_ENTRY}",
            exe.display()
        );
        return Some(ExitCode::from(EXIT_USAGE));
    };
    let args: Vec<String> = std::env::args().skip(1).collect();
    // The program was checked at build time; a failure here means the payload
    // does not come from a matching `ferro bundle`.
    Some(match compile_text(Path::new(MAIN_ENTRY), &text) {
        Ok(module) => execute(&text, &module, &args),
        Err(_) => ExitCode::from(EXIT_USAGE),
    })
}

/// `ferro bundle`: check the module, then write this executable with the
/// program embedded (`docs/spec/cli.md`).
fn bundle(path: &Path, out: Option<&Path>) -> ExitCode {
    let (text, _) = match compile(path) {
        Ok(compiled) => compiled,
        Err(code) => return code,
    };
    let out = match out {
        Some(out) => out.to_path_buf(),
        None => default_output(path),
    };
    let written = std::env::current_exe().and_then(|exe| {
        let entries = [standalone::Entry {
            name: MAIN_ENTRY.into(),
            bytes: text.into_bytes(),
        }];
        let bytes = standalone::standalone_bytes(&exe, &entries)?;
        // Sign the staged file: `out` is replaced only by a finished binary.
        standalone::write_standalone(&bytes, &out, sign_for_intel_mac)
    });
    if let Err(error) = written {
        eprintln!("ferro bundle: cannot write {}: {error}", out.display());
        return ExitCode::from(EXIT_USAGE);
    }
    ExitCode::SUCCESS
}

/// `prices_map.bas` → `./prices_map` (`.exe` on Windows).
fn default_output(path: &Path) -> PathBuf {
    let stem = path.file_stem().unwrap_or(path.as_os_str());
    let mut name = stem.to_os_string();
    name.push(std::env::consts::EXE_SUFFIX);
    PathBuf::from(name)
}

/// On Intel Macs `libsui` leaves the binary unsigned: sign it ad hoc, and
/// report a failure rather than writing a binary with a broken signature.
/// On Apple Silicon `libsui` has already signed it.
#[cfg(all(target_os = "macos", not(target_arch = "aarch64")))]
fn sign_for_intel_mac(staged: &Path) -> std::io::Result<()> {
    let status = std::process::Command::new("codesign")
        .args(["--force", "--sign", "-"])
        .arg(staged)
        .status()
        .map_err(|e| std::io::Error::other(format!("cannot run codesign: {e}")))?;
    if status.success() {
        Ok(())
    } else {
        Err(std::io::Error::other(format!(
            "codesign could not sign ({status})"
        )))
    }
}

#[cfg(not(all(target_os = "macos", not(target_arch = "aarch64"))))]
fn sign_for_intel_mac(_staged: &Path) -> std::io::Result<()> {
    Ok(())
}

/// A process exit status from an `ExitApplication2` code: the low 8 bits,
/// as POSIX shells see a JVM exit code.
fn exit_status(code: i32) -> u8 {
    (code & 0xff) as u8
}

/// 1-based line and character column of a byte offset; `\r\n`, `\n` and
/// a lone `\r` each end a line.
fn line_and_column(text: &str, offset: usize) -> (usize, usize) {
    let offset = offset.min(text.len());
    let line_start = text[..offset].rfind(['\n', '\r']).map_or(0, |i| i + 1);
    let before = &text[..line_start];
    let line = before.matches('\n').count() + before.matches('\r').count()
        - before.matches("\r\n").count()
        + 1;
    (line, text[line_start..offset].chars().count() + 1)
}

fn dump_cst(path: &Path) -> ExitCode {
    match read_source(path) {
        Ok(text) => {
            print!(
                "{}",
                ferro_syntax::debug_tree(&ferro_syntax::parse(&text).syntax())
            );
            ExitCode::SUCCESS
        }
        Err(code) => code,
    }
}

/// `path:line:column: error: message`, then the source line and a caret.
/// Columns count characters, starting at 1.
fn render_error(path: &Path, text: &str, range: ferro_syntax::TextRange, message: &str) -> String {
    let offset = usize::from(range.start()).min(text.len());
    let line_start = text[..offset].rfind(['\n', '\r']).map_or(0, |i| i + 1);
    let line_end = text[offset..]
        .find(['\n', '\r'])
        .map_or(text.len(), |i| offset + i);
    let (line_number, column) = line_and_column(text, offset);
    let source_line = text[line_start..line_end].trim_start_matches('\u{feff}');
    let caret_pad: String = text[line_start..offset]
        .trim_start_matches('\u{feff}')
        .chars()
        .map(|c| if c == '\t' { '\t' } else { ' ' })
        .collect();
    format!(
        "{}:{line_number}:{column}: error: {}\n  {source_line}\n  {caret_pad}^\n",
        path.display(),
        message
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_count_every_line_ending() {
        let text = "a\r\nb\nc\rd";
        assert_eq!(line_and_column(text, 0), (1, 1));
        assert_eq!(line_and_column(text, 3), (2, 1));
        assert_eq!(line_and_column(text, 5), (3, 1));
        assert_eq!(line_and_column(text, 7), (4, 1));
        assert_eq!(line_and_column("x = é1", 6), (1, 6));
    }

    #[test]
    fn exit_codes_keep_the_low_byte() {
        assert_eq!(exit_status(0), 0);
        assert_eq!(exit_status(1), 1);
        assert_eq!(exit_status(300), 44);
        assert_eq!(exit_status(-1), 255);
    }

    #[test]
    fn error_locations_count_lines_and_characters() {
        let text = "Sub A\r\n\tx = = 1\r\nEnd Sub\r\n";
        let parse = ferro_syntax::parse(text);
        let error = &parse.errors()[0];
        let rendered = render_error(Path::new("a.bas"), text, error.range, &error.message);
        assert_eq!(
            rendered,
            "a.bas:2:6: error: expected an expression\n  \tx = = 1\n  \t    ^\n"
        );
    }
}
