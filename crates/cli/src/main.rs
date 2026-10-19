//! `ferro`: the FerroBASIC command-line tool.
//!
//! `check`, `run` and `bundle` work for Level 0 modules; `build`, `compile`,
//! `targets`, `repl`, `test` and `fmt` are reserved. See `docs/spec/cli.md`.

mod standalone;

use std::ffi::OsString;
use std::io::Write;
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

/// Most compile errors printed for one file; the rest are only counted.
const MAX_ERRORS_SHOWN: usize = 100;

/// Writes to stdout, ignoring errors: a closed pipe (`ferro help | head`)
/// must not turn into a panic or change the exit code.
fn out(text: &str) {
    let _ = std::io::stdout().lock().write_all(text.as_bytes());
}

/// Writes to stderr, ignoring errors, for the same reason as [`out`].
fn err(text: &str) {
    let _ = std::io::stderr().lock().write_all(text.as_bytes());
}

/// Arguments for `AppStart`. Like the JVM, undecodable bytes become U+FFFD
/// rather than stopping the program.
fn program_args(args: &[OsString]) -> Vec<String> {
    args.iter()
        .map(|a| a.to_string_lossy().into_owned())
        .collect()
}

fn main() -> ExitCode {
    // A standalone binary runs its embedded program, with every argument
    // passed to `AppStart`; only a plain `ferro` parses its own CLI.
    if let Some(code) = run_embedded() {
        return code;
    }
    // File names and program arguments may hold any bytes: never assume UTF-8.
    let args: Vec<OsString> = std::env::args_os().skip(1).collect();
    let command = args.first().map(|a| a.to_string_lossy());
    match command.as_deref() {
        Some("version" | "--version" | "-V") => {
            out(&format!(
                "ferro {} ({})\n",
                env!("CARGO_PKG_VERSION"),
                platform()
            ));
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
                    out(&ferro_mir::dump(&ferro_mir::lower(&module)));
                    ExitCode::SUCCESS
                }
                Err(code) => code,
            },
            _ => usage_error("ferro dump-mir: expected exactly one file"),
        },
        Some("run") => match &args[1..] {
            [file] => run(Path::new(file), &[]),
            [file, sep, rest @ ..] if sep == "--" => run(Path::new(file), &program_args(rest)),
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
            out(&usage());
            ExitCode::SUCCESS
        }
        Some(other) => usage_error(&format!("ferro: unknown command '{other}'")),
    }
}

/// A reserved command (`docs/spec/cli.md`): says what is missing and when it
/// arrives.
fn not_implemented(command: &str, detail: &str) -> ExitCode {
    err(&format!("ferro {command}: not implemented yet: {detail}\n"));
    ExitCode::from(EXIT_USAGE)
}

fn usage_error(message: &str) -> ExitCode {
    err(&format!("{message}\n\n{}", usage()));
    ExitCode::from(EXIT_USAGE)
}

