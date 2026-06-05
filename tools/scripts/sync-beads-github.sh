#!/usr/bin/env bash
# Mirror beads issues to GitHub, one-way (beads is the source of truth).
# Usage: sync-beads-github.sh [--dry-run]
set -euo pipefail

DRY_RUN=0
[[ "${1:-}" == "--dry-run" ]] && DRY_RUN=1

echo "==> Hydrating beads data from Dolt remote"
bd dolt pull

echo "==> Syncing beads -> GitHub (push-only, prefer-local)"
if [[ "$DRY_RUN" -eq 1 ]]; then
  bd github sync --push-only --prefer-local --dry-run
  echo "==> Dry run: skipping 'bd dolt push'"
  exit 0
fi
bd github sync --push-only --prefer-local

echo "==> Persisting bd<->GitHub mapping back to Dolt remote"
bd dolt push
echo "==> Sync complete"
