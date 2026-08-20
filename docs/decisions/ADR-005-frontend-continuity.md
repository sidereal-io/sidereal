# 005: Frontend Continuity

**Status:** Accepted
**Date:** 2026-07-29
**Context:** M0 of [RFC #213](https://github.com/sidereal-io/sidereal/issues/213). M5 (frontend parity) starts at M1 and runs in parallel with the backend rewrite — the RFC calls this "the single most important scheduling decision in the plan." This ADR decides what M5 starts *from*.

## Problem

The frontend stays TypeScript/React either way; that is settled and is what keeps existing frontend contributors productive through a backend language switch. The question is whether M5 evolves `apps/client` against the new API or starts fresh.

The v2 API is not compatible with v0.10.x — different data model, different resources. So "evolve" does not mean "keep working throughout"; it means "keep the codebase, rewrite the data layer." The existing client is React 19 + Vite + Tailwind 4 + shadcn/ui + TanStack Query + Wouter, with pages for gallery, targets, equipment, plate solving, sky map, locations, and admin.

This decision is also entangled with a **human** factor the RFC names as its top risk: the project's second-largest contributor works in TypeScript. Whichever option is chosen should be chosen *with* them, not for them.

## Options

### Option A: Evolve the existing client

**Pros:**
- Keeps the component library, styling system, deep-zoom viewer, Aladin sky-map integration, and form patterns — genuinely expensive, working pieces.
- Contributors keep working in a codebase they know from day one of M1.
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
- The RFC's parallelism benefit shrinks — M5 becomes a bigger effort with a later first demo.

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

1. **Have the contributor conversation first.** This decision is the most visible one to the frontend workstream, and the RFC's own mitigation for its top risk is to talk before M0 starts.
2. **Get a green E2E baseline on v0.10.x before deciding.** The analysis package notes the Playwright suite was never executed (`Q14`, `OQ-16`), and no CI workflow references it. Whether the existing suite is a usable parity harness materially affects A vs. C, and right now nobody knows.

## Decision

Accepted 2026-08-20 (M0 of RFC #213) — **Option C: new shell, port components.** The data layer is
rewritten under every option, so the real question is only what happens to the presentational
components. C keeps the expensive, working pieces (deep-zoom viewer, Aladin sky map, admin forms) and
takes structural freedom in the routing, data layer, and information architecture — the layer where the
asset/collection/lineage model actually reshapes the UI. A and B are both dominated: A contorts the new
model into an Immich-mirror shell; B rebuilds working UI for no product gain.

**Two conditions bind the M5 *build*, not this direction-setting call:**

1. **The contributor conversation happens before M5 frontend work starts in earnest** — this is the
   most visible decision to the frontend workstream and the RFC's named mitigation for its top risk.
2. **A green v0.10.x Playwright baseline is established first** (`Q14`, `OQ-16`), so per-component
   "port vs. rewrite" judgments rest on a runnable parity harness.

**Consequence for M1:** the minimal read-only view M1 needs for its exit criterion is built as the
**first screen of this new Option-C shell** (a fresh Vite/React/Tailwind app scaffold, read-only asset
list + detail), not throwaway work — M5 grows the same shell. The two conditions above still gate the
broader M5 parity build, not this minimal first screen.
