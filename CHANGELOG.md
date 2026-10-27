# Changelog

FerroBASIC is pre-alpha: every release can change behaviour, and the
command line may still change. Expected outputs are recorded from B4J
10.70 (Windows 10, OpenJDK 19) by `tools/recorder`; a test B4J has not run
yet is pending until the next recording run (`docs/compatibility-testing.md`).

## Unreleased

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
