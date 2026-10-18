# Open questions

Unresolved points. Agents: do not guess answers to these. Add a pending test
and a new entry here instead. Close an entry by linking the recording or the
source that settled it.

## Language

Answers from public sources (runtime source code, official docs, forum) are in
[research/level0-semantics.md](research/level0-semantics.md). An entry checked
"answered" there is still unconfirmed until a recording agrees with it; the
related tests stay `pending-recording`.

- [ ] Precedence of `&` relative to comparison and arithmetic operators.
      No source found; needs a recorded test.
- [ ] Relative precedence of `And` and `Or`, and of both versus comparisons.
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
- [ ] Whether `Log(number)` and number-to-String assignments also use `BA.NumberToString`.
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
- [ ] Output and exit code of a B4J non-UI program that stops on an uncaught error.
- [ ] `AppType` value of a non-UI `.b4j` project.
- [ ] Whether `LogDebug` prints in release mode (provisionally not).
- [ ] Exact text of JDK-generated exception messages seen through
      `LastException.Message` (index and number-parse errors), and which JDK.
- [ ] Default JVM locale on the recording machine (affects `ToLowerCase` /
      `ToUpperCase` for Turkish, Azeri and Lithuanian locales).
- [ ] Standalone binaries on macOS: whether ad-hoc re-signing works with
      appended payload bytes (see `docs/spec/cli.md`).

Provisional runtime behaviour in `crates/mir` and `crates/interp` (each
needs a recorded test):

- [ ] `For` evaluation order: FerroBASIC assigns the start value, then
      evaluates the limit, then the step. B4J's generated Java may evaluate
      limit and step first; it only matters when they read the counter or
      have side effects.
- [ ] Call depth before `java.lang.StackOverflowError` (FerroBASIC: 10,000
      frames). The error is not caught by `Try`, as Java `Error`s escape the
      generated `catch (Exception e)`.
- [ ] How a record prints (`Log(p)`), and `GetType` of a record.
- [ ] How a variable of type `Exception` and an array print.
- [ ] Exact texts of JDK-generated messages (`ClassCastException`,
      `IndexOutOfBoundsException`, …) seen through `LastException.Message`.
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

- [ ] Choose the licence: Apache-2.0/MIT dual or EUPL-1.2.
- [ ] Evaluate Cranelift for the REPL JIT and debug builds.
- [ ] Evaluate `zig cc` / `cargo-zigbuild` as the cross-linker.
