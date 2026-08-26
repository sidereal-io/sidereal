# 010: Migration Strategy — Clean Break with a One-Way Importer

**Status:** Accepted · **Date:** 2026-08-19

**Context:** v2 is a new data model; existing v0.10.x users have live data and files. This ADR records
*how they reach v2*, and why that path knowingly overrides the analysis package's compatibility
requirements. It coordinates with the PostgreSQL-only engine decision ([ADR-004 — Database engine &
schema strategy](ADR-004-database-engine-and-schema.md)).

## Problem

v0.10.x users hold real data — Immich-mirrored images, plate solves, equipment, acquisitions, saved
locations — in a SQLite/Postgres database and a storage tree laid out as
`{STORAGE_PATH}/processed/{id % 1000}/{id}/…`. v2's model is fundamentally different: stable Assets with
immutable content-addressed Versions, Collections, facets, and lineage, on Postgres only.

The analysis package (`13-compatibility-requirements.md`) states a replacement **MUST** preserve image
IDs, that storage layout, every API path and shape, and reconciling-sync semantics. Those requirements
describe a system where Immich owns the tree and an `Image` is a finished photo — the two premises v2
exists to change. So the question is unavoidable: honour that compatibility contract, or break it — and
if we break it, how do users cross over safely?

## Options

### Option A: Backward-compatible / in-place migration

Preserve image IDs, the storage layout, and the API surface; upgrade the existing database in place.

**Pros:**
- No user-visible break; honours the analysis package's MUST requirements literally.

**Cons:**
- Re-imposes the old model's premises (Immich-owned tree, `Image` = finished photo) on the new one,
  defeating the reason for the rewrite.
- Requires keeping every legacy API path and shape alive on top of a different data model — a large,
  permanent compatibility surface for a single-maintainer product.

### Option B: Clean break with a one-way importer

v2 is a new model. An importer reads the old database and tree and produces v2 assets best-effort,
emitting a report of exactly what didn't map. The TypeScript app goes to maintenance, then retires at
cutover.

**Pros:**
- The v2 model is unconstrained by v0.10.x shapes.
- The compatibility surface collapses to one bounded, testable importer instead of a living dual API.
- Non-goals are explicit and the importer's report makes losses visible rather than silent.

**Cons:**
- Not seamless; lossy where the models genuinely differ.
- One-way — after cutover, rollback is restore-from-backup, not a live downgrade.

### Option C: Dual-run / strangler

Run v2 alongside v0.10.x and migrate incrementally, resource by resource.

**Pros:**
- No single big-bang cutover moment.

**Cons:**
- Two data models and two frontends live at once for a long time — a very large cost for a single-user,
  single-maintainer product.
- The models are too different to share one storage tree safely, which is where the strangler pattern
  normally gets its leverage.

## Recommendation

**Option B — clean break with a one-way importer**, gated on a non-negotiable cutover feature set. The
MUST-preserve-compatibility requirements are not a
neutral constraint; they encode the exact architecture v2 replaces, so honouring them (A) forecloses the
rewrite, and a strangler (C) pays dual-stack cost indefinitely for a product with one user and one
maintainer. A bounded importer that is honest about what it drops is the proportionate path.

- **The analysis package's compatibility MUSTs are explicitly overridden**, not overlooked. It remains
  valuable as a behavioural inventory and as the source of the cutover gate, but v2 is not bound by its
  ID/layout/API-shape preservation requirements.
- **Both SQLite and Postgres v0.10.x installs** reach v2 through the importer, not in-place dialect
  migration.
- **The importer is read-only against the source tree**, supports dry-run and resumable execution,
  records legacy-ID mappings, verifies checksums, and reconciles counts. Its hard invariant: every
  irreplaceable original is either imported and verified or named in the failure report — an unaccounted
  original blocks cutover.
- **Rollback** is disposable-root discard before cutover and restore-from-verified-backup after; the
  importer stays one-way by design.

The full checklist, compatibility-break list, filesystem-safety rules, and rollback detail are
maintained separately as cutover execution; this ADR owns the decision.

## Decision

Accepted 2026-08-19 — **clean break with a one-way importer**, as recommended.
