# Roadmap

Work starts at M0, and a milestone closes only when its gate passes; the UI
comes last because it is the most expensive part. Milestones have no dates.
From M2 on, gates are measured by the compatibility report against the original
B4X; M3 is the first milestone where real B4J projects build unchanged.

M0 and M1 are complete; M2 is under way, planned in the
[M2 task breakdown](#m2-task-breakdown). Versions follow the milestones: the
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
      binaries. [Recording run 24](research/recording-run-24.md) (2026-11-21)
      records the 39 Level 0 probes of
      [#148](https://github.com/FeliceMente/ferrobasic/pull/148). Both
      targets now report 355 tests: 338 passing, 6 documented deviations,
      11 pending, no failures or provisional passes. One pending test is
      recorded and waits for the change that passes it; 10 are new probes.

The conformance gate and the M1 tooling items above are implemented.
REPL history and line editing are implemented with bounded in-memory multiline
recall on supported terminals. The remaining
[deferred REPL improvements](spec/cli.md#deferred-repl-improvements), session file
loading and recoverable execution interruption, have no
assigned milestone or schedule. Run 21 imported
eight expected-output files and one build-time refusal; its retained
[report](research/recording-run-21.md) records 89 selected tests, one build
failure and no disagreements. The refusal is now a negative conformance test.

## M2 task breakdown

Levels 1 and 2 ([spec/level0.md](spec/level0.md#language-levels)): several
modules, classes, events, and resumable subs. The draft specification,
with its sources and the questions still open, is
[spec/level1.md](spec/level1.md). The tracking issue is
[#146](https://github.com/FeliceMente/ferrobasic/issues/146); its tasks A
to D are the first four items. Items are in dependency order: each needs
the ones above it, except where it says otherwise. Every feature lands
with conformance tests and its row in
[language-support.md](language-support.md), as `AGENTS.md` requires, and
every refusal of something still open is a "not known yet" diagnostic.

- [x] **The plan** (task A): this breakdown and the
      [Level 1 spec draft](spec/level1.md), with each rule's source and
      the questions for probes.
- [ ] **Test infrastructure for several modules** (task B). Today the
      recorder wraps one `.bas` in a project with `NumberOfModules=0`
      (`tools/recorder/src/project.rs`). Needed:
  - a layout for a test of several modules in `tests/conformance`
    (owner decision [OD1](#m2-owner-decisions)), and `level1/` and
    `level2/` folders for the new tests;
  - `crates/corpus` reading such a test: its modules, their kinds
    (`Type=StaticCode`, `Type=Class`) and names;
  - the recorder writing `NumberOfModules`, one `ModuleN=` key per module
    and their files, and mapping a builder's line in any module back to
    the test's file (`test_line`);
  - `ferro-difftest`, the front-end harness and `ferro-recorder --dry-run`
    taking such tests, still refused by FerroBASIC (`ferro_rejects`) until
    the features below land.
- [ ] **Level 1 library research** (task C), independent of B: `DateTime`
      (and the `$date{}`, `$time{}`, `$datetime{}` formatters), `Regex`,
      `BytesToString`, `CharsToString`, `GetEnvironmentVariable`,
      `GetSystemProperty`, `SetSystemProperty`, `IsDevTool`, and jCore
      10.70's `Initialized`/`NotInitialized`, from the sources
      [research/level0-semantics.md](research/level0-semantics.md) cites,
      in a new `research/level1-library.md`, with pending probes in
      `tests/conformance/level1/` (single-module programs, which today's
      recorder can record). Owner decision [OD5](#m2-owner-decisions) on
      clock, time zone and environment.
- [ ] **First probe batch** (task D), after B: the
      [questions for probes](spec/level1.md#questions-for-probes) as
      `pending-recording` tests (with `ferro_rejects` where FerroBASIC
      refuses them), their entries in `docs/open-questions.md`, and C's
      library questions.
- [ ] **Recording run 25** on the owner's Windows machine (B4J 10.70), with
      the recorder of task B: the batch above and task C's probes,
      imported as the steward skill says ("Recording runs"). Run 24 came
      first, with today's recorder: it recorded the 39 Level 0 probes of
      [#148](https://github.com/FeliceMente/ferrobasic/pull/148)
      ([report](research/recording-run-24.md)). The spec draft's *(open)* rules then
      become recorded ones or refusals, and `spec/level1.md` stops being a
      draft for what the run settles. Later batches and runs follow each
      item below, as Level 0's did.
- [ ] **Several modules** (Level 1), after the run:
  - `ferro-project`: load a console `.b4j` with its code modules (paths
    as [spec/project-files.md](spec/project-files.md) says); `ferro check`,
    `run` and `bundle` on a project, the bundle embedding every module;
  - `ferro-hir`: a module table; `Module2.F` and `Module2.X`; `Public`,
    `Private` and `Dim` visibility of process variables and Subs; the
    same name in two modules; modules' `Type`s; diagnostics for an
    unprefixed call and for private members, as recorded (Q1–Q8);
  - each module's `Process_Globals`, run once, in the recorded order;
  - `ferro lsp` checking a buffer with its project's other modules
    (today it checks one module: [language-server.md](language-server.md)).
- [ ] **Classes** (Level 1), after several modules:
  - `Type=Class` modules: `Class_Globals`, `Initialize` with any
    signature, instances as references, `Dim` giving an uninitialised one;
  - `Initialize`: the context, then `Class_Globals`, then its body, at
    every call, as recorded (Q12, Q13); `IsInitialized`;
  - `Me`; public and private Subs and variables of an instance;
    properties (`get`/`set`, Q11);
  - instances printed, converted, compared and named by `GetType` as
    recorded (Q15, Q16); class variables of other modules (`Module2.P`,
    Q17).
- [ ] **Events and the message loop** (Level 1), after classes:
  - the interpreter's message loop: one queue, run on the main thread,
    with the scheduler that timers and `Sleep` post to (owner decision
    [OD3](#m2-owner-decisions));
  - `StartMessageLoop`, `StopMessageLoop` (owner decision
    [OD2](#m2-owner-decisions) on their level);
  - name-bound `Prefix_Event` Subs, resolved case-insensitively against
    the module or instance that initialised the object (Q18–Q21);
  - `Sender` (Q22, Q23);
  - `Timer`: `Initialize`, `Enabled`, `Interval`, `Tick`, its errors;
  - uncaught errors in an event, `Application_Error`, and the exit code
    (Q25).
- [ ] **Resumable subs** (Level 2), after events:
  - MIR: a Sub containing `Sleep` or `Wait For` lowered to a state
    machine with a heap frame (its locals and parameters), resumed at a
    state ([architecture.md](architecture.md#semantics-summary)); the
    interpreter runs it from the loop, and the planned backends reuse
    the same lowering;
  - `Sleep`; `Wait For` with and without a sender filter, replacing an
    earlier wait, taking precedence over the event's Sub, binding the
    event's parameters (Q29), mixing filtered and unfiltered waits (Q34);
  - `ResumableSub`, `Wait For (rs) Complete (R As T)`, `Completed`
    (Q26); errors after a pause and `Try` around a pause (Q27, Q28);
  - `CallSub`, `CallSub2`, `CallSub3`, `CallSubDelayed`,
    `CallSubDelayed2`, `CallSubDelayed3`, `SubExists` (Q30–Q33, Q35).
- [ ] **Level 1 library members**, after C's research and recordings, in
      any order with the items above (`DateTime` needs nothing else;
      `Regex` neither): the members listed under task C, with tests.
- [ ] **Tooling and docs**: `ferro check` messages for every Level 1 and
      2 refusal; the REPL's scope (owner decision [OD7](#m2-owner-decisions));
      [language-support.md](language-support.md) rows; a tutorial section
      on a project of several modules; `ferro-compare` against
      `origin/dev` on every change to what `ferro check` accepts.
- [ ] **The gate**: real B4J console programs match the reference.
  - **Choosing them.** Public B4J console programs (`AppType=StandardJava`)
    that, together, use several modules, a class, events with a `Timer`
    or `CallSubDelayed`, and `Sleep` or `Wait For`. Each must: be published
    in a repository under a licence `crates/corpus/src/projects.rs`
    accepts, with none of the exclusions of
    [tests/corpus/README.md](../tests/corpus/README.md) (code another
    author may own, credentials); use only jCore, or libraries FerroBASIC
    implements by then; end on its own, with no input but its
    `#CommandLineArgs`; and print the same output at every run (no
    network, clock, randomness or environment in its output, or only
    where the normalisation of
    [compatibility-testing.md](compatibility-testing.md#normalization)
    covers it). The current corpus gives one candidate, `mteeval-buildlib`
    (four modules, a class); `juros` already passes `ferro check`. The
    others are found by searching GitHub
    (`gh api -X GET search/code -f q='AppType=StandardJava extension:b4j'`)
    and the forum's open-source project threads, and reading each
    candidate before it is proposed. How many, owner decision
    [OD4](#m2-owner-decisions).
  - **Referencing them.** Each in `tests/corpus/corpus.toml` as today
    (repository, full commit, licence and its URL, each file's SHA-256),
    fetched by `ferro-corpus-fetch`, never committed. A gate program also
    names its arguments, if any, and that it belongs to the gate (a new
    key, documented in the README).
  - **Recording them.** The recorder gains a mode that fetches a corpus
    project at its commit, checks its hashes, builds it as it is (no
    wrapping, no edits) and runs it, writing its stdout and exit code with
    the `[recording]` metadata; where the recorded output is kept is owner
    decision [OD4](#m2-owner-decisions).
  - **Passing.** `ferro-difftest` gains a corpus target: the gate is met
    when every gate program's output and exit code match its recording in
    the interpreter and as a standalone binary, with no failure and
    nothing pending, and deviations documented in
    [deviations.md](deviations.md). Then 0.2.0 is released
    ([versions](#roadmap)).

### M2 owner decisions

The plan cannot settle these; each is the owner's to make, and blocks
the item that names it.

- **OD1 — layout of a test of several modules** (blocks task B). For
  example a folder per test, `tests/conformance/level1/<name>/`, holding
  `Main.bas`, the other modules as `<Module>.bas` with their headers, and
  `<name>.meta.toml`, `.expected` beside the folder; or one `.b4j` project
  file per test. The recorder, `crates/corpus` and every harness follow
  the choice.
- **OD2 — the level of `StartMessageLoop` and `StopMessageLoop`.**
  [language-support.md](language-support.md) lists them as Level 2, but a
  `Timer` (Level 1) ticks only inside the loop
  ([spec/level1.md](spec/level1.md#the-message-loop)). The plan puts them
  in Level 1, with events; the row then moves.
- **OD3 — time in the interpreter.** `Sleep(1000)` and a `Timer` can wait
  in real time, as B4J does, or the tests can run on a virtual clock
  that jumps to the next due resumption. And B4J does not order
  resumptions due at nearly the same moment deterministically
  ([spec/level1.md](spec/level1.md#sleep-and-wait-for)): FerroBASIC's
  order (by due time, then by posting order) is then a documented
  deviation, or not, and tests must not depend on it.
- **OD4 — the gate's size and records.** How many real programs (M0
  collected 5–10 projects), whether one that needs a library other than
  jCore can count once that library exists, and whether the recorded
  outputs of third-party programs are committed (under their licence) or
  kept by the owner and fetched like the projects.
- **OD5 — nondeterministic library members in tests.** `DateTime.Now`,
  the time zone, `GetEnvironmentVariable` and system properties differ
  between the recording machine and FerroBASIC's run: whether the
  recorder fixes them (time zone, a known environment variable) or tests
  print only what is stable.
- **OD6 — where Level 1 research lives.** The library research of task
  C goes in `research/level1-library.md`. The language rules of
  [spec/level1.md](spec/level1.md) cite their sources in the spec itself,
  while Level 0 keeps them in
  [research/level0-semantics.md](research/level0-semantics.md): whether
  they move to a research document too, or stay in the spec.
- **OD7 — the REPL and the language server in M2.** Whether the REPL
  accepts class definitions and several modules, or stays a single code
  module in M2; whether project-wide diagnostics in `ferro lsp` are part
  of the gate or follow it.
- **OD8 — releases inside M2.** Whether Level 1 ships as a 0.1.x release
  before Level 2, or M2 ships once, as 0.2.0.
