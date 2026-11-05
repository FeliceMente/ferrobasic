---
name: steward
description: How to drive a FerroBASIC pull request to a mergeable state - checks before a push, review threads, CI on release pull requests, the recording workflow, and the release sequence. Read it before acting on CI or review events on a pull request of this repository, and before preparing a release.
---

# Stewarding a FerroBASIC pull request

`AGENTS.md` holds the project's rules: the sources of truth, what a change
must come with, the branch flow. This skill does not repeat them; it says
how to drive a pull request through review and CI under them. Where the two
seem to disagree, `AGENTS.md` wins.

## Which checks stand in for CI

- A pull request into `dev` gets no CI (`docs/compatibility-testing.md`,
  "CI"). `scripts/check-all.sh` is its CI: run it before every push, and
  push only when every step passed. `--quick` (no static Linux build) is
  enough for a push that changes only comments or docs, with no Rust,
  workflow or build change; name the skipped step in the pull request. A
  red step is fixed before the push, never left for a reviewer.
- A change to what `ferro check` accepts or refuses, or a refactor of
  `crates/hir`, also runs `cargo run --release -p ferro-compare` against
  `origin/dev`. Read every group it reports, not only the counts:
  - **newly accepted**: each shape must be settled by a recording (name the
    test). An accepted shape that no recording settles is a bug: refuse it
    with a "not known yet" diagnostic and add a pending probe, or ask.
  - **fewer errors**: expected alongside newly accepted shapes, in programs
    with another error too; check they are the same shapes.
  - **newly refused** or **more errors**: a regression unless the pull
    request says why.
  - **other messages**: check that the new wording is still true of every
    program listed.
  - A refactor must show nothing in any group.
- `cargo run -p ferro-difftest` (and `-- --target standalone` after a
  `cargo build -p ferro-cli`) gives the counts the pull request quotes:
  pass, fail, deviation, pending.

## Review threads

- Reply on every thread before the push that addresses it: what the commit
  (not pushed yet, by its short hash) changes, or why it does not change
  anything. Then push, then resolve the threads you answered.
- A finding marked optional or a nit rides the next push that changes the
  same files; if no push comes, reply once saying it stays as it is and
  why, and resolve it.
- A finding outside the pull request's scope (pre-existing on `dev`) gets a
  pending probe or a note in the pull request, not a fix that widens it.
- After the push, update the pull request description: the counts, the
  `ferro-compare` result, and a "Review" section naming the fix commits.
- Comments posted by the agent end with the attribution footer the session
  sets.

## Recording runs

- The recorder runs on the owner's Windows machine. Give the steps: the
  `dev` ZIP (`https://github.com/FeliceMente/ferrobasic/archive/refs/heads/dev.zip`),
  `ferro-recorder.exe --repo C:\ferrobasic-dev --out C:\recordingsNN
  --toolchain-version <the B4J version AGENTS.md names>` (a `--dry-run`
  first gives the test count),
  then the zipped output folder.
- Unpack an uploaded recording into its own empty folder in the scratchpad,
  check its `report.md` (test count, failures, disagreements) and that it
  is the run asked for, then copy its `tests/` over the repository and
  delete what `remove.txt` lists. Compare `deviation-outputs/` with the
  previous run's.
- Each newly recorded test whose `.meta.toml` still has `ferro_rejects`
  needs a decision:
  - B4J built it (an `.expected` file): implement what it printed, and
    remove `ferro_rejects`.
  - B4J refused it (`rejected = true` in its `[recording]` table): keep
    refusing, with B4J's message where the recording shows it, and make it
    a `negative` test (`status = "negative"`, no `ferro_rejects`), as
    `for_each_untyped_read_before` is.

  Present the plan (what each recording says, what changes) and wait for
  the owner's go-ahead before writing code.
- Update the test's comment to give B4J's answer, close its entry in
  `docs/open-questions.md`, and update the spec, `docs/language-support.md`,
  `docs/research/level0-semantics.md` and the pending count in
  `docs/roadmap.md`.

## After a pull request is merged

Delete any pending check-in for it, then reset the working branch to the
new `dev` (`git fetch origin dev && git checkout -B <branch> origin/dev`,
then `git push --force-with-lease`), and wait for the owner before the next
step.

## Releases

The sequence is in `AGENTS.md`, "Branches and releases". In practice:

1. **Release preparation**, a pull request into `dev`: the version in
   `Cargo.toml` (and `Cargo.lock`, through `cargo build`), a new section at
   the top of `CHANGELOG.md` built from the pull requests merged since the
   last tag (`git log vX.Y.Z..origin/dev --merges`) and grouped like the
   earlier sections, and `docs/tutorial.md` re-run: every example's output
   compared with what `ferro` prints now, and the version line updated.
2. **The release**, a pull request from `dev` into `main`. CI runs on it, on
   Linux, Windows, macOS arm64 and macOS Intel. Every red job is this pull
   request's to fix (on `dev`, then merged again), never skipped or
   re-run until green: a Windows- or macOS-only failure is usually a path,
   a line ending or a platform `cfg`, which `check-all.sh`'s cross-target
   clippy does not exercise at run time. Merge only when every job is
   green.
3. **The tag**: `vX.Y.Z` on the merge commit on `main`, and the GitHub
   release, with notes in the style of the previous release.
4. **Back into `dev`**: a pull request from `main` into `dev` that also
   bumps the version to the next `-dev`.

Merging, tagging and publishing are the owner's to do or to ask for.
