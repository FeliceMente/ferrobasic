# FerroBASIC

**A B4X-compatible, multi-target BASIC compiler, written in Rust.**

FerroBASIC aims to build existing B4X projects without changes, and to compile
the same source to the JVM and Android, native desktop (Windows, macOS, Linux),
iOS, WebAssembly and microcontrollers such as the Raspberry Pi Pico. It comes
with a reference interpreter, a REPL and a Language Server, so you can use
VS Code or any LSP-capable editor instead of a dedicated IDE.

> **Status: pre-alpha (milestone M1).** Level 0 programs (one module,
> language core, `List`/`Map`) can be checked, run with the reference
> interpreter and built into standalone executables for the host. Start with
> the [tutorial](docs/tutorial.md); see [`docs/roadmap.md`](docs/roadmap.md)
> for what comes next.

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
  corpus/         the conformance corpus and the real-world corpus's
                  references, as the test harnesses read them; fetches
                  the real-world projects
  cli/            the `ferro` command
tests/conformance/  test programs with recorded expected outputs
tests/corpus/     references to real-world B4X projects (URL, commit,
                  licence, SHA-256), fetched for the tests, never copied
tools/recorder/   records expected outputs from the original toolchain
tools/licenses/   generates THIRD-PARTY-NOTICES.txt
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
ferro bundle prog.bas -o prog     # bundle into a standalone executable
./prog a b
```

A standalone binary is `ferro` with the program appended: it needs neither
FerroBASIC nor a JVM, only the system C library (or nothing at all when
`ferro` is built for `x86_64-unknown-linux-musl`).

> **Note:** `ferro bundle` does not compile. The executable is the `ferro`
> interpreter plus the program's checked source, which is parsed and interpreted at
> start-up exactly as `ferro run` does. So it runs at interpreter speed, and the
> source can be recovered from the file. Native code will come with
> `ferro compile` (M4); `bundle` stays, as the quickest way to ship a program
> that behaves exactly like `ferro run`.

## Working with AI coding agents

Agent instructions live in [`AGENTS.md`](AGENTS.md), which Claude Code, Codex,
Cursor and other tools read automatically. The repository deliberately has no
CLAUDE.md, so Claude Code falls back to AGENTS.md. For Claude Code cloud
sessions, `scripts/cloud-setup.sh` can be used as the environment setup script.

## Documentation

- [Tutorial: the `ferro` command line](docs/tutorial.md)
- [Language support: what works today and what does not yet](docs/language-support.md)
- [Architecture](docs/architecture.md)
- [Porting existing B4X projects](docs/porting.md)
- [Language spec, Level 0](docs/spec/level0.md), [grammar](docs/spec/level0.ebnf) and [built-ins](docs/spec/level0-builtins.md)
- [CLI, interpreter and standalone builds](docs/spec/cli.md) and [project files](docs/spec/project-files.md)
- [Level 0 semantics from public sources](docs/research/level0-semantics.md)
- [Compatibility testing](docs/compatibility-testing.md)
- [Roadmap](docs/roadmap.md) and [changelog](CHANGELOG.md)
- [Open questions](docs/open-questions.md) and [deviations](docs/deviations.md)

## License

FerroBASIC is released under the [MIT License](LICENSE).

Programs you write, check, run, bundle or compile with FerroBASIC remain
yours, under whatever licence you choose. A bundled executable contains a copy
of `ferro`, which includes third-party code under MIT and similar licences
([THIRD-PARTY-NOTICES.txt](THIRD-PARTY-NOTICES.txt)). So when you distribute
one, include the notices `ferro licenses` prints, FerroBASIC's licence
first, for example as a `NOTICES.txt` next to the program:

```sh
ferro licenses > NOTICES.txt
```

Unless you state otherwise, any contribution you submit for inclusion in
FerroBASIC is licensed under the MIT License, without additional terms.
