# FerroBASIC

**A B4X-compatible, multi-target BASIC compiler, written in Rust.**

FerroBASIC aims to build existing B4X projects without changes, and to compile
the same source to the JVM and Android, native desktop (Windows, macOS, Linux),
iOS, WebAssembly and microcontrollers such as the Raspberry Pi Pico. It
currently provides a reference interpreter, a stateful REPL and a diagnostics-only
[language server](docs/language-server.md) for LSP-capable editors.

> **Status: pre-alpha (milestone M1).** Level 0 programs (one module,
> language core, `List`/`Map`) can be checked, run with the reference
> interpreter and built into standalone executables for the host. A stateful
> REPL supports interactive experiments. Start with the [tutorial](docs/tutorial.md),
> including its [interactive REPL section](docs/tutorial.md#9-the-interactive-repl); see [`docs/roadmap.md`](docs/roadmap.md)
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
  project/        reader for B4J project and module headers
  hir/            name resolution, types, target profiles
  mir/            typed IR and lowering
  interp/         reference interpreter and persistent runtime storage
  repl/           interactive submissions, definitions and source mapping
  difftest/       differential runner, normalizer, differ, report
  compare/        compares how two builds of ferro check generated programs
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
runtime, DAP, program generator) and editor integrations are added when
their milestone starts: see `docs/architecture.md` and `docs/roadmap.md`.

## Building

Requires a recent stable Rust toolchain (`rust-toolchain.toml` selects the
`stable` channel with rustfmt and Clippy; it does not pin a version).

```sh
cargo build --workspace
cargo test --workspace
cargo run -p ferro-cli -- --help
```

`cargo test --workspace` runs the non-ignored workspace tests, including
interpreter and standalone conformance and the REPL tutorial transcripts.
Before a PR into `dev`, run `scripts/check-all.sh`: it also runs format,
cross-platform Clippy, explicit conformance reports, the recorder dry run,
fetched real-world corpus tests and the static Linux check. Docs-only changes
may use `scripts/check-all.sh --quick`, skipping the static Linux binary.
See [the testing guide](docs/compatibility-testing.md#local-checks).

After building, use `target/debug/ferro` (or add `target/debug` to `PATH`)
for these commands. Running and bundling a Level 0 program:

```sh
ferro lsp                         # start the stdio language server
ferro repl                        # start an interactive session
ferro check prog.bas              # parse and type-check
ferro run prog.bas -- a b         # run with the reference interpreter
ferro bundle prog.bas -o prog     # bundle into a standalone executable
./prog a b
```

A standalone binary is `ferro` with the program embedded (appended on Linux
and Windows, placed inside the executable on macOS): it needs neither
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
