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
- [x] `AppType` value of a non-UI `.b4j` project: `StandardJava`
      ([public samples](spec/project-files.md#sources)).
- [ ] Whether `LogDebug` prints in release mode (provisionally not).
- [ ] Exact text of JDK-generated exception messages seen through
      `LastException.Message` (index and number-parse errors), and which JDK.
- [ ] Default JVM locale on the recording machine (affects `ToLowerCase` /
      `ToUpperCase` for Turkish, Azeri and Lithuanian locales).
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
- [ ] How deeply nested Lists, Maps and records can be compared or printed
      before Java's recursive `equals` / `toString` overflow (FerroBASIC: 1000
      levels), and whether a record whose List holds the record overflows when
      printed (FerroBASIC: it does, as the List prints through its wrapper).
      Fixtures: `level0/deep_printing`, `deep_equals`, `self_printing_record`.
- [ ] When B4J raises `java.lang.OutOfMemoryError` depends on the JVM's heap
      size, which a recording must state. FerroBASIC always says `Java heap
      space`; the JDK words an over-long String differently.
- [ ] How a record prints (`Log(p)`), and `GetType` of a record.
- [ ] How a variable of type `Exception` and an array print.
- [ ] `Sin`, `Cos`, `Tan`, `Power` and `Logarithm` call `Math` methods that
      HotSpot replaces with CPU intrinsics, whose last bit may depend on the
      JVM and CPU; FerroBASIC uses the platform's maths library. Needs recordings from the reference machine. (`ASin`, `ACos`,
      `ATan`, `ATan2` are settled: fdlibm, bit for bit.)
- [ ] The type of `IIf`'s result, and so how it converts when assigned or
      printed (`Dim n As Int = IIf(True, 1, 2.5)`, `Log(IIf(True, 3, 4.5))`).
      That it evaluates only the chosen branch is settled. Fixture:
      `level0/iif_branches`.
- [ ] Whether all `Case` values are evaluated before matching, as they would
      be if the compiler passes them to `BA.switchObjectToInt` as arguments;
      FerroBASIC evaluates them in order until one matches. It only matters
      for cases with side effects.
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
- [ ] A `For` with an undeclared counter, or `For Each v As T`, inside
      `Process_Globals`: does B4X declare a module variable, a local, or
      reject it? FerroBASIC rejects it until a recording shows
      (`docs/spec/level0.md`, "Program structure"). Fixture:
      `level0/process_globals_for`.
- [ ] Whether a comment may follow a line continuation (` _ ' comment`);
      FerroBASIC rejects it. Fixture: `level0/continuation_comment`.
- [ ] How deep B4J (javac) lets expressions and blocks nest before failing;
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
- [ ] Ship the licence notices of FerroBASIC's dependencies with released
      binaries (e.g. generated with `cargo-about`).
- [ ] Evaluate Cranelift for the REPL JIT and debug builds.
- [ ] Evaluate `zig cc` / `cargo-zigbuild` as the cross-linker.
