//! What the interpreter's integration tests share.

use ferro_interp::Outcome;

/// Parses, checks, lowers and runs `source`, which must compile cleanly;
/// returns stdout and the outcome.
pub fn run(source: &str) -> (String, Outcome) {
    let parse = ferro_syntax::parse(source);
    assert!(parse.errors().is_empty(), "{:?}", parse.errors());
    let (module, diags) = ferro_hir::check(&parse.syntax());
    assert!(diags.is_empty(), "{diags:?}");
    let program = ferro_mir::lower(&module);
    let (mut stdout, mut stderr) = (Vec::new(), Vec::new());
    let outcome = ferro_interp::run(&program, &[], &mut stdout, &mut stderr);
    (String::from_utf8(stdout).unwrap(), outcome)
}