fn read_source(path: &Path) -> Result<String, ExitCode> {
    std::fs::read_to_string(path).map_err(|error| {
        err(&format!("ferro: cannot read {}: {error}\n", path.display()));
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
    let lines = LineIndex::new(text);
    let mut report = String::new();
    for (range, message) in errors.iter().take(MAX_ERRORS_SHOWN) {
        report.push_str(&render_error(path, text, &lines, *range, message));
    }
    if errors.len() > MAX_ERRORS_SHOWN {
        report.push_str(&format!(
            "... and {} more error(s) not shown\n",
            errors.len() - MAX_ERRORS_SHOWN
        ));
    }
    match module {
        Some(module) if errors.is_empty() => Ok(module),
        _ => {
            report.push_str(&format!("{} error(s)\n", errors.len()));
            err(&report);
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
        let (line, _) = LineIndex::new(text).locate(text, usize::from(error.range.start()));
        let detail = if error.message.is_empty() {
            error.kind.to_string()
        } else {
            format!("{}: {}", error.kind, error.message)
        };
        err(&format!(
            "Error occurred on line: {line} (Main)\n{detail}\n"
        ));
    }
    ExitCode::from(exit_status(outcome.exit_code))
}

/// Runs the program embedded in this executable, if it has one. `None` only
/// once its own file has been read and holds no program: an executable that
/// cannot be inspected must not fall back to the CLI in place of its program.
fn run_embedded() -> Option<ExitCode> {
    let fail = |what: &dyn std::fmt::Display| {
        err(&format!("{what}\n"));
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
    let Some(entry) = entries.iter().find(|e| e.name == MAIN_ENTRY) else {
        return fail(&format_args!(
            "{}: embedded program is corrupt: no {MAIN_ENTRY}",
            exe.display()
        ));
    };
    let Ok(text) = String::from_utf8(entry.bytes.clone()) else {
        return fail(&format_args!(
            "{}: embedded program is corrupt: {MAIN_ENTRY} is not UTF-8",
            exe.display()
        ));
    };
    let args = program_args(&std::env::args_os().skip(1).collect::<Vec<_>>());
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
        check_output(path, &exe, &out).map_err(std::io::Error::other)?;
        let entries = [standalone::Entry {
            name: MAIN_ENTRY.into(),
            bytes: text.into_bytes(),
        }];
        let bytes = standalone::standalone_bytes(&exe, &entries)?;
        // Sign the staged file: `out` is replaced only by a finished binary.
        standalone::write_standalone(&bytes, &out, sign_for_intel_mac)
    });
    if let Err(error) = written {
        err(&format!(
            "ferro bundle: cannot write {}: {error}\n",
            out.display()
        ));
        return ExitCode::from(EXIT_USAGE);
    }
    ExitCode::SUCCESS
}

/// `ferro bundle` replaces its output, so it refuses an output that is the
/// input itself or the running `ferro`, or that exists and is not a regular
/// file (a directory, a device such as `/dev/null`, a pipe). A symlink is
/// replaced, not followed.
fn check_output(input: &Path, exe: &Path, out: &Path) -> Result<(), String> {
    let Ok(meta) = std::fs::symlink_metadata(out) else {
        return Ok(()); // Nothing there yet.
    };
    if meta.file_type().is_symlink() {
        return Ok(());
    }
    if !meta.is_file() {
        return Err("it exists and is not a regular file".into());
    }
    if same_file(out, input) {
        return Err("it is the input file; choose another output with -o".into());
    }
    if same_file(out, exe) {
        return Err("it is the running ferro executable".into());
    }
    Ok(())
}

/// Whether two paths name the same file (hard links and symlinks included).
fn same_file(a: &Path, b: &Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        match (std::fs::metadata(a), std::fs::metadata(b)) {
            (Ok(a), Ok(b)) => a.dev() == b.dev() && a.ino() == b.ino(),
            _ => false,
        }
    }
    #[cfg(not(unix))]
    {
        match (std::fs::canonicalize(a), std::fs::canonicalize(b)) {
            (Ok(a), Ok(b)) => a == b,
            _ => false,
        }
    }
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

/// Where each line of a source text starts, computed once so that many
/// diagnostics cost no more than one pass. `\r\n`, `\n` and a lone `\r` each
/// end a line.
struct LineIndex {
    starts: Vec<usize>,
}

impl LineIndex {
    fn new(text: &str) -> Self {
        let bytes = text.as_bytes();
        let mut starts = vec![0];
        let mut i = 0;
        while i < bytes.len() {
            match bytes[i] {
                b'\r' if bytes.get(i + 1) == Some(&b'\n') => {
                    starts.push(i + 2);
                    i += 2;
                    continue;
                }
                b'\r' | b'\n' => starts.push(i + 1),
                _ => {}
            }
            i += 1;
        }
        Self { starts }
    }

    /// The 1-based line of a byte offset, and the offset where it starts.
    fn line_of(&self, offset: usize) -> (usize, usize) {
        let line = self.starts.partition_point(|&s| s <= offset);
        (line, self.starts[line - 1])
    }

    /// 1-based line and column of a byte offset. Columns count characters
    /// (Unicode scalar values); the byte order mark that may start the file
    /// is not one of them.
    fn locate(&self, text: &str, offset: usize) -> (usize, usize) {
        let offset = offset.min(text.len());
        let (line, start) = self.line_of(offset);
        let column = without_bom(text, start, &text[start..offset])
            .chars()
            .count()
            + 1;
        (line, column)
    }
}

/// `part`, which begins at byte `start` of `text`, without the byte order mark
/// that may start the file.
fn without_bom<'a>(text: &str, start: usize, part: &'a str) -> &'a str {
    if start == 0 && text.starts_with('\u{feff}') {
        part.strip_prefix('\u{feff}').unwrap_or(part)
    } else {
        part
    }
}

