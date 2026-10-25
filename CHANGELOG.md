# Changelog

FerroBASIC is pre-alpha: every release can change behaviour, and the
command line may still change. Expected outputs are still derived by hand
from the original runtime's source and from JDK 21, not yet recorded from
B4J itself (`docs/compatibility-testing.md`).

## Unreleased

- The first conformance tests are recorded from B4J itself (10.70, OpenJDK
  19, Windows 10), with the new recorder, `tools/recorder`: 24 tests record
  the same output FerroBASIC gives, and replace their hand-derived outputs.
  The recordings also found differences, fixed separately.
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
- A sort that raises leaves the list as it was, where B4J leaves it partly
  sorted (deviation D5).
- `Const` declarations, type tests (`x Is T`), `Wait For` and inline Java
  (`#If Java` … `#End If`) get one precise error each, saying what is
  missing, instead of generic syntax errors (#32). None of them is supported
  yet.

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
