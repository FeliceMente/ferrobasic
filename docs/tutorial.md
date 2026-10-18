# Tutorial: the `ferro` command line

This tutorial walks through the `ferro` tool with small, complete programs:
checking them, running them and turning them into standalone executables.

It covers what works today, **Level 0**: one code module, the language core,
`List`, `Map`, `StringBuilder` and `Type` records, with output through `Log`.
The full list is in [spec/level0.md](spec/level0.md) and
[spec/level0-builtins.md](spec/level0-builtins.md). Every output shown below
was produced by `ferro` itself.

## 1. Get `ferro`

You need a recent stable Rust toolchain. From the repository root:

```sh
cargo build --release -p ferro-cli
```

The executable is `target/release/ferro`. Put it on your `PATH`, or call it
by its full path. The examples below just write `ferro`.

```console
$ ferro version
ferro 0.0.2-dev
```

`ferro help` (or `ferro` with no arguments) lists the commands. Three of them
work at Level 0:

| Command | What it does |
| --- | --- |
| `ferro check <file>` | parses and type-checks the program and prints any errors |
| `ferro run <file> [-- <args>...]` | checks it, then runs it with the interpreter |
| `ferro build <file> [-o <output>]` | checks it, then bundles it with the interpreter into a standalone executable |

`repl`, `test` and `fmt` are listed but not implemented yet.

## 2. Hello, world

A program is one module. Execution starts at `Sub AppStart`, which receives
the command-line arguments. Save this as `hello.bas`:

```vb
Sub AppStart (Args() As String)
    Log("Hello, world!")
End Sub
```

`Log` writes a line to standard output. Check the program, then run it:

```console
$ ferro check hello.bas
$ ferro run hello.bas
Hello, world!
```

`check` prints nothing when the program is correct, and exits with code 0.

The file can be just the module's members, as here. A B4J module file, with its
`Key=Value` header lines up to `@EndOfDesignText@`, works too.

## 3. Arguments and exit codes

`ferro run` passes everything after `--` to `AppStart`. Save this as
`greet.bas`:

```vb
Sub AppStart (Args() As String)
    If Args.Length = 0 Then
        Log("Usage: greet <name>...")
        ExitApplication2(1)
    End If
    For Each Name As String In Args
        Log($"Hello, ${Name}!"$)
    Next
End Sub
```

```console
$ ferro run greet.bas -- Ada Linus
Hello, Ada!
Hello, Linus!
$ ferro run greet.bas
Usage: greet <name>...
$ echo $?
1
```

- `$"..."$` is a *smart string*: `${expr}` inserts a value.
- `ExitApplication2(n)` ends the program at once with exit code `n`.
- When `AppStart` returns normally, the exit code is 0.
- The `--` is required: `ferro run greet.bas Ada` is a usage error (exit code 2).

## 4. Numbers, loops and formatting

Save this as `table.bas`:

```vb
Sub AppStart (Args() As String)
    Dim Rate As Double = 0.22
    For Price = 10 To 50 Step 20
        Dim Gross As Double = Price * (1 + Rate)
        Log($"${Price} -> $1.2{Gross}"$)
    Next
    Log(7 / 2)
    Log(7 Mod 2)
    Log(NumberFormat(1234567.891, 0, 2))
End Sub
```

```console
$ ferro run table.bas
10 -> 12.2
30 -> 36.6
50 -> 61
3.5
1
1,234,567.89
```

Things to notice, all as in B4J:

- The `For` counter `Price` needs no `Dim`: an undeclared counter is an `Int`.
- `$1.2{Gross}` formats with `NumberFormat(Gross, 1, 2)`: at least 1 integer
  digit and at most 2 decimals. That is why 61 prints without a fraction.
- `/` always divides as `Double`, so `7 / 2` is `3.5`. `Mod` is the remainder.
- A `Double` with no fraction prints without `.0`.
- `NumberFormat` groups thousands with `,` and rounds half-even.

## 5. Subs, return values and `Select`

