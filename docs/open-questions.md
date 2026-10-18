# Open questions

Unresolved points. Agents: do not guess answers to these. Add a pending test
and a new entry here instead. Close an entry by linking the recording or the
source that settled it.

## Language

- [ ] Precedence of `&` relative to comparison and arithmetic operators.
- [ ] Whether `Select Case` is accepted besides `Select`, and whether `Case` accepts ranges.
- [ ] Whether `Do ... Loop While` and an unconditional `Do ... Loop` exist.
- [ ] Formatting inside smart strings (`$1.2{x}`) and escape sequences.
- [ ] `:` as a separator for several statements on one line.
- [ ] Result type of `/` between two integers.
- [ ] Whether a variable declared with `Dim` inside a block is visible in the whole Sub.
- [ ] `Map` iteration order (insertion order?).
- [ ] Exact number-to-string formatting rules (Double, Float, large/small values).

## Porting

- [ ] Collect a corpus of real projects per product and document the project-file format from it.
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
