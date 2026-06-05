# 001: How CI hydrates beads data for the GitHub issue mirror

**Status:** Accepted
**Date:** 2026-06-04
**Context:** beads issue sidereal-ehr (mirror beads issues to GitHub); the first run of `.github/workflows/beads-github-sync.yml` failed at `bd dolt pull` with "no beads database found" (Task sidereal-ehr.1.3).

## Problem

The sync workflow must run in a fresh GitHub Actions checkout. The beads Dolt database (`.beads/embeddeddolt/`) is **gitignored**, so a checkout contains no database. `bd dolt pull` updates an *existing* DB — it cannot bootstrap one — and bd exposes **no `dolt clone`** to reconstruct the embedded DB from `refs/dolt/data`. So the original three-command design (`bd dolt pull` → `bd github sync --push-only` → `bd dolt push`) fails on the first command.

A correct design must answer two questions:
1. **Hydration** — where does CI get the beads issues to push?
2. **Idempotency** — where is the bd↔GitHub issue mapping persisted between runs so reruns don't create duplicate GitHub issues?

Evidence gathered:
- `bd import` (default source `.beads/issues.jsonl`, the git-tracked export) auto-creates the embedded DB and loads issues in a fresh checkout — verified offline.
- The mapping bd uses is each bead's `external_ref` (e.g. `gh-123`). `external_ref` **round-trips through `bd export`/`bd import`** — verified (`"external_ref":"gh-999"` appears in export).
- `bd import` currently exits non-zero on a clean hydrate (benign `dolt commit: nothing to commit`) even though the import succeeds.

## Options

### Option A: Dolt-over-git in CI (original intent)
Fetch `refs/dolt/data`, `bd init` to create the embedded DB + git-backed remote, `bd dolt pull`, sync, `bd dolt push`.

**Pros:**
- Dolt stays the single source of truth; mapping persists in the canonical store.
- No new write-back path.

**Cons:**
- No supported `bd dolt clone`; bootstrapping the embedded DB from the ref headlessly is undocumented and fragile.
- `actions/checkout` doesn't fetch `refs/dolt/data`; requires custom ref plumbing.
- `bd dolt push` write-back needs authenticated push to a custom ref (the original high-risk item, still unproven).

### Option B: JSONL import/export with commit-back (recommended)
`bd import` (hydrate from the tracked `.beads/issues.jsonl`) → `bd github sync --push-only --prefer-local` → `bd export -o .beads/issues.jsonl` → commit & push `issues.jsonl` if changed.

**Pros:**
- Uses data already in the checkout; offline, no Dolt remote, no Dolt-over-git auth.
- Uses documented, verified primitives (`bd import`/`bd export`).
- Mapping persists because `external_ref` round-trips through the JSONL.

**Cons:**
- Write-back is a bot commit of `.beads/issues.jsonl` (to `main` on scheduled runs) whenever the mapping changes.
- Must tolerate `bd import`'s spurious non-zero exit and gate success on `bd list` instead.
- **Depends on an unverified assumption:** that `bd github sync` writes the GitHub issue number to `external_ref` (not to a gitignored sync-state). Must be confirmed by a token-authenticated dry-run before trusting idempotency.

### Option C: Run the sync where the Dolt DB already exists
A self-hosted runner (or scheduled job on a machine with the hydrated workspace) runs `bd github sync` directly.

**Pros:**
- No hydration problem; mapping persists in the real Dolt DB.

**Cons:**
- Defeats the goal of a portable, hosted GitHub Action; adds infra to own and secure.

## Recommendation

**Option B.** It relies only on verified, documented bd primitives and the git-tracked JSONL that already ships in the checkout, and it eliminates the Dolt-over-git auth unknown. The key differentiator is that `external_ref` round-trips through the JSONL, so idempotency is achievable without Dolt as the CI store.

**Before implementing, confirm the one open assumption** with a token-authenticated `bd github sync --push-only --dry-run` (or a real run on a throwaway): verify it sets `external_ref=gh-<n>` on the beads and that the value appears in `bd export`. If instead the mapping lives only in a gitignored sync-state, Option B cannot guarantee idempotency and we fall back to Option A or C.

## Decision

**Option B accepted.** CI hydrates with `bd import` from the git-tracked
`.beads/issues.jsonl` and persists the mapping by re-exporting and committing that
file. No Dolt remote is used in CI.

**Caveat / outstanding verification:** idempotency depends on `bd github sync`
writing each created GitHub issue's number to the bead's `external_ref` (which
round-trips through the JSONL). This is not yet confirmed by a real (mutating)
sync. The workflow is to remain **disabled** (`gh workflow disable`) until a
single authorized real `bd github sync --push-only` confirms `external_ref` is
written; only then re-enable via `gh workflow enable`.
