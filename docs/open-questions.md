# Open questions

Unresolved points. Agents: do not guess answers to these. Add a pending test
and a new entry here instead. Close an entry by linking the recording or the
source that settled it.

## Language

Answers from public sources (runtime source code, official docs, forum) are in
[research/level0-semantics.md](research/level0-semantics.md). An entry checked
"answered" there is still unconfirmed until a recording agrees with it; the
related tests stay `pending-recording`.

- [x] An operand of `&` that holds an arithmetic operator (`+`, `-`, `*`,
      `/`, `Mod`) or a negation outside parentheses is a syntax error, on
      either side and at any depth (`"x" & 1 + 2`, `1 + 2 & "x"`,
      `"x" & -1`, `"x" & 6 / 2`, `"x" & -6 / 2`); a comparison binds looser
      than `&` (`"a" & "b" = "ab"` is `True`). Recorded: `level0/concat_*`;
      FerroBASIC gives a precise error.
- [x] `And` binds tighter than `Or`: `True Or False And False` is `True`
      (recording: `level0/precedence_and_or`). Against comparisons it is
      still open.
      Not stated in the official docs.
- [x] Whether `Case` accepts ranges: no ([docs](research/level0-semantics.md#statements)).
- [ ] Whether `Select Case` is rejected or accepted as a synonym of `Select`.
- [x] `Do ... Loop While` / `Loop Until` and a bare `Do ... Loop` do not exist
      ([docs](research/level0-semantics.md#statements)).
- [x] Smart string formatters (`$1.2{x}`, `$xml{}`)
      ([source](research/level0-semantics.md#smart-strings-docs-forum-source)).
- [ ] How to write a literal `${` or `"$` inside a smart string.
- [x] `:` separates statements like a line break, also after a single-line `If`
      ([docs](research/level0-semantics.md#statements)).
- [x] `/` between two integers returns a Double
      ([forum, Erel](research/level0-semantics.md#division-forum-erel)).
- [ ] Double-to-Int conversion on assignment: truncation toward zero (forum) or rounding.
- [x] A variable declared inside a block is visible in the whole Sub; the
      declaration is hoisted and re-running it resets the variable
      ([forum](research/level0-semantics.md#declarations-and-scope)).
- [x] `Map` iterates in insertion order (`LinkedHashMap`)
      ([source](research/level0-semantics.md#map-source)).
- [x] Number-to-string formatting: `Double.toString` without a trailing `.0`,
      Float widened to Double, JDK ≥ 19 rules
      ([source](research/level0-semantics.md#number-to-string-conversion-source)).
- [x] Numbers in `&` are converted with `BA.NumberToString`
      ([generated](research/level0-semantics.md#number-to-string-conversion-source)).
- [x] `Log(number)` uses `BA.NumberToString` too (generated code of
      `level0/number_parsing`); number-to-String assignments are still open.
- [x] `Null` converted to a `String` (assigned, passed, returned) is the
      text `"null"`: B4J compiles it with `BA.ObjectToString` (generated code
      and recording: `level0/number_parsing`).
- [x] A `String` holding `"null"` is not `Null` (`s = Null` is `False`);
      an unset `String` field is (recording: `level0/string_null_compare`).
- [ ] Which keywords besides `In` are also accepted as names (`Step`, `To`,
      `Each`, `Until`, `Type`, …): `In` is
      ([generated](research/level0-semantics.md#keywords-used-as-names-generated)).
- [ ] Whether a line break inside parentheses continues the line without ` _`.
- [ ] Whether chained comparisons (`a < b < c`, `If a = b = c`) compile, and
      how they group. The parser accepts them left-associatively for now.
      Settled part: `x = a = b` as a statement assigns the comparison `a = b`
      ([generated](research/level0-semantics.md#operators-and-precedence-docs-forum-open)).
- [ ] Which boxed type a numeric literal gets when stored as `Object`
      (matters for `Map` keys: `Int` 1 and `Double` 1.0 are different keys).
- [ ] `For`: whether the limit and step are evaluated once, and the value of
      the loop variable after the loop.
- [ ] Undeclared variables other than `For` counters: implicit `String` (old
      forum posts) or a compile error.
- [ ] Type of a variable declared without `As` (`Dim x`, `Dim x = 5`): `String`, or inferred.
- [ ] Two `Dim`s of the same name in one Sub, with the same or different types.
- [ ] Return value of a typed Sub that ends without `Return`.
- [ ] Comparing Strings with `<` / `>`, and comparing a String with a number.
- [ ] Changing a `List` or `Map` while iterating it with `For Each`.
- [x] Output and exit code of a B4J non-UI program that stops on an uncaught
      error: what it logged so far, then exit code 1, with the error on stderr
      (recording: `level0/uncaught_error`).
- [x] `AppType` value of a non-UI `.b4j` project: `StandardJava`
      ([public samples](spec/project-files.md#sources)).
- [ ] Whether `LogDebug` prints in release mode (provisionally not).
- [ ] Exact text of JDK-generated exception messages seen through
      `LastException.Message` (index and number-parse errors), and which JDK.
- [ ] Default JVM locale on the recording machine (affects `ToLowerCase` /
      `ToUpperCase` for Turkish, Azeri and Lithuanian locales). Recordings
      force `en-US`; the recorder also runs each test in the machine's own
      locale and keeps that output when it differs (`<name>.<locale>.expected`).
      The first recordings gave the same output in `it-IT` for every test,
      `NumberFormat` and number printing included. No test covers Turkish
      case mapping yet.
- [x] Standalone binaries on macOS: re-signing appended payload bytes does
      not work (`codesign`: "main executable failed strict validation", on
      Apple Silicon and Intel). The package is embedded with `libsui`
      instead (see `docs/spec/cli.md`).
- [ ] Planned CLI (`docs/spec/cli.md`, "Planned commands"): the final target
      names; whether `js` is its own backend or goes through `wasm`; the
      format and timing of a `ferro.toml` project file.

Provisional runtime behaviour in `crates/mir` and `crates/interp` (each
needs a recorded test):

- [ ] `For` evaluation order: FerroBASIC assigns the start value, then
      evaluates the limit, then the step. B4J's generated Java may evaluate
      limit and step first; it only matters when they read the counter or
      have side effects.
- [ ] Call depth before `java.lang.StackOverflowError` (FerroBASIC: 10,000
      frames). The error is not caught by `Try`, as Java `Error`s escape the
      generated `catch (Exception e)`.
- [x] How deeply nested Lists can be compared or printed before Java's
      recursive `equals` / `toString` overflow: it depends on the JVM's
      stack; the recording machine compares 2000 levels and prints 5000
      (recordings: `level0/deep_equals`, `deep_printing`). FerroBASIC's fixed
      limits, 3000 and 7000, are deviation D2.
- [x] Whether a record whose List holds the record overflows when printed:
      B4J prints it 49 levels deep, then `[]`, as jCore's `BA.TypeToString`
      gives up once the Java stack is 150 frames deep (recording:
      `level0/self_printing_record`). FerroBASIC overflows: deviation D6.
- [ ] When B4J raises `java.lang.OutOfMemoryError` depends on the JVM's heap
      size, which a recording must state. FerroBASIC always says `Java heap
      space`; the JDK words an over-long String differently.
- [x] How a record prints: jCore's `BA.TypeToString`, `IsInitialized` first
      and a line break after every third field (recording:
      `level0/record_printing`).
- [ ] `GetType` of a record.
- [ ] How a variable of type `Exception` and an array print.
- [ ] `Sin`, `Cos`, `Tan`, `Power` and `Logarithm` call `Math` methods that
      HotSpot replaces with CPU intrinsics, whose last bit may depend on the
      JVM and CPU; FerroBASIC uses the platform's maths library. On the
      recording machine (Intel, OpenJDK 19) they match: `level0/math_builtins`
      is recorded; other machines may differ. (`ASin`, `ACos`,
      `ATan`, `ATan2` are settled: fdlibm, bit for bit.)
- [x] `Grid(i, j) = v` evaluates `i`, `j`, then `v`, and a bad row index
      fails before `j` is evaluated, as Java's `a[i][j] = v` does
      (recording: `level0/array_row_order`).
- [x] An array given to a `List` parameter through an `Object` raises
      `ClassCastException`, whatever its rank; only an array declared as one
      is wrapped (recordings: `level0/list_from_1d_array_object`,
      `list_from_2d_array_object`).
- [x] The type of `IIf`'s result, and so how it converts when assigned or
      printed: the chosen value keeps its own type, so `Log(IIf(True, 3,
      4.5))` prints `3` (recording: `level0/iif_branches`).
- [ ] Whether all `Case` values are evaluated before matching, as they would
      be if the compiler passes them to `BA.switchObjectToInt` as arguments;
      FerroBASIC evaluates them in order until one matches. It only matters
      for cases with side effects.
- [ ] Exact texts of JDK-generated messages (`ClassCastException`,
      `IndexOutOfBoundsException`, …) seen through `LastException.Message`.
      `SortType`'s wrapped `IllegalArgumentException` (an item of another
      type) has no message in FerroBASIC; on JDK 21 `Field.get` says
      `Can not get java.lang.Object field <class>.N on <class>`, naming B4J's
      generated classes, whose names are open too (see "How a record
      prints").
- [x] Which fields `SortType` finds on the first item's class (jCore calls
      `getClass().getDeclaredField(name)`, then `setAccessible(true)`): a
      record has `IsInitialized` too, and sorts by it; a field a JDK class
      declares privately (`Integer.value`) raises
      `InaccessibleObjectException` on OpenJDK 19 (recording:
      `level0/list_sort_type_class_fields`, kept in its `.meta.toml`; test
      `level0/list_sort_type_initialized`). FerroBASIC gives plain values no
      fields: deviation D7.
- [x] `GetType` returns B4J's Java class names (e.g. a Map's wrapper class),
      even though they contain the product's package name: kept for
      compatibility, since it is program output, not a FerroBASIC message.
- [ ] A List or Map used as a Map key: compared by identity in FerroBASIC.

Provisional type-checking rules in `crates/hir` (each needs a recorded test):

- [ ] Calling a member on an `Object` (`o.Size`) is a compile error: the value
      must first be assigned to a typed variable or cast with `.As(T)`.
- [ ] `Return <value>` in a Sub declared without `As` is a compile error.
- [ ] A parameter without `As` is a `String`.
- [ ] Comparing a `Char` with a number is a compile error; `Char` with a
      `String` compares as Strings.
- [ ] In a mixed comparison the left operand decides: number vs String
      compares numerically, String vs number compares as Strings (forum).
- [ ] `<` / `>` on Booleans or objects is a compile error.
- [ ] `Null` compared with a number, Boolean or Char is a compile error.
- [ ] A hex literal up to `0xFFFFFFFF` is an `Int` (so `0xFF000000` is
      -16777216, as the forum shows); a decimal literal that does not fit an
      `Int` is a `Long`.
- [x] A `For` with an undeclared counter inside `Process_Globals` runs,
      and the counter is a module variable: `AppStart` reads it, and the
      undeclared counters of the same name in other Subs share it
      (recordings: `level0/process_globals_for`,
      `level0/process_globals_counter_visible`,
      `level0/process_globals_counter_shared`).
- [x] `For Each v As T` inside `Process_Globals` declares a module
      variable too, which other Subs see and share; with a `Dim` of the same
      type first, it is that variable (recordings:
      `level0/process_globals_for_each`, `level0/process_globals_for_each_visible`,
      `level0/process_globals_for_each_shared`,
      `level0/process_globals_for_each_declared`).
- [x] Declarations of one name in `Process_Globals` must agree on its type:
      a counter (`Int`) then `For Each n As String` is refused (recording:
      `level0/process_globals_counter_and_for_each`).
- [x] With the same type, declarations of one name there are one
      variable, in any order (recordings:
      `level0/process_globals_counter_and_for_each_int`,
      `level0/process_globals_for_each_then_counter`,
      `level0/process_globals_for_each_twice`).
- [x] A loop variable of `Process_Globals` may have a Sub's name, and a
      statement calling the Sub still works (recording:
      `level0/process_globals_loop_named_like_sub`).
- [x] In another Sub, reading that name calls the Sub (recording:
      `level0/process_globals_loop_named_like_sub_read`). A `Dim` there
      cannot have a Sub's name (recording:
      `level0/module_variable_named_like_sub`).
- [x] Reading that name inside `Process_Globals` calls the Sub too, and
      assigning it in another Sub is a syntax error (recordings:
      `level0/process_globals_loop_named_like_sub_read_inside`,
      `level0/process_globals_loop_named_like_sub_assign`).
- [x] Looping over that name in another Sub is accepted, `Bump(args)`
      calls the Sub, and assigning it inside its own loop is an error too
      (recordings: `level0/process_globals_loop_named_like_sub_loop_elsewhere`,
      `level0/process_globals_loop_named_like_sub_args`,
      `level0/process_globals_loop_named_like_sub_assign_inside`).
- [x] A `For` may loop over that name again: in `Process_Globals`, in
      another Sub over a `For Each` variable of `Process_Globals`, or in a
      Sub over its own loop variable (recordings:
      `level0/process_globals_loop_named_like_sub_loop_again`,
      `level0/process_globals_for_each_named_like_sub_loop_elsewhere`,
      `level0/local_counter_named_like_sub_loop_again`).
- [ ] A `For Each` without `As` over that name again: in another Sub over
      a loop variable of `Process_Globals`, or in a Sub over its own loop
      variable. FerroBASIC refuses it (tests:
      `level0/process_globals_loop_named_like_sub_for_each_elsewhere`,
      `level0/local_for_each_named_like_sub_loop_again`).
- [x] A `For Each v As T` of a Sub whose name is a Sub's and a module
      variable's of the same type is accepted, and reading it calls the
      Sub (recording:
      `level0/local_for_each_named_like_sub_over_module_variable`).
- [ ] The same with another type than the module variable's: refused as
      for any other name, or accepted? FerroBASIC refuses it (test:
      `level0/local_for_each_named_like_sub_other_type`).
- [x] The name is the Sub whatever the loop variable's type: `Bump.Length`
      on a `String` loop variable, with `Sub Bump As Int`, is refused
      ("Object expected", recording:
      `level0/local_for_each_named_like_sub_member`).
- [x] A typed `For v As T` of a Sub cannot have a Sub's name ("Regular
      variable followed by '=' expected", recording:
      `level0/local_typed_counter_named_like_sub`).
- [ ] A `Dim` of `Class_Globals` with a Sub's name: refused as in
      `Process_Globals`? FerroBASIC refuses it. Classes arrive at Level 1,
      so it needs a class module in the corpus to record.
- [ ] A loop variable of `Process_Globals` named like a `Type`, `Bit` or a
      keyword: FerroBASIC refuses it.
- [x] A local `Dim` cannot have a Sub's name (recording:
      `level0/local_dim_named_like_sub`).
- [x] A loop variable of a Sub may have a Sub's name, and reading it calls
      the Sub (recordings: `level0/local_counter_named_like_sub`,
      `level0/local_for_each_named_like_sub`).
- [x] A local with a module variable's name and another type is refused
      (recording: `level0/process_globals_counter_hidden`).
- [x] With the same type, the local `Dim` means the module variable
      (recording: `level0/local_dim_like_module_variable`).
- [x] That `Dim` resets the module variable (recording:
      `level0/local_dim_resets_module_variable`).
- [x] A `For Each v As T` in a Sub, with a module variable's name and
      type, means the module variable too (recording:
      `level0/local_for_each_like_module_variable`).
- [x] A parameter cannot have a module variable's name: "Parameter name
      cannot hide global variable name" (recording:
      `level0/parameter_like_module_variable`).
- [x] A comment may follow a line continuation, with or without a space
      before the `'` (recordings: `level0/continuation_comment`,
      `level0/continuation_comment_tight`).
- [ ] An inline Java block (`#If Java` … `#End If`): whether B4X accepts a
      comment after `#If Java` or `#End If`, and whether an `#If` inside the
      block nests. FerroBASIC accepts the comments and ends the block at the
      first `#End If`; it only reports the block (Java interop is Level 3),
      so the answer matters once that level runs it.
- [ ] How deep B4J (javac) lets expressions and blocks nest before failing
      (B4J 10.70 builds 1100 levels: recorded);
      FerroBASIC stops at 1000 levels (`docs/deviations.md`). Fixture:
      `level0/deep_nesting`.

## Porting

- [ ] Collect a corpus of real projects per product and document the project-file format from it.
      Lead: the official [Booklets.zip](https://www.b4x.com/android/files/Booklets.zip) holds 34
      small `.b4j` projects (plus `.b4a`/`.b4i`); its licence is unknown, so use it locally only.
- [ ] List which AnywhereSoftware repositories cover which core libraries, and which are missing.
- [ ] Decide whether B4XPages-on-native is the first non-JVM porting target.
- [ ] Legal check on reverse-engineering the project and layout formats.

## Compatibility testing

- [ ] Check that the B4X licence allows automated use of its compilers for testing; never redistribute B4X binaries.
- [ ] Confirm the command-line builders for B4J, B4A and B4R and their options.
- [ ] Decide which deviations are acceptable by design.

## Project

- [x] Choose the licence: MIT (`LICENSE`).
- [x] Ship the licence notices of FerroBASIC's dependencies with released
      binaries: `THIRD-PARTY-NOTICES.txt`, generated with `cargo-about`
      (`tools/licenses/`), compiled into `ferro` and printed by
      `ferro licenses`.
- [ ] Trim `libsui`'s dependencies: on macOS it brings in about 40 crates
      (image decoding, compression, hashing) that `ferro` never uses. Check
      whether its features can be turned off, which would shorten the
      notices and shrink the macOS binary.
- [ ] Evaluate Cranelift for the REPL JIT and debug builds.
- [ ] Evaluate `zig cc` / `cargo-zigbuild` as the cross-linker.
