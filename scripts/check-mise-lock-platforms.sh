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

# On GitHub Actions, each failure is also an annotation, so it shows on the pull request.
annotate=0
[ "${GITHUB_ACTIONS:-}" != true ] || annotate=1
fail() {
    echo "check-mise-lock-platforms: FAILED: $*" >&2
    [ "$annotate" -eq 0 ] || echo "::error::check-mise-lock-platforms: $*" >&2
    exit 1
}
[ -f "$lock" ] || fail "$lock is not a file"

# Exit 2 means no tool entry: a layout this script cannot read must not pass.
status=0
awk -v required="$required_platforms" -v exceptions="$exceptions" -v annotate="$annotate" '
    function unquote(name) {
        if (name ~ /^".*"$/) name = substr(name, 2, length(name) - 2)
        return name
    }
    /^\[\[tools\..+\]\]$/ {
        name = $0
        sub(/^\[\[tools\./, "", name)
        sub(/\]\]$/, "", name)
        tool[++tools] = unquote(name)
        next
    }
    /^\[tools\..+\."platforms\.[^"]+"\]$/ {
        name = $0
        sub(/^\[tools\./, "", name)
        sub(/"\]$/, "", name)
        match(name, /\."platforms\.[^"]+$/)
        platform = substr(name, RSTART + length(".\"platforms."))
        has[unquote(substr(name, 1, RSTART - 1)), platform] = 1
    }
    END {
        if (!tools) exit 2
        split(exceptions, excepted)
        for (i in excepted) skip[excepted[i]] = 1
        n = split(required, platforms)
        for (t = 1; t <= tools; t++) {
            if (tool[t] in skip) continue
            missing = ""
            for (p = 1; p <= n; p++)
                if (!((tool[t], platforms[p]) in has)) missing = missing " " platforms[p]
            if (missing != "") {
                print "check-mise-lock-platforms: FAILED: " tool[t] " lacks" missing
                if (annotate) print "::error::check-mise-lock-platforms: " tool[t] " lacks" missing
                failed = 1
            }
        }
        exit failed
    }
' "$lock" >&2 || status=$?
[ "$status" -ne 2 ] || fail "$lock holds no [[tools.*]] entry"
exit "$status"
