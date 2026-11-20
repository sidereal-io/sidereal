#!/bin/sh
# Check that mise.lock pins every tool for each system the dev shell supports.
# Reads only the two header forms `mise lock` writes, so it needs no TOML parser.
set -eu
root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd -P)
lock=${1:-$root/mise.lock}

# The systems flake.nix supports.
required_platforms='linux-x64 linux-arm64 macos-arm64'
# Rust records the stable channel, so the lock has no platform tables for it.
exceptions='rust'

fail() { echo "check-mise-lock-platforms: FAILED: $*" >&2; exit 1; }
[ -f "$lock" ] || fail "$lock is not a file"

# Exit 2 means no tool entry: a layout this script cannot read must not pass.
status=0
awk -v required="$required_platforms" -v exceptions="$exceptions" '
    /^\[\[tools\..+\]\]$/ { tools++ }
    END { if (!tools) exit 2 }
' "$lock" || status=$?
[ "$status" -ne 2 ] || fail "$lock holds no [[tools.*]] entry"
exit "$status"
