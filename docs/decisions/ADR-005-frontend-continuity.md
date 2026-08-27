---
id: adrs-adr005
date: 2026-08-26
status: accepted
title: 'ADR005: Frontend Continuity'
description: Architecture Decision Record (ADR) for how the v2 web frontend is built relative to the existing React client — a new application shell with ported presentational components.
---

# ADR-005: Frontend Continuity

## Context

The frontend stays TypeScript/React through the backend rewrite. That is what keeps existing frontend contributors productive across a backend language switch.

The open question is what the v2 UI *starts from*. The v2 API is a new data model (asset / collection / lineage), incompatible with the current Immich-mirror client. So "keep the codebase" really means keeping components while rewriting the entire data layer — either way.

The existing client (React 19 · Vite · Tailwind 4 · shadcn/ui · TanStack Query · Wouter) carries genuinely expensive, working pieces: a deep-zoom viewer and an Aladin sky-map integration. It also carries filtering and paging patterns fitted to a flat image list.

This is also the most visible decision to the frontend workstream, whose largest contributor works in TypeScript.

## Decision

Build a **new application shell, routing, and data layer, and port presentational components across as their screens are built.** The deep-zoom viewer, sky map, and admin forms move over; the old query/filter layer does not.

Two conditions apply as **gates on the frontend build, not on this decision:**

- The contributor conversation happens before that work starts.
- A green v0.10.x Playwright baseline is established before components are ported, so parity is measured against something real.

The first deliverable is a minimal read-only view as the first screen of the new shell — not throwaway scaffolding.

## Consequences

- Structural freedom where the new model actually reshapes the UI (routing, data layer, information architecture), without rebuilding the viewer or sky map.
- Work is incremental and demoable per screen; cutover parity is tracked screen by screen against the baseline.
- Two component sets coexist during the transition, and each ported component needs a judgment call. Some are entangled with old data shapes and will be effectively rewritten, not lifted — which is where this approach can quietly cost as much as a from-scratch rebuild.
- The v0.10.x client stays in maintenance in parallel. Whoever carries that *plus* the porting must not be silently double-loaded — a scheduling risk to watch, since the parallel track exists precisely to keep that person productive.
- The Playwright gate presumes a CI workflow that actually runs the suite. None does today, so standing one up is a prerequisite of the build, not an afterthought.

## Alternatives Considered

### Alternative 1: Evolve the existing client in place
- **Pros:** keeps every working piece and the contributors' familiar codebase from day one; parity is a diff against something real; the existing Playwright suite stays runnable.
- **Cons:** carries the flat-image-list architecture (client-side filter and paging) into an asset/collection/lineage model it may not fit; the query layer is rewritten regardless; risks contorting the new UI to old assumptions.
- **Why not:** the data layer is rewritten under any option, so evolving buys familiarity at the price of dragging the old model's shape into the new UI — surrendering the one freedom that matters most here.

### Alternative 2: Start fresh, rebuild everything
- **Pros:** a UI shaped purely by the new domain model (lineage graphs, calibration sets, run history), with no inherited assumptions.
- **Cons:** rebuilds the deep-zoom viewer, Aladin integration, and admin forms for no product gain; risks two frontends or a long undemoable gap; shrinks the parallelism benefit.
- **Why not:** there is no case for rebuilding the viewer or sky map from scratch. A clean shell captures the model-shaped freedom without paying to rebuild the expensive working pieces.
