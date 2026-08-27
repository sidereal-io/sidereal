---
id: adrs-adr003
date: 2026-08-18
status: accepted
title: 'ADR003: Asset Identity and Content Revisions'
description: Architecture Decision Record (ADR) for the identity model of the v2 store — a stable Asset plus immutable, content-hashed AssetVersions.
---

# ADR-003: Asset Identity and Content Revisions

## Context

Sidereal v2 becomes the system of record for files it renames, moves, and organises, so identity cannot be path-derived. A single mutable Asset record is also insufficient: if an operation rewrites bytes under a stable id, the previous hash and state vanish and lineage cannot tell before from after. The model must separately answer what stable identity users/collections/external mappings refer to, how immutable byte states are identified and retained, which changes create a new revision, and it must be settled before the core spine is built. On-disk layout and move semantics are a distinct decision, taken in [ADR-011 — Storage tree layout & cross-filesystem moves](ADR-011-storage-tree-layout.md).

## Decision

Adopt a **stable `Asset` plus immutable `AssetVersion`**. `Asset.id` is a stable opaque surrogate independent of path and content; `AssetVersion.id` is an opaque id carrying a mandatory content hash (indexed for dedup and integrity, not the user-facing key), byte size, format, and provenance. Lineage edges and Operation Run inputs/outputs reference exact **versions**. A rename/move is a path event and creates no version; any byte change creates a new immutable version; new scientific products (thumbnails, masters, stacks, exports) are normally new Assets. Core — not plugins — mints identities, hashes bytes, advances current-version pointers, and writes lineage; retention is lineage-aware (anything referenced by lineage, a run, a hold, or migration audit cannot be GC'd). The "current" selection is a separate `Asset.current_version_id` pointer, not a moving `v0` sentinel; `isLatest` is computed. Navigation aids (`version_seq` ordinal, optional `label`/`aliases`/`note`) are metadata, not identity. Advancing the current pointer uses optimistic concurrency — compare-and-swap guarded by an `Asset.revision` counter. Reconciliation never silently updates on out-of-band byte change: it records an integrity mismatch needing explicit adopt/restore/ignore, and a missing path marks the Asset unavailable without deleting identity, versions, or lineage.

## Consequences

- Stable links, collections, and source mappings survive moves and byte revisions; lineage records exact content, not a mutable placeholder.
- Byte-editing operations consume additional storage until retention safely reclaims old revisions; a current-state query joins Asset to its current version.
- Whether an Operator's output is a *new version of an input* or a *new Asset* rests on "Operator intent," which every Operator must declare correctly — a mis-declaration corrupts lineage semantics (e.g. a stack recorded as a version of one of its 187 inputs). This classification rule is load-bearing and must be pinned down when the Operator contract is specified.

## Alternatives Considered

### Alternative 1: Mutable Asset with a surrogate id (one current hash, byte rewrites update the row)
- **Pros:** paths and references stay stable; simplest model.
- **Cons:** destroys byte history — a self-edge cannot say which pre/post-operation bytes participated.
- **Why not:** lineage is the whole point of the rewrite, and this cannot express it.

### Alternative 2: Content hash as Asset identity (every byte change is a new Asset)
- **Pros:** strong integrity by construction.
- **Cons:** logical identity becomes unstable; mechanical revisions mix with scientifically meaningful assets; collections and external mappings must chase replacements.
- **Why not:** it sacrifices the stable user-facing identity that links and collections depend on.
