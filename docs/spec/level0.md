# Language specification — Level 0

Level 0 covers single-module console programs, like a B4J console app: enough
for algorithms, tests and the REPL, with no UI and no events.

The grammar is in [level0.ebnf](level0.ebnf). It is reconstructed from the
public B4X documentation and is the parser's starting point; the source of truth
remains the conformance corpus recorded from the original toolchain.

## Language levels

| Level | Contents | Milestone |
| --- | --- | --- |
| 0 | Code modules, `Sub`, `Dim`/`Private`/`Public`, primitive types, `String`, arrays, `Type`, `If`, `Select`, `For`, `For Each`, `Do`, `Try`/`Catch`, `List`, `Map`, `Log`, smart strings | M0–M1 |
| 1 | Classes (`Class_Globals`, `Initialize`, `Me`), multiple modules, `IsInitialized`, name-bound events | M2 |
| 2 | `Sleep`, `Wait For`, resumable subs, `CallSubDelayed` | M2 |
| 3 | Platform libraries, Java and native interop | M3+ |
| 4 | UI and layouts | M6 |

## Sample program

Level 0 must compile and run this program (also in
`tests/conformance/level0/prices_map.bas`):

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

## Types

| Type | Representation | JVM | LLVM |
| --- | --- | --- | --- |
| Boolean | true/false | `boolean` | `i1` (`i8` in memory) |
| Byte | signed 8-bit | `byte` | `i8` |
| Short | signed 16-bit | `short` | `i16` |
| Int | signed 32-bit | `int` | `i32` |
| Long | signed 64-bit | `long` | `i64` |
| Float | IEEE 754, 32-bit | `float` | `float` |
| Double | IEEE 754, 64-bit | `double` | `double` |
| Char | UTF-16 | `char` | `i16` |
| String | immutable | `java.lang.String` | pointer to a refcounted runtime string |
| Object | reference | `Object` | pointer with a type header |

## Rules

- **Lexical**: keywords and identifiers are case-insensitive. `'` starts a
  comment; ` _` at the end of a line continues it.
- **Conversions**: numbers and strings convert in assignments and calls; the
  checker inserts `Convert(from, to)` into the MIR.
- **Strings**: `=` compares by value; `&` concatenates, converting operands to strings.
- **Scope**: `Process_Globals` is module-level; a variable declared with `Dim`
  in a Sub is visible in the whole Sub (to verify).
- **Uninitialized objects**: `List` and `Map` need `Initialize`; using them
  before is a runtime error, as in B4X.
- **Number formatting**: number-to-string conversion is observable behaviour and
  must match B4X exactly.

Unverified points are listed in [../open-questions.md](../open-questions.md#language).
