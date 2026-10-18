# FerroBASIC

**A B4X-compatible, multi-target BASIC compiler, written in Rust.**

FerroBASIC aims to build existing B4X projects without changes, and to compile
the same source to the JVM and Android, native desktop (Windows, macOS, Linux),
iOS, WebAssembly and microcontrollers such as the Raspberry Pi Pico. It comes
with a reference interpreter, a REPL and a Language Server, so you can use
VS Code or any LSP-capable editor instead of a dedicated IDE.

> **Status: pre-alpha (milestone M0).** Nothing works yet beyond the project
> skeleton. See [`docs/roadmap.md`](docs/roadmap.md).

FerroBASIC is an independent project. It is not affiliated with or endorsed by
Anywhere Software, the maker of B4X. "B4X", "B4A", "B4i", "B4J" and "B4R" are
used only to describe compatibility.

## Goals

- Open an existing B4X project folder and build it with no edits.
- One frontend, many backends: JVM/Android, LLVM (native, iOS, WASM, MCUs), interpreter.
- Compatibility measured by differential tests against the original toolchain,
  not claimed.
- First-class editor support through LSP and DAP.

## Repository layout

```text
crates/
  syntax/         lexer, parser, lossless CST
  project/        reader for .b4j/.b4a/.b4i/.b4r and .b4xlib
  hir/            name resolution, types, target profiles
  mir/            typed IR and lowering
  interp/         reference interpreter and REPL
  difftest/       differential runner, normalizer, differ, report
  cli/            the `ferro` command
tests/conformance/  test programs with recorded expected outputs
tests/corpus/     real-world B4X projects (licence-checked)
tools/recorder/   records expected outputs from the original toolchain
docs/             architecture, porting, spec, testing, roadmap
```

Planned crates (library descriptors, layouts, JVM and LLVM backends, native
runtime, LSP, DAP, program generator) and editor integrations are added when
their milestone starts: see `docs/architecture.md` and `docs/roadmap.md`.

## Building

Requires a recent stable Rust toolchain (pinned in `rust-toolchain.toml`).

```sh
cargo build --workspace
cargo test --workspace
cargo run -p ferro-cli -- --help
```

Running and building a Level 0 program:

```sh
ferro check prog.bas              # parse and type-check
ferro run prog.bas -- a b         # run with the reference interpreter
ferro build prog.bas -o prog      # standalone executable for this machine
./prog a b
```

A standalone binary is `ferro` with the program appended: it needs neither
FerroBASIC nor a JVM, only the system C library (or nothing at all when
`ferro` is built for `x86_64-unknown-linux-musl`).

## Working with AI coding agents

Agent instructions live in [`AGENTS.md`](AGENTS.md), which Claude Code, Codex,
Cursor and other tools read automatically. The repository deliberately has no
CLAUDE.md, so Claude Code falls back to AGENTS.md. For Claude Code cloud
sessions, `scripts/cloud-setup.sh` can be used as the environment setup script.

## Documentation

- [Architecture](docs/architecture.md)
- [Porting existing B4X projects](docs/porting.md)
- [Language spec, Level 0](docs/spec/level0.md), [grammar](docs/spec/level0.ebnf) and [built-ins](docs/spec/level0-builtins.md)
- [CLI, interpreter and standalone builds](docs/spec/cli.md)
- [Level 0 semantics from public sources](docs/research/level0-semantics.md)
- [Compatibility testing](docs/compatibility-testing.md)
- [Roadmap](docs/roadmap.md)
- [Open questions](docs/open-questions.md) and [deviations](docs/deviations.md)

## License

Not chosen yet (candidates: Apache-2.0/MIT dual licence, or EUPL-1.2).
Until a LICENSE file is added, all rights are reserved by the authors.
