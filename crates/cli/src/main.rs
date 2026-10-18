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
  run <file|project> Run with the reference interpreter
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
        Some(cmd @ ("build" | "run" | "repl" | "test" | "fmt")) => {
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

fn check(path: &Path) -> ExitCode {
    let text = match read_source(path) {
        Ok(text) => text,
        Err(code) => return code,
    };
    let parse = ferro_syntax::parse(&text);
    // Type checking runs only on a tree without syntax errors.
    let errors: Vec<(ferro_syntax::TextRange, String)> = if parse.errors().is_empty() {
        let (_, diags) = ferro_hir::check(&parse.syntax());
        diags.into_iter().map(|d| (d.range, d.message)).collect()
    } else {
        parse
            .errors()
            .iter()
            .map(|e| (e.range, e.message.clone()))
            .collect()
    };
    for (range, message) in &errors {
        eprint!("{}", render_error(path, &text, *range, message));
    }
    if errors.is_empty() {
        ExitCode::SUCCESS
    } else {
        eprintln!("{} error(s)", errors.len());
        ExitCode::from(EXIT_COMPILE_ERRORS)
    }
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
    let line_number = text[..line_start].matches('\n').count()
        + text[..line_start].matches('\r').count()
        - text[..line_start].matches("\r\n").count()
        + 1;
    let column = text[line_start..offset].chars().count() + 1;
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
