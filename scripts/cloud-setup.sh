#!/usr/bin/env bash
# Setup script for Claude Code cloud sessions (and other fresh Linux VMs).
# Paste it as the environment's setup script, or run it manually.
# Must finish in a few minutes; keep heavy installs out of it.
set -euo pipefail

if ! command -v rustup >/dev/null 2>&1; then
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
  # shellcheck disable=SC1091
  . "$HOME/.cargo/env"
fi

# Installs the toolchain and components pinned in rust-toolchain.toml.
rustup show active-toolchain || rustup toolchain install

# Warm the dependency cache so the first build is faster.
cargo fetch || true
