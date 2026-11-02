#!/usr/bin/env bash
# Builds ferro fully static for Linux x86_64 (musl), bundles a program with
# it, and checks that the program is statically linked and prints its
# recorded output. CI's static-linux job and scripts/check-all.sh run it.
#
#   scripts/check-static-linux.sh
#
# It needs Linux x86_64, the x86_64-unknown-linux-musl target
# (`rustup target add x86_64-unknown-linux-musl`) and the `file` command.
set -euo pipefail

musl=x86_64-unknown-linux-musl
target_dir=${CARGO_TARGET_DIR:-target}

fail() {
  printf 'check-static-linux: %s\n' "$1" >&2
  exit 1
}

command -v file >/dev/null 2>&1 || fail "the file command is needed to check the binary is static"

cargo build --release -p ferro-cli --target "$musl" || fail "building ferro for $musl"
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
"$target_dir/$musl/release/ferro" bundle tests/conformance/level0/prices_map.bas \
  -o "$work/prices_map" || fail "bundling prices_map"
file "$work/prices_map"
file "$work/prices_map" | grep -Eq 'statically linked|static-pie linked' ||
  fail "the bundled program is not statically linked"
"$work/prices_map" | diff - tests/conformance/level0/prices_map.expected ||
  fail "the bundled program does not print its recorded output"
