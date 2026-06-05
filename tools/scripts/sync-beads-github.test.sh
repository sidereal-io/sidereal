#!/usr/bin/env bash
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TARGET="$SCRIPT_DIR/sync-beads-github.sh"

run_case() { # $1 = "" | "--dry-run" ; prints captured bd calls
  local tmp; tmp="$(mktemp -d)"
  cat >"$tmp/bd" <<'STUB'
#!/usr/bin/env bash
echo "$@" >> "$BD_CALLS"
STUB
  chmod +x "$tmp/bd"
  BD_CALLS="$tmp/calls.log" PATH="$tmp:$PATH" bash "$TARGET" $1 >/dev/null
  cat "$tmp/calls.log"
}

calls="$(run_case "")"
grep -qx "dolt pull" <<<"$calls"                              || { echo "FAIL: dolt pull"; exit 1; }
grep -qx "github sync --push-only --prefer-local" <<<"$calls" || { echo "FAIL: sync"; exit 1; }
grep -qx "dolt push" <<<"$calls"                              || { echo "FAIL: dolt push"; exit 1; }

calls="$(run_case "--dry-run")"
grep -q -- "github sync --push-only --prefer-local --dry-run" <<<"$calls" || { echo "FAIL: dry-run sync"; exit 1; }
grep -qx "dolt push" <<<"$calls"                              && { echo "FAIL: push ran in dry-run"; exit 1; }

echo "PASS"
