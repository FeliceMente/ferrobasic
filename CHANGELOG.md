# Changelog

FerroBASIC is pre-alpha: every release can change behaviour, and the
command line may still change. Expected outputs are still derived by hand
from the original runtime's source and from JDK 21, not yet recorded from
B4J itself (`docs/compatibility-testing.md`).

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
