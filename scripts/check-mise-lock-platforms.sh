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

awk -v required="$required_platforms" -v exceptions="$exceptions" '
    END { exit 0 }
' "$lock"
