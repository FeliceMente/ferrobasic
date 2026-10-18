# CLI, interpreter and standalone builds (Level 0)

The first deliverable has two parts:

1. **The `ferro` executable**, built for the host (`cargo build --release -p ferro-cli`).
   It checks and runs Level 0 programs with the reference interpreter.
2. **Standalone binaries**: `ferro build` turns a Level 0 program into a single
   native executable for the same OS and architecture as the `ferro` that built it.
   The result runs without FerroBASIC installed and without a JVM.

Both use the interpreter, the semantic reference. Native code generation stays
in M4 ([../roadmap.md](../roadmap.md)). A standalone binary is the `ferro`
runtime plus the embedded program, the same approach as `deno compile`.

## Inputs

Every command takes one `<input>`:

| Input | Meaning |
| --- | --- |
| `file.bas` | One code module. Two forms are accepted: the module members alone (as in `tests/conformance`), or a B4J module file with its header (lines `Key=Value` up to `@EndOfDesignText@`). A header must have `Type=StaticCode` or no `Type` at all. |
| `project.b4j` | A B4J project. The header is read by `ferro-project`; the main module is the text after `@EndOfDesignText@`. |
| `dir/` | A directory containing exactly one `.b4j`. |

Level 0 accepts a `.b4j` only if it is a non-UI project:

- the libraries are only `jCore`;
- there are no extra modules (`NumberOfModules=0`);
- there are no `Files`.

Anything else is rejected with a diagnostic that names the cause, for example:
`project uses library 'jFX': not supported at Level 0`. The value of `AppType`
for non-UI projects is *(open)* until the project-file format is documented
(`docs/spec/project-files.md`, M0).

Source text:

- encoded as UTF-8, with an optional BOM;
- line endings are `\n` or `\r\n` (B4X files use `\r\n`);
- the lossless CST keeps the bytes exactly as read.

## Commands

```text
ferro check <input>                  parse and type-check, print diagnostics
ferro run   <input> [-- <args>...]   check, then interpret; args go to AppStart
ferro build <input> [-o <output>]    check, then write a standalone executable
ferro version | help
```

- `run` and `build` refuse to continue if `check` reports any error.
- `build`:
  - the default output is the input's file stem in the current directory, with
    `.exe` on Windows: `prices_map.bas` → `./prices_map`;
  - `-o` overrides it;
  - an existing output file is replaced;
  - the output gets the executable permission bits on Unix.
- `repl`, `test` and `fmt` stay stubs. `ferro-difftest` drives the
  conformance tests.

## Exit codes

| Code | `ferro check` / `run` / `build` | Standalone binary |
| --- | --- | --- |
| 0 | success / program ended normally | program ended normally |
| *n* | `run`: the program called `ExitApplication2(n)` | the program called `ExitApplication2(n)` |
| 1 | `run`: uncaught runtime error | uncaught runtime error |
| 2 | usage error, unreadable input, cannot write output | corrupt embedded payload |
| 3 | the program has compile errors | — |

The B4J exit code for an uncaught error is *(open)*; 1 is provisional.

## Diagnostics

Compile errors go to stderr, one per problem, as
`path:line:column: error: message`, followed by the source line and a caret.
Warnings use `warning:` and do not fail the command. Messages never use the
B4X product name. Each says what is missing and at which level or target
it arrives.

## Runtime errors

- A runtime error has a **kind**, named after the Java exception B4J would raise:
  - `java.lang.RuntimeException`, e.g. an uninitialised `List`/`Map`;
  - `java.lang.NumberFormatException`;
  - `java.lang.IndexOutOfBoundsException`;
  - `java.lang.StringIndexOutOfBoundsException`;
  - `java.lang.ArithmeticException`;
  - `java.lang.ClassCastException`;
  - `java.lang.UnsupportedOperationException`;
  - `java.lang.ArrayIndexOutOfBoundsException`;
  - `java.lang.ArrayStoreException`;
  - `java.lang.NullPointerException`;
  - `java.util.ConcurrentModificationException`;
  - `java.lang.StackOverflowError`, which `Try` does not catch.

  The kind is what `ferro-difftest` compares. Messages are compared only where
  [level0-builtins.md](level0-builtins.md) quotes them exactly.
