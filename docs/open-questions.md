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
- [ ] Whether the compiler routes every numeric `&` / `Log` conversion through
      `BA.NumberToString`.
- [ ] Which boxed type a numeric literal gets when stored as `Object`
      (matters for `Map` keys: `Int` 1 and `Double` 1.0 are different keys).
- [ ] `For`: whether the limit and step are evaluated once, and the value of
      the loop variable after the loop.
- [ ] Undeclared variables other than `For` counters: implicit `String` (old
      forum posts) or a compile error.

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
