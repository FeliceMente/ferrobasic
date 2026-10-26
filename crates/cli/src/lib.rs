//! The steps `ferro` runs a program with, shared with `ferro-difftest` so
//! that its `interp` target behaves exactly as `ferro run`: what is
//! printed, and how, stays in the `ferro` binary.

use ferro_syntax::{TextRange, TextSize};

/// The exit code of `ferro check`, `run` and `bundle` on a program with
/// errors (`docs/spec/cli.md`).
pub const EXIT_COMPILE_ERRORS: u8 = 3;

/// Stack for all of `ferro`'s work. The parser limits nesting
/// (`MAX_NESTING` in `ferro-syntax`), but the later stages and the
/// interpreter still recurse over trees and values; a large stack keeps any
/// remaining deep recursion from crashing. Only touched pages use memory.
pub const STACK_SIZE: usize = 256 * 1024 * 1024;

/// What Level 0 cannot honour in a header: a project's type, libraries,
/// modules and files, or a module's type. Each error is at the offset of
/// its header line.
pub fn header_errors(text: &str, is_project: bool) -> Vec<(TextRange, String)> {
    let problems = if is_project {
        ferro_project::check_project(text)
    } else {
        ferro_project::check_module(text)
    };
    problems
        .into_iter()
        .map(|p| {
            let at = TextSize::try_from(p.offset).unwrap_or_default();
            (TextRange::empty(at), p.message)
        })
        .collect()
}

/// Parses and type-checks `text`. Its errors follow `header_errors`; the
/// module comes back only when there is none. Type checking runs only on a
/// tree without syntax errors.
pub fn check(
    text: &str,
    header_errors: Vec<(TextRange, String)>,
) -> (Option<ferro_hir::Module>, Vec<(TextRange, String)>) {
    let parse = ferro_syntax::parse(text);
    let mut errors = header_errors;
    let module = if parse.errors().is_empty() {
        let (module, diags) = ferro_hir::check(&parse.syntax());
        errors.extend(diags.into_iter().map(|d| (d.range, d.message)));
        Some(module)
    } else {
        let syntax = parse.errors().iter().map(|e| (e.range, e.message.clone()));
        errors.extend(syntax);
        None
    };
    (module.filter(|_| errors.is_empty()), errors)
}

/// Lowers and interprets a checked module.
pub fn run(
    module: &ferro_hir::Module,
    args: &[String],
    stdout: &mut dyn std::io::Write,
    stderr: &mut dyn std::io::Write,
) -> ferro_interp::Outcome {
    let program = ferro_mir::lower(module);
    ferro_interp::run(&program, args, stdout, stderr)
}

/// A process exit status from an `ExitApplication2` code: the low 8 bits,
/// as POSIX shells see a JVM exit code.
pub fn exit_status(code: i32) -> u8 {
    (code & 0xff) as u8
}
