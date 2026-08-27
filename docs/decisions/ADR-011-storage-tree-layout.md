---
id: adrs-adr011
date: 2026-08-26
status: accepted
title: 'ADR011: Storage Tree Layout and Cross-Filesystem Moves'
description: Architecture Decision Record (ADR) for the v2 on-disk asset store — a protected internal object store content-addressed by BLAKE3, with a browsable projection deferred.
---

# ADR-011: Storage Tree Layout and Cross-Filesystem Moves

## Context

[ADR-003 — Asset identity & content revisions](ADR-003-asset-identity-and-content-revisions.md) settled that identity is independent of layout: an `Asset` is a stable surrogate and each `AssetVersion` is an immutable, content-hashed byte state — but it left the on-disk layout and move semantics open. Sidereal is the system of record that renames, moves, and organises files, so these govern interruption recovery and data safety and must be decided before any storage code is written. Any layout must satisfy five invariants: database and filesystem updates recoverable after an interruption; a user-visible move never rewrites content; internal historical versions never surface as duplicate current assets; path traversal and symlink escape rejected; every stored file reconcilable to an `AssetVersion` and its hash.

## Decision

Split the store in two. The bytes of every `AssetVersion` live in a **protected internal object store the UI never lists, content-addressed by BLAKE3**; a browsable target/session/date tree is a **projection** over it, deferred until organize/move operators and domain facets exist. The internal store is the only part the first storage build ships:

```
{STORAGE_ROOT}/
  store/{h[0:2]}/{h[2:4]}/{h}   # h = hex BLAKE3 of the version's bytes
  tmp/{uuid}                     # ingest staging, same filesystem as store/
```

Ingest is **copy-then-commit-last**: copy the candidate into `tmp/` (copy, never move — an in-place move is the crash risk being eliminated), BLAKE3-hash, stat, and sniff format; dedup by hash (an existing object shares the file); `fsync` the file, atomically `rename` into `store/`, **`fsync` the `store/` directory**, then commit the DB row. Commit-last ordering — with the directory sync before commit — means a crash leaves at worst an orphan `tmp/` file (swept on startup), never a committed row pointing at bytes the filesystem dropped. A re-hash `verify` pass detects out-of-band byte changes and missing objects; the identity-level response to a mismatch (record it, never silently update, mark a missing Asset unavailable without touching lineage) is owned by ADR-003. Directionally set here but **deferred**: the projection's link mechanism (symlink / hardlink / reflink / copy), and cross-filesystem moves as **copy → verify hash → atomically swap the DB reference → reclaim the source** (the reference swap is the commit point; no non-atomic cross-device `rename` is ever used).

## Consequences

- User-visible reorganisation can never corrupt byte truth: moves act on the projection, and the content-addressed object never moves.
- Content addressing yields uniform directory fan-out, free byte-level dedup, and self-reconciliation (the filename *is* the hash). Because identical bytes share one object, `asset_version.content_hash` is **indexed but not unique** — so GC must be **refcounted**: an object is deletable only when no asset version, lineage edge, operation run, hold, or migration audit references its hash. The first build records references only and never deletes, which keeps the delete-vs-ingest race (GC unlinking an object a concurrent dedup ingest is adopting) out of scope until GC is actually built.
- Store paths are opaque to a human starting from a UI id; the storage-path/content-hash mapping is kept as read-only metadata and surfaced in the UI.
- Ingest copies before rename, so peak disk is roughly source + object during ingest — a real concern on a near-full volume; a same-filesystem reflink can reclaim this cost where the platform supports it.
- Only the internal store, ingest, and integrity reconciliation are built now; the projection and the cross-filesystem move protocol are additive later and cannot invalidate stored bytes.

## Alternatives Considered

### Alternative 1: Single user-meaningful tree
- **Pros:** what the user browses on disk *is* the managed tree — no indirection and no projection layer.
- **Cons:** its organizing keys (target, session, date) are domain-specific values a domain-agnostic core cannot depend on and that do not exist until a domain pack classifies the asset — yet the store must be addressable from the first ingest; and every rename/move rewrites where real bytes live, the operation most at risk from a crash.
- **Why not:** it couples the physical store to domain vocabulary the core must stay ignorant of and puts irreplaceable bytes in the path of every reorganisation — the opposite of the data safety the invariants demand.

### Alternative 2: Opaque-UUID object naming (instead of content-hash)
- **Pros:** the stored path derives directly from the version's own id; no separate hash is needed to locate bytes.
- **Cons:** a time-ordered UUID prefix clusters a night's session into one directory instead of fanning out; no free dedup; the filename carries no integrity signal, so reconciliation needs a side lookup.
- **Why not:** content-hash naming gives uniform fan-out, dedup, and self-verifying files for free, and its only real cost — opacity from a UI id — is cheaply mitigated by storing the id→path mapping.
