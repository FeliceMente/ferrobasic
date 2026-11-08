# Language server

Build with `cargo build --release -p ferro-cli`, then start
`/absolute/path/to/ferro lsp` from an LSP client. No arguments are needed.
Use the plain FerroBASIC executable, not a bundled BASIC program.

The first M1 server publishes Level 0 compiler diagnostics. Completion, hover,
definitions, rename, formatting and debugging are not implemented. There is
no IDE or editor extension to install from this repository yet.

## Protocol and diagnostics

- LSP over stdin/stdout, using JSON-RPC 2.0 and `Content-Length` framing.
  Stdout contains only protocol frames; transport failures go to stderr.
  Do not merge stderr into stdout or run it through the REPL.
- Supports `initialize`, `initialized`, `textDocument/didOpen`, `didChange`,
  `didClose`, `shutdown` and `exit`. Unknown requests return MethodNotFound;
  unknown notifications are ignored. Requests before initialization return
  ServerNotInitialized. Shutdown returns null, then the client sends exit.
- Advertises **full** document synchronization (`change: 1`), with open/close.
  Send the entire buffer for each change. Incremental ranges are not supported.
  Duplicate opens and malformed changes are ignored; close then reopen to
  reset the document version sequence.
- Stores open-buffer text and checks it with the same header validation,
  parser and HIR checker as `ferro check`. Syntax errors suppress type checking
  until parsing succeeds, just as in the CLI. Incomplete input is diagnosed;
  it does not wait for REPL-style continuation. BASIC code is never executed.
- Publishes `textDocument/publishDiagnostics`, with the original URI, document
  version, error severity, `ferro` source and compiler message. Ranges use
  zero-based lines and columns; compiler empty ranges (including EOF and header
  diagnostics) stay empty. Header/BOM bytes count in editor positions.
- Chooses UTF-8 if the client's `general.positionEncodings` offers it;
  otherwise uses the LSP default, UTF-16. Both ends of every range are converted
  from compiler UTF-8 byte offsets. Supplementary characters count as two UTF-16
  units. CRLF is one line ending; LF and lone CR also work.
- Accepts only increasing document versions; checks and publications run in
  message order, synchronously. Earlier analyses cannot finish after later
  ones. Clients should use the published version to discard diagnostics for
  buffers they have already edited. Large files can delay processing; there is
  no debounce, background analysis or cancellation in this version.
- Publishes an empty list after a successful fix and on close. Closed documents
  are forgotten and later changes are ignored until another open.
- Input bodies are limited to 16 MiB and headers to 8 KiB. Invalid framing or
  JSON ends the server with exit code 2 and a stderr message. Exit is 0 after
  shutdown, 1 without shutdown; EOF uses the same status rule.

## Project and module behavior

`ferro-project::load` can read project metadata, resolve local and relative
module references (including case-insensitive disk lookup), and report missing
files. That is a **loader**, not a multi-module compiler. The current compiler
checks a single module. The server deliberately uses this supported boundary:

- An open URI ending in `.b4j` (case-insensitive) is a project: it checks its
  unsaved header and main code. Only console projects using jCore, with no
  `Module<n>` or `File<n>` entries, pass Level 0 header validation. Unsupported
  entries are diagnosed at the project header line, even if their files exist.
- Every other URI is checked as an independent code module, including module
  header validation (`Type=Class`, for example, is unsupported). As in
  `ferro check file.bas`, file programs require the Level 0 program structure.
  A module expecting names from another module may therefore report missing
  structure or unresolved names; these are not project-wide diagnostics.
- Project discovery, cross-file name/type resolution, workspace scanning,
  library descriptors and diagnostics for unopened files are unavailable.
  Opening a project and its modules does not link their scopes. Workspace roots
  and folders do not change checking. Multi-module compilation is Level 1.
- When project context is absent, ambiguous, unsupported or inaccessible, open
  buffers are still checked independently. No project is silently selected.
  The server does not call the disk loader, so it does not publish its missing
  module or metadata I/O problems. Open the project buffer for header errors.
- Sources are never read from or written to disk: unsaved changes are always
  authoritative, including new files and `untitled:` buffers. Closing a buffer
  clears diagnostics instead of falling back to a disk copy. No save or file
  watcher notification is needed; unopened siblings are never substituted.

## Neovim example

Use Neovim 0.10 or later with its built-in LSP client. Save the following as
`ferro-lsp.lua`, replacing the executable path with your built `ferro`:

```lua
local ferro = "/absolute/path/to/ferro"
vim.api.nvim_create_autocmd("BufReadPost", {
  pattern = { "*.bas", "*.b4j", "*.B4J" },
  callback = function()
    vim.lsp.start({
      name = "ferrobasic",
      cmd = { ferro, "lsp" },
      root_dir = vim.fn.getcwd(),
    })
  end,
})
```

Run `nvim -u NONE --cmd 'luafile ferro-lsp.lua' example.bas`. Edit this program
without saving:

```vb
Sub Process_Globals
End Sub
Sub AppStart (Args() As String)
    Log(Missing)
End Sub
```

The editor displays an unresolved-name error for `Missing`. Replace it with
`"hello"`; the diagnostic clears without a save. `:lua vim.diagnostic.open_float()`
shows the message at the cursor. The root directory is client configuration,
not a promise of project-wide analysis. For untitled buffers, attach manually
with `vim.lsp.start` using the same configuration.

The stdio integration tests in `crates/cli/tests/lsp.rs` exercise initialization,
open/change/close, syntax/type/incomplete diagnostics, versions, rapid edits,
Unicode, project/module limits, protocol purity and shutdown. The example above
is also exercised against Neovim's built-in client during validation.
