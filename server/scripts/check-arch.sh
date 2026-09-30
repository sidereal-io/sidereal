#!/usr/bin/env bash
#
# Dependency-direction lint (ADR-002).
#
# The first-party astro pack must code against the public plugin contract
# (`sidereal-plugin-abi`) only — never against `sidereal-core` internals. This
# script fails if `sidereal-core` appears anywhere in the astro pack's runtime
# dependency tree, catching a forbidden `packs/astro -> core` edge the moment it
# is introduced.
#
# Uses `cargo tree` (built into cargo) so there is no extra tool to install.
set -euo pipefail

# Run from the workspace root regardless of caller CWD.
cd "$(dirname "$0")/.."

PACKAGE="sidereal-pack-astro"
FORBIDDEN="sidereal-core"

# `--edges normal` restricts to runtime deps (ignores dev/build deps);
# `--prefix none` prints one "name version" per line.
tree="$(cargo tree --package "$PACKAGE" --edges normal --prefix none)"

if grep -qE "^${FORBIDDEN}( |\$)" <<<"$tree"; then
    echo "ARCH LINT FAILED: '${PACKAGE}' must not depend on '${FORBIDDEN}'." >&2
    echo "The astro pack codes against 'sidereal-plugin-abi' only (ADR-002)." >&2
    echo >&2
    echo "Offending dependency tree:" >&2
    echo "$tree" | sed 's/^/  /' >&2
    exit 1
fi

echo "arch lint ok: '${PACKAGE}' does not depend on '${FORBIDDEN}'."
