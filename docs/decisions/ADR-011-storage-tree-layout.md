---
id: adrs-adr011
date: 2026-08-26
status: accepted
title: 'ADR011: Storage Tree Layout and Cross-Filesystem Moves'
description: Architecture Decision Record (ADR) for the v2 on-disk asset store — a protected internal object store content-addressed by BLAKE3, with a browsable projection deferred.
---

# ADR-011: Storage Tree Layout and Cross-Filesystem Moves

## Context

[ADR-003 — Asset identity & content revisions](ADR-003-asset-identity-and-content-revisions.md) settled that identity is independent of layout: an `Asset` is a stable surrogate, and each `AssetVersion` is an immutable, content-hashed byte state. But it left the on-disk layout and move semantics open.

Sidereal is the system of record that renames, moves, and organises files, so these choices govern interruption recovery and data safety. They must be decided before any storage code is written.

Any layout must satisfy five invariants:

1. Database and filesystem updates are recoverable after an interruption.
2. A user-visible move never rewrites content.
3. Internal historical versions never surface as duplicate current assets.
4. Path traversal and symlink escape are rejected.
5. Every stored file is reconcilable to an `AssetVersion` and its hash.

## Decision

Split the store in two. The bytes of every `AssetVersion` live in a **protected internal object store the UI never lists, content-addressed by BLAKE3.** A browsable target/session/date tree is a **projection** over it, deferred until organize/move operators and domain facets exist. The internal store is the only part the first storage build ships:

```
{STORAGE_ROOT}/
  store/{h[0:2]}/{h[2:4]}/{h}   # h = hex BLAKE3 of the version's bytes
  tmp/{uuid}                     # ingest staging, same filesystem as store/
```

Ingest is **copy-then-commit-last.** Copy the candidate into `tmp/` — copy, never move, since an in-place move is the crash risk being eliminated — then BLAKE3-hash it, stat it, and sniff its format. Dedup by hash: an existing object shares the file. Then `fsync` the file, atomically `rename` it into `store/`, **`fsync` the `store/` directory**, and finally commit the DB row. This commit-last ordering, with the directory sync before commit, means a crash leaves at worst an orphan `tmp/` file (swept on startup) — never a committed row pointing at bytes the filesystem dropped.

A re-hash `verify` pass detects out-of-band byte changes and missing objects. The identity-level response to a mismatch is owned by ADR-003.

Set directionally here but **deferred:** the projection's link mechanism (symlink / hardlink / reflink / copy), and cross-filesystem moves as **copy → verify hash → atomically swap the DB reference → reclaim the source.** The reference swap is the commit point; no non-atomic cross-device `rename` is ever used.

## Consequences

- User-visible reorganisation can never corrupt byte truth: moves act on the projection, and the content-addressed object never moves.
- Content addressing yields uniform directory fan-out, free byte-level dedup, and self-reconciliation — the filename *is* the hash. Because identical bytes share one object, `asset_version.content_hash` is **indexed but not unique.** So GC must be **refcounted:** an object is deletable only when no asset version, lineage edge, operation run, hold, or migration audit references its hash. The first build records references only and never deletes. That keeps the delete-vs-ingest race — GC unlinking an object a concurrent dedup ingest is adopting — out of scope until GC is actually built.
- Store paths are opaque to a human starting from a UI id. The storage-path/content-hash mapping is kept as read-only metadata and surfaced in the UI.
- Ingest copies before rename, so peak disk is roughly source + object during ingest — a real concern on a near-full volume. A same-filesystem reflink can reclaim this cost where the platform supports it.
- Only the internal store, ingest, and integrity reconciliation are built now. The projection and the cross-filesystem move protocol are additive later and cannot invalidate stored bytes.

## Alternatives Considered

### Alternative 1: Single user-meaningful tree
- **Pros:** what the user browses on disk *is* the managed tree — no indirection and no projection layer.
- **Cons:** its organizing keys (target, session, date) are domain-specific values a domain-agnostic core cannot depend on, and they do not exist until a domain pack classifies the asset — yet the store must be addressable from the first ingest. And every rename or move rewrites where real bytes live, the operation most at risk from a crash.
- **Why not:** it couples the physical store to domain vocabulary the core must stay ignorant of, and it puts irreplaceable bytes in the path of every reorganisation — the opposite of the data safety the invariants demand.

### Alternative 2: Opaque-UUID object naming (instead of content-hash)
- **Pros:** the stored path derives directly from the version's own id; no separate hash is needed to locate bytes.
- **Cons:** a time-ordered UUID prefix clusters a night's session into one directory instead of fanning out; no free dedup; the filename carries no integrity signal, so reconciliation needs a side lookup.
- **Why not:** content-hash naming gives uniform fan-out, dedup, and self-verifying files for free, and its only real cost — opacity from a UI id — is cheaply mitigated by storing the id→path mapping.
