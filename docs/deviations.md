# Deviations from B4X

Every known, accepted difference between FerroBASIC and the original B4X
toolchain. Each entry needs a conformance test with `status = "deviation"`.

| ID | Area | Target(s) | Original behaviour | FerroBASIC behaviour | Reason | Test |
| --- | --- | --- | --- | --- | --- | --- |
| D1 | Syntax | all | Nesting depth is limited only by javac, whose limit is unknown (open question) | Statements and expressions nested more than 1000 levels deep in a Sub (operator and member chains count one level per operator), or smart strings nested more than 100 deep, are a compile error | Every stage walks the tree recursively; an unbounded depth crashed `ferro` with a stack overflow and made parsing quadratic | `tests/conformance/level0/deep_nesting`; `crates/syntax/src/tests.rs` (`nesting_*`), `crates/cli/tests/safety.rs` (`deep_*`) |
| D2 | Runtime | all | Memory is limited by the JVM heap; an `Int` array element takes 4 bytes, and arrays may hold up to about 2^31 elements | Every array element takes 16 bytes on 64-bit platforms, whatever its type, and arrays hold at most 2^28 elements; running out raises `java.lang.OutOfMemoryError` as in Java | One value representation for every slot keeps the reference interpreter simple; typed storage can come with the compiled backends | `crates/cli/tests/safety.rs` (`running_out_of_memory_is_a_java_error`) |
