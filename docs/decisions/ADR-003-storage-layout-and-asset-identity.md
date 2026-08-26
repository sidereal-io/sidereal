# 003: Asset Identity and Content Revisions

**Status:** Accepted · **Date:** 2026-07-29

**Context:** This ADR settles the identity and versioning model that Lineage and Operation Runs depend on; it must be decided before the core spine is built.

## Problem

Sidereal v2 becomes the system of record for files it renames, moves, and organises, so identity cannot be path-derived. A single mutable Asset record is also insufficient: if an operation rewrites bytes under a stable ID, the previous hash and state disappear and lineage cannot distinguish before from after.

The model must answer three coupled questions at once:

1. What is the stable identity that users, collections, and external mappings refer to?
2. How are immutable byte states identified and retained for lineage and audit?
3. Which changes create a new content revision versus mutating an existing one?

The on-disk tree and cross-filesystem move behaviour are a *separate* concern, decided in [ADR-011 — Storage tree layout & cross-filesystem moves](ADR-011-storage-tree-layout.md); this ADR fixes only the logical model those layout invariants attach to.

## Options

### Option A: Mutable Asset with a surrogate ID

An Asset has a stable ID and one current hash. Byte rewrites update the row.

This keeps paths and references stable but destroys byte history. An edge from an Asset to itself cannot say which pre- and post-operation bytes participated.

### Option B: Content hash as Asset identity

Every byte change creates a new Asset.

This gives strong integrity but makes logical identity unstable, mixes mechanical revisions with scientifically meaningful assets, and forces collections and external mappings to chase replacements.

### Option C: Stable Asset plus immutable AssetVersion

An Asset is the logical file identity. Each byte state is an immutable AssetVersion with a content hash. The Asset points to its current version; lineage and Operation Run inputs/outputs point to versions.

This adds one level to the model but preserves both stable user identity and exact byte provenance.

## Recommendation

**Option C.** A alone loses byte history, so lineage can't tell before from after; B keeps integrity but makes logical identity chase every mechanical rewrite. C separates the two axes that are actually distinct — a stable identity users and collections hold, and immutable byte states lineage points at — at the cost of one extra level in the model. The direct rules that follow from C:

- `Asset.id` is a stable opaque surrogate independent of path and content.
- `AssetVersion.id` is an opaque identifier with a mandatory content hash, byte size, format, and creation provenance. The hash is unique/indexed for dedup and integrity but is not the user-facing primary key.
- Lineage edges and Operation Run inputs/outputs point from produced AssetVersions to the exact consumed AssetVersions, making replay and audit unambiguous.
- A rename or move is a path/location event and does **not** create a version — bytes did not change.
- Any byte change — including embedded FITS metadata or an XMP rewrite — creates a new immutable AssetVersion. It may advance the current version of the same Asset or create a new Asset depending on Operator intent; the previous version stays addressable by history.
- New scientific products (thumbnails, masters, stacks, exports) are normally new Assets, not merely versions of their inputs.
- Core, not plugins, mints identities, computes hashes, advances current-version pointers, and writes lineage.

Retention is lineage-aware: a version referenced by lineage, an Operation Run, a hold, or migration audit cannot be garbage-collected. A later storage policy may prune unreferenced mechanical revisions after warning and backup checks.

## Decision

Accepted 2026-08-18 — **Option C, as recommended** (identity and versioning only). The current-version pointer, computed-latest, and concurrency mechanics that implement C are recorded as design facts in [the architecture overview](../architecture/README.md#design-facts). On-disk tree layout, cross-filesystem moves, and the integrity-reconciliation protocol are decided separately in [ADR-011](ADR-011-storage-tree-layout.md).
