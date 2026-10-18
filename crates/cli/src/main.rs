//! `ferro`: the FerroBASIC command-line tool.
//!
//! Status: stub. Commands are wired but not implemented yet.

use std::process::ExitCode;

const USAGE: &str = "\
FerroBASIC - a B4X-compatible, multi-target BASIC compiler

Usage: ferro <command> [args]

Commands:
  build <project>   Build a B4X project (.b4j/.b4a/.b4i/.b4r)
  run <file|project> Run with the reference interpreter
  repl              Start the interactive REPL
  check <project>   Parse and type-check without building
  test              Run conformance tests
  fmt <file>        Format source files
  version           Print the version
";

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("version") | Some("--version") | Some("-V") => {
            println!("ferro {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        Some(cmd @ ("build" | "run" | "repl" | "check" | "test" | "fmt")) => {
            eprintln!("ferro {cmd}: not implemented yet (milestone M0)");
            ExitCode::from(2)
        }
        Some("help") | Some("--help") | Some("-h") | None => {
            print!("{USAGE}");
            ExitCode::SUCCESS
        }
        Some(other) => {
            eprintln!("ferro: unknown command '{other}'\n");
            eprint!("{USAGE}");
            ExitCode::from(2)
        }
    }
}
