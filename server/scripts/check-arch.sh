#!/usr/bin/env bash
#
# Dependency-direction lint (ADR-002).
#
# A pack codes against the public plugin contract (`sidereal-plugin-abi`)
# only. This script fails if `sidereal-core`, `sidereal-server` or another
# pack appears anywhere in a pack's runtime dependency tree. Every crate under
# `crates/packs/` is a pack.
#
#   check-arch.sh                lint this workspace, then show that the
#                                violation fixture is refused
#   check-arch.sh <workspace>    lint only that workspace
#
# Uses `cargo tree` (built into cargo) so there is no extra tool to install.
set -euo pipefail

here="$(cd "$(dirname "$0")/.." && pwd)"
FIXTURE="$here/tests/fixtures/arch-violation"

# The package name a manifest declares.
package_name() {
    sed -n 's/^name *= *"\([^"]*\)".*/\1/p' "$1" | head -n 1
}

# Lint the packs of the workspace in $1. Prints each violation and returns 1
# when there is one.
lint() {
    local workspace="$1" manifest pack tree forbidden failed=0
    local packs=()
    for manifest in "$workspace"/crates/packs/*/Cargo.toml; do
        [ -e "$manifest" ] || continue
        packs+=("$(package_name "$manifest")")
    done
    if [ "${#packs[@]}" -eq 0 ]; then
        echo "ARCH LINT FAILED: no packs found under $workspace/crates/packs." >&2
        return 1
    fi
    for pack in "${packs[@]}"; do
        # `--edges normal` restricts to runtime deps (ignores dev/build deps);
        # `--prefix none` prints one "name version" per line.
        tree="$(cd "$workspace" && cargo tree --package "$pack" --edges normal --prefix none)"
        for forbidden in sidereal-core sidereal-server "${packs[@]}"; do
            [ "$forbidden" = "$pack" ] && continue
            if grep -qE "^${forbidden}( |\$)" <<<"$tree"; then
                echo "ARCH LINT FAILED: '${pack}' must not depend on '${forbidden}'." >&2
                failed=1
            fi
        done
        if [ "$failed" -ne 0 ]; then
            echo "A pack codes against 'sidereal-plugin-abi' only (ADR-002)." >&2
            echo "Dependency tree of '${pack}':" >&2
            echo "$tree" | sed 's/^/  /' >&2
            return 1
        fi
    done
    echo "arch lint ok: ${packs[*]} depend on no core, server or other pack."
}

if [ "$#" -gt 0 ]; then
    lint "$1"
    exit
fi

lint "$here"

# The lint must refuse a pack that breaks the rule, or it proves nothing.
if refused="$(lint "$FIXTURE" 2>&1)"; then
    echo "ARCH LINT FAILED: the violation fixture was accepted:" >&2
    echo "$refused" | sed 's/^/  /' >&2
    exit 1
fi
echo "arch lint ok: the violation fixture was refused:"
echo "$refused" | grep 'ARCH LINT FAILED' | sed 's/^/  /'
