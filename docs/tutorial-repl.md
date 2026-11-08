# Tutorial: using the FerroBASIC REPL

The REPL lets you try BASIC code without creating a project. It uses the
same language and interpreter as `ferro run`. For the language itself, see
the [CLI and BASIC tutorial](tutorial.md).

Build FerroBASIC as explained there, then start a session:

```sh
ferro repl
```

`> ` asks for a new submission. `... ` asks you to finish a block, Sub,
string or explicit line continuation. Type the code after the prompt;
do not type the prompt itself. Output appears underneath. Each transcript
below starts in a **new session**. All displayed results are checked by tests.

## Getting help and leaving

```repl
> :help
Enter BASIC statements, expressions, Types or complete Subs.
:help    Show this help
:cancel  Discard pending input
:reset   Clear the session
:quit    Leave the REPL
> :quit
```

EOF also leaves the session: Ctrl+D on Unix, or Ctrl+Z followed by Enter in
a Windows console. Ctrl+C terminates the process; it does not cancel running
BASIC code and return you to a prompt. History and line editing are deferred;
your terminal may provide its own basic editing.

## Expressions and statements

Expressions print automatically, with the same formatting as `Log`.
Statements execute once. Variables and their values stay in the session.

```repl
> Dim n As Int = 2
> n + 3
5
> n = n + 1
> n
3
> (n = 3)
true
> Log("hello")
hello
> :quit
```

`n = 3` is an assignment. Parenthesize `(n = 3)` when you want a comparison.
A call returning a value prints it once; a void call adds no extra output.

Variable types stay fixed. A second `Dim` of the same type resets its value,
just as in an ordinary BASIC program. To change a variable's type, use
`:reset` to clear the whole session first. Constants cannot be redefined.

## Multiline code and Subs

Blocks and Subs submit when you enter their closing line; blank lines do not
finish them. Whole Subs can refer to session variables. Replace a Sub by
entering its definition again with the same parameter types/count and return
type. Existing callers then use the new body.

```repl
> Sub NextNumber(n As Int) As Int
...     Return n + 1
... End Sub
> NextNumber(2)
3
> Sub NextNumber(n As Int) As Int
...     Return n + 2
... End Sub
> NextNumber(2)
4
> For i = 1 To 2
...     Log(i)
... Next
1
2
> :quit
```

Submit definitions one at a time. A new Sub must use names already declared
in the session (or itself for recursion); define helper Subs before callers.
The REPL supplies the module structure: do not wrap input in `AppStart` or
`Process_Globals`, or paste project-file headers and attributes.

## Lists, aliases and records

Objects retain their identity between submissions. An alias sees mutations
made through the original variable, and vice versa.

```repl
> Dim items As List
> items.Initialize
> items.Add("first")
> Dim other As List = items
> other.Add("second")
> items.Size
2
> items.Get(1)
second
> Type Point(X As Int)
> Dim p As Point
> p.Initialize
> p.X = 7
> p.X
7
> :quit
```

Types remain available in subsequent submissions. A Type cannot be redeclared;
use `:reset` to change its fields. Session variables are module globals, so
Subs may use them, and a parameter cannot hide one of their names. New
names that would change the local bindings of an existing Sub are refused;
choose a different name instead of changing what old code means.

## Errors do not end an interactive session

A compile error rejects the submission without changing the accepted session.
A runtime error stops that submission, but completed changes remain. The
location `<repl:N>` identifies a submission, with line and column after it.
For a multiline Sub, subsequent runtime errors refer to its original input.

```repl
> Dim n As Int = 0
> Dim n As String = "wrong type"
<repl:2>:1:5: error: `n` is already declared with another type
> If True Then
...     n = n + 1
...     Dim bad As Int = "oops"
... End If
<repl:3>:3:9: error: java.lang.NumberFormatException: For input string: "oops"
> n
1
> bad
0
> :quit
```

The assignment to `n` happened before the runtime error. The declaration of
`bad` was accepted before execution, but its failing initializer never stored
a value, so its default remains 0. Previous output and initializers are never
replayed when you enter the next submission.

## Cancelling input and resetting

`:cancel` discards unfinished input without clearing variables. `:reset`
clears pending input, variables, definitions, random state and LastException.
Control lines are recognized outside string literals, including while a
block is pending; a line such as `:quit` inside a multiline string is text.

```repl
> Dim n As Int = 2
> If True Then
... :cancel
> n
2
> :reset
> Dim n As String = "fresh"
> n
fresh
> :quit
```

## Piping a saved session

Save this code as `session.bas` (without prompts):

```repl-script
Dim n As Int = 4
n * 2
```

Run it with:

```sh
ferro repl < session.bas
```

There are no prompts or input echo. Its output is:

```repl-output
8
```

Unlike interactive input, piped input stops at its first error. For example,
this session prints `before`, reports the numeric conversion error, and exits
with code 1; the final Log is never executed:

```repl-script
Log("before")
Dim bad As Int = "oops"
Log("never")
```

```repl-output
before
```

Compile errors exit with code 3; incomplete input at EOF is a compile error.
Invalid command arguments or input-read failures exit with code 2.
`ExitApplication2(n)` ends either kind of session with the ordinary program
exit status. Variables are not saved when the process exits.