- An uncaught error prints to stderr, in the shape B4J uses:

  ```text
  Error occurred on line: <line> (<module>)
  <kind>: <message>
  ```

  It then exits with code 1. stdout produced before the error is kept and flushed.

## Interpreter

- Pipeline:
  - `ferro-syntax` builds the lossless CST;
  - `ferro-hir` resolves names, hoists locals, infers and checks types, and
    marks implicit conversions;
  - `ferro-mir` lowers to the typed MIR: explicit `Convert(from, to)`, no
    `For Each`, no smart strings;
  - `ferro-interp` executes the MIR.
- Values are tagged: `Boolean`, `Byte`, `Short`, `Int`, `Long`, `Float`,
  `Double`, `Char`, `String`, `Null`, and references to `List`, `Map`, arrays
  and `Type` records.
  - A boxed `Object` keeps its runtime tag.
  - Strings must behave as UTF-16 (`Length`, `CharAt`, `SubString` indices);
    the internal representation is free.
- Integer arithmetic wraps like Java. Floating point is IEEE 754 binary64 or
  binary32, with no fused or extended precision.
- `Log` writes through a buffered stdout that is flushed at exit, at
  `ExitApplication`, and before a runtime error is printed.
- Recursion depth is limited. Overflowing it is a runtime error of kind
  `java.lang.StackOverflowError`, not a crash of `ferro`.

## Standalone binary format

`ferro build` writes the bytes of the running `ferro` executable, followed by
a payload and a trailer:

```text
[ ferro executable ][ payload ][ payload length: u64 LE ][ magic: "FERROBASIC\0PKG" ][ format version: u16 LE ]
```

- The **payload** is the checked program as source text. It is the input module,
  or the project header plus main module, as UTF-8 bytes in a small container:
  - a list of `(name: string, bytes)` entries;
  - each string is a length (u32 LE) followed by UTF-8 bytes.

  Source rather than MIR keeps the format free of any IR serialisation. The
  program was already fully checked at build time, so re-parsing at start-up
  cannot fail on a well-formed payload.
- **At start-up** `ferro` reads the end of its own executable
  (`std::env::current_exe`).
  - If the trailer is present, it runs the embedded program: every
    command-line argument goes to `AppStart`, and `ferro`'s own CLI is not
    parsed.
  - If the trailer is absent, it behaves as the normal CLI.
  - A trailer whose version it does not know, or whose length is impossible,
    exits with code 2.
- Linux (ELF) and Windows (PE) ignore appended bytes, so the result runs as is.
  - **macOS** is *(open)*: appended bytes break the code signature, and arm64
    refuses unsigned binaries.
  - Until it is verified, `ferro build` on macOS re-signs ad hoc
    (`codesign --force --sign -`). If that fails, it reports a precise error
    rather than writing a binary that will not start.
- **Dependencies**: the output depends on exactly what `ferro` itself depends on.
  On Linux that is the system C library (glibc), unless `ferro` was built for
  `*-linux-musl`, which makes the output fully static. On Windows it needs no
  extra DLLs.
- Building for another OS or architecture is out of scope here (see M4 and the
  `zig cc` / `cargo-zigbuild` question).

## Acceptance criteria

The first implementation is done when, on Linux x86_64 and in CI:

1. Every `tests/conformance/level0/*.bas`, negative tests included, parses
   into a CST that prints back byte-identical.
2. `ferro check` passes, with no errors, on every test whose status is not
   `negative`.
3. The negative tests are rejected by `ferro check` with exit code 3.
4. `ferro run` on every test that has an `.expected` file or, while none is
   recorded, a `.provisional` file produces identical stdout, and the uncaught
   error named in `uncaught_error` when there is one
   (see [../compatibility-testing.md](../compatibility-testing.md#test-file-layout)).
   Tests with neither file, such as `precedence_open`, are only checked.
5. For every test in point 4, `ferro build` produces a binary whose stdout and
   exit code are identical to `ferro run`'s. The binary must also keep working
   after the build tree and `ferro` itself are removed.
6. `cargo fmt`, `cargo clippy -D warnings` and `cargo test` pass. There is no
   `unsafe` code.
