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
    let args = value::Value::new_array(Ty::String, 1, args);
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

/// Persistent runtime storage, independent of any compiled program's lifetime.
/// Values stay opaque and retain reference identity across submissions.
#[derive(Default)]
pub struct SessionState {
    globals: Vec<value::Value>,
    schema: Vec<(String, Ty)>,
    records: Vec<(String, Vec<(String, Ty)>)>,
    random: Option<number::JavaRandom>,
    last_exception: Option<std::rc::Rc<value::Exc>>,
}

/// A submission's result; an explicit ExitApplication is distinct from return.
pub struct SessionOutcome {
    pub outcome: Outcome,
    pub exited: bool,
}

impl SessionState {
    /// Executes just init, never AppStart or any previous initializer.
    /// Validation/schema failure leaves persistent state unchanged.
    pub fn execute(
        &mut self,
        program: &Program,
        stdout: &mut dyn Write,
        stderr: &mut dyn Write,
    ) -> SessionOutcome {
        let schema: Vec<_> = program
            .globals
            .iter()
            .map(|g| (g.name.to_ascii_lowercase(), g.ty.clone()))
            .collect();
        let records: Vec<_> = program
            .records
            .iter()
            .map(|r| {
                (
                    r.name.to_ascii_lowercase(),
                    r.fields
                        .iter()
                        .map(|f| (f.name.to_ascii_lowercase(), f.ty.clone()))
                        .collect(),
                )
            })
            .collect();
        let error = ferro_mir::validate(program).err().or_else(|| {
            (!schema.starts_with(&self.schema) || !records.starts_with(&self.records))
                .then(|| "incompatible session storage schema".into())
        });
        if let Some(message) = error {
            return SessionOutcome {
                outcome: failure(value::Exc::internal(message), TextRange::default()),
                exited: false,
            };
        }
        self.schema = schema;
        self.records = records;
        self.globals.extend(
            program.globals[self.globals.len()..]
                .iter()
                .map(|g| convert::default_value(&g.ty, program)),
        );
        // A fresh machine has no frames or address-keyed string cache. Only
        // persistent values, RNG and LastException move across executions.
        let mut machine = machine::Machine::with_globals(
            program,
            std::mem::take(&mut self.globals),
            stdout,
            stderr,
        );
        machine.random = self.random.take();
        machine.last_exception = self.last_exception.take();
        let result = machine.call(program.init, Vec::new());
        machine.out.flush();
        self.globals = std::mem::take(&mut machine.globals);
        self.random = machine.random.take();
        self.last_exception = machine.last_exception.take();
        let (outcome, exited) = match result {
            Ok(()) => (
                Outcome {
                    exit_code: 0,
                    error: None,
                },
                false,
            ),
            Err(machine::Stop::Exit(code)) => (
                Outcome {
                    exit_code: code,
                    error: None,
                },
                true,
            ),
            Err(machine::Stop::Error(error, range)) => (failure(error, range), false),
        };
        SessionOutcome { outcome, exited }
    }
}
