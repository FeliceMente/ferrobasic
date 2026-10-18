//! The reference interpreter: runs a MIR [`Program`].
//!
//! It is FerroBASIC's semantic reference (`AGENTS.md`): output must match
//! the B4X recordings and `docs/spec/level0.md` exactly.

use std::io::Write;

use ferro_mir::{Program, TextRange};

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
    pub message: String,
    /// The statement that raised it.
    pub range: TextRange,
}

/// Runs `init`, then `entry` with `args`. `Log` writes to `stdout`,
/// `LogError` to `stderr`. Never panics on a program that passes
/// [`ferro_mir::validate`].
pub fn run(
    program: &Program,
    args: &[String],
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> Outcome {
    let _ = (program, args, stdout, stderr);
    todo!("MIR interpreter (milestone 4d)")
}
