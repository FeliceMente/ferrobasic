#!/usr/bin/env bash
# Every check a pull request into `dev` needs before it is pushed
# (AGENTS.md, "Before closing a task"): CI runs only on pull requests into
# `main`. Stops at the first failure, naming the step.
#
#   scripts/check-all.sh [--offline] [--quick]
#
# Clippy runs for every platform CI tests, this machine's own included; the
# others need `rustup target add <target>`. The corpus step downloads the
# real-world corpus the first time (tests/corpus): `--offline` skips the
# download, and the corpus tests too when the corpus is not there yet. The
# static Linux binary (scripts/check-static-linux.sh, as CI's static-linux
# job) runs only on Linux x86_64 with the x86_64-unknown-linux-musl target
# and the `file` command; `--quick` skips it, as its release build is slow.
# A skipped step is named at the end.
set -euo pipefail

offline=false
quick=false
for arg in "$@"; do
  case "$arg" in
    --offline) offline=true ;;
    --quick) quick=true ;;
    -h | --help)
      # The header: the comment lines after the first one.
      sed -n '2,/^[^#]/{/^#/s/^# \{0,1\}//p;}' "$0"
      exit 0
      ;;
    *)
      printf 'check-all: unknown argument %s\n' "$arg" >&2
      exit 2
      ;;
  esac
done

# The checkout holding this script, through any symlinks to it.
script=$0
while [ -L "$script" ]; do
  link=$(readlink "$script")
  case "$link" in
    /*) script=$link ;;
    *) script=$(dirname "$script")/$link ;;
  esac
done
cd "$(dirname "$script")/.."
target_dir=${CARGO_TARGET_DIR:-target}

# The platforms CI tests (.github/workflows/ci.yml).
ci_targets="x86_64-unknown-linux-gnu x86_64-pc-windows-gnu aarch64-apple-darwin x86_64-apple-darwin"
skipped=""

step() {
  printf '\n==> %s\n' "$1"
}

fail() {
  printf '\ncheck-all: %s\n' "$1" >&2
  exit 1
}

skip() {
  printf 'skipped: %s\n' "$1"
  skipped="$skipped
  - $1"
}

if ! command -v rustup >/dev/null 2>&1; then
  fail "rustup is needed for the clippy targets of the other platforms (https://rustup.rs)"
fi
host=$(rustc -vV | sed -n 's/^host: //p')
installed=$(rustup target list --installed)
extra_targets=""
for target in $ci_targets; do
  [ "$target" = "$host" ] && continue
  if ! grep -qx "$target" <<<"$installed"; then
    fail "the clippy target $target is missing: rustup target add $target"
  fi
  extra_targets="$extra_targets $target"
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
if [ "$offline" = false ]; then
  cargo run -q -p ferro-corpus --bin ferro-corpus-fetch || fail "fetching the corpus"
fi
if [ "$offline" = true ] && [ ! -d "$target_dir/corpus" ]; then
  skip "real-world corpus (--offline, and not downloaded yet)"
else
  cargo test -p ferro-syntax --test projects -- --ignored || fail "corpus: syntax"
  cargo test -p ferro-project --test projects -- --ignored || fail "corpus: projects"
fi

step "static Linux binary"
musl=x86_64-unknown-linux-musl
if [ "$quick" = true ]; then
  skip "static Linux binary (--quick)"
elif [ "$host" != x86_64-unknown-linux-gnu ]; then
  skip "static Linux binary (CI only: it needs Linux x86_64)"
elif ! grep -qx "$musl" <<<"$installed"; then
  skip "static Linux binary (rustup target add $musl to run it)"
elif ! command -v file >/dev/null 2>&1; then
  skip "static Linux binary (it needs the file command)"
else
  scripts/check-static-linux.sh || fail "static Linux binary"
fi

if [ -n "$skipped" ]; then
  printf '\ncheck-all: every check passed, but these were skipped:%s\n' "$skipped"
else
  printf '\ncheck-all: every check passed\n'
fi
