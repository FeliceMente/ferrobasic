# REPL terminal input decision

The input loop in `crates/repl` already owns grammar-based accumulation and
control commands; `Session::submit` owns checking and execution. Keep those
responsibilities. `terminal.rs` supplies editable input and remembers the engine's
completed submissions; the original stream reader remains the fallback.
No checking/lowering or BASIC semantics change.

## Libraries evaluated

Source/API inspection used published crates from crates.io, not terminal runtime
claims for foreign operating systems.

| Consideration | Rustyline 17.0.2 / 19.0.0 | Reedline 0.49.0 / 0.52.1 |
| --- | --- | --- |
| Platforms | Unix (including Linux/macOS) and Windows console backends | Crossterm backend for Linux, macOS and Windows |
| Editing/history | Cursor editing, bounded memory history, editable multiline entries | Cursor editing, bounded memory history, editable multiline entries |
| Multiline | Validator can request more input; continuation rendering would need extra integration | Validator and explicit multiline prompt interface fit `> ` / `... ` directly |
| Stream routing | Public editor API uses stdout (or controlling terminal), without a stderr output setting | Painter uses stderr; Crossterm cursor queries still use stdout on Unix |
| Restoration | Raw-mode guards; application must decide what interruption means | `read_line` disables raw mode before returning, including errors; editor Drop also cleans up |
| Redirected input | Library fallback exists; FerroBASIC must select its own stream reader to preserve error policy | FerroBASIC selects its own stream reader before creating the editor |
| Dependencies | Smaller graph; default directory, file-history and custom-binding features can be disabled | Larger graph (Crossterm, history, time, Unicode and derive support); optional SQLite/clipboard features stay disabled |
| Licence | MIT | MIT |
| Toolchain | 19 uses edition 2024 and newer dependency requirements; no advertised rust-version | 0.52.1 requires Rust 1.95; 0.49 advertises 1.63 and its resolved graph was checked with Rust 1.85 |

Choose Reedline **0.49.0**, pinned to avoid an accidental toolchain increase,
with default features disabled. Its native multiline prompt/validator model
avoids a custom terminal editor or custom multiline rendering. The lockfile adds
64 third-party packages across all platforms, including `filedescriptor`; this
is a meaningful dependency cost for portable terminal behavior. Direct
Crossterm access only configures keybindings. No completion/highlighter/hinter,
external buffer editor or file history is installed.

Crossterm's Unix cursor-position function writes its query to stdout even though
Reedline paints on stderr. A small `filedescriptor` (MIT) guard temporarily
routes stdout to stderr during `read_line`, flushes and restores stdout before
any BASIC executes, and retries restoration on Drop. Its safe portable API
avoids adding unsafe code to FerroBASIC. The CLI is serial: no BASIC output or
other CLI work runs while this process-wide descriptor routing is active.
Restoration failure is an input error, never permission to execute with changed
routing. Libraries therefore have concrete integration costs, but neither
justifies writing our own editor.

## Interaction policy and validation

The [CLI specification](../spec/cli.md#interactive-repl) defines terminal
selection, EOF, history filtering and control commands. History is a 100-entry
in-memory recall list, independent of runtime state. Before each read, its
contents replace Reedline's navigation cache, excluding automatically saved
control lines and cancelled buffers. This small copy keeps history policy in
the existing submission engine and avoids a custom history backend.

Enter uses the BASIC fragment parser. Control lines outside string literals
return to the engine immediately; `:help` preserves pending BASIC input,
`:cancel` discards it, and `:reset` clears language state while keeping recall.
Ctrl+D signals EOF even with a nonempty editor buffer, diagnosing incomplete
input; Ctrl+Z does the same on Windows. Ctrl+C at a prompt exits 130 after
cleanup. BASIC executes with normal terminal modes and OS interruption behavior.

The Linux PTY suite exercises all eight stdin/stdout/stderr terminal/redirected
combinations, cursor edits, multiline Sub/block recall, control commands, EOF,
Ctrl+C at a prompt and during execution, cursor-query errors and terminal mode
restoration. It also checks an empty private HOME after each session. Unit tests
cover the 100-entry bound, duplicate/blank filtering, parser validation and the
shared input engine. Deterministic tutorial tests continue through the plain
reader. Cross-target compilation is separate from terminal runtime validation.

The editor dependency graph compiles with Rust 1.85 in an isolated probe. The
whole repository currently does not: existing HIR let-chains require a newer
compiler than its declared 1.85. This task does not modify compiler sources or
change the declared toolchain.
