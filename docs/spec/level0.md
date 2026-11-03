# Language specification — Level 0

Level 0 covers single-module console programs, like a B4J non-UI app: enough
for algorithms, tests and the REPL, with no UI, no classes and no events.

- Grammar: [level0.ebnf](level0.ebnf).
- Built-in keywords and library members: [level0-builtins.md](level0-builtins.md).
- How programs are run and built: [cli.md](cli.md).
- Where each rule comes from: [../research/level0-semantics.md](../research/level0-semantics.md).

The source of truth remains the conformance corpus recorded from the original
toolchain. Rules marked *(open)* are not settled. Implement them as stated, keep
them in one place so a recording can change them, and do not add tests that
depend on them, except pending ones.

## Language levels

| Level | Contents | Milestone |
| --- | --- | --- |
| 0 | Code module, `Sub`, `Dim`/`Private`/`Public`, primitive types, `String`, arrays, `Type`, `If`, `Select`, `For`, `For Each`, `Do`, `Try`/`Catch`, `List`, `Map`, `Log`, smart strings | M0–M1 |
| 1 | Classes (`Class_Globals`, `Initialize`, `Me`), multiple modules, `IsInitialized`, name-bound events | M2 |
| 2 | `Sleep`, `Wait For`, resumable subs, `CallSubDelayed` | M2 |
| 3 | Platform libraries, Java and native interop | M3+ |
| 4 | UI and layouts | M6 |

## Sample program

Level 0 must run this program (`tests/conformance/level0/prices_map.bas`):

```vb
Sub Process_Globals
    Private Prices As Map
End Sub

Sub AppStart (Args() As String)
    Prices.Initialize
    Prices.Put("coffee", 1.2)
    Prices.Put("croissant", 1.5)
    Dim Total As Double = 0
    For Each K As String In Prices.Keys
        Total = Total + Prices.Get(K)
    Next
    If Total > 2 Then Log("Big breakfast")
    Log($"Total: ${Total}"$)
End Sub
```

## Program structure

- A program is one code module: the members of the main module of a B4J
  non-UI project.
