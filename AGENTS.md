# FerroBASIC — instructions for coding agents

These instructions apply to every AI coding agent working in this repository
(Claude Code, Codex, Cursor, Copilot, …). Keep this file current: update it at
every milestone. Do not add a CLAUDE.md, or Claude Code will stop reading this file.

## Context

- FerroBASIC is a compiler + interpreter + REPL + Language Server for a BASIC
  dialect that is source-compatible with B4X. There is no IDE.
- The defining promise: an existing B4X project (`.b4j`, `.b4a`, `.b4i`, `.b4r`)
  builds **without changes** to code, project files or layouts.
- Pipeline: `syntax` (lossless CST) -> `hir` (names, types, target profiles)
  -> `mir` (typed IR) -> `interp` | `backend-java` | `backend-llvm`.
- Language: Rust (stable, edition 2024). Workspace in `crates/`, one crate per
  component, package names prefixed `ferro-`. The CLI binary is `ferro`.
- Current milestone: **M0 → M1** — lexer, parser, lossless CST, then the
  Level 0 interpreter, `ferro run` and standalone `ferro bundle` (see
  `docs/spec/cli.md`). The recorder is deferred. See `docs/roadmap.md`.

## Read before working

- `docs/architecture.md` — components, IR, backends, tooling.
- `docs/porting.md` — what "builds without changes" requires, layer by layer.
- `docs/spec/level0.md` and `docs/spec/level0.ebnf` — the current language subset.
- `docs/spec/level0-builtins.md` — keywords and library members at Level 0.
- `docs/spec/cli.md` — `ferro check/run/build`, exit codes, standalone binaries.
- `docs/research/level0-semantics.md` — where each Level 0 rule comes from.
- `docs/compatibility-testing.md` — how compatibility is measured.
- `docs/open-questions.md` — unresolved B4X semantics. Check here before guessing.

## Sources of truth

- `tests/conformance/` holds `*.bas` programs with `*.expected` outputs recorded
  from the ORIGINAL B4X toolchain (`*.meta.toml` records the B4X version).
  Expected behaviour is what those files say, not your memory of B4X.
- Never write or edit a `*.expected` file by hand: only `tools/recorder` does.
- Until recordings exist, `*.provisional` files hold hand-derived expected
  output, only for behaviour settled in `docs/research/level0-semantics.md`.
  Never write one for an open question, never change one to make a test pass,
  and never let it override an `*.expected` file (see
  `docs/compatibility-testing.md`).
- The interpreter is FerroBASIC's semantic reference: every backend must
  produce the same output as the interpreter and as the recording.
- Library `.xml` descriptors are the type-checking interface for libraries;
  reimplemented libraries ship identical descriptors.

## Rules

- Never require changes to B4X sources, project files or layouts. An unsupported
  construct gets a precise diagnostic (what is missing, on which target) and an
  entry in `docs/deviations.md`. Never build something that silently behaves differently.
- Every language feature lands with at least one conformance test.
- If B4X semantics are unclear, do NOT invent them: add a test with
  `status = "pending-recording"` in its `.meta.toml` and an entry in
  `docs/open-questions.md`.
- A mismatch found by the program generator is minimized and kept as a regression test.
- The CST is lossless (whitespace and comments included): LSP and the formatter
  depend on it.
- Implicit conversions become explicit MIR nodes (`Convert(from, to)`).
- No target-specific code in `syntax`, `hir` or `mir`.
- `crates/runtime` (planned, M4) must stay `no_std`-compatible.
- No `unsafe` outside crates that explicitly opt in and document why.
- Keep external dependencies few and justified in the PR description.
- Never use the B4X name in binaries, crate names or user-facing messages;
  "B4X-compatible" in documentation is fine.
- Never commit third-party B4X libraries, B4X binaries or corpus projects whose
  licence does not allow redistribution.

## Commands

```sh
cargo build --workspace
cargo test --workspace
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo run -p ferro-cli -- run tests/conformance/level0/<file>.bas
cargo run -p ferro-cli -- bundle tests/conformance/level0/<file>.bas -o /tmp/prog
cargo run -p ferro-difftest -- --target interp   # compare against recordings (once implemented)
```

## Before closing a task

1. `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`,
   `cargo test --workspace` all pass.
2. New behaviour has conformance tests; open questions and deviations are recorded.
3. Docs in `docs/` reflect any design change; `docs/tutorial.md` shows real
   `ferro` output, so re-run its examples when CLI behaviour changes.

## Commits and PRs

- Small, focused commits; message in the imperative ("Add lexer for smart strings").
- One topic per PR; describe what changed, why, and how it was tested.
