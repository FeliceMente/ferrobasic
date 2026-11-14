# Changelog

FerroBASIC is pre-alpha: every release can change behaviour, and the
command line may still change. Expected outputs are recorded from B4J
10.70 (Windows 10, OpenJDK 19) by `tools/recorder`; a test B4J has not run
yet is pending until the next recording run (`docs/compatibility-testing.md`).

## Unreleased

### Tools

- `ferro repl` supplies terminal line editing and editable history of complete
  BASIC submissions, including multiline Subs and blocks. History keeps up to
  100 nonblank submissions in memory, skips consecutive exact duplicates, and
  survives `:reset`; it is never saved to disk. Prompts/editing stay on stderr
  and BASIC output on stdout. Redirected input retains its plain, prompt-free,
  stop-on-error behavior. See `docs/spec/cli.md` for terminal fallback and EOF.

### Diagnostics

- A name declared by two `Const`s in `Process_Globals` is refused as "declared by two `Const`s", no longer as "declared both by a `Const` and by a `Dim`".

## 0.0.7 (pre-alpha)

M1 is complete: `ferro repl` and `ferro lsp` join `check`, `run` and
`bundle`, and recording run 21 answered the last pending tests.

### Tools

- `ferro repl` starts an interactive Level 0 session
  (`docs/tutorial.md`, section 9). It runs statements, prints bare
  expressions with `Log`'s conversions, and accepts multiline blocks,
  complete Subs and `Type` declarations, with a continuation prompt while
  a construct is incomplete. Variables, collections and definitions
  persist between submissions; a Sub can be replaced with the same
  signature. A compile error leaves the session unchanged, and a runtime
  error keeps what ran before it. `:help`, `:cancel`, `:reset` and
  `:quit` are control lines. Piped input stops at the first error, with
  `run`'s exit codes (`docs/spec/cli.md`). History, line editing and
  recoverable cancellation are not there yet.
- `ferro lsp` is a language server over stdio that publishes the same
  diagnostics as `ferro check` for each open buffer, unsaved text
  included, with full-document sync and UTF-8 or UTF-16 positions. It
  checks one module at a time: a `.b4j` buffer as a project, anything
  else as a module, with no cross-module names. Completion, hover and
  the other editor features are not implemented, and there is no editor
  extension: `docs/language-server.md` shows how to connect VS Code
  (through a generic LSP client) and Neovim, and Zed's status. It adds
  `serde_json` as a dependency.

### Runtime, as in B4J

- A value held in an `Object` converts to a number, a `Boolean` or a
  `Char` through its text, as B4J's `BA.ObjectToNumber`,
  `ObjectToLongNumber`, `ObjectToBoolean` and `ObjectToChar` parse
  `String.valueOf(o)`:
  - to a number, `Null` (`"null"`), a `List`, a `Map` or a `Boolean`
    raise `NumberFormatException`, and a `Char` `"7"` is 7. Before, they
    raised `ClassCastException` or `NullPointerException`.
  - to `Long`, the text is parsed as `Long.parseLong` does: `"1.5"`
    raises `NumberFormatException`, while to `Int` it gives 1. A direct
    `String`-to-`Long` conversion still parses a Double and narrows it,
    so `"1.5"` gives 1.
  - to `Boolean`, text other than `true` or `false` raises
    `RuntimeException` ("Cannot parse: … as boolean").
  - to `Char`, the first character of the text: an `Object` holding 65
    gives `"6"`.

### Language, as in B4J

- A `For Each` without `As` in `Process_Globals` over a name that is a
  `Type`'s declares an `Object` module variable; the `Type` stays usable.
- A `For Each` over a collection whose static type is `Object` is an
  error at build time, as in B4J, whose generated Java does not compile.
  Before, an `Object` holding a number stopped FerroBASIC with an
  internal error at run time.

### Recorded from B4J

- Recording run 21 settled the last pending tests. 287 conformance tests:
  281 pass against recordings, in the interpreter and as standalone
  binaries, 6 are documented deviations (D1, D3–D7), and none are
  pending.

### Testing and tools

- `ferro-compare` retries starting `ferro` when the system reports the
  freshly built binary as busy (`ETXTBSY`), which made it fail at random
  on CI. This fix also shipped in 0.0.6's code on `main`.
- The documentation was audited against `dev`: `docs/roadmap.md` marks M1
  complete, and the guides describe what is implemented today.

## 0.0.6 (pre-alpha)

Recording runs 14 to 20 (B4J 10.70) answered 131 pending tests; all of
them are applied.

### Language, as in B4J

