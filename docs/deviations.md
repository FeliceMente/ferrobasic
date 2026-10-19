# Deviations from B4X

Every known, accepted difference between FerroBASIC and the original B4X
toolchain. Each entry needs a conformance test with `status = "deviation"`.

| ID | Area | Target(s) | Original behaviour | FerroBASIC behaviour | Reason | Test |
| --- | --- | --- | --- | --- | --- | --- |
| D1 | Syntax | all | Nesting depth is limited only by javac, whose limit is unknown (open question) | Statements and expressions nested more than 1000 levels deep in a Sub (operator and member chains count one level per operator), or smart strings nested more than 100 deep, are a compile error | Every stage walks the tree recursively; an unbounded depth crashed `ferro` with a stack overflow and made parsing quadratic | `crates/syntax/src/tests.rs` (`nesting_*`), `crates/cli/tests/safety.rs` (`deep_*`); a `status = "deviation"` conformance test needs harness support first |