fn dump_cst(path: &Path) -> ExitCode {
    match read_source(path) {
        Ok(text) => {
            out(&ferro_syntax::debug_tree(
                &ferro_syntax::parse(&text).syntax(),
            ));
            ExitCode::SUCCESS
        }
        Err(code) => code,
    }
}

/// `path:line:column: error: message`, then the source line and a caret.
/// Columns count characters, starting at 1.
fn render_error(
    path: &Path,
    text: &str,
    lines: &LineIndex,
    range: ferro_syntax::TextRange,
    message: &str,
) -> String {
    let offset = usize::from(range.start()).min(text.len());
    let (line_number, column) = lines.locate(text, offset);
    let (_, line_start) = lines.line_of(offset);
    let line_end = text[offset..]
        .find(['\n', '\r'])
        .map_or(text.len(), |i| offset + i);
    let source_line = without_bom(text, line_start, &text[line_start..line_end]);
    let caret_pad: String = without_bom(text, line_start, &text[line_start..offset])
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

    fn locate(text: &str, offset: usize) -> (usize, usize) {
        LineIndex::new(text).locate(text, offset)
    }

    #[test]
    fn lines_count_every_line_ending() {
        let text = "a\r\nb\nc\rd";
        assert_eq!(locate(text, 0), (1, 1));
        assert_eq!(locate(text, 3), (2, 1));
        assert_eq!(locate(text, 5), (3, 1));
        assert_eq!(locate(text, 7), (4, 1));
        assert_eq!(locate(text, 8), (4, 2));
        assert_eq!(locate("x = é1", 6), (1, 6));
        // A text ending in a line break: the offset past it is on a new line.
        assert_eq!(locate("a\n", 2), (2, 1));
    }

    #[test]
    fn the_byte_order_mark_is_not_a_column() {
        let text = "\u{feff}x = = 1\n";
        assert_eq!(locate(text, 3), (1, 1));
        assert_eq!(locate(text, 7), (1, 5));
        // Only the mark that starts the file is skipped.
        assert_eq!(locate("a\n\u{feff}b", 5), (2, 2));
        let parse = ferro_syntax::parse(text);
        let error = &parse.errors()[0];
        let lines = LineIndex::new(text);
        let rendered = render_error(
            Path::new("a.bas"),
            text,
            &lines,
            error.range,
            &error.message,
        );
        assert!(rendered.starts_with("a.bas:1:1: error:"), "{rendered}");
        assert!(rendered.ends_with("\n  x = = 1\n  ^\n"), "{rendered}");
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
        let lines = LineIndex::new(text);
        let rendered = render_error(
            Path::new("a.bas"),
            text,
            &lines,
            error.range,
            &error.message,
        );
        assert_eq!(
            rendered,
            "a.bas:2:6: error: expected an expression\n  \tx = = 1\n  \t    ^\n"
        );
    }
}
