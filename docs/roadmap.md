# Roadmap

Work starts at M0, and a milestone closes only when its gate passes; the UI
comes last because it is the most expensive part. Milestones have no dates.
From M2 on, gates are measured by the compatibility report against the original
B4X; M3 is the first milestone where real B4J projects build unchanged.

| Milestone | Scope | Gate |
| --- | --- | --- |
| **M0 · Foundations** | Project-file reader, lexer, parser, lossless CST, Level 0 test corpus, recorder | The whole corpus parses without errors |
| **M1 · Interpreter and REPL** | Level 0 execution, REPL, LSP with diagnostics | Conformance tests pass in the interpreter |
| **M2 · Classes and events** | Levels 1 and 2: classes, events, Sleep and Wait For | Real B4J console programs match the reference |
| **M3 · JVM backend** | Java backend, library descriptors, original jars, Android | Real B4J projects build unchanged and match the reference |
| **M4 · Native LLVM and WASM** | Native runtime with ARC, desktop executables, wasm32 | Same output, binaries with no external dependencies |
| **M5 · Microcontrollers** | Embedded profile, Raspberry Pi Pico first | GPIO and serial working on the board |
| **M6 · User interface** | B4X layouts and designer scripts, preview in VS Code | B4XPages projects run unchanged on desktop, Android, web |

## M0 task breakdown

- [ ] Collect 5–10 small B4J console projects and document the project-file
      format in `docs/spec/project-files.md`.
- [ ] `ferro-project`: read `.b4j` headers and modules; round-trip tests.
- [ ] `ferro-syntax`: token kinds, lexer (case-insensitive keywords, smart
      strings, line continuation, comments as trivia).
- [ ] `ferro-syntax`: parser producing a lossless CST for the Level 0 grammar,
      with error recovery.
- [ ] Snapshot tests: every file in `tests/conformance` parses and prints back
      byte-identical.
- [ ] `tools/recorder`: wrap a `.bas` in a B4J console project, build and run it
      with the original toolchain, write `.expected` and `.meta.toml`.
- [ ] CI green on Linux, macOS and Windows.
