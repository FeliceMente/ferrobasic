# Roadmap

Work starts at M0, and a milestone closes only when its gate passes; the UI
comes last because it is the most expensive part. Milestones have no dates.
From M2 on, gates are measured by the compatibility report against the original
B4X; M3 is the first milestone where real B4J projects build unchanged.

| Milestone | Scope | Gate |
| --- | --- | --- |
| **M0 · Foundations** | Project-file reader, lexer, parser, lossless CST, Level 0 test corpus, recorder | The whole corpus parses without errors |
| **M1 · Interpreter and REPL** | Level 0 execution, `ferro run`, standalone `ferro bundle` (interpreter + embedded program), REPL, LSP with diagnostics | Conformance tests pass in the interpreter and as standalone binaries |
| **M2 · Classes and events** | Levels 1 and 2: classes, events, Sleep and Wait For | Real B4J console programs match the reference |
| **M3 · JVM backend** | Java backend, library descriptors, original jars, Android; `ferro compile --target jvm`, `ferro build` for projects | Real B4J projects build unchanged and match the reference |
| **M4 · Native LLVM and WASM** | Native runtime with ARC, desktop executables and cross-compilation with `ferro compile --target`, wasm32 | Same output, binaries with no external dependencies |
| **M5 · Microcontrollers** | Embedded profile, Raspberry Pi Pico first | GPIO and serial working on the board |
| **M6 · User interface** | B4X layouts and designer scripts, preview in VS Code | B4XPages projects run unchanged on desktop, Android, web |

## M0 task breakdown

Order changed: the interpreter comes right after the parser, and recording is
deferred, with `.provisional` outputs standing in
(see [compatibility-testing.md](compatibility-testing.md#provisional-outputs)).

- [ ] Collect 5–10 small B4J console projects and document the project-file
      format in `docs/spec/project-files.md`.
- [ ] `ferro-project`: read `.b4j` headers and modules; round-trip tests.
- [x] `ferro-syntax`: token kinds, lexer (case-insensitive keywords, smart
      strings, line continuation, comments as trivia).
- [x] `ferro-syntax`: parser producing a lossless CST for the Level 0 grammar,
      with error recovery.
- [x] Snapshot tests: every file in `tests/conformance` parses and prints back
      byte-identical.
- [x] Level 0 semantics researched from public sources
      ([research/level0-semantics.md](research/level0-semantics.md)); specs
      updated; pending tests with provisional outputs added.
- [ ] `tools/recorder`: wrap a `.bas` in a B4J console project, build and run it
      with the original toolchain, write `.expected` and `.meta.toml`. *Deferred.*
- [ ] CI green on Linux, macOS and Windows. *Deferred.*

## M1 task breakdown (first part)

Acceptance criteria are in [spec/cli.md](spec/cli.md#acceptance-criteria).

- [x] `ferro-project`: read `.bas` module headers and single-module non-UI `.b4j`
      ([spec/project-files.md](spec/project-files.md)); a directory with one
      `.b4j` is an input too.
- [x] `ferro-hir`: name resolution with Sub-wide hoisting, types, conversions,
      Level 0 built-ins from [spec/level0-builtins.md](spec/level0-builtins.md).
      Provisional rules are listed in [open-questions.md](open-questions.md).
- [x] `ferro-mir`: typed IR with explicit `Convert`, lowered loops and smart strings.
- [x] `ferro-interp`: values, number formatting, `List`, `Map`, errors with kinds.
- [x] `ferro check` / `ferro run`: every conformance test with a provisional
      output matches it (`crates/interp/tests/conformance.rs`).
- [x] `ferro bundle`: standalone executable for the host (payload appended to
      `ferro`; embedded with `libsui` on macOS). Verified on Linux x86_64,
      glibc and musl (fully static), and on macOS: arm64, and x86_64 through
      Rosetta. Windows is not tested yet.
- [x] CLI command names settled: `build` renamed `bundle`; `compile`,
      `targets` and project-level `build` reserved, with their plan in
      `docs/spec/cli.md` ("Planned commands").
- [ ] `ferro-difftest --target interp` and `--target standalone` against
      `.expected` / `.provisional`.
