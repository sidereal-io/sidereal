# 011: Storage Tree Layout and Cross-Filesystem Moves

**Status:** Proposed · **Date:** 2026-08-19

**Context:** Builds on [ADR-003 — Asset identity & content
revisions](ADR-003-storage-layout-and-asset-identity.md), which settled asset identity — a stable `Asset`
surrogate plus immutable, content-hashed `AssetVersion` byte states — and deliberately left on-disk
layout and move semantics open. Those govern interruption recovery and data safety, so they are decided
here, and must be Accepted before any storage code is written.

## Problem

Identity does not prescribe layout: an `Asset` is a stable surrogate, each `AssetVersion` is an
immutable content-hashed byte state, and Sidereal is the system of record that renames, moves, and
organises files. Two questions remain open and must be answered before code touches a real tree:

1. **The tree shape.** How are user-visible originals, derived renditions, and superseded versions laid
   out on disk — one user-meaningful tree, a split between a browsable tree and a protected internal
   subtree, or something else? How does the layout address a version by hash without exposing opaque IDs
   as paths?
2. **Cross-filesystem move behavior.** How is a user-visible move performed and made crash-safe when the
   source and destination are on different filesystems (where an atomic `rename(2)` is unavailable)?

## Invariants (binding on any option)

Whatever layout and move strategy are chosen must satisfy:

- database and filesystem updates are recoverable after an interruption;
- a user-visible move never rewrites content (bytes are preserved; only the path changes);
- internal historical versions are never presented as duplicate current assets;
- path traversal and symlink escape are rejected;
- every stored file can be reconciled to an `AssetVersion` and its hash.

## Options

Two independent choices: the **tree shape** (A vs B) and how the byte store **addresses** a version.

### Option A: Single user-meaningful tree

Originals and current renditions live in one tree organised by target/session/date; superseded versions
are flagged in the database but not surfaced as separate current files.

- **Pro:** what the user browses on disk *is* the managed tree — no indirection.
- **Con:** the organizing keys (target, session, date) are domain-specific values a domain-agnostic core
  cannot depend on, and they are not populated until a domain pack classifies the asset — yet the store
  must be addressable from the first ingest. It couples the physical store to vocabulary the core stays
  ignorant of, and every rename/move rewrites where real bytes live — the operation most at risk from a
  crash.

### Option B: Split — protected internal object store + browsable projection

The bytes of every `AssetVersion` (originals, renditions, superseded alike) live in a protected internal
store the UI never lists. A separate **browsable** tree organised by target/session/date is a
*projection* over that store, materialized later once organize/move operators and domain facets exist. A
user-visible move is a projection operation; it never rewrites the stored bytes.

- **Pro:** the internal store is domain-agnostic and stable; user-visible reorganisation can never
  corrupt byte truth; the first build ships only the internal store and defers the projection cleanly.
- **Con:** two layers, and the projection's materialization mechanism (symlink / hardlink / reflink /
  copy) is a later sub-decision.

### Addressing the internal store: opaque-id vs content-hash

Independently of A/B, the internal store can name a version file by its **opaque UUID** or by its
**BLAKE3 content hash** (git-object style). Content-hash naming gives uniform directory fan-out by
construction (a night's session spreads evenly, where a time-ordered UUID prefix would cluster), free
content dedup, immutable-friendly files, and self-reconciliation (the filename *is* the hash). Its cost
is that the path is opaque to a human starting from a UI id — mitigated by storing the
storage-path/content-hash mapping as read-only metadata surfaced in the UI — and hash-refcounted GC.

## Recommendation

**Option B, with the internal object store content-addressed by BLAKE3.**

**Internal object store** — the only part the first storage build ships:

```
{STORAGE_ROOT}/
  store/{h[0:2]}/{h[2:4]}/{h}   # h = hex BLAKE3 of the version's bytes
  tmp/{uuid}                     # ingest staging, same filesystem as store/
```

- **Atomic ingest:** copy the candidate into `tmp/` (**copy, never move** — an in-place move during
  ingest is the crash risk we are eliminating); BLAKE3-hash, stat, sniff format; **dedup by hash** (an
  existing hash shares the file); `fsync`; **atomic `rename` into `store/`**; then commit the DB row.
  Commit-last ordering means a crash leaves at worst an orphan `tmp/` file (swept on startup), never a DB
  row pointing at nothing.
- **Integrity reconciliation:** re-hash on a `verify` pass; a mismatch is recorded
  (`integrity_mismatch`) and needs an explicit adopt/restore/ignore, never a silent update; a missing
  file marks the Asset unavailable without deleting identity, versions, or lineage.
- **Path safety:** every path canonicalized and asserted under `STORAGE_ROOT`; `..` traversal and symlink
  escape rejected.
- **GC:** hash-**refcounted** — a stored file is deletable only when no asset version, lineage edge,
  operation run, hold, or migration audit references its hash. The first build records references only;
  it does not garbage-collect.

**Browsable projection** — *deferred:* a target/session/date tree materialized as links over the object
store, built once organize/move operators and domain facets exist. Because it is a projection, a
user-visible move/rename changes only the projection, never the stored bytes. The materialization
mechanism (symlink / hardlink / reflink / copy) is settled then; whichever is chosen, bytes are
preserved.

**Cross-filesystem moves** — *deferred:* when a user-visible move must cross filesystems (no atomic
`rename(2)`), the protocol is **copy → verify hash → atomically swap the reference → reclaim the
source**, with the DB reference as the commit point so an interruption is recoverable. No non-atomic
cross-device `rename` is ever used.

### Invariant check

| Invariant | How B + content-addressing satisfies it |
|---|---|
| DB + FS updates recoverable after interruption | commit-last ingest; orphan `tmp/` sweep; reference-swap as commit point for moves |
| A user-visible move never rewrites content | moves are projection operations; the content-addressed object never moves |
| Historical versions never surface as duplicate current assets | the object store is never browsed; the UI resolves current via the DB pointer |
| Path traversal / symlink escape rejected | canonicalize + assert under `STORAGE_ROOT` on every access |
| Every stored file reconciles to an `AssetVersion` + hash | filename *is* the hash; `asset_version.content_hash` is unique/indexed |

## Decision

*[Proposed for acceptance — pending operator review. On acceptance: Option B with a content-addressed
BLAKE3 internal object store, as recommended above; the browsable projection and cross-filesystem move
protocol are directionally set here but finalized when their operators are built. The first storage build
delivers only the internal object store, ingest, and integrity reconciliation.]*
