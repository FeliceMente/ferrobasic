#!/usr/bin/env bash
# Every check a pull request into `dev` needs before it is pushed
# (AGENTS.md, "Before closing a task"): CI runs only on pull requests into
# `main`. Stops at the first failure, naming the step.
#
#   scripts/check-all.sh [--offline]
#
# Clippy runs for every platform CI tests, this machine's own included; the
# others need `rustup target add <target>`. The corpus step downloads the
# real-world corpus the first time (tests/corpus): `--offline` skips the
# download, and the corpus tests too when the corpus is not there yet. The
# static Linux build of CI's static-linux job runs only on Linux x86_64 with
# the x86_64-unknown-linux-musl target. A skipped step is named at the end.
set -euo pipefail

offline=false
for arg in "$@"; do
  case "$arg" in
    --offline) offline=true ;;
    -h | --help)
      sed -n '2,13s/^# \{0,1\}//p' "$0"
      exit 0
      ;;
    *)
      printf 'check-all: unknown argument %s\n' "$arg" >&2
      exit 2
      ;;
  esac
done

# The checkout: the current one, or else the one holding this script.
root=$(git rev-parse --show-toplevel 2>/dev/null) || root=""
if [ -z "$root" ] || [ ! -f "$root/scripts/check-all.sh" ]; then
  root=$(cd "$(dirname "$0")/.." && pwd)
fi
cd "$root"

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
if [ "$offline" = true ] && [ ! -d target/corpus ]; then
  skip "real-world corpus (--offline, and not downloaded yet)"
else
  cargo test -p ferro-syntax --test projects -- --ignored || fail "corpus: syntax"
  cargo test -p ferro-project --test projects -- --ignored || fail "corpus: projects"
fi

step "static Linux binary"
musl=x86_64-unknown-linux-musl
if [ "$host" != x86_64-unknown-linux-gnu ]; then
  skip "static Linux binary (CI only: it needs Linux x86_64)"
elif ! grep -qx "$musl" <<<"$installed"; then
  skip "static Linux binary (rustup target add $musl to run it)"
else
  # As CI's static-linux job does.
  cargo build -q --release -p ferro-cli --target "$musl" || fail "static Linux binary: build"
  work=$(mktemp -d)
  trap 'rm -rf "$work"' EXIT
  "target/$musl/release/ferro" bundle tests/conformance/level0/prices_map.bas \
    -o "$work/prices_map" || fail "static Linux binary: bundle"
  file "$work/prices_map" | grep -Eq 'statically linked|static-pie linked' ||
    fail "static Linux binary: the bundled program is not static"
  "$work/prices_map" | diff - tests/conformance/level0/prices_map.expected ||
    fail "static Linux binary: the bundled program's output"
fi

if [ -n "$skipped" ]; then
  printf '\ncheck-all: every check passed, but these were skipped:%s\n' "$skipped"
else
  printf '\ncheck-all: every check passed\n'
fi