Save this as `subs.bas`:

```vb
Sub AppStart (Args() As String)
    For n = 1 To 5
        Log(n & "! = " & Factorial(n))
    Next
    Log("20! = " & Factorial(20))
    Log(Describe(7))
    Log(Describe(0))
End Sub

Sub Factorial (n As Int) As Long
    If n <= 1 Then Return 1
    Return n * Factorial(n - 1)
End Sub

Sub Describe (n As Int) As String
    Select n
        Case 0
            Return "zero"
        Case 1, 3, 5, 7, 9
            Return "odd digit"
        Case Else
            Return "something else"
    End Select
End Sub
```

```console
$ ferro run subs.bas
1! = 1
2! = 2
3! = 6
4! = 24
5! = 120
20! = 2432902008176640000
odd digit
zero
```

- A Sub with `As <type>` returns a value.
- `&` joins values as text.
- `Long` is 64-bit, so `20!` fits. Integer overflow wraps around, as in Java.
- A `Case` can list several values. The first matching `Case` runs, and there
  is no fall-through.

## 6. Lists, maps and `Type` records

This program counts words. Save it as `words.bas`:

```vb
Sub Process_Globals
    Type Word (Text As String, Count As Int)
End Sub

Sub AppStart (Args() As String)
    Dim Text As String = "the cat and the dog and the bird"
    Dim Counts As Map
    Counts.Initialize
    For Each W As String In SplitWords(Text)
        Counts.Put(W, Counts.GetDefault(W, 0) + 1)
    Next
    Log(Counts.Size & " distinct words")

    Dim Words As List
    Words.Initialize
    For Each K As String In Counts.Keys
        Dim Entry As Word
        Entry.Initialize
        Entry.Text = K
        Entry.Count = Counts.Get(K)
        Words.Add(Entry)
    Next
    Words.SortType("Count", False)
    For Each Entry As Word In Words
        Log($"${Entry.Text}: ${Entry.Count}"$)
    Next
End Sub

' Level 0 has no Regex: split on spaces by hand.
Sub SplitWords (Text As String) As List
    Dim Result As List
    Result.Initialize
    Dim Start As Int = 0
    For i = 0 To Text.Length
        If i = Text.Length Or Text.CharAt(i) = " " Then
            If i > Start Then Result.Add(Text.SubString2(Start, i))
            Start = i + 1
        End If
    Next
    Return Result
End Sub
```

```console
$ ferro run words.bas
5 distinct words
the: 3
and: 2
cat: 1
dog: 1
bird: 1
```

- `Process_Globals` runs once, before `AppStart`. It declares module
  variables and `Type`s.
- A `List` or `Map` must be `Initialize`d before use.
- A `Map` keeps keys in insertion order.
- `SortType` sorts records by a field and is stable: `cat`, `dog` and `bird`
  keep their order.
- `Dim Entry As Word` inside the loop creates a fresh record on each
  iteration. Without it, every list item would be the same record.
- Lines starting with `'` are comments.

## 7. When things go wrong

### Compile errors

`check`, `run` and `build` all refuse a program with errors. Each error shows
the file, line and column, then the source line with a caret under the problem:

```vb
Sub AppStart (Args() As String)
    Dim Count As Int = 3
    Log(Cuont + 1)
    Dim s As String = "abc"
    Log(s.Lenght)
    Sleep(100)
End Sub
```

```console
$ ferro check typo.bas
typo.bas:3:9: error: `Cuont` is not declared
      Log(Cuont + 1)
          ^
typo.bas:5:11: error: String has no member `Lenght` at Level 0
      Log(s.Lenght)
            ^
typo.bas:6:5: error: `Sleep` is not supported at Level 0: it needs resumable subs (Level 2)
      Sleep(100)
      ^
3 error(s)
$ echo $?
3
```

A feature from a later level is never silently ignored: the message names
what is missing and which level adds it.

### Runtime errors

An error that is not caught stops the program. Output printed before the error
is kept:

```vb
Sub AppStart (Args() As String)
    Dim Items As List
    Items.Initialize
    Items.Add("first")
    Log(Items.Get(0))
    Log(Items.Get(5))
    Log("never printed")
End Sub
```

```console
$ ferro run crash.bas
first
Error occurred on line: 6 (Main)
java.lang.IndexOutOfBoundsException: Index 5 out of bounds for length 1
$ echo $?
1
```

The error is named after the Java exception B4J raises in the same place.

### Catching errors

`Try … Catch` catches runtime errors, including errors in called Subs.
`LastException.Message` describes the error:

```vb
Sub AppStart (Args() As String)
    Log(ParseOr("42", -1))
    Log(ParseOr("forty-two", -1))
End Sub

Sub ParseOr (Text As String, Fallback As Int) As Int
    Try
        Dim n As Int = Text
        Return n
    Catch
        Log("cannot parse '" & Text & "': " & LastException.Message)
        Return Fallback
    End Try
End Sub
```

```console
$ ferro run safe.bas
42
cannot parse 'forty-two': java.lang.NumberFormatException: For input string: "forty-two"
-1
```

Assigning a String to an `Int` parses it, and raises `NumberFormatException`
if it is not a number.

## 8. Standalone executables

`ferro build` turns a program into a single executable for the machine you
are on:

```console
$ ferro build greet.bas
$ ./greet Grace
Hello, Grace!
$ ./greet
Usage: greet <name>...
$ echo $?
1
```

- By default the output is named after the source file (`greet`, or
  `greet.exe` on Windows) and goes in the current directory.
- `-o` chooses another path, for example `ferro build greet.bas -o bin/greet`.
  The directory must already exist.
- An existing file at the output path is replaced.
- Every argument goes to `AppStart`; there is no `--` here.
- Output and exit codes are the same as `ferro run`'s, runtime errors included.

The executable needs neither FerroBASIC nor a JVM. It is `ferro` itself, with
the checked program appended (about 1.4 MB). You can copy it to another machine
with the same OS and architecture.

> **Note:** at Level 0, `ferro build` *bundles* rather than compiles. The
> executable is the `ferro` interpreter plus your program's checked source,
> which is parsed and interpreted at start-up exactly as `ferro run` does. So
> it runs at interpreter speed, and the source can be recovered from the file.
> Native code generation (M4) will replace this behind the same command.

- On Linux it depends only on the system C library.
- For a fully static Linux binary, build `ferro` for musl first:

  ```sh
  rustup target add x86_64-unknown-linux-musl
  cargo build --release -p ferro-cli --target x86_64-unknown-linux-musl
  target/x86_64-unknown-linux-musl/release/ferro build greet.bas
  ```

- Building for another OS or architecture is not supported yet.
- On macOS the program goes inside the executable rather than after it, so
  the result can be code-signed. It is signed ad hoc, which is enough to run
  it on your own Mac.
- Tested on Linux and macOS (Apple Silicon, and Intel binaries through
  Rosetta). Windows is not tested yet.

## 9. Exit codes

| Code | Meaning |
| --- | --- |
| 0 | success, or the program ended normally |
| *n* | the program called `ExitApplication2(n)` |
| 1 | the program stopped on an uncaught runtime error |
| 2 | usage error: bad arguments, unreadable input, output not writable |
| 3 | the program has compile errors |

A standalone executable uses 0, *n* and 1 the same way, and 2 if its embedded
program is damaged.

## 10. What Level 0 does not have yet

The compiler tells you when you use one of these:

- **classes and several modules:** `Class_Globals` (Level 1);
- **libraries:** `DateTime`, `File`, `Regex`, `Timer`;
- **resumable subs and events:** `Sleep`, `Wait For` (Level 2);
- **conditional compilation:** `#If`;
- **real project files:** `ferro` reads single modules today, and `.b4j`
  projects come next.

See [roadmap.md](roadmap.md) for when each arrives, and
[deviations.md](deviations.md) for known differences from B4J.
