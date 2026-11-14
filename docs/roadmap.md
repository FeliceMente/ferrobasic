# Roadmap

Work starts at M0, and a milestone closes only when its gate passes; the UI
comes last because it is the most expensive part. Milestones have no dates.
From M2 on, gates are measured by the compatibility report against the original
B4X; M3 is the first milestone where real B4J projects build unchanged.

M0 and M1 are complete; M2 is next. Versions follow the milestones: the
release that completes a milestone raises the minor version (0.1.0 for M0
and M1, 0.2.0 for M2), and releases in between raise the patch version.

| Milestone | Scope | Gate |
| --- | --- | --- |
| **M0 · Foundations** | Project-file reader, lexer, parser, lossless CST, Level 0 test corpus, recorder | The whole corpus parses without errors |
| **M1 · Interpreter and REPL** | Level 0 execution, `ferro run`, standalone `ferro bundle` (interpreter + embedded program), REPL, LSP with diagnostics | Conformance tests pass in the interpreter and as standalone binaries |
| **M2 · Classes and events** | Levels 1 and 2: classes, events, Sleep and Wait For | Real B4J console programs match the reference |
| **M3 · JVM backend** | Java backend, library descriptors, original jars, Android; `ferro compile --target jvm`, `ferro build` for projects | Real B4J projects build unchanged and match the reference |
| **M4 · Native LLVM and WASM** | Native runtime with ARC, desktop executables and cross-compilation with `ferro compile --target`, wasm32 | Same output, binaries with no external dependencies |
| **M5 · Microcontrollers** | Embedded profile, Raspberry Pi Pico first | GPIO and serial working on the board |
| **M6 · User interface** | B4X layouts and designer scripts, preview in VS Code | B4XPages projects run unchanged on desktop, Android, web |

## M0 task breakdown (complete)

Order changed: the interpreter comes right after the parser, and recording
came later, with `.provisional` outputs standing in
(see [compatibility-testing.md](compatibility-testing.md#provisional-outputs)).

- [x] Collect 5–10 small B4J console projects and document the project-file
      format in `docs/spec/project-files.md`: six projects, saved by B4J 2.18
      to 10.5, referenced in `tests/corpus/corpus.toml` (original URL,
      commit, licence, SHA-256) and fetched for the tests.
- [x] `ferro-project`: read `.b4j` headers and modules; round-trip tests
      (byte for byte, on hand-written files and on the real-world corpus).
- [x] `ferro-syntax`: token kinds, lexer (case-insensitive keywords, smart
      strings, line continuation, comments as trivia).
- [x] `ferro-syntax`: parser producing a lossless CST for the Level 0 grammar,
      with error recovery.
- [x] Snapshot tests: every file in `tests/conformance` parses and prints back
      byte-identical.
- [x] Level 0 semantics researched from public sources
      ([research/level0-semantics.md](research/level0-semantics.md)); specs
      updated; pending tests with provisional outputs added.
- [x] `tools/recorder`: wrap a `.bas` in a B4J console project, build and run it
      with the original toolchain, write `.expected` and `.meta.toml`.
- [x] First recordings of the Level 0 corpus (B4J 10.70 on Windows 10,
      OpenJDK 19): 24 tests recorded and matching. The other recordings land
      with the fixes they call for.
- [x] CI green on Linux, macOS and Windows (`.github/workflows/ci.yml`;
      runs on release pull requests and on demand).

## M1 task breakdown (first part, complete)

Acceptance criteria are in [spec/cli.md](spec/cli.md#acceptance-criteria).

- [x] `ferro-project`: read `.bas` module headers and single-module non-UI `.b4j`
      ([spec/project-files.md](spec/project-files.md)); a directory with one
      `.b4j` is an input too.
- [x] `ferro-hir`: name resolution with Sub-wide hoisting, types, conversions,
      Level 0 built-ins from [spec/level0-builtins.md](spec/level0-builtins.md).
      Provisional rules are listed in [open-questions.md](open-questions.md).
- [x] `ferro-mir`: typed IR with explicit `Convert`, lowered loops and smart strings.
- [x] `ferro-interp`: values, number formatting, `List`, `Map`, errors with kinds.
- [x] `ferro check` / `ferro run`: conformance matches the recordings, with
      documented deviations (`ferro-difftest`, in `crates/difftest`).
- [x] `ferro bundle`: standalone executable for the host (payload appended to
      `ferro`; embedded with `libsui` on macOS). Verified on Linux x86_64,
      glibc and musl (fully static), on Windows x64, and on macOS arm64 and
      x86_64.
- [x] CLI command names settled: `build` renamed `bundle`; `compile`,
      `targets` and project-level `build` reserved, with their plan in
      `docs/spec/cli.md` ("Planned commands").
- [x] `ferro-difftest --target interp` and `--target standalone` against
      `.expected` / `.provisional`, with the report by level and feature
      ([compatibility-testing.md](compatibility-testing.md#the-report)).
- [x] `Const` declarations at Level 0
      ([#33](https://github.com/FeliceMente/ferrobasic/issues/33)), as
      recording runs 12 and 13 settled them.

## M1 task breakdown (second part, complete)

The tooling plan is in [architecture.md](architecture.md#tooling-repl-lsp-dap-vs-code).

- [x] `ferro repl` on the interpreter: statements, bare expressions, Subs
      and Types, with persistent state and interactive error recovery.
      Specified in `docs/spec/cli.md`; [tutorial](tutorial.md#9-the-interactive-repl).
- [x] Language Server with diagnostics: `ferro lsp` shares `ferro check`,
      checking open buffers as the user types. [Setup and limits](language-server.md).
      Multi-module linking remains Level 1; it is not supplied by the server.
- [x] Conformance gate: the conformance tests pass in the interpreter and as standalone
      binaries. Recording run 21 (2026-11-08) settles the remaining probes:
      287 tests, 281 pass, 6 documented deviations, none pending or passing
      against provisional output, and none failing on either target.

The conformance gate and the M1 tooling items above are implemented.
REPL history and line editing are implemented with bounded in-memory multiline
recall on supported terminals. The remaining
[deferred REPL improvements](spec/cli.md#deferred-repl-improvements), session file
loading and recoverable execution interruption, have no
assigned milestone or schedule. Run 21 imported
eight expected-output files and one build-time refusal; its retained
[report](research/recording-run-21.md) records 89 selected tests, one build
failure and no disagreements. The refusal is now a negative conformance test.
