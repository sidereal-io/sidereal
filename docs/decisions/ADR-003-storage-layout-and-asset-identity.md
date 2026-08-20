# 003: Storage Layout, Asset Identity, and Content Revisions

**Status:** Accepted
**Date:** 2026-07-29
**Context:** M0 of [RFC #213](https://github.com/sidereal-io/sidereal/issues/213). The outcome constrains the [Lineage](../architecture/README.md#core-concepts) model and must be decided before M1 builds the core spine.

## Problem

Sidereal v2 becomes the system of record for files it renames, moves, and organises. Identity cannot
be path-derived. A single mutable Asset record is also insufficient: if an operation rewrites bytes
under a stable ID, the previous hash and state disappear and lineage cannot distinguish before from
after.

The model must separately answer:

1. What is the stable identity users, collections, and external mappings refer to?
2. How are immutable byte states identified and retained for lineage and audit?
3. Which changes create a new content revision?
4. What is the user-visible and internal on-disk layout?

## Options considered

### Option A: Mutable Asset with a surrogate ID

An Asset has a stable ID and one current hash. Byte rewrites update the row.

This keeps paths and references stable but destroys byte history. An edge from an Asset to itself
cannot say which pre- and post-operation bytes participated.

### Option B: Content hash as Asset identity

Every byte change creates a new Asset.

This gives strong integrity but makes logical identity unstable, mixes mechanical revisions with
scientifically meaningful assets, and forces collections and external mappings to chase replacements.

### Option C: Stable Asset plus immutable AssetVersion

An Asset is the logical file identity. Each byte state is an immutable AssetVersion with a content
hash. The Asset points to its current version; lineage and Operation Run inputs/outputs point to
versions.

This adds one level to the model but preserves both stable user identity and exact byte provenance.

## Recommendation

Choose **Option C**.

### Identity and revision rules

- `Asset.id` is a stable opaque surrogate independent of path and content.
- `AssetVersion.id` is an opaque identifier with a mandatory content hash, byte size, format, and
  creation provenance. The hash has a unique/indexed role for dedup and integrity but is not the
  user-facing primary key.
- Lineage edges point from produced AssetVersions to the exact consumed AssetVersions.
- Operation Run inputs and outputs also reference versions, making replay and audit unambiguous.
- A rename or move changes an Asset path/location event and does not create a version because bytes
  did not change.
- Any byte change — including embedded FITS metadata or an XMP rewrite — creates a new immutable
  AssetVersion. It may advance the current version of the same Asset or create a new Asset depending
  on Operator intent, but the previous version remains addressable by history.
- New scientific products such as thumbnails, masters, stacks, and exports are normally new Assets,
  not merely versions of their inputs.
- Core, not plugins, mints identities, computes hashes, advances current-version pointers, and writes
  lineage.

The retention policy for superseded versions is explicit and lineage-aware. A version referenced by
lineage, an Operation Run, a hold, or migration audit cannot be garbage-collected. A later storage
policy may prune unreferenced mechanical revisions after warning and backup checks.

### Reconciliation

Core hashes every asset version at ingest. If bytes change outside Sidereal, reconciliation does not
silently update the known version. It records an integrity mismatch and requires an explicit adopt,
restore, or ignore decision. Adopting creates a new version with out-of-band provenance.

Missing paths mark an Asset unavailable; they do not delete identity, versions, or lineage. A found
file can be re-associated by hash and additional safety checks.

### Storage layout

Identity does not prescribe layout. The exact on-disk tree and cross-filesystem move behavior are a
separate decision — split out to [ADR-011](ADR-011-storage-tree-layout.md), which carries the binding
invariants (recoverable DB+FS updates, moves never rewrite content, historical versions never surface as
duplicate current assets, path-traversal/symlink escape rejected, every file reconcilable to an
AssetVersion + hash) and must be Accepted before M1 storage design.

### Version navigation and concurrency (input from #217 / DataHub)

A [#217](https://github.com/sidereal-io/sidereal/issues/217) / DataHub comparison confirmed the Option C
shape, with these deliberate divergences from DataHub's moving `v0` sentinel:

- The "current" selection is a separate `Asset.current_version_id` pointer, **not** a moving `v0`
  sentinel; `isLatest` is **computed**, never a stored per-version boolean.
- Navigation aids that are **metadata, not identity**: a dense per-Asset `version_seq` ordinal plus
  optional curated `label` / `aliases` / `note`.
- Advancing the current pointer uses **optimistic concurrency** — compare-and-swap guarded by an
  `Asset.revision` counter.

## Decision

Accepted 2026-08-18 (M0 of RFC #213) — **Option C**, as recommended (identity and versioning only).
On-disk tree layout and cross-filesystem moves are decided separately in
[ADR-011](ADR-011-storage-tree-layout.md) (Proposed), which gates M1.
