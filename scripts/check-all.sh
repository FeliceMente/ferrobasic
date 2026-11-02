#!/usr/bin/env bash
# Every check a pull request into `dev` needs before it is pushed
# (AGENTS.md, "Before closing a task"): CI runs only on pull requests into
# `main`. Stops at the first failure, naming the step.
#
#   scripts/check-all.sh
#
# The extra clippy targets need `rustup target add <target>`; the corpus
# step downloads the real-world corpus the first time (tests/corpus).
set -euo pipefail

cd "$(dirname "$0")/.."

# Clippy for the platforms CI tests but this machine is not.
extra_targets="x86_64-pc-windows-gnu aarch64-apple-darwin x86_64-apple-darwin"

step() {
  printf '\n==> %s\n' "$1"
}

fail() {
  printf '\ncheck-all: %s\n' "$1" >&2
  exit 1
}

installed=$(rustup target list --installed)
for target in $extra_targets; do
  if ! printf '%s\n' "$installed" | grep -qx "$target"; then
    fail "the clippy target $target is missing: rustup target add $target"
  fi
done

step "format"
cargo fmt --all --check || fail "format: run cargo fmt --all"

step "clippy"
cargo clippy --workspace --all-targets -- -D warnings || fail "clippy"
for target in $extra_targets; do
  step "clippy for $target"
  cargo clippy --workspace --all-targets --target "$target" -- -D warnings ||
    fail "clippy for $target"
done

step "tests"
cargo test --workspace || fail "tests"

step "conformance tests in the interpreter"
cargo build -p ferro-cli || fail "building ferro"
cargo run -q -p ferro-difftest || fail "conformance tests in the interpreter"

step "conformance tests as standalone binaries"
cargo run -q -p ferro-difftest -- --target standalone ||
  fail "conformance tests as standalone binaries"

step "recorder dry run"
cargo run -q -p ferro-recorder -- --dry-run >/dev/null || fail "recorder dry run"

step "real-world corpus"
cargo run -q -p ferro-corpus --bin ferro-corpus-fetch || fail "fetching the corpus"
cargo test -p ferro-syntax --test projects -- --ignored || fail "corpus: syntax"
cargo test -p ferro-project --test projects -- --ignored || fail "corpus: projects"

printf '\ncheck-all: every check passed\n'
