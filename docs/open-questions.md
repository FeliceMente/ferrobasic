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
- [x] `Select Case x` is accepted as a synonym of `Select x` (recording:
      `level0/select_case_synonym`).
- [x] `Do ... Loop While` / `Loop Until` and a bare `Do ... Loop` do not exist
      ([docs](research/level0-semantics.md#statements)).
- [x] Smart string formatters (`$1.2{x}`, `$xml{}`)
      ([source](research/level0-semantics.md#smart-strings-docs-forum-source)).
- [ ] How to write a literal `${` or `"$` inside a smart string.
- [x] `:` separates statements like a line break, also after a single-line `If`
      ([docs](research/level0-semantics.md#statements)).
- [x] `/` between two integers returns a Double
      ([forum, Erel](research/level0-semantics.md#division-forum-erel)).
- [x] Double-to-Int conversion on assignment truncates toward zero: 2.7,
      2.5 and 3.5 give 2, 2 and 3, -2.7 and -2.5 give -2 (recording:
      `level0/double_to_int_assign`).
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
- [ ] Which keywords besides `In` are also accepted as names: `In` is
      ([generated](research/level0-semantics.md#keywords-used-as-names-generated)),
      and so is `Each` (recording: `level0/keyword_name_each`). `Step`, `To`, `Type` and `Until`
      are not (recordings: `level0/keyword_name_step`, `_to`, `_type`,
      `_until`). The other keywords are open.
- [x] A line break inside parentheses does not continue the line: B4J
      says "Invalid number of parentheses." (recording:
      `level0/paren_line_break`).
- [x] Chained comparisons do not compile: `Log(3 < 2 < 1)` gives ")
      expected" and `If a = b = c Then` "'Then' expected" (recordings:
      `level0/chained_comparison_less`, `level0/chained_comparison_equal`).
      `x = a = b` as a statement assigns the comparison `a = b`
      ([generated](research/level0-semantics.md#operators-and-precedence-docs-forum-open)).
- [x] A numeric literal stored as `Object` is an `Integer`, or a `Double`
      with a point, so `Int` 1 and `Double` 1.0 are different `Map` keys
      (recording: `level0/literal_boxed_type`).
- [x] `For` evaluates its limit and step once; after the loop the counter
      holds the first value past the limit, or the start when the loop never
      ran (recordings: `level0/for_limit_evaluated_once`,
      `level0/for_counter_after_loop`). A `Double` counter with a fractional
      step works (recording: `level0/for_double_counter`).
- [x] An assignment to a name nothing declared builds and declares it
      (recording: `level0/undeclared_variable`). FerroBASIC makes it a
      `String`, as old forum posts say, unless a `Dim` or a loop of the
      Sub declares the name, before or after (declarations are hoisted).
- [x] Such a variable is a `String`: `GetType` gives `java.lang.String`,
      and `+ 1` on it gives `6` (recording: `level0/undeclared_variable_type`).
- [ ] Whether a read of a name nothing declares or assigns builds. Before
      a `For Each` without `As` that declares it, it does not: "Undeclared
      variable 'u' is used before it was assigned any value." (recording:
      `level0/for_each_untyped_read_before`); with nothing declaring it
      anywhere, FerroBASIC refuses it as undeclared.
- [x] A variable declared without `As` (`Dim x`, `Dim x = 5`) is a `String`
      (recording: `level0/dim_without_as`).
- [x] Two `Dim`s of the same name and type in one Sub are one variable,
      and the second resets it; with different types they are refused
      ("Current declaration does not match previous one"). In
      `Process_Globals` two `Dim`s of one name and type are one variable
      too, set by each in turn: to 0 by one without a value (recordings:
      `level0/dim_twice_same_type`, `level0/dim_twice_other_type`,
      `level0/process_globals_dim_twice`,
      `level0/process_globals_dim_twice_no_value`).
- [x] A typed Sub that ends without `Return` returns its type's default:
      0, or an empty `String` (recordings: `level0/typed_sub_without_return`,
      `level0/typed_string_sub_without_return`).
- [x] `<` / `>` compare numbers, also on Strings, which are parsed: `"a" <
      "b"` raises `NumberFormatException`, and `"10" < 9` is `False`
      (recordings: `level0/string_compare_less`,
      `level0/mixed_compare_string_left`).
- [ ] Changing a `Map` while iterating it with `For Each`. A `List` that
      grows inside its `For Each` is iterated only over the items it had
      when the loop began (recording: `level0/list_modified_in_for_each`).
- [x] Output and exit code of a B4J non-UI program that stops on an uncaught
      error: what it logged so far, then exit code 1, with the error on stderr
      (recording: `level0/uncaught_error`).
- [x] `AppType` value of a non-UI `.b4j` project: `StandardJava`
      ([public samples](spec/project-files.md#sources)).
- [x] `LogDebug` prints nothing in release mode (recording:
      `level0/log_debug`).
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

- [x] `For` evaluates the step, then the limit, then the start (recording:
      `level0/for_evaluation_order`). That the start is assigned after the
      limit is evaluated, so a limit reading the counter reads its old
      value, follows: `For i = 1 To i` with `i = 3` counts to 3 (recording:
      `level0/for_limit_reads_counter`).
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
- [x] `GetType` of a record: `<package>.main$_<name in lower case>`, e.g.
      `b4j.example.main$_point` (recording: `level0/record_get_type`).
- [ ] A project's package in `GetType` of a record: FerroBASIC takes it from
      the first build (`Build1=Default,<package>`) as written; the recorder's
      projects all use `b4j.example`, so no recording settles another
      package, or a build chosen in the IDE other than the first.
- [x] A variable of type `Exception` prints as `(<Class>) <toString>` once
      caught, and as `(Exception) Not initialized` when never set
      (recording: `level0/exception_printing`).
- [ ] How an array prints: its identity hash, which a recording cannot
      settle.
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
- [x] Every `Case` value is evaluated, in order, before any is compared, as
      when they are passed to `BA.switchObjectToInt` (recording:
      `level0/case_values_evaluated`), also when `Case Else` runs
      (recording: `level0/case_values_evaluated_else`).
- [ ] Exact texts of JDK-generated messages (`ClassCastException`,
      `IndexOutOfBoundsException`, …) seen through `LastException.Message`.
      `SortType`'s wrapped `IllegalArgumentException` (an item of another
      type) has no message in FerroBASIC; on JDK 21 `Field.get` says
      `Can not get java.lang.Object field <class>.N on <class>`, naming B4J's
      generated classes (a record's is `b4j.example.main$_point`).
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
- [x] A List used as a Map key compares by its contents, as an `ArrayList`
      does (recording: `level0/list_as_map_key`). FerroBASIC follows Java's
      `HashMap` for a List changed after it was put, or one that holds
      itself, as B4J does (recordings: `level0/list_key_changed`,
      `level0/list_key_self`).
- [x] A Map used as a Map key, alone or inside a List key, compares by
      identity: two Maps with the same contents are two keys (recording:
      `level0/map_as_map_key`).

Provisional type-checking rules in `crates/hir` (each needs a recorded test):

- [x] Calling a member on an `Object` (`o.Size`) is a compile error: B4J
      says "Unknown type: Object" (recording: `level0/member_on_object`).
- [x] `Return <value>` in a Sub declared without `As` builds (recording:
      `level0/return_value_untyped_sub`).
- [x] The call of such a Sub has a value, a `String`, as B4J declares such
      Subs (it generates `return "";` at their end): `Log(NoType)` prints
      `5`, `GetType` gives `java.lang.String`, `+ 1` gives `6`, and one that
      returns nothing gives `""` (recordings:
      `level0/untyped_sub_return_value_used`, `level0/untyped_sub_return_type`).
- [x] A parameter without `As` is a `String` (recording:
      `level0/parameter_without_as`).
- [x] A `Char` `=` an `Int` converts the `Int` to a `Char`, the first
      character of its text: B4J generates `_c==BA.ObjectToChar(65)`, and
      jCore's `ObjectToChar` takes `"65".charAt(0)`, `'6'`, so `"A" = 65` is
      `False` (recording and generated code: `level0/char_number_compare`).
- [x] A `Char` `=` a `Long`, `Short`, `Byte` or `Double` converts it as an
      `Int`: `"6" = 65` is `True` with each, and `"1" = 0.00001` is `True`,
      the first character of `1.0E-5` (recordings: `level0/char_eq_long`,
      `level0/char_eq_double`).
- [x] A `Char` `=` a `Float` takes the first character of the Float's
      Java text, `Float.toString` (`"1" = 0.00001` is `True`, from
      `1.0E-5`), not of B4J's text of it widened to a Double (recording:
      `level0/char_eq_float`).
- [x] The number on the left is refused ("Cannot cast type:
      {Type=Char,…} to: number", recording: `level0/number_eq_char`), as
      FerroBASIC refuses it.
- [x] A `Char` with `<` is refused ("Cannot cast type: {Type=Char,…} to:
      number"), next to a String or another Char (recordings:
      `level0/char_string_compare`, `level0/char_less_char`).
- [x] `<` between a String and a Boolean is refused, as between two
      Booleans (recordings: `level0/string_less_boolean`,
      `level0/boolean_less`).
- [x] `<` between a String and an `Object` parses both as numbers:
      `"10" < o` with `o = "5"` is `False` (recording:
      `level0/string_less_object`).
- [x] With `=` and `<>` the left operand decides: number vs String
      compares numerically (`10 = "10.0"` is `True`), String vs number as
      Strings (`"10.0" = 10` is `False`); with `<` and `>` both compare
      numbers (recordings: `level0/mixed_compare_number_left`,
      `level0/mixed_compare_string_left`).
- [x] `<` / `>` on Booleans is a compile error: "Cannot cast type:
      {Type=Boolean,…} to: number." (recording: `level0/boolean_less`). On
      objects it is still open.
- [ ] `Null` compared with a Boolean or a Char, with `<`, or written on
      the left of a number (`Null = n`), is a compile error. An `Int` `=`
      `Null` builds and raises `NumberFormatException` when it runs, as
      `BA.ObjectToNumber` parses `"null"` (recording:
      `level0/null_number_compare`), and so do a `Double` and a `Long`
      (recordings: `level0/double_eq_null`, `level0/long_eq_null`), and a
      `Float`, a `Short` and a `Byte` (recording:
      `level0/number_types_eq_null`).
- [x] A real literal may start with its decimal point (`.08`), also with
      an exponent (`.5e2`) (recordings: `level0/real_leading_point`,
      `level0/real_leading_point_exponent`; `MteBuildLib.b4j` in the
      real-world corpus writes `Array (320000,.08, .05, .03)`). After an
      operand or a dot, a `.` stays a member access's (`a.5` is an error).
- [x] Its type is `Double`, as `0.08`'s (recording:
      `level0/real_leading_point_type`).
- [x] A hex literal up to `0xFFFFFFFF` is an `Int` (so `0xFF000000` is
      -16777216, as the forum shows); a decimal literal that does not fit an
      `Int` is a `Long` (recordings: `level0/hex_literal`,
      `level0/decimal_literal_over_int`).
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
- [x] A `For` may loop over that name again, after a `For` or a
      `For Each v As Int`, in a Sub or in `Process_Globals`, and in
      another Sub over a loop variable of `Process_Globals` (recordings:
      `level0/process_globals_loop_named_like_sub_loop_again`,
      `level0/process_globals_for_each_named_like_sub_loop_elsewhere`,
      `level0/local_counter_named_like_sub_loop_again`,
      `level0/local_counter_after_for_each_named_like_sub`,
      `level0/process_globals_counter_after_for_each_named_like_sub`), and
      only over a number ("Loop iterator should be a numeric variable",
      recording:
      `level0/process_globals_string_loop_named_like_sub_counter_elsewhere`).
- [x] Another `For Each` without `As` over that name is refused ("Current
      declaration does not match previous one"): in a Sub after a loop over
      its own `For Each` variable or `For` counter, or inside a `For` over
      it;
      in another Sub over a counter or `For Each` variable of
      `Process_Globals`; and in `Process_Globals` after its own counter or
      typed `For Each` (recordings:
      `level0/local_for_each_named_like_sub_loop_again`,
      `level0/local_for_each_after_counter_named_like_sub`,
      `level0/local_for_each_named_like_sub_nested`,
      `level0/process_globals_loop_named_like_sub_for_each_elsewhere`,
      `level0/process_globals_for_each_named_like_sub_for_each_elsewhere`,
      `level0/process_globals_loop_named_like_sub_for_each_again`,
      `level0/process_globals_for_each_named_like_sub_for_each_again`).
- [x] So is one as the first loop over that name, which only a later
      `For Each v As T` declares, in a Sub or in `Process_Globals`, and one
      nested in a loop of `Process_Globals` over it (recordings:
      `level0/local_for_each_named_like_sub_before_typed`,
      `level0/process_globals_for_each_named_like_sub_before_typed`,
      `level0/process_globals_for_each_named_like_sub_nested`).
- [x] Another `For Each v As T` over that name is accepted after typed
      `For Each` loops or an `Int` counter over it, one after the other, in
      a Sub or in `Process_Globals`, and inside a typed `For Each` of a Sub
      over it, and reading it still calls the Sub (recordings:
      `level0/local_typed_for_each_named_like_sub_loop_again`,
      `level0/local_typed_for_each_after_counter_named_like_sub`,
      `level0/local_typed_for_each_after_module_counter_named_like_sub`,
      `level0/local_typed_for_each_named_like_sub_nested`,
      `level0/process_globals_counter_named_like_sub_typed_for_each_again`,
      `level0/process_globals_for_each_named_like_sub_typed_for_each_again`).
      After a `For` of a Sub over a counter of `Process_Globals`, the
      recording shows the program builds and the name calls the Sub; which
      of the two variables that `For` counts it cannot show, as every read
      of the name calls the Sub (FerroBASIC: the Sub's own, which the
      `For Each` declares).
- [x] Every other `For` and `For Each v As T` over that name builds, as the
      variable's own rules allow, and the name still calls the Sub
      (recording run 14): a `For` inside a loop over it, in a Sub or in
      `Process_Globals`; three loops, one after the other or nested; a
      `For Each v As Double` again, also from another Sub over a loop
      variable of `Process_Globals`; loops after another one in another
      Sub; a `For` after a `For Each v As Double`, which counts in
      `Double` steps; and nested loops share the variable (recordings:
      `level0/local_counter_in_loop_named_like_sub`,
      `level0/process_globals_counter_in_loop_named_like_sub`,
      `level0/local_counter_named_like_sub_loop_three_times`,
      `level0/process_globals_loop_named_like_sub_loop_three_times`,
      `level0/local_typed_for_each_counter_typed_named_like_sub`,
      `level0/local_typed_for_each_named_like_sub_nested_three_deep`,
      `level0/local_typed_for_each_double_loop_again_named_like_sub`,
      `level0/process_globals_typed_for_each_double_loop_again_named_like_sub`,
      `level0/local_typed_for_each_double_over_module_variable_named_like_sub`,
      `level0/process_globals_loop_named_like_sub_loop_elsewhere_again`,
      `level0/local_typed_for_each_over_module_variable_loop_again_named_like_sub`,
      `level0/local_counter_after_double_for_each_named_like_sub`,
      `level0/local_typed_for_each_in_counter_named_like_sub`,
      `level0/process_globals_typed_for_each_named_like_sub_nested`). A
      `For` first declares it an `Int`, so a `For Each` of another type
      after it is refused (recording:
      `level0/local_typed_for_each_other_type_after_counter_named_like_sub`).
      FerroBASIC now follows that rule instead of a table of recorded
      combinations.
- [x] A `For Each v As String` over that name again builds, as with `Int`
      and `Double` (recording:
      `level0/local_typed_for_each_string_loop_again_named_like_sub`).
- [x] After a `For` over an ordinary name that nothing declared before, a
      `For Each v As T` or a `Dim` of another type than `Int` is refused
      ("Current declaration does not match previous one"), as the counter
      declares the name an `Int` first, and so is any typed `For v As T`
      ("Regular variable followed by '=' expected") (recordings:
      `level0/for_each_other_type_after_counter`,
      `level0/dim_other_type_after_counter`,
      `level0/typed_counter_other_type_after_counter`).
- [x] A `For Each v As T` in another Sub over a loop variable of
      `Process_Globals` is a `For Each` over a module variable of type
      `T`, accepted as recorded below.
- [x] A `For Each v As T` of a Sub whose name is a Sub's and a module
      variable's of the same type is accepted, and reading it calls the
      Sub (recording:
      `level0/local_for_each_named_like_sub_over_module_variable`).
- [x] With another type than the module variable's, it is refused as for
      any other name ("Current declaration does not match previous one",
      recording: `level0/local_for_each_named_like_sub_other_type`).
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
- [x] A loop variable of `Process_Globals` named like a `Type` builds
      (recording: `level0/process_globals_loop_named_like_type`); the
      `Type` stays usable, and the name reads as the variable (recording:
      `level0/process_globals_loop_named_like_type_used`).
- [ ] Named `Bit` or `Until`, B4J's compiler
      fails (recordings: `level0/process_globals_loop_named_like_bit`,
      `level0/process_globals_loop_named_like_keyword`); other keywords and
      built-in names are open.
- [x] A `For Each v As T` of another type than `Int` in `Process_Globals`,
      over a Sub's name, as the only loop over it, builds, and reading the
      name calls the Sub (recording:
      `level0/process_globals_typed_for_each_double_named_like_sub`).
- [x] A `For Each` without `As` over a local declared before it, of the
      items' type or of another one, is refused ("Current declaration does
      not match previous one", recordings: `level0/for_each_untyped_local`,
      `level0/for_each_untyped_other_type`). `ferro-compare` found that
      FerroBASIC accepted it.
- [x] A `For Each` without `As` declares its variable an `Object`: over an
      `Int` module variable, from a Sub or in `Process_Globals`, over a
      `String` parameter, and before a `Dim n As Int` of the same name, it
      is refused ("Current declaration does not match previous one",
      "Previous: Object" for the later `Dim`; recordings:
      `level0/for_each_untyped_module_variable`,
      `level0/process_globals_for_each_untyped`,
      `level0/for_each_untyped_parameter`,
      `level0/for_each_untyped_declared_after`).
- [x] Over a local `Object` declared before it, and over a name never
      declared, which it declares, it builds (recordings:
      `level0/for_each_untyped_object`, `level0/for_each_untyped_undeclared`).
      `JSONWebToken.bas`, a class of the real-world corpus, loops
      `For Each Key In Claims.Keys` with `Key` declared nowhere.
- [x] Over any `Object` from a Sub it builds: a module variable, declared
      by a `Dim` or by a loop of `Process_Globals`, a parameter, and an
      `Object` that a typed loop declared first (recordings:
      `level0/for_each_untyped_module_object`,
      `level0/for_each_untyped_pg_loop_object`,
      `level0/for_each_untyped_param_object`,
      `level0/for_each_untyped_after_typed_object`). After an assignment of
      the name, which declares a `String`, it is refused ("Current
      declaration does not match previous one", recording:
      `level0/for_each_untyped_assigned_before`), and after a read of it
      ("Undeclared variable 'u' is used before it was assigned any
      value.", recording: `level0/for_each_untyped_read_before`).
- [ ] Inside `Process_Globals`, over a variable `Object` declared there,
      and followed by a `For Each … As Object` over the same name:
      FerroBASIC refuses them
      (`level0/process_globals_for_each_untyped_object`,
      `level0/for_each_untyped_then_typed_object`).
- [x] With a `Dim` of the name as an `Object` after the loop, B4J builds it
      and the `Dim` gives the variable a new `Object`: `5`, `6`,
      `java.lang.Object` (recording run 17:
      `level0/for_each_untyped_object_declared_after`), as FerroBASIC does.
- [x] `Dim o As Object` with no value gives the variable a new `Object`:
      `o = Null` is `False` and `GetType(o)` is `java.lang.Object` (B4J's
      output for `level0/object_dim_default` in recording run 17, then a
      deviation test). FerroBASIC gives one too (it was deviation D8); the
      next run records the test.
- [ ] A module variable `Object` with no value, a `Dim` in a Sub that
      resets a module variable `Object`, and a `Type` field `Object` after
      `Initialize`: a new `Object` too? FerroBASIC gives `Null` for all
      three meanwhile (`level0/object_dim_global_default`,
      `level0/local_dim_resets_module_object`,
      `level0/record_object_field_default`). And how new Objects compare
      and key a Map: by identity in FerroBASIC
      (`level0/object_plain_identity`).
- [ ] After a `For Each` without `As`, a `Dim` of the name as an `Object`
      with a value (`Dim n As Object = 7`): FerroBASIC refuses it, as no
      recording has one (`level0/for_each_untyped_then_object_valued`).
- [x] A new `Object` converted to a number: `BA.ObjectToNumber` parses its
      text, a `NumberFormatException` (source; provisional output for
      `level0/object_plain_to_number`, which the next run records), as
      FerroBASIC does.
- [ ] How a new `Object` prints: `java.lang.Object@<hash>`, the identity
      hash, which changes on every run, as an array's does; FerroBASIC
      prints a hash too, which no recording can compare.
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
- [x] `Const` ([#33](https://github.com/FeliceMente/ferrobasic/issues/33);
      sources in [research](research/level0-semantics.md#constants-docs-forum-recorded)),
      recorded with B4J 10.70, one test per question:
  - [x] the booklet's example, `Private Const Size As Int = 10` and
        `Public Const ItemNumber As Int = 100` (`level0/const_forms`);
  - [x] other types: `Long` (`level0/const_long`), `Double`
        (`level0/const_double`), `String` (`level0/const_string`),
        `Boolean` (`level0/const_boolean`); with no `As` it is a `String`
        (`level0/const_no_as`), as a `Dim` with no `As` is
        (`level0/dim_without_as`);
  - [x] forms from real projects: lower-case `const`
        (`level0/const_lower_case`), the value before `As`
        (`level0/const_value_before_as`);
  - [x] `Dim Const` in `Process_Globals` (`level0/const_dim_module`), with
        several names (`level0/const_dim_several`), and inside a Sub
        (`level0/const_dim_in_sub`);
  - [x] assigning one is refused: "Cannot modify Const variable."
        (`level0/const_assign`);
  - [x] one with no value is refused: "Const variable must be set in the
        declaration line." (`level0/const_no_value`);
  - [x] any expression as the value: arithmetic on literals
        (`level0/const_literal_arithmetic`), a concatenation
        (`level0/const_concatenation`), another constant
        (`level0/const_from_const`), a call (`level0/const_from_call`), a
        member of a literal (`level0/const_from_member`);
  - [x] a variable initialised from a constant declared after it reads it
        before it is set, `0`: constants are not inlined
        (`level0/const_used_before`);
  - [x] a constant as a `Select Case` label (`level0/const_select_case`);
  - [x] a `For` over an `Int` constant, and a `For Each v As Int` in a
        Sub over a module one, are refused: "Cannot modify Const
        variable." (`level0/const_for_counter`,
        `level0/const_for_each_variable`); so is assigning a field of a
        constant record (`level0/const_record_field`);
  - [x] a `Dim` in a Sub with a module `Int` constant's name, with a
        value, is refused: "Cannot redeclare a const variable."
        (`level0/const_local_dim`);
  - [x] a `Const` array with bounds and no value is refused as a `Const`
        without a value is (`level0/const_array_bounds`);
  - [x] in `Process_Globals`, a `Dim` of an `Int` constant's name after it,
        and a `Const` of a variable's name after its `Dim`, both with a
        literal value, build: each sets its value in turn (`level0/const_dim_after`
        prints 3, `level0/const_after_dim` prints 10);
  - [x] constants of type `List` (`level0/const_list`), `Map`
        (`level0/const_map`) and an array (`level0/const_array`); a `List`
        or `Map` constant's contents may change (`level0/const_list_mutate`,
        `level0/const_map_mutate`), an array constant's items may not:
        "Cannot modify Const variable." (`level0/const_array_mutate`).
- [ ] What `Const` leaves open. FerroBASIC refuses each until it is
      recorded:
  - [x] a variable declared both by a `Const` and by a `Dim` in
        `Process_Globals` may then be assigned in a Sub with the `Dim`
        last (recording: `level0/const_dim_after_assign`), not with the
        `Const` last ("Cannot modify Const variable.", recording:
        `level0/const_after_dim_assign`);
  - [x] after a `Const`, a `Dim` with no value builds and resets it to 0,
        and so does one of a `String`, one inside an `If`, and one with a
        value reading the constant (recordings:
        `level0/const_dim_after_no_value`, `level0/const_dim_after_string`,
        `level0/const_dim_after_in_if`, `level0/const_dim_after_from_const`);
  - [x] the same pair assigned after a `Dim` with no value, or of a
        `String`, a `String` `Dim` inside an `If`, a `Dim` with no value
        inside an `If`, and one inside an `If` that does not run, which
        keeps the constant's value (recordings:
        `level0/const_dim_after_no_value_assign`,
        `level0/const_dim_after_string_assign`,
        `level0/const_dim_after_in_if_string`,
        `level0/const_dim_after_in_if_no_value`,
        `level0/const_dim_after_in_if_not_taken`);
  - [ ] a loop over the pair or an item of it, assigning it in
        `Process_Globals`, and other types;
  - [x] a `Dim` with no value in a Sub with a module constant's name is
        refused: "Cannot redeclare a const variable." (recording:
        `level0/const_local_dim_no_value`);
  - [ ] a `For Each` without `As` over a constant is refused ("Current
        declaration does not match previous one", recording:
        `level0/const_for_each_untyped`); two `Const`s of an `Int` build,
        the second setting its value (recording: `level0/const_twice`),
        and whether they may then be assigned is open; any other loop over a constant
        (in `Process_Globals`, over a local constant, of another type than
        `Int`) or declaration again of its name (a third one).
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
