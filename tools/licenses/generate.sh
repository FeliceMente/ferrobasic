#!/bin/sh
# Regenerates THIRD-PARTY-NOTICES.txt from Cargo.lock; see README.md here.
set -eu
root=$(cd "$(dirname "$0")/../.." && pwd)
cd "$root"
# --offline reads licence files from the crates themselves only, so the
# result depends on Cargo.lock alone; fetch them first.
cargo fetch --locked
log=$(mktemp)
trap 'rm -f "$log"' EXIT
cargo about generate --locked --offline --fail \
    -c tools/licenses/about.toml \
    -m crates/cli/Cargo.toml \
    -o THIRD-PARTY-NOTICES.txt \
    tools/licenses/about.hbs 2>"$log" || { cat "$log" >&2; exit 1; }
# cargo-about only warns when a clarification no longer matches its file,
# then falls back to a generic licence text without the copyright lines.
if grep -q WARN "$log"; then
    cat "$log" >&2
    echo "generate.sh: fix the warnings above (README.md, \"Regenerating\")" >&2
    exit 1
fi
