# Language server

Build with `cargo build --release -p ferro-cli`, then start
`/absolute/path/to/ferro lsp` from an LSP client. No arguments are needed.
Use the plain FerroBASIC executable, not a bundled BASIC program.

The first M1 server publishes Level 0 compiler diagnostics. Completion, hover,
definitions, rename, formatting and debugging are not implemented. There is
no IDE or editor extension to install from this repository yet.

For editor setup, see the [VS Code example](#vs-code-example),
[Zed integration status](#zed-integration-status) or
[Neovim example](#neovim-example) below.

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
  units. CRLF is one line ending; LF and lone CR also work. A line and encoding
  index is built once per analysis and reused for every diagnostic range.
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

- An open URI whose decoded path ends in `.b4j` (case-insensitive) is a
  project: it checks its unsaved header and main code. URI queries and fragments
  do not change the file type, and publications preserve the original URI.
  Only console projects using jCore, with no
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

## VS Code example

Until a dedicated FerroBASIC extension is available, use
[Generic LSP Client (v2)](https://marketplace.visualstudio.com/items?itemName=zsol.vscode-glspc)
to connect VS Code to the server:

1. From the repository root, build the executable:

   ```sh
   cargo build --release -p ferro-cli
   ```

2. Install **Generic LSP Client (v2)** (`zsol.vscode-glspc`) from VS Code's
   Extensions view.
3. Merge the following into your project's `.vscode/settings.json`, replacing
   the generic executable path with the absolute path to your built `ferro`:

   ```json
   {
     "files.associations": {
       "*.bas": "vb",
       "*.b4j": "vb"
     },
     "glspc.server.command": "/absolute/path/to/ferrobasic/target/release/ferro",
     "glspc.server.commandArguments": ["lsp"],
     "glspc.server.languageId": ["vb"]
   }
   ```

   On Windows, use the path to `ferro.exe`; forward slashes in the JSON path
   avoid backslash escaping. The client launches `ferro lsp` automatically;
   you do not need to start the server in a terminal.
4. Run **Developer: Reload Window** from the Command Palette, then open
   `example.bas` with this content:

   ```vb
   Sub Process_Globals
   End Sub

   Sub AppStart (Args() As String)
       Log(Missing)
   End Sub
   ```

   An unresolved-name diagnostic should underline `Missing` and appear in the
   **Problems** panel. Replace `Missing` with `"hello"` without saving: the
   diagnostic should clear. Remove the final `End Sub` to try incomplete-code
   diagnostics.

The bottom-right language selector should show **Visual Basic**. This is a
temporary association: VS Code supplies its built-in Visual Basic syntax
highlighting, while `ferro lsp` supplies FerroBASIC compiler diagnostics.
Highlighting can differ from the FerroBASIC dialect. We have not yet provided
a FerroBASIC syntax grammar or a dedicated editor extension. A future extension
would register a `ferrobasic` language ID, supply the grammar and start the
server automatically, replacing this manual setup.

Keep `glspc.server.languageId` restricted to `vb`. Selecting `plaintext` also
sends unrelated Plain Text buffers, such as `Cargo.lock`, to the BASIC checker
and produces irrelevant diagnostics. This temporary configuration also checks
any other files you open in Visual Basic mode within the workspace.

For startup problems, check **View → Output → Generic LSP Client** and verify
the executable path. After changing the configuration, reload the window to
restart the client and clear diagnostics from the previous setup. Only the
diagnostic features described above are available; the same single-module
limits apply as in other editors.

## Zed integration status

Zed supports LSP servers through language extensions. A FerroBASIC extension
must register the language and its server, then launch
`/absolute/path/to/ferro` with the argument `lsp`. Adding a `ferro` entry to
Zed's `lsp` settings alone does not register that language/server association.
The VS Code Generic LSP Client extension and its settings are specific to
VS Code.

There is currently no FerroBASIC extension or syntax grammar for Zed in this
repository, so there is no ready-to-use Zed configuration to install yet.
Compiler diagnostics come from `ferro lsp`; syntax highlighting would be
provided separately by the editor extension's grammar. The temporary Visual
Basic highlighting in the VS Code example is not a FerroBASIC grammar.

For a local Zed integration, follow Zed's
[language-extension guide](https://zed.dev/docs/extensions/languages#language-servers)
to register the server in `extension.toml` and implement its
`language_server_command` in the extension's Rust code. Load it through
**zed: install dev extension**, as described in
[Developing Extensions](https://zed.dev/docs/extensions/developing-extensions).
Once connected, use the same `example.bas` and unsaved correction from the
VS Code example to exercise diagnostics. A Zed integration has not yet been
tested, and no extension release is scheduled here.

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
