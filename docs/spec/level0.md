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
  wherever it is (recorded). A `For` may loop over it again anywhere,
  whichever loop declares it, and counts with a number as any `For` does
  (B4J: "Loop iterator should be a numeric variable", recorded). Another
  `For Each` without `As` over it is an error (B4J: "Current declaration
  does not match previous one", recorded); with `As` it is *(open)*. A
  `For Each v As T` of a Sub may have the name of a Sub and of a module
  variable of type `T`, and with another type it is an error as for any
  other name (recorded). In `Class_Globals`, a `Dim` with a Sub's
  name is *(open)*. A loop variable of `Process_Globals` named like a `Type`, `Bit`
  or a keyword is *(open)* too, and so is `For i As T` there: declare `i`
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
  construct no level adds yet says so: `Const` declarations and type tests
  (`x Is T`). Each gets one error, without follow-on errors.

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
  literal gets is *(open)*. Provisionally, an integer literal that fits is
  `Int`, a real literal is `Double`.

## Declarations and scope

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
- With no `As` at all, the type is *(open)*. Provisionally it is `String`.
- Arrays: `Dim a(10) As Int`, `Dim m(3, 4) As Double`. The bounds are
  expressions, evaluated when the `Dim` runs. `Dim a() As Int` is an empty array.
- **Scope is the whole Sub.** Every local is hoisted to the top of its Sub and
  is visible from any statement in it, whatever block declares it. Each time a
  `Dim` statement executes, its variables are reset: to the initialiser if
  there is one, otherwise to the default value. For `List`, `Map`, arrays and
  `Type` records this means a fresh object.
- Locals start at their default value on every call.
- The counter of `For i = …` is implicitly declared as `Int` if it was not
  declared before. Other undeclared names are *(open)*. Provisionally they are
  a compile error.
- Two declarations of the same name in one Sub: *(open)*. Provisionally they
  are allowed if the types match, and rejected otherwise.

## Conversions

Implicit conversions become explicit `Convert(from, to)` nodes in the MIR.

| From → to | Rule |
| --- | --- |
| number → wider number | Java widening. Exact for Byte/Short/Int → Int/Long/Double, Float → Double and every integer → Float/Double whose value fits the mantissa; otherwise rounded to nearest (Int 16777217 → Float 16777216, Long 9007199254740993 → Double 9007199254740992). |
| number → narrower integer | Java narrowing. Double → Int truncates toward zero *(open: forum level)*; out-of-range values follow Java rules (NaN → 0, saturating). Long → Int keeps the low 32 bits. |
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
  - two numbers compare numerically, as Double;
  - two Strings compare by value, case-sensitive; `<` and `>` on Strings are
    *(open)*;
  - `x = Null` / `x <> Null` test for null;
  - mixing a String and a number in a comparison is *(open)*.

## Statements

- **`:`** separates statements exactly like a line break. After a single-line
  `If c Then a : b`, only `a` is conditional.
- **`If`**: single line `If c Then s [Else s]`, or the block form with `Else If`
  (two words) and `Else`.
- **`Select expr`**: each `Case` holds a list of expressions, compared in order.
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
  - `a`, `b` and `s` are evaluated once, before the first iteration *(open)*;
    `s` defaults to 1;
  - the loop runs while `i <= b` (positive step) or `i >= b` (negative step);
  - after `Next`, `i` is incremented by `s`;
  - the value of `i` after the loop is *(open)*.
- **`For Each x [As T] In c`**: iterates arrays, `List`, `Map.Keys` and
  `Map.Values` in order (insertion order for a `Map`). Changing a collection
  while iterating it is *(open)*.
- **`Do While c … Loop`** / **`Do Until c … Loop`**: tests before each iteration.
  `Loop While`, `Loop Until` and a bare `Do … Loop` are syntax errors.
- **`Exit`** leaves the innermost enclosing loop, **or the innermost enclosing
  `Select`** if that is closer.
- **`Continue`** jumps to the next iteration of the innermost loop.
- **`Return [expr]`** leaves the Sub. A Sub with an `As` type that falls off its
  end returns that type's default value *(open)*.
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
