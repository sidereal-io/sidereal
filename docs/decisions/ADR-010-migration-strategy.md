---
id: adrs-adr010
date: 2026-08-19
status: accepted
title: 'ADR010: Migration Strategy — Clean Break with a One-Way Importer'
description: Architecture Decision Record (ADR) for how v0.10.x users reach v2 — a clean break with a one-way importer, knowingly overriding the old compatibility requirements.
---

# ADR-010: Migration Strategy — Clean Break with a One-Way Importer

## Context

v2 is a new data model, and existing v0.10.x users hold real data — Immich-mirrored images, plate solves, equipment, acquisitions, saved locations — in a database and a storage tree laid out as `{STORAGE_PATH}/processed/{id % 1000}/{id}/…`. The prior compatibility document states a replacement MUST preserve image IDs, that storage layout, every API path and shape, and reconciling-sync semantics — but those describe a system where Immich owns the tree and an `Image` is a finished photo, the two premises v2 exists to change. So the question is unavoidable: honour that contract, or break it, and if broken, how do users cross over safely. The cutover checklist, compatibility-break list, filesystem-safety rules, and rollback detail are maintained separately as cutover execution; this ADR owns the decision.

## Decision

Adopt a **clean break with a one-way importer**, gated on a non-negotiable cutover feature set. The MUST-preserve requirements encode the exact architecture v2 replaces, so honouring them forecloses the rewrite; a strangler pays dual-stack cost indefinitely for a single-user, single-maintainer product. The importer reads the old database and storage tree and produces v2 assets best-effort, read-only against the source, with dry-run and resumable execution, recorded legacy-id mappings, checksum verification, and count reconciliation; its hard invariant is that every irreplaceable original is either imported and verified or named in the failure report — an unaccounted original blocks cutover. The old TypeScript app goes to maintenance and retires at cutover. The prior compatibility MUSTs are explicitly overridden, not overlooked; the analysis package remains valuable as a behavioural inventory and the source of the cutover gate. Rollback is disposable-root discard before cutover and restore-from-verified-backup after; the importer stays one-way by design.

## Consequences

- The v2 model is unconstrained by v0.10.x shapes, and the compatibility surface collapses to one bounded, testable importer instead of a living dual API; losses are made visible by the importer's report rather than silent.
- Migration is not seamless and is lossy where the models genuinely differ; after cutover, recovery is restore-from-backup, not a live downgrade.
- The importer produces a *new* v2 store rather than adopting files in place, so cutover transiently needs room for both copies — for a multi-terabyte astro library this disk-space cost can itself be the binding constraint, and the import runbook must state the space requirement and any adopt-in-place or staged option.

## Alternatives Considered

### Alternative 1: Backward-compatible / in-place migration
- **Pros:** no user-visible break; honours the prior MUST requirements literally.
- **Cons:** re-imposes the old model's premises (Immich-owned tree, `Image` = finished photo) on the new one; requires keeping every legacy API path and shape alive atop a different data model — a large, permanent compatibility surface for a single maintainer.
- **Why not:** it preserves exactly the constraints the rewrite exists to shed.

### Alternative 2: Dual-run / strangler
- **Pros:** no single big-bang cutover moment; incremental resource-by-resource migration.
- **Cons:** two data models and two frontends live at once for a long time; the models are too different to safely share one storage tree, which is where the strangler normally gets its leverage.
- **Why not:** it pays indefinite dual-stack cost for a product with one user and one maintainer, without the shared-substrate that makes strangling worthwhile.
