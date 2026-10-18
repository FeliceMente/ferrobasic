//! The reference interpreter: runs a MIR [`Program`].
//!
//! It is FerroBASIC's semantic reference (`AGENTS.md`): output must match
//! the B4X recordings and `docs/spec/level0.md` exactly.
//!
//! - [`value`]: tagged values; Lists, Maps and StringBuilders follow the
//!   B4J wrapper model.
//! - [`convert`]: defaults and the conversion table.
//! - [`number`]: Java number formatting and parsing, `java.util.Random`.
//! - [`machine`]: the frame stack, exception handlers, operators.
//! - [`builtins`]: the Level 0 built-ins.

use std::io::Write;
use std::rc::Rc;

use ferro_mir::{Program, TextRange, Ty};

mod builtins;
mod convert;
mod machine;
mod number;
mod strings;
mod value;

#[cfg(test)]
mod tests;

/// How a run ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    /// 0 when `AppStart` returns, the code of `ExitApplication2`, or 1 after
    /// an uncaught runtime error (`docs/spec/cli.md`).
    pub exit_code: i32,
    /// The uncaught runtime error, if the program stopped on one.
    pub error: Option<RuntimeError>,
}

/// A runtime error, named after the Java exception B4J would raise.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeError {
    /// e.g. `java.lang.RuntimeException`, `java.lang.NumberFormatException`.
    pub kind: String,
    /// Empty for an exception without a message (Java `getMessage()` null).
    pub message: String,
    /// The statement that raised it.
    pub range: TextRange,
}

/// Runs `init`, then `entry` with `args`. `Log` writes to `stdout`,
/// `LogError` to `stderr`. Never panics on a program that passes
/// [`ferro_mir::validate`].
///
/// A program that fails validation is not run: the outcome is exit code 1
/// with an error of kind `ferro.InternalError`.
pub fn run(
    program: &Program,
    args: &[String],
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> Outcome {
    if let Err(message) = ferro_mir::validate(program) {
        return failure(value::Exc::internal(message), TextRange::default());
    }
    let mut m = machine::Machine::new(program, stdout, stderr);
    let args: Vec<value::Value> = args.iter().map(|a| value::Value::str(a)).collect();
    let args = value::Value::Array(Rc::new(value::ArrayObj {
        elem: Ty::String,
        dims: vec![args.len()],
        items: Rc::new(std::cell::RefCell::new(args)),
    }));
    let result = m
        .call(program.init, Vec::new())
        .and_then(|()| m.call(program.entry, vec![args]));
    m.out.flush();
    match result {
        Ok(()) => Outcome {
            exit_code: 0,
            error: None,
        },
        Err(machine::Stop::Exit(code)) => Outcome {
            exit_code: code,
            error: None,
        },
        Err(machine::Stop::Error(e, range)) => failure(e, range),
    }
}

fn failure(e: value::Exc, range: TextRange) -> Outcome {
    Outcome {
        exit_code: 1,
        error: Some(RuntimeError {
            kind: e.kind.to_string(),
            message: e.message.unwrap_or_default(),
            range,
        }),
    }
}