- `Sub Process_Globals` declares module variables. The main module must have
  one, even an empty one: B4J refuses to build it otherwise ("Sub
  process_globals is missing", recorded). It runs once,
  before anything else. Statements there run too. An undeclared `For`
  counter there is a module variable, an `Int`: other Subs read it, and
  their own undeclared counters of that name are the same variable
  (recorded). A `For Each v As T` variable there is a module variable of
  type `T` the same way (recorded). Every declaration of a name there, a
  `Dim`, a counter or a `For Each`, declares the same module variable, and
  must agree on its type, in source order: B4J refuses a later one that
  does not ("Current declaration does not match previous one", recorded).
  With the same type, declarations of a name there are one variable,
  whichever kinds and in any order (recorded). A counter over a name that
  a `Dim` declared is that variable, whatever its type. A `Dim` there
  cannot have a Sub's name (B4J: "Variable name cannot be the same as a
  sub name", recorded), but a loop variable may, there or in any Sub
  (recorded): an undeclared counter or a `For Each v As T`, as a typed
  `For v As T` with a Sub's name is an error (B4J: "Regular variable
  followed by '=' expected", recorded). Every use of such a name in an
  expression, bare, with arguments or as a statement, calls the Sub,
  whatever the loop variable's type, and assigning it is an error,
  wherever it is (recorded). Otherwise the name is a loop variable like
  any other, which the first loop over it in a Sub or in `Process_Globals`
  declares: every `For`, and every `For Each v As T`, may loop over it, one
  after the other, nested, three deep, from another Sub over a loop
  variable of `Process_Globals`, and of any type its variable has; nested,
  they share it (recorded: the `*_named_like_sub*` tests). A `For` counts
  with a number as any `For` does (B4J: "Loop iterator should be a
  numeric variable", recorded), and as the variable's type: after a
  `For Each v As Double`, in `Double` steps (recorded). A `For` first
  declares it an `Int`, so a later `For Each v As T` of another type is
  an error (B4J: "Current declaration does not match previous one",
  recorded); FerroBASIC applies this to any name a `For` declares, and to
  a later `Dim` or typed `For` of another type too, as declarations are
  taken in source order, as B4J does for an ordinary name (recorded:
  `for_each_other_type_after_counter`, `dim_other_type_after_counter`,
  `typed_counter_other_type_after_counter`). A `For Each` without `As` over it is an error wherever it is,
  first or not (B4J: the same message, recorded). After a refused loop
  over it, the program has an error already, and the loops after it are
  not judged. A
  `For Each v As T` of a Sub may have the name of a Sub and of a module
  variable of type `T`, and with another type it is an error as for any
  other name (recorded). In `Class_Globals`, a `Dim` with a Sub's
  name is *(open)*. A loop variable of `Process_Globals` cannot be named
  `Bit` (B4J's compiler fails, recorded); named like a `Type` it builds
  (recorded), and like a keyword or
  built-in it is *(open)*, and so is `For i As T` there: declare `i`
  with `Dim` and loop without `As`. All of this is only for the main module:
  in `Class_Globals` a loop must use a variable declared with `Dim`,
  without `As`.
- `Sub AppStart (Args() As String)` is the entry point. It is called after
  `Process_Globals` with the command-line arguments. The program ends when
  `AppStart` returns, with exit code 0.
- A program without `Process_Globals` or `AppStart` is rejected with a
  diagnostic.
- `ExitApplication` / `ExitApplication2(code)` end the program at once.
- **Nesting limit:** statements and expressions may nest at most 1000 levels
  deep in a Sub, counting each operator of a chain such as `1 + 1 + ...` and
  each member access of `s.Trim.Trim...` as a level; smart strings may nest
  100 deep. Deeper code is a compile error, one per Sub, instead of a crash.
  B4J's own limit (javac's) is *(open)*.
- A line continuation ` _` may be followed by a comment, with or without a
  space before the `'` (` _ ' comment`, ` _' comment`, recorded).
- `#Region` / `#End Region` lines are trivia. Project attributes (`#Name: value`)
  are parsed and kept. Level 0 ignores them, except as listed in [cli.md](cli.md).
- Unsupported constructs get a diagnostic naming the construct and the level
  that adds it. Examples are `Class_Globals`, `Wait For`, `Sleep`, `#If`,
  inline Java (`#If Java`), and a library other than the core one. A
  construct no level adds yet says so: type tests (`x Is T`). Each gets one
  error, without follow-on errors.

## Types

| Type | Representation | Default value | JVM | LLVM |
| --- | --- | --- | --- | --- |
| Boolean | true/false | `False` | `boolean` | `i1` (`i8` in memory) |
| Byte | signed 8-bit | 0 | `byte` | `i8` |
| Short | signed 16-bit | 0 | `short` | `i16` |
| Int | signed 32-bit | 0 | `int` | `i32` |
| Long | signed 64-bit | 0 | `long` | `i64` |
| Float | IEEE 754, 32-bit | 0 | `float` | `float` |
| Double | IEEE 754, 64-bit | 0 | `double` | `double` |
| Char | UTF-16 code unit | `Chr(0)` | `char` | `i16` |
| String | immutable UTF-16 text | `""` | `java.lang.String` | pointer to a refcounted runtime string |
| Object | any value or `Null` | `Null` | `Object` | pointer with a type header |
| List, Map | reference to a collection, uninitialised by default | uninitialised | wrapper | runtime object |
| arrays `T()` | fixed size, zero-based, any rank | per element | `T[]` | runtime object |
| `Type` records | reference to an object with fields | an object whose fields hold Java defaults (0, `False`, `Null` for String and objects); `Initialize` then sets B4X's defaults (`""`, fresh uninitialised List/Map) and makes `IsInitialized` True. Fields can be read and written before `Initialize`. | class | runtime object |

- Primitives (the numbers, `Boolean`, `Char`) are copied on assignment and
  when passed. Everything else, `String` included, copies the reference.
- `Object` holds a boxed value. The box remembers its type: `Int` 1 and
  `Double` 1 stored in a `Map` are different keys. Which boxed type a numeric
  literal gets is recorded: an integer literal that fits is an `Int`, a
  larger one a `Long`, a hex one up to `0xFFFFFFFF` an `Int`, and a real
  literal a `Double` (`literal_boxed_type`, `decimal_literal_over_int`,
  `hex_literal`).

## Declarations and scope

- `Const` after `Dim`, `Private` or `Public` (in any letter case) makes
  every name of the statement a constant, in `Process_Globals` or in a Sub,
  of any type, `List`, `Map` and arrays included, with any expression as
  its value (recorded). Each must be given its value there (B4J: "Const
  variable must be set in the declaration line.", recorded). Without `As`
  it is a `String` (recorded), as a `Dim` without `As` is (recorded:
  `dim_without_as`). Its value is
  computed where it is declared, in order, as a `Dim`'s: a module variable
  initialised from a constant declared after it reads `0` (recorded).
  A `Const` array with bounds must be given its value too (recorded).
  Assigning a constant, an item of a constant array or a field of a
  constant record is an error, and so are a `For` over an `Int` constant
  and a `For Each v As Int` in a Sub over a module one (B4J: "Cannot
  modify Const variable.", recorded); calls that change a `List` or `Map`
  constant's contents are not (`Set`, `Put`, recorded). A `Dim` in a Sub
  with a module `Int` constant's name, with a value, is an error (B4J:
  "Cannot redeclare a const variable.", recorded). In `Process_Globals`,
  two declarations of a name, with no other declaration of or loop over
  it there, are one variable, each setting its value in turn (recorded):
  a `Const` of an `Int` or a `String` with a literal value, then a `Dim`
  of its type, with a value or not, a statement of `Process_Globals` or of
  an `If` there, which a Sub may then assign (`const_dim_after*`); an
  `Int` `Dim`, then a `Const`, which cannot be assigned
  (`const_after_dim_assign`); and two `Const`s of an `Int` (`const_twice`),
  the last two with literal values and statements of `Process_Globals`
  itself. Assigning one of two `Const`s, assigning any of them in
  `Process_Globals`, a loop over them or an item of them is *(open)*, so a
  compile error, reported once. A
  constant may be a `Select Case` label (recorded). Any other loop over a
  constant or declaration again of its name is *(open)*, so a compile
  error. After a declaration again in `Process_Globals` refused so, the
  program has an error already: the uses of the name are refused with no
  error of their own, wherever the Subs are.
- `Dim`, `Private` and `Public` in a Sub all declare a local. In
  `Process_Globals` they declare module variables. `Private` and `Public` only
  matter with several modules (Level 1).
- A local `Dim` or `For Each v As T` with a module variable's name and
  type is that module variable, and re-executing the `Dim` resets it
  (recorded). With another type B4J refuses it ("Current declaration does
  not match previous one", recorded). An undeclared `For` counter of that
  name is the module variable; a `For i As T` of that name in a Sub is
  *(open)*, so a compile error. A local `Dim` cannot have a Sub's name
  (B4J: "Variable name cannot be the same as a sub name", recorded); a loop
  variable may, and its uses mean the Sub (recorded, see "Program
  structure").
- A parameter cannot have a module variable's name (B4J: "Parameter name
  cannot hide global variable name", recorded).
- Forms: `Dim x As Int = 5`; `Dim a = 1, b = 2 As Int`; `Private i, j, k As Int`.
  A name without `As` takes the type of the next `As` to its right in the same
  statement.
- With no `As` at all, the type is `String` (recorded: `dim_without_as`).
- Arrays: `Dim a(10) As Int`, `Dim m(3, 4) As Double`. The bounds are
  expressions, evaluated when the `Dim` runs. `Dim a() As Int` is an empty array.
- **Scope is the whole Sub.** Every local is hoisted to the top of its Sub and
  is visible from any statement in it, whatever block declares it. Each time a
  `Dim` statement executes, its variables are reset: to the initialiser if
  there is one, otherwise to the default value. For `List`, `Map`, arrays and
  `Type` records this means a fresh object.
- Locals start at their default value on every call.
- The counter of `For i = …` is implicitly declared as `Int` if it was not
  declared before. A name that nothing in the Sub or the module declares,
  and that is not a Sub, a built-in or a `Type`, is declared by an
  assignment to it, as a `String` (recorded: `undeclared_variable`,
  `undeclared_variable_type`); with a `Dim` or a loop of
  it anywhere in the Sub, the hoisted declaration is that variable. Reading a name nothing
  declares or assigns is *(open)*, so a compile error.
- Two declarations of the same name in one Sub: *(open)*. Provisionally they
  are allowed if the types match, and rejected otherwise.

## Conversions

Implicit conversions become explicit `Convert(from, to)` nodes in the MIR.

| From → to | Rule |
| --- | --- |
| number → wider number | Java widening. Exact for Byte/Short/Int → Int/Long/Double, Float → Double and every integer → Float/Double whose value fits the mantissa; otherwise rounded to nearest (Int 16777217 → Float 16777216, Long 9007199254740993 → Double 9007199254740992). |
| number → narrower integer | Java narrowing. Double → Int truncates toward zero (recorded: `double_to_int_assign`); out-of-range values follow Java rules (NaN → 0, saturating). Long → Int keeps the low 32 bits. |
| number → String | `NumberToString`, see below |
| Boolean → String | `true` / `false` (lower case) |
| String → number | parsed as a Java `double` literal, then narrowed. An unparsable string raises `NumberFormatException`. |
| String → Boolean | only `"true"` / `"false"` (case-sensitive), else a runtime error |
| String → Char | first character, or `Chr(0)` for an empty string |
| Object → T | unbox. A number box converts to any numeric type; a String converts as above; otherwise a runtime `ClassCastException`. |
| Null → String | the text `"null"`, also when assigned, passed or returned as a String: B4J compiles `s = Null` to `_s = BA.ObjectToString(Common.Null)` (generated code, recorded). A String that is null (an unset record field) stays null. |
| any other value → String | `BA.ObjectToString` (Java `String.valueOf`): B4X compiles `Log(LastException)` and `"e: " & LastException` this way (generated code) |

### Number to string

This is observable behaviour and must match exactly:

- `Byte`, `Short`, `Int`, `Long`: plain decimal, `-` for negatives.
- `Double`: the shortest decimal that reads back as the same double, in Java
  `Double.toString` format (JDK 19 and later), and then a trailing `.0` is
  removed:
  - plain notation when 1e-3 ≤ |x| < 1e7: `2.7`, `3` (not `3.0`), `0.001`;
  - otherwise `d.dddE±n` with at least one digit after the point: `1.0E7`,
    `1.0E-4`, `1.2345E10` (the `.0` before `E` stays);
  - `NaN`, `Infinity`, `-Infinity`, and `-0.0` → `-0`.
- `Float`: widened to Double first, so `1.1` as a Float prints `1.100000023841858`.

## Operators

From lowest to highest precedence. Levels marked *(open)* are provisional.

| Level | Operators | Notes |
| --- | --- | --- |
| 1 | `Or` | short-circuit (recorded: `And` binds tighter) |
| 2 | `And` | short-circuit |
| 3 | `=` `<>` `<` `>` `<=` `>=` | left-associative; whether B4X accepts a chain like `a < b < c` is *(open)*, so the parser accepts it |
| 4 | `&` | binds tighter than comparisons; an operand holding an arithmetic operator (`+ - * / Mod`) or a negation needs parentheses (a syntax error otherwise, as in B4J) |
| 5 | `+` `-` | |
| 6 | `*` `/` `Mod` | |
| 7 | unary `-` | |

- `Not(x)` is a function. Bitwise operations are the `Bit` functions; there are
  no bitwise operators.
- **Arithmetic type**: if both operands are integers, the result is the wider of
  `Int` and `Long` (`Byte` and `Short` widen to `Int`), with Java wrap-around on
  overflow. If either operand is `Float` or `Double`, the result is `Double`.
  A String operand of `+ - * Mod` is parsed as a number.
- `/` **always returns Double**, even for two integers: `55 / 10` is `5.5`.
- `Mod` is the Java remainder: its sign follows the dividend (`-1 Mod 24` is `-1`).
- Integer division by zero (`Mod`) raises `ArithmeticException`. Double
  division by zero gives `Infinity` or `NaN`.
- `&` converts both sides to String and concatenates. `"x" & 1 + 2`,
  `1 + 2 & "x"`, `"x" & 6 / 2` and `"x" & -1` are errors: write
  `"x" & (1 + 2)`.
- Comparisons:
  - `=` and `<>`: the left operand decides. A number on the left compares
    numerically, as Double, converting a String on the right; a String on
    the left compares as Strings, by value and case-sensitive (recorded:
    `mixed_compare_number_left`, `mixed_compare_string_left`);
  - `<`, `>`, `<=` and `>=` always compare numbers, as Double: a String
    operand is parsed, so `"a" < "b"` raises `NumberFormatException`
    (recorded: `string_compare_less`); a Boolean or a Char operand is an
    error (recorded: `boolean_less`, `char_string_compare`); a String next
    to an `Object` is *(open)*, so an error;
  - `x = Null` / `x <> Null` test for null; with an `Int` on the left, the
    `Null` is converted as its text, `"null"`, which raises
    `NumberFormatException` (recorded: `null_number_compare`); with another
    number type it is *(open)*, so an error;
  - comparisons do not chain: `a < b < c` and `If a = b = c Then` are
    errors (recorded: `chained_comparison_less`,
    `chained_comparison_equal`), while `x = a = b` assigns one comparison;
  - a Char `=` an `Int` converts the `Int` to a Char, the first character
    of its text, so `"A" = 65` is `False` (recorded: `char_number_compare`);
    the number on the left, or another number type, is *(open)*, so an
    error.

## Statements

- **`:`** separates statements exactly like a line break. After a single-line
  `If c Then a : b`, only `a` is conditional.
- **`If`**: single line `If c Then s [Else s]`, or the block form with `Else If`
  (two words) and `Else`.
- **`Select expr`**, also written `Select Case expr` (recorded:
  `select_case_synonym`): each `Case` holds a list of expressions. The scrutinee
  is evaluated, then every `Case` value, in order, then they are compared
  in order (recorded: `case_values_evaluated`).
  - Numbers match by numeric value (as Double), also when an `Object` holds
    the number; other values match with `equals` on the boxed values, so
    String "1" does not match `Case 1`. If an `Object` scrutinee holds a
    number, a case that is not a number raises `ClassCastException` (`Null`:
    `NullPointerException`), and a `Null` scrutinee raises
    `NullPointerException`. With a Char
    scrutinee the cases are converted to Char (`Select Txt.CharAt(i)` with
    `Case "A"`, from the booklet).
  - The first matching `Case` runs, then control leaves the `Select`; there is no
    fall-through. `Case Else` runs if nothing matched.
  - There are no ranges. `Select True` with `Case cond` is the idiom for conditions.
- **`For i = a To b [Step s]`**:
  - `a`, `b` and `s` are evaluated once, before the first iteration
    (recorded: `for_limit_evaluated_once`): `s` first, then `b`, then `a`,
    which is assigned to `i` last (recorded: `for_evaluation_order`), so `s`
    or `b` reading `i` reads its value before the loop;
    `s` defaults to 1;
  - the loop runs while `i <= b` (positive step) or `i >= b` (negative step);
  - after `Next`, `i` is incremented by `s`;
  - after the loop, `i` holds the first value past `b`, or `a` when the
    loop never ran (recorded: `for_counter_after_loop`).
- **`For Each x [As T] In c`**: iterates arrays, `List`, `Map.Keys` and
  `Map.Values` in order (insertion order for a `Map`). A `List` that grows
  while it is iterated is iterated over the items it had when the loop
  began (recorded: `list_modified_in_for_each`); changing a `Map` while
  iterating it is *(open)*. Without `As`, `x` cannot be a local declared
  before the loop, whatever its type (B4J: "Current declaration does not
  match previous one", recorded: `for_each_untyped_local`,
  `for_each_untyped_other_type`); over a local `Object`, a parameter, a
  local declared only after the loop, a module variable, in
  `Process_Globals` or over an undeclared name it is *(open)*, so an
  error. In `Class_Globals` a loop uses a variable declared with `Dim`.
- **`Do While c … Loop`** / **`Do Until c … Loop`**: tests before each iteration.
  `Loop While`, `Loop Until` and a bare `Do … Loop` are syntax errors.
- **`Exit`** leaves the innermost enclosing loop, **or the innermost enclosing
  `Select`** if that is closer.
- **`Continue`** jumps to the next iteration of the innermost loop.
- **`Return [expr]`** leaves the Sub. In a Sub without `As`, `Return expr`
  builds (recorded: `return_value_untyped_sub`); the value is evaluated, and
  the call has no value *(open: B4J declares such Subs as returning a
  `String`)*. A Sub with an `As` type that falls off its
  end returns that type's default value: 0, or an empty `String` (recorded:
  `typed_sub_without_return`, `typed_string_sub_without_return`).
- **`Try … Catch … End Try`** catches every runtime error raised in the `Try`
  block, including from called Subs. `LastException` then holds the innermost
  cause. `LastException.Message` is `<kind>`, then `": "` and the message when
  there is one, e.g. `java.lang.RuntimeException: Object should first be
  initialized (List).`. It is observable output; messages produced by the JDK
  itself (index and parse errors) are *(open)*.
- **Uncaught runtime errors**: the program stops. A diagnostic goes to stderr,
  with the error type and the source line, and the exit code is non-zero
  *(open: B4J's exact output and exit code)*. Only the error type and the fact
  that the program stopped are compared.

## Strings

- Literals: `"..."`, with no escape sequences; they may span lines. `QUOTE`,
  `CRLF` (a line feed, `Chr(10)`) and `TAB` are constants.
- Smart strings `$"..."$` may span lines and contain `"`. A placeholder is
  `$[formatter]{expr}`:
  - no formatter: the value converted as by `&`;
  - `N` or `N.M`: `NumberFormat(value, N, M)`. With no `.M`, the fraction digits
    are unlimited. If the value is not a number, the result is `NaN`;
  - `xml`: escapes `"` `'` `<` `>` `&`;
  - `date`, `time`, `datetime`: Level 1 (they need `DateTime`);
  - formatter names are case-insensitive.
- `NumberFormat(x, minInt, maxFrac)`:
  - US locale, with three-digit grouping by `,`;
  - rounding is half-even, on the decimal value of the double;
  - the digits are those of the JDK's older double conversion, which differs
    from `Double.toString` on integers from 2^53 to 2^63 and on powers of two
    ([research](../research/level0-semantics.md#java-exact-numbers-source-measured));
  - for example `NumberFormat(12345.6789, 0, 2)` is `12,345.68`.
- String members are listed in [level0-builtins.md](level0-builtins.md).
  Indices are zero-based UTF-16 positions.

## Output

`Log(x)` writes `x`, converted to String, plus `\n` to stdout. `LogError` writes
to stderr. The conformance tests compare stdout byte for byte, after the
normalisation in [compatibility-testing.md](../compatibility-testing.md#normalization).

Unverified points are listed in [../open-questions.md](../open-questions.md#language).
