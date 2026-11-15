---
name: new-probe
description: How to write a pending conformance probe for a FerroBASIC question that B4X semantics leave unclear - when one is needed, the .bas and .meta.toml, when a .provisional output is allowed, the docs to update, and how to check and hand it over. Read it before adding a pending-recording test.
---

# Writing a pending probe

`AGENTS.md` holds the rules ("Sources of truth" and "Rules"): a test with
`status = "pending-recording"` and an entry in `docs/open-questions.md` when
B4X semantics are unclear, never an invented answer. This skill does not
repeat them; it says how to write such a probe so the next recording run
settles the question. Where the two seem to disagree, `AGENTS.md` wins.

## When a probe is needed

Only when nothing settles the question yet. Look first in:

- the recordings: `grep -rl` the construct in `tests/conformance/` and read
  the `.expected` files and the tests' comments;
- `docs/research/level0-semantics.md`, for a rule settled by `source`,
  `generated`, `docs` or `forum`;
- `docs/open-questions.md`: an open entry may already have a probe, or
  name the one to extend.

If a recording or the research settles it, implement that, with a test;
no probe.

## The `.bas`

Write `<name>.bas` next to the tests of its level
(`tests/conformance/level0/` for Level 0; `crates/corpus` reads every
folder under `tests/conformance`). Model it on recorded probes:
`grep -l "Recording run" tests/conformance/level0/*.bas` lists them, and
`git log --diff-filter=A -- 'tests/conformance/level0/*.meta.toml'` the
commits that added tests, probes among them (for example `5ab87f7`,
`c09adba`).

- The shape: `Sub Process_Globals` / `End Sub`, then
  `Sub AppStart (Args() As String)`.
- A comment, at the top of `AppStart` (or above what it asks about),
  states the question, what each possible answer would print, and what
  FerroBASIC does meanwhile, pointing to `docs/open-questions.md`.
- Plain `Log` output that tells the answers apart: a value that differs
  between them (in `object_number_to_char`, `6` if B4J converts with
  `BA.ObjectToChar`, `true` from the `Catch` if it raises
  `ClassCastException`), or a label per step, so that one line of the
  recording answers it.
- One question per probe, or a few closely related ones. Several unrelated
  questions in one program make one refusal or exception hide the rest.
- To see an exception's kind, wrap the statement in `Try`/`Catch` and log
  only what is stable across JDKs, such as
  `Log(LastException.Message.Contains("ClassCastException"))`, never the
  whole message.

## The `.meta.toml`

Copy the template from a pending test added in `git log` (for example
`git show c09adba -- '*.meta.toml'`); the fields and what `crates/corpus`
checks are in `crates/corpus/src/lib.rs` (`from_meta`):

```toml
# Conformance test metadata. See docs/compatibility-testing.md.
status = "pending-recording"   # pending-recording | recorded | deviation | negative
ferro_rejects = true   # rejected until a recording shows what B4X does
level = 0
features = ["for-each"]

# Filled in by tools/recorder when the expected output is recorded:
# [recording]
# product = "B4J"
# version = ""
# platform = ""
# date = ""
```

- `ferro_rejects = true` only if FerroBASIC refuses the program meanwhile;
  otherwise leave the line out. The refusal is a "not known yet"
  diagnostic (`git grep "not known yet" crates/hir` shows the wording),
  never one that claims an answer.
- Prefer `Try`/`Catch` to an uncaught error. A program that must end on
  one, or with `ExitApplication2`, gives FerroBASIC's ending in
  `uncaught_error` or `exit_code` (`docs/compatibility-testing.md`, "Test
  file layout"): `ferro-difftest` checks them on a pending test too.
- `features` reuses the names other tests use, so the report groups them.
- Never write a `.expected` file: only `tools/recorder` does.

## A `.provisional` output

Allowed only when `docs/research/level0-semantics.md` settles the
behaviour (basis `source`, `generated`, `docs` or `forum`): the expected
stdout in `<name>.provisional`, and a `[provisional]` table with its basis
and links, as `6d46fe2` added:

```toml
[provisional]
basis = "source"   # source | generated | docs | forum
refs = [
    "https://github.com/AnywhereSoftware/B4J/blob/<commit>/<file>#L1-L2",
]
```

Never for an open question, and never with `ferro_rejects`. The rules are
in `docs/compatibility-testing.md`, "Provisional outputs". Only Level 0 has
a research document today, so a probe of a later level has no provisional
output until its research settles the behaviour.

## The docs

- `docs/open-questions.md`: the entry (new, or extended) names the probe
  (`level0/<name>`), the possible answers and what FerroBASIC does
  meanwhile.
- `docs/language-support.md`: the row of the construct, if any, marks
  the case *(open)* and names the test, as its rows do.
- `docs/roadmap.md`: the pending count, if it states one.

## Checking it

- `cargo run -p ferro-recorder -- --dry-run` lists the probe.
- `cargo run -p ferro-difftest` counts it as pending (or as a pass against
  provisional output); it fails if FerroBASIC accepts a probe marked
  `ferro_rejects`, or refuses one that is not.
- With `ferro_rejects`, `cargo run --release -p ferro-compare` shows
  nothing newly accepted that no recording settles; read its groups as the
  [steward skill](../steward/SKILL.md#which-checks-stand-in-for-ci) says.
- `scripts/check-all.sh` before the push, as the steward skill says.

## Handing it over

The probe goes into the next recording run; the pull request names the
run it waits for. When B4J refuses a pending probe with an error the recorder
can read, the recorder writes a `[recording]` table with `rejected = true`
and B4J's error and line as comments. It keeps `status = "pending-recording"`;
the report says "refused by the original toolchain: decide whether it is
negative". Making it `negative` (or fixing the probe for the next run) is
the owner's decision. A build failure without a readable error still gets
no metadata, with the builder output in the report. The run's steps, and
what to do with its answer, are in
the [steward skill](../steward/SKILL.md#recording-runs), "Recording runs".
