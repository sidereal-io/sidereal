#!/usr/bin/env bash
# Mirror beads issues to GitHub, one-way (beads is the source of truth).
#
# CI-friendly (see docs/decisions/ADR-001): hydrates a local beads DB from the
# git-tracked .beads/issues.jsonl export — no Dolt remote needed — pushes to
# GitHub, then re-exports so the bd<->GitHub mapping (each bead's external_ref)
# is captured back in .beads/issues.jsonl. The caller (CI) commits that file to
# persist the mapping so reruns are idempotent.
#
# Usage: sync-beads-github.sh [--dry-run]
set -euo pipefail
export BD_NON_INTERACTIVE=1

DRY_RUN=0
[[ "${1:-}" == "--dry-run" ]] && DRY_RUN=1

echo "==> Hydrating beads DB from git-tracked .beads/issues.jsonl"
# bd import can exit non-zero on a benign 'nothing to commit' even when the
# import itself succeeds, so don't gate on its exit code...
bd import || true
# ...gate on the DB actually being queryable. This is the real failure mode the
# original 'bd dolt pull' hit in CI: "no beads database found".
if ! bd list >/dev/null 2>&1; then
  echo "ERROR: beads database not available after import" >&2
  exit 1
fi

echo "==> Syncing beads -> GitHub (push-only, prefer-local)"
if [[ "$DRY_RUN" -eq 1 ]]; then
  bd github sync --push-only --prefer-local --dry-run
  echo "==> Dry run: skipping export"
  exit 0
fi
bd github sync --push-only --prefer-local

echo "==> Capturing bd<->GitHub mapping into .beads/issues.jsonl"
bd export -o .beads/issues.jsonl
echo "==> Sync complete"
