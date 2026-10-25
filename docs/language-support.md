# Language support

What a B4X-compatible program can use in FerroBASIC today, and what it cannot
use yet. This page is an index: the rules themselves are in
[spec/level0.md](spec/level0.md) and
[spec/level0-builtins.md](spec/level0-builtins.md), and each area links to its
conformance tests in [`tests/conformance/level0/`](../tests/conformance/level0/).

FerroBASIC currently implements **Level 0**: one console code module, run by
the interpreter (`ferro run`) or bundled into a standalone executable
(`ferro bundle`). What a level contains, and the milestone that brings it, is
in [spec/level0.md](spec/level0.md#language-levels) and
[roadmap.md](roadmap.md).

| Status | Meaning |
| --- | --- |
| ✅ | supported |
| 🟡 | partly supported: the row says what is missing |
| ❌ | not supported yet: the row says which level adds it, when one is planned |
| ⚠️ | supported, with a known difference from B4J ([deviations.md](deviations.md)) |

Using something marked ❌ is a compile error that names the construct and the
level that adds it, except where the row says otherwise. Behaviour that is not
yet settled by a recording from B4J is marked *(open)* in the specs and listed
in [open-questions.md](open-questions.md).

The "not yet" rows come from the
[B4J Core library reference](https://www.b4x.com/b4j/help/core.html), the
*B4X Language* booklet and the Apache-2.0 B4J runtime source (`jCore`); see
[research/level0-semantics.md](research/level0-semantics.md#sources).

## Projects and modules

| Feature | Status | Notes |
| --- | --- | --- |
| A single code module (`.bas`) | ✅ | the main module of a B4J non-UI app; [cli.md](spec/cli.md#inputs) |
| A B4J console project (`.b4j`), or the folder holding it | 🟡 | only a console app with the `jcore` library and no other modules or files; each line that stands in the way is an error ([project-files.md](spec/project-files.md)). `ferro bundle` writes its program into the project's folder ([cli.md](spec/cli.md#commands)). |
| `Sub Process_Globals`, `Sub AppStart (Args() As String)` | ✅ | [level0.md](spec/level0.md#program-structure); tests `process_globals_for`, `exit_code` |
| Several modules, calling another module's Subs | ❌ | Level 1. Today a call such as `Module2.F` is reported as an undeclared name. |
| Class modules (`Class_Globals`, `Initialize`, `Me`) | ❌ | Level 1 |
| UI apps (`AppType=JavaFX`, `Form`, layouts), B4XPages | ❌ | Level 4 |
| B4A, B4i and B4R projects | ❌ | later milestones ([roadmap.md](roadmap.md)) |
| Libraries other than the core one, `.b4xlib` | ❌ | Level 3 (library descriptors, M3) |

## Types

| Feature | Status | Notes |
| --- | --- | --- |
| `Boolean`, `Byte`, `Short`, `Int`, `Long`, `Float`, `Double`, `Char` | ✅ | [level0.md](spec/level0.md#types); tests `chars_and_ints`, `number_parsing` |
| `String`, `Object`, `Null` | ✅ | |
| Arrays of any rank: `Dim a(10) As Int`, `Dim m(3, 4) As Double` | ⚠️ | memory use differs (D2); tests `array_row_order`, `deep_equals` |
| `Type` records, also with array fields | ✅ | test `self_printing_record` |
| `List`, `Map`, `StringBuilder`, `Exception` | ✅ | see "Built-in objects" below |
| Library types: `Timer`, `JavaObject`, `B4XView`… | ❌ | Level 1 (`Timer`) and Level 3 (libraries) |

## Declarations

| Feature | Status | Notes |
| --- | --- | --- |
| `Dim`, `Private`, `Public`, several names in one statement, initialisers | ✅ | [level0.md](spec/level0.md#declarations-and-scope); test `dim_scope` |
| `Dim` with no `As` | 🟡 | provisionally `String` *(open)* |
| `Const` (`Private Const Size As Int = 10`) | ❌ | not assigned to a level yet ([#33](https://github.com/FeliceMente/ferrobasic/issues/33)); a precise diagnostic |
| Implicit `Int` counter of `For i = …` | ✅ | test `for_implicit_continue`; not in `Process_Globals` *(open)* |
| `For Each x In …` with no `As` and `x` undeclared | ❌ | the type is *(open)*: today it is an error asking for `As <type>` |

## Operators and expressions

| Feature | Status | Notes |
| --- | --- | --- |
| `+ - * /`, `Mod`, unary `-` | ✅ | `/` always gives `Double`; [level0.md](spec/level0.md#operators); test `division_and_mod` |
| `&` string concatenation | ✅ | its precedence is *(open)* |
| `= <> < > <= >=` | ✅ | `<` and `>` on Strings, and String-number comparisons, are *(open)* |
| `And`, `Or` (short-circuit), `Not(x)` | ✅ | their relative precedence is *(open)*; test `precedence_open` |
| Inline cast `x.As(T)` | ✅ | [level0-builtins.md](spec/level0-builtins.md#keywords-and-constants) |
| `IIf(condition, a, b)` | ✅ | test `iif_branches` |
| `Is` (`If v Is Button`) | ❌ | not assigned to a level yet; a precise diagnostic |
| Number literals: decimal, `0x` hexadecimal, exponent (`1e3`) | ✅ | test `number_parsing` |
| Line continuation ` _`, `:` between statements | ✅ | a comment after ` _` is *(open)*; tests `continuation_comment`, `colon_separator` |

## Statements

| Feature | Status | Notes |
| --- | --- | --- |
| `If … Then … Else If … Else … End If`, single-line `If` | ✅ | [level0.md](spec/level0.md#statements); test `control_flow` |
| `Select … Case … Case Else … End Select`, `Select True` | ✅ | tests `select_range`, `exit_in_select` |
| `For … To … Step … Next`, `For Each … In … Next` | ✅ | over arrays, `List`, `Map.Keys`, `Map.Values` |
| `Do While … Loop`, `Do Until … Loop` | ✅ | `Loop While`/`Loop Until` are syntax errors, as in B4X; test `do_loop_while` |
| `Exit`, `Continue`, `Return` | ✅ | `Exit` also leaves a `Select` |
| `Try … Catch … End Try`, `LastException` | ✅ | JDK messages are *(open)*; test `uncaught_error` |
| Statements nested more than 1000 deep | ⚠️ | a compile error (D1); test `deep_nesting` |
| `Sleep`, `Wait For`, resumable subs | ❌ | Level 2 |
| Events (`Sub Name_Event`), `Sender` | ❌ | Level 1 |

## Subs

| Feature | Status | Notes |
| --- | --- | --- |
| `Sub`, `Private Sub`, `Public Sub`, parameters, `As` return types | ✅ | arrays as parameters and results too |
| Recursion | ✅ | |
| Calls with or without parentheses for no arguments | ✅ | |
| `CallSub`, `CallSub2`, `CallSub3`, `CallSubDelayed`, `CallSubDelayed2`, `CallSubDelayed3`, `SubExists` | ❌ | Level 2 |

## Strings

| Feature | Status | Notes |
| --- | --- | --- |
| Literals `"…"`, also over several lines; `QUOTE`, `CRLF`, `TAB` | ✅ | no escape sequences, as in B4X; [level0.md](spec/level0.md#strings) |
| Smart strings `$"…${x}…"$` | ✅ | test `smart_string_format` |
| Formatters `$1.2{x}` (number) and `$xml{x}` | ✅ | |
| Formatters `$date{}`, `$time{}`, `$datetime{}` | ❌ | Level 1 (they need `DateTime`) |
| `NumberFormat`, `NumberFormat2` | ⚠️ | some doubles above 2^63 and subnormals differ in their 16th–17th digits (D3); tests `number_format_d3`, `java_exact_numbers` |
| Number to string conversion (`Log(2.7)`, `"" & x`) | ✅ | tests `number_to_string`, `java_exact_numbers` |
| `SmartStringFormatter` called directly | ❌ | not assigned to a level yet; today it is an undeclared name |

## Built-in objects

| Object | Status | Members |
| --- | --- | --- |
| `String` | 🟡 | all 18: `CharAt`, `CompareTo`, `Contains`, `EndsWith`, `EqualsIgnoreCase`, `GetBytes`, `IndexOf`, `IndexOf2`, `LastIndexOf`, `LastIndexOf2`, `Length`, `Replace`, `StartsWith`, `SubString`, `SubString2`, `ToLowerCase`, `ToUpperCase`, `Trim`. `GetBytes` only for `"UTF8"`/`"UTF-8"`. Test `string_members`. |
| `StringBuilder` | ✅ | all 7: `Initialize`, `Append`, `Insert`, `Remove`, `Length`, `ToString`, `IsInitialized`. Test `string_builder`. |
| `List` | ✅ | all 18: `Initialize`, `Initialize2`, `IsInitialized`, `Add`, `AddAll`, `AddAllAt`, `InsertAt`, `Get`, `Set`, `RemoveAt`, `Clear`, `Size`, `IndexOf`, `SubList`, `Sort`, `SortCaseInsensitive`, `SortType`, `SortTypeCaseInsensitive`. Tests `list_members`, `list_add_all_empty`, `list_from_2d_array`, `list_stale_sublist`, `list_sort_type_errors`. |
| `Map` | ✅ | all 13: `Initialize`, `IsInitialized`, `Put`, `Get`, `GetDefault`, `Remove`, `ContainsKey`, `Clear`, `Size`, `Keys`, `Values`, `GetKeyAt`, `GetValueAt`. Tests `map_members`, `map_order`, `map_get_key_at`. |
| Arrays | ✅ | `Length` |
| `Type` records | ✅ | `Initialize`, `IsInitialized`, fields |
| `Exception` (`LastException`) | ✅ | `Message`, `IsInitialized` |
| `Bit` | 🟡 | 11 of the 13 in the reference: `And`, `Or`, `Xor`, `Not`, `ShiftLeft`, `ShiftRight`, `UnsignedShiftRight`, `ToHexString`, `ToBinaryString`, `ToOctalString`, `ParseInt`. Missing: `ArrayCopy`, `InputStreamToBytes`, and the `Long` variants the runtime also has (`AndLong`, `ParseLong`, `ToHexStringLong`…). Test `bit_functions`. |
| `DateTime` | ❌ | Level 1 |
| `Regex` | ❌ | Level 1 |
| `Timer` | ❌ | Level 1 (needs events) |
| `File`, streams, `TextReader`/`TextWriter` | ❌ | Level 3 |

## Keywords and functions

| Group | Status | Members |
| --- | --- | --- |
| Constants | ✅ | `True`, `False`, `Null`, `CRLF`, `TAB`, `QUOTE`, `cPI`, `cE` |
| Output | 🟡 | `Log`, `LogError`, `LogDebug` (prints nothing, as in a release build *(open)*). Missing: `LogColor` (not assigned to a level yet). |
| Maths | ✅ | `Abs`, `Sqrt`, `Power`, `Floor`, `Ceil`, `Round`, `Round2`, `Logarithm`, `Max`, `Min`, `Sin`, `Cos`, `Tan`, `ASin`, `ACos`, `ATan`, `ATan2`, and the degree variants `SinD`, `CosD`, `TanD`, `ASinD`, `ACosD`, `ATanD`, `ATan2D`. Test `math_builtins`. |
| Numbers and characters | ✅ | `NumberFormat`, `NumberFormat2`, `IsNumber`, `Asc`, `Chr`. Test `chars_and_ints`. |
| Random numbers | ✅ | `Rnd`, `RndSeed`; a seeded sequence matches B4J. Test `random_seeded`. |
| Collections | ✅ | `Array`, `CreateMap` |
| Types | ⚠️ | `GetType` gives B4J's names for primitives, boxes, `String`, `List` and `Map`; other values differ *(open)* |
| Program | 🟡 | `ExitApplication`, `ExitApplication2`. Missing: `StartMessageLoop`, `StopMessageLoop` (Level 2). Test `exit_code`. |
| Errors | 🟡 | `LastException`. Missing: `ThrowException` (not assigned to a level yet). |
| Conversions | ❌ | `BytesToString`, `CharsToString`: Level 1 |
| Process environment | ❌ | `GetEnvironmentVariable`, `GetSystemProperty`, `SetSystemProperty`, `IsDevTool`: Level 1 |
| Dynamic calls and events | ❌ | `CallSub*`, `CallSubDelayed*`, `SubExists` (Level 2), `Sender`, `Me` (Level 1) |
| Resumable subs | ❌ | `Sleep`, `Wait For`: Level 2 |
| UI | ❌ | `Density`, `DipToCurrent`: Level 4 |

## Directives and attributes

| Feature | Status | Notes |
| --- | --- | --- |
| `#Region` / `#End Region` | ✅ | ignored, as in B4X |
| Project attributes (`#CommandLineArgs:`, `#MergeLibraries:`…) | ✅ | parsed and kept; Level 0 ignores them, except as listed in [cli.md](spec/cli.md) |
| Conditional compilation `#If` / `#Else` / `#End If` | ❌ | not assigned to a level yet; a precise diagnostic |
| Inline Java (`#If Java`) | ❌ | Level 3 |

## Running programs

| Feature | Status | Notes |
| --- | --- | --- |
| Interpreter: `ferro run` | ✅ | [cli.md](spec/cli.md) |
| Standalone executable for the current machine: `ferro bundle` | ✅ | Linux, Windows, macOS; programs run at interpreter speed |
| Uncaught runtime errors | 🟡 | the program stops with exit code 1 and the error type; B4J's exact output and exit code are *(open)* |
| JVM backend: real B4J projects unchanged | ❌ | M3 |
| Native and WebAssembly backends, microcontrollers | ❌ | M4, M5 |
| REPL, language server | ❌ | M1; `ferro repl` is reserved |
| Formatter | ❌ | planned, not scheduled; `ferro fmt` is reserved |

## Keeping this page current

A pull request that adds or changes a feature updates its row here.
`crates/hir/tests/language_support.rs` checks that every built-in the compiler
knows, and every construct it reports as belonging to a later level, is named
on this page.
