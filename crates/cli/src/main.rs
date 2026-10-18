//! `ferro`: the FerroBASIC command-line tool.
//!
//! Status: `check` parses and type-checks a module; the other commands are
//! stubs. See `docs/spec/cli.md`.

use std::path::Path;
use std::process::ExitCode;

const USAGE: &str = "\
FerroBASIC - a B4X-compatible, multi-target BASIC compiler

Usage: ferro <command> [args]

Commands:
  check <file>      Parse and type-check a module
  build <project>   Build a B4X project (.b4j/.b4a/.b4i/.b4r)
  run <file> [-- <args>...]  Run with the reference interpreter
  repl              Start the interactive REPL
  test              Run conformance tests
  fmt <file>        Format source files
  version           Print the version
";

/// Exit codes from `docs/spec/cli.md`.
const EXIT_USAGE: u8 = 2;
const EXIT_COMPILE_ERRORS: u8 = 3;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("version" | "--version" | "-V") => {
            println!("ferro {}", env!("CARGO_PKG_VERSION"));
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
        Some("run") => match &args[1..] {
            [file] => run(Path::new(file), &[]),
            [file, sep, program_args @ ..] if sep == "--" => run(Path::new(file), program_args),
            _ => usage_error("ferro run: expected `ferro run <file> [-- <args>...]`"),
        },
        Some(cmd @ ("build" | "repl" | "test" | "fmt")) => {
            eprintln!("ferro {cmd}: not implemented yet (milestone M1)");
            ExitCode::from(EXIT_USAGE)
        }
        Some("help" | "--help" | "-h") | None => {
            print!("{USAGE}");
            ExitCode::SUCCESS
        }
        Some(other) => usage_error(&format!("ferro: unknown command '{other}'")),
    }
}

fn usage_error(message: &str) -> ExitCode {
    eprintln!("{message}\n");
    eprint!("{USAGE}");
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
    let parse = ferro_syntax::parse(&text);
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
        eprint!("{}", render_error(path, &text, *range, message));
    }
    match module {
        Some(module) if errors.is_empty() => Ok((text, module)),
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
    let (text, module) = match compile(path) {
        Ok(compiled) => compiled,
        Err(code) => return code,
    };
    let program = ferro_mir::lower(&module);
    let stdout = std::io::stdout();
    let stderr = std::io::stderr();
    let outcome = ferro_interp::run(&program, args, &mut stdout.lock(), &mut stderr.lock());
    if let Some(error) = &outcome.error {
        let (line, _) = line_and_column(&text, usize::from(error.range.start()));
        eprintln!("Error occurred on line: {line} (Main)");
        if error.message.is_empty() {
            eprintln!("{}", error.kind);
        } else {
            eprintln!("{}: {}", error.kind, error.message);
        }
    }
    ExitCode::from(exit_status(outcome.exit_code))
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
