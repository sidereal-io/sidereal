# 005: Frontend Continuity

**Status:** Accepted · **Date:** 2026-07-29

**Context:** The frontend workstream runs in parallel with the backend rewrite — the single most important scheduling decision in the plan. This ADR decides what that workstream starts *from*.

## Problem

The frontend stays TypeScript/React either way; that is settled and is what keeps existing frontend contributors productive through a backend language switch. The question is whether the frontend workstream evolves `apps/client` against the new API or starts fresh.

The v2 API is not compatible with v0.10.x — different data model, different resources. So "evolve" does not mean "keep working throughout"; it means "keep the codebase, rewrite the data layer." The existing client is React 19 + Vite + Tailwind 4 + shadcn/ui + TanStack Query + Wouter, with pages for gallery, targets, equipment, plate solving, sky map, locations, and admin.

This decision is also entangled with a **human** factor that is the project's top risk: the project's second-largest contributor works in TypeScript. Whichever option is chosen should be chosen *with* them, not for them.

## Options

### Option A: Evolve the existing client

**Pros:**
- Keeps the component library, styling system, deep-zoom viewer, Aladin sky-map integration, and form patterns — genuinely expensive, working pieces.
- Contributors keep working in a codebase they know from day one of the frontend build.
- Cutover parity is a diff against something real rather than a from-scratch build.
- The existing Playwright suite stays meaningfully runnable against it.

**Cons:**
- Carries the current architecture's shape, including client-side filtering and paging patterns fitted to a flat image list — the new model is asset/collection/lineage-shaped and may not fit them.
- The queryClient layer maps filters to query params for the old API; that whole layer is rewritten anyway.
- Risk of contorting the UI to the old model's assumptions.

### Option B: Start fresh

**Pros:**
- UI shaped by the new domain model — lineage graphs, calibration sets, operation-run history have no v0.10.x analogue and are the interesting new surface.
- No inherited assumptions from an Immich-mirror data model.

**Cons:**
- Rebuilds working, non-trivial UI (deep zoom, Aladin, admin forms) for no product gain.
- Two frontends during the transition, or a long gap with nothing demoable.
- The parallelism benefit shrinks — the frontend becomes a bigger effort with a later first demo.

### Option C: New shell, port components

Fresh application shell, routing, and data layer; migrate presentational components across as their screens are built.

**Pros:**
- Structural freedom where it matters (routing, data layer, information architecture) without rebuilding the deep-zoom viewer or sky map.
- Incremental and demoable per screen.

**Cons:**
- Requires judgment per component; some will be entangled with old data shapes and effectively rewritten anyway.
- Temporarily two component sets.

## Recommendation

**Leaning Option C.** The data layer is rewritten under any option, so the only real question is what happens to the presentational components — and there is no argument for rebuilding the deep-zoom viewer or the Aladin integration from scratch. C keeps those and buys freedom in the layer where the new model actually changes the UI.

Two conditions:

1. **Have the contributor conversation first.** This decision is the most visible one to the frontend workstream, and the mitigation for that top risk is to talk before the rewrite starts.
2. **Get a green E2E baseline on v0.10.x before deciding.** The analysis package notes the Playwright suite was never executed (`Q14`, `OQ-16`), and no CI workflow references it. Whether the existing suite is a usable parity harness materially affects A vs. C, and right now nobody knows.

## Decision

Accepted 2026-08-20 — **Option C, as recommended**, including its two conditions (contributor
conversation; green v0.10.x E2E baseline) as gates on the frontend build, not on this decision.

Consequence: the first minimal read-only view is the first screen of the new shell, not throwaway work.