- `Select Case x` is a synonym of `Select x`, and `Each` may be a name, as
  `In` already could.
- An assignment to a name nothing declares declares it, as a `String`.
  A read of it before anything declares or assigns it is an error; when a
  later `For Each` without `As` declares the name, with B4J's message
  ("Undeclared variable … is used before it was assigned any value.").
- A Sub without `As` returns a `String`: `Return <value>` builds, and its
  call has a value. `Process_Globals` and `AppStart` still return nothing.
- `Dim o As Object` with no value gives a new `java.lang.Object`, local or
  module variable, and so does an `Object` field of a `Type` when
  `Initialize` runs. New Objects compare by identity, also as `Map` keys
  and in `IndexOf`, print as `java.lang.Object@<hash>`, and converted to
  a number raise `NumberFormatException`. Before, they were `Null`
  (deviation D8, now gone).
- A `For Each` without `As` declares its variable an `Object`:
  - over a name nothing declares, in a Sub or in `Process_Globals`, where
    it is a module variable every Sub sees;
  - over an `Object`: a local, a parameter, or a module variable however
    and wherever it is declared, also when a `Dim` or a typed loop
    declares it after the loop;
  - over a variable of another type it is an error, as in B4J ("Current
    declaration does not match previous one"), and so is one after an
    assignment or a read of the name. Before, it was accepted.
- In `Process_Globals`, a name declared twice is one variable, each
  declaration setting its value in turn: two `Dim`s; a `Const` of an
  `Int` or a `String`, then a `Dim` of its type, with a value or not, also
  inside an `If`; an `Int` `Dim`, then a `Const`, each with a literal
  value; or two `Int` `Const`s. After two `Dim`s, or a `Const` then a
  `Dim`, a Sub may assign it; after a `Dim` then a `Const` that is an
  error, as in B4J. A read or an assignment there before a
  `Dim … As Object` builds too.
- A counter of a `For` over an undeclared name declares an `Int` where the
  loop is, in source order: a later `Dim`, typed `For` or `For Each` of
  another type is an error, as in B4J.
- Every `For` and `For Each v As T` over a loop variable named like a Sub
  builds where the variable's own rules allow it, also nested, three in a
  row, or from another Sub; only a `For Each` without `As` over it is an
  error.
- A loop variable of `Process_Globals` may have a `Type`'s name, and the
  `Type` stays usable; one named `Bit` is an error, as in B4J.

### Runtime, as in B4J

- `For` evaluates its step, then its limit, then its start, which it
  assigns last, so a limit or step reading the counter reads its value
  before the loop. `Select` evaluates every `Case` value, in order, before
  comparing any.
- `<`, `>`, `<=` and `>=` compare numbers, parsing a `String` operand:
  `"a" < "b"` raises `NumberFormatException`, and `"10" < 9` is `False`.
  Ordering a `Boolean` or a `Char` is an error; comparisons do not chain
  (`a = b = c` is an error).
- A number of any type `=` or `<>` `Null` raises `NumberFormatException`,
  as `BA.ObjectToNumber` parses `"null"`.
- A `Char` compared with a number takes the first character of the
  number's text, as `BA.ObjectToChar` does: `"A" = 65` is `False`, and
  `"1" = 0.00001` is `True` (from `1.0E-5`). With the number on the left
  it is an error.
- A `List` used as a `Map` key keys by its contents, as Java's `ArrayList`
  does, also when it changes after being put; a `Map` used as a key keys
  by identity.
- `GetType` of a record names its class `<package>.main$_<name>`, from the
  project's package (`b4j.example` for a lone module).

### Recorded from B4J

- 131 tests recorded in runs 14 to 20, which settle the cases above. 272
  conformance tests now pass against recordings, in the interpreter and
  as standalone binaries (outputs, and refusals with B4J's error), 6 are
  documented deviations (D1, D3–D7), and 2 probes wait for the next run.
- Shapes whose behaviour neither a recording nor the research settles are
  refused with a "not known yet" message and a pending test, as before. Some programs accepted in 0.0.5 that B4J
  refuses are now errors, listed above.

### Testing and tools

- `scripts/check-all.sh` runs every check a pull request into `dev` needs:
  format, clippy for Linux, Windows and both macOS targets, the tests,
  `ferro-difftest` in the interpreter and as standalone binaries, the
  recorder's dry run, the real-world corpus, and the static Linux binary
  (`scripts/check-static-linux.sh`, shared with CI). `--offline` and
  `--quick` skip the download and the static build.
- `ferro-compare` builds `ferro` from a git ref (`origin/dev` by default)
  and checks about 94,000 generated programs with both builds, in four
  families (constants, loops named like a Sub, comparisons,
  declarations), reporting what is newly accepted or refused, what gets
  more or fewer errors or other messages, and crashes.
- `.claude/skills/steward/SKILL.md` says how an agent drives a pull
  request, a recording run and a release under `AGENTS.md`'s rules.

### Known limitations

- Two pending tests wait for recording run 21: a `For Each` without `As`
  in `Process_Globals` over a `Type`'s name, refused until then, and a
  `For Each` over an `Object` that holds a number, where FerroBASIC stops
  with an internal error instead of a Java exception.
- Converting an `Object` that holds a `List` or `Null` to a number raises
  `ClassCastException` or `NullPointerException`, where B4J's
  `ObjectToNumber` raises `NumberFormatException`.
- Classes, other modules, libraries other than jCore, `DateTime`, `Sleep`
  and `Wait For` arrive at later levels. The REPL and the Language Server
  are next (M1).

## 0.0.5 (pre-alpha)

### Language, as in B4J

- `Const` declarations are supported, as recorded from B4J (#33): every
  form (`Private Const`, `Public Const`, `Dim Const`, several names, the
  value before `As`, lower-case `const`), of any type, with any value, in
  `Process_Globals` or in a Sub. Assigning a constant, or an item of an
  array constant, and a constant without a value are errors, as in B4J.
  Without `As` a constant is a `String`. Recording run 13 settles more:
  a loop over an `Int` constant, assigning a field of a record constant,
  an array constant with bounds and no value, and a `Dim` in a Sub with a
  module constant's name are errors with B4J's answer, and in
  `Process_Globals` an `Int` declared by a `Const` and by a `Dim`, each
  with a literal value, in either order, builds, each setting its value
  in turn. Still refused until recorded: changing such a variable, and any
  other loop over a constant or declaration again of its name (new
  pending tests).
- A real literal may start with its decimal point (`.08`), as in
  `MteBuildLib.b4j` of the real-world corpus, which B4J builds. Before, it
  was a syntax error. After an operand the dot is still a member access's
  (`a.5` is an error about the member name).

### Recorded from B4J

- Recording runs 12 and 13 (B4J 10.70) settle loops over a variable named
  like a Sub, and FerroBASIC now settles only the combinations they
  recorded, refusing any other as open until it is recorded (new pending
  tests). Accepted, as in B4J, and the name still calls the Sub: another
  `For Each v As Int` after a `For` or a `For Each v As Int` over the
  name, in a Sub or in `Process_Globals`, or inside a `For Each v As Int`
  of a Sub; these were refused until recorded. Refused with the settled
  message: one without `As` after such a loop, or inside a `For` of a Sub
  over the name. Now refused as open, though accepted before: a `For`
  inside a loop over the name; a `For` over a variable that a `For Each`
  of another number type than `Int` declares; a `For Each` of another
  type than `Int` again, or in another Sub over a name that a loop of
  `Process_Globals` declares; a second loop in another Sub over a loop
  variable of `Process_Globals`; and a third loop over the name, one after
  the other or nested, as no recording has more than two. After a loop
  refused or open, the loops over the name after it get no error of their
  own.
- The leading-point reals are recorded as FerroBASIC runs them, and are
  `Double`s.

### Projects

- `ferro-project` reads a whole project: its project file, its `.b4j.meta`
  file and the modules it names, local or `|relative|`, also when a
  module's file is named in another letter case than in the project file,
  as on Windows. Each file writes back byte for byte. `ferro check`, `run`
  and `bundle` still accept only a project with no module (Level 0).

## 0.0.4 (pre-alpha)

### Recorded from B4J

- The first conformance tests are recorded from B4J itself (10.70, OpenJDK
  19, Windows 10), with the new recorder, `tools/recorder`: 24 tests record
  the same output FerroBASIC gives, and replace their hand-derived outputs.
  The recordings also found differences, fixed separately.
- More conformance tests recorded from B4J 10.70.

### Language, as in B4J

- A comment may follow a line continuation (`1 + _ ' first part`, and
  `1 + _' first part`), as in B4J. Before, it was an error.
- A `For` with an undeclared counter runs in `Process_Globals` too, as in
  B4J, and the counter is a module variable: other Subs read it, and their
  undeclared counters of the same name share it. `For Each v As T` runs
  there too, and its variable is a module variable the same way. Before,
  both were errors. Declarations of one name there are one variable with
  the same type, and an error with another.
- A local `Dim` or `For Each v As T` with a module variable's name and
  type is that module variable, as in B4J, and the `Dim` resets it; with
  another type it is an error, as in B4J. Before, it was a new local.
- A parameter with a module variable's name is an error, as in B4J
  ("Parameter name cannot hide global variable name"). Before, it hid the
  module variable.
- A `Dim` with a Sub's name is an error, as in B4J, in `Process_Globals`
  and in a Sub. A loop variable may have one, in `Process_Globals` or in
  a Sub, unless it is a typed `For v As T`: every use of the name in an
  expression calls the Sub, whatever the loop variable's type, a `For`
  may loop over it again, another `For Each` without `As` may not where
  B4J refuses it, and assigning it is an error, as in B4J.
  Before, a `Dim`'d one silently meant the variable.
- An arithmetic operator (`+ - * / Mod`) or a negation in an operand of
  `&` (`"x" & 1 + 2`, `"x" & 6 / 2`) is an error that asks for
  parentheses, as B4J refuses it.
- A main module without `Sub Process_Globals` is an error, as in B4J, even
  when the program declares no module variables (found by the recordings).
- An array of 2 dimensions or more where a `List` is expected (for example
  `List.Initialize2(Grid)`) is a compile error, as in B4J. Before, it gave a
  list of the array's rows (since 0.0.2).
- `Const` declarations, type tests (`x Is T`), `Wait For` and inline Java
  (`#If Java` … `#End If`) get one precise error each, saying what is
  missing, instead of generic syntax errors (#32). None of them is supported
  yet.

### Runtime, as in B4J

- `Null` assigned, passed or returned as a `String` is the text `"null"`,
  as in B4J, which converts it with `BA.ObjectToString`: before, it stayed
  null, so `IsNumber` or `.Length` on it raised `NullPointerException`.
- A record prints with a line break after every third field
  (`IsInitialized` counts), as B4J's `BA.TypeToString` does.
- An array given to a `List` through an `Object` raises
  `ClassCastException`, as in B4J, whatever its rank: only an array declared
  as one is wrapped. Before, it gave a list of the items, or of the rows.
- Changes through a `SubList` view whose parent has changed (`Add`,
  `InsertAt`, `RemoveAt`, `Set`, `Clear`, `Sort`, `SortCaseInsensitive`)
  raise `UnsupportedOperationException`, as in B4J, instead of
  `ConcurrentModificationException` (#21).
- `SortType` and `SortTypeCaseInsensitive` take the field from the first
  item, as B4J does, on every list. A list mixing `Type`s, or a record with
  other values, is no longer sorted silently or reported as a missing field:
  errors while sorting raise `RuntimeException`, wrapping the cause, as in
  B4J. In a list with several different problems the cause can differ
  (deviation D4). `SortType("IsInitialized", …)` sorts by whether each
  record is initialised (still to be confirmed against B4J).
- `LastException` in a `Catch` is the innermost cause, as in B4J: after a
  failed `SortType` it is the error that `RuntimeException` wraps, not the
  `RuntimeException` (found by the recordings).

### Known deviations

- `SortType` on a list of plain values, by the name of a field their JDK
  class declares, raises `NoSuchFieldException`, where B4J sorts by a
  public one (`"MAX_VALUE"` on `Int`s) and raises
  `InaccessibleObjectException` for a private one (`"value"`, recorded):
  deviation D7. Sorting records by `IsInitialized` is confirmed by the same
  recording (test `list_sort_type_initialized`).
- Lists can be compared 3000 levels deep and printed 7000 levels deep
  before `StackOverflowError`, up from 1000: limits chosen within the depths
  B4J reached and overflowed at on the recording machine (deviation D2).
- A record that holds itself through a List, or records nested in Lists
  more than about 49 deep, print differently from B4J, which stops printing
  them there (deviation D6).
- A sort that raises leaves the list as it was, where B4J leaves it partly
  sorted (deviation D5).

### Testing and tools

- `ferro-difftest` runs the conformance corpus in the interpreter
  (`--target interp`) or as `ferro bundle` binaries (`--target standalone`)
  and prints the compatibility report: pass, fail, deviation and pending, by
  level and, with `--by-feature`, by feature.
- A real-world corpus: six open source B4J console projects, referenced in
  `tests/corpus/corpus.toml` by original URL, commit, licence and each
  file's SHA-256, not copied. `ferro-corpus-fetch` downloads and checks
  them; their sources must round-trip through the lossless syntax tree.

## 0.0.3 (pre-alpha)

### Projects

- A B4J console project (`.b4j`), or the folder that holds it, is an input
  of `ferro check`, `run` and `bundle`. A project Level 0 cannot run is an
  error at each header line that stands in the way: a UI `AppType`, another
  library, another module, a file. Before, such a project ran its main
  module as if nothing were missing.
- `ferro bundle` writes a project's program into the project's folder, named
  after the project: `ferro bundle MyApp` gives `MyApp/MyApp`, wherever it
  runs from. A module's program still goes in the current folder. An output
  that is a directory is refused with a message saying to use `-o`.
- A module file whose header says `Type=Class` (or any type other than
  `StaticCode`) is an error that says so.

### Licences

- `ferro licenses` prints FerroBASIC's licence and the notices of the
  third-party code compiled into `ferro`, and so into every program built
  with `ferro bundle`. Whoever distributes such a program must include
  them: `ferro licenses > NOTICES.txt`. The notices are also in
  `THIRD-PARTY-NOTICES.txt`.

### Testing and documentation

- CI on GitHub Actions: formatting, clippy and the tests on Linux, Windows,
  macOS arm64 and macOS Intel, plus a static Linux build, for release pull
  requests and on demand. `ferro run` and `ferro bundle` are now tested on
  Windows and on macOS Intel.
- New conformance tests for the members of `String`, `StringBuilder`, `List`
  and `Map`, and for maths, `Bit`, characters, integer arithmetic, seeded
  `Rnd` and exit codes. A conformance test can now declare the exit code it
  expects.
- `docs/language-support.md` lists what FerroBASIC supports today and what it
  does not yet, area by area: types, statements, built-in objects, keywords,
  directives.

### Known limitations

- Changes through a `SubList` view whose parent changed raise
  `ConcurrentModificationException` instead of
  `UnsupportedOperationException` (#21).
- `Const`, `Is`, `Wait For` and inline Java are rejected with a generic
  syntax error instead of a precise diagnostic (#32); `Const` is not
  supported yet (#33).

## 0.0.2 (pre-alpha)

### Command line

- `ferro build` is now **`ferro bundle`**. `build` is reserved for building
  whole projects later; on a single module it points to `bundle`.
- `ferro version` and `ferro help` show the version and the platform.
- `ferro bundle` works on macOS: the program is embedded in the executable,
  which is signed ad hoc. Tested on Apple Silicon, and on Intel through
  Rosetta.
- `ferro` never overwrites its input, and handles invalid UTF-8, pipes and
  closed output without crashing.

### Robustness

- Statements and expressions nested more than 1000 levels deep are a compile
  error instead of a crash (deviation D1).
- Very deep or very large data no longer crashes the interpreter: freeing,
  printing and comparing it does not recurse, printing and comparing more
  than 1000 levels deep raise `java.lang.StackOverflowError` as in Java, and
  running out of memory raises `java.lang.OutOfMemoryError` (deviation D2).
- Several front-end results that were silently wrong or misleading are now
  correct or a precise error.

### Java-exact behaviour

- `Double` and `Float` values print with Java's digits, ties included:
  `27.899368` as a `Float` prints `27.899368286132812`.
- `NumberFormat` and `NumberFormat2` use the digits of Java's `DecimalFormat`,
  which differ from `Double.toString` for large integers and powers of two.
  The rare remaining differences are deviation D3.
- `ASin`, `ACos`, `ATan` and `ATan2` match Java bit for bit, and the degree
  variants compute in Java's order.
- `Select` on an `Object` that holds a number compares by numeric value.
- Multi-dimensional arrays are arrays of rows, as in Java: `List.Initialize2`
  with one gives a list of its rows, shared with the array, and `Grid(i, j)`
  selects the row before evaluating `j` or the assigned value.
- `Map.GetKeyAt` / `GetValueAt` behave as B4J's do on bad indices
  (`NoSuchElementException` past the end).
- `Bit.ParseInt` accepts any Unicode decimal digit; `IsNumber(Null)` raises
  `NullPointerException`; `AddAll` of an empty list into a fixed-size list is
  allowed.

### Other

- FerroBASIC is released under the MIT License.
- New conformance tests, and a shared reader for the test corpus
  (`crates/corpus`).

### Known limitations

- `ferro bundle` is untested on Windows.
- There is no CI yet.
- Changes through a `SubList` view whose parent changed raise
  `ConcurrentModificationException` instead of
  `UnsupportedOperationException` (#21).

## 0.0.1 (pre-alpha)

First release: Level 0 of a B4X-compatible BASIC (one code module, the
language core, `List`, `Map`, `StringBuilder` and `Type` records), with
`ferro check`, `ferro run` (the reference interpreter) and `ferro build`,
which bundles a program with the interpreter into a standalone executable.
