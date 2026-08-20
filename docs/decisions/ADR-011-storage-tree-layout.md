# 011: Storage Tree Layout and Cross-Filesystem Moves

**Status:** Proposed
**Date:** 2026-08-19
**Context:** M0 of [RFC #213](https://github.com/sidereal-io/sidereal/issues/213). Split out of
[ADR-003](ADR-003-storage-layout-and-asset-identity.md), which settled Asset/AssetVersion **identity**
but explicitly left the **on-disk layout** and **move semantics** open. Those govern interruption
recovery and data safety, so they get their own decision rather than lingering inside an accepted ADR.

**This ADR gates M1.** Per the [STOP-on-open-ADR rule](../../AGENTS.md#open-adrs-block-design--stop), no
M1 storage design may assume an answer here while this ADR is Proposed — it must be worked with the
operator and Accepted first.

## Problem

ADR-003 established that identity does not prescribe layout: an `Asset` is a stable surrogate, each
`AssetVersion` is an immutable content-addressed byte state, and Sidereal is the system of record that
renames, moves, and organises files. Two questions remain open and must be answered before code touches
a real tree:

1. **The tree shape.** How are user-visible originals, derived renditions, and superseded versions laid
   out on disk — one user-meaningful tree, a split between a browsable tree and a protected internal
   subtree, or something else? How does the layout address a version by hash without exposing opaque IDs
   as paths?
2. **Cross-filesystem move behavior.** How is a user-visible move performed and made crash-safe when the
   source and destination are on different filesystems (where an atomic `rename(2)` is unavailable)?

## Invariants (inherited from ADR-003, binding on any option)

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
- **Con:** the organizing keys (target, session, date) are astro **facets** that do not exist until M3,
  and the tree must be addressable *now* (M1). It couples the physical store to domain vocabulary the
  core is supposed to stay ignorant of, and every rename/move rewrites where real bytes live — the exact
  operation most at risk from a crash.

### Option B: Split — protected internal object store + browsable projection

The bytes of every `AssetVersion` (originals, renditions, superseded alike) live in a protected internal
store the UI never lists. A separate **browsable** tree organised by target/session/date is a
*projection* over that store, materialized later (M2/M3) when organize/move operators and astro facets
exist. A user-visible move is a projection operation; it never rewrites the stored bytes.

- **Pro:** the internal store is domain-agnostic and stable; user-visible reorganisation can never
  corrupt byte truth; M1 builds only the internal store and defers the projection cleanly.
- **Con:** two layers, and the projection's materialization mechanism (symlink / hardlink / reflink /
  copy) is a later sub-decision.

### Addressing the internal store: opaque-id vs content-hash

Independently of A/B, the internal store can name a version file by its **opaque UUID** or by its
**BLAKE3 content hash** (git-object style). Content-hash naming gives uniform directory fan-out by
construction (a night's session spreads evenly, where a time-ordered UUID prefix would cluster), free
content dedup, immutable-friendly files, and self-reconciliation (the filename *is* the hash). Its cost
is that the path is opaque to a human starting from a UI id — mitigated by storing and surfacing the
`storage_path`/`content_hash` mapping as read-only metadata ([#217](https://github.com/sidereal-io/sidereal/issues/217),
per ADR-008) — and hash-refcounted GC.

## Recommendation

**Option B, with the internal object store content-addressed by BLAKE3.**

**Internal object store** — the only part M1 builds:

```
{STORAGE_ROOT}/
  store/{h[0:2]}/{h[2:4]}/{h}   # h = hex BLAKE3 of the version's bytes
  tmp/{uuid}                     # ingest staging, same filesystem as store/
```

- **Atomic ingest:** copy the candidate into `tmp/` (**copy, never move** — filesystem-safety per
  [migration.md](../architecture/migration.md)); BLAKE3-hash, stat, sniff format; **dedup by hash** (an
  existing hash shares the file); `fsync`; **atomic `rename` into `store/`**; then commit the DB row.
  Commit-last ordering means a crash leaves at worst an orphan `tmp/` file (swept on startup), never a DB
  row pointing at nothing.
- **Integrity reconciliation** (identity/versioning rules in [ADR-003](ADR-003-storage-layout-and-asset-identity.md)):
  re-hash on a `verify` pass; a mismatch is recorded (`integrity_mismatch`) and needs an explicit
  adopt/restore/ignore, never a silent update; a missing file marks the Asset unavailable without
  deleting identity/versions/lineage.
- **Path safety:** every path canonicalized and asserted under `STORAGE_ROOT`; `..` traversal and symlink
  escape rejected.
- **GC:** hash-**refcounted** — a stored file is deletable only when no `asset_version`, lineage edge,
  Operation Run, hold, or migration audit references its hash (ADR-003 retention). M1 records references;
  it does not garbage-collect.

**Browsable projection** — *deferred, not M1:* a target/session/date tree materialized as links over the
object store, built when organize/move operators and astro facets land (M2/M3). Because it is a
projection, a user-visible move/rename changes only the projection, never the stored bytes. The
materialization mechanism (symlink / hardlink / reflink / copy) is settled then; whichever is chosen,
bytes are preserved.

**Cross-filesystem moves** — *deferred, not M1:* when a user-visible move must cross filesystems (no
atomic `rename(2)`), the protocol is **copy → verify hash → atomically swap the reference → reclaim the
source**, with the DB reference as the commit point so an interruption is recoverable. No non-atomic
cross-device `rename` is ever used.

### Invariant check

| Invariant (from ADR-003) | How B + content-addressing satisfies it |
|---|---|
| DB + FS updates recoverable after interruption | commit-last ingest; orphan `tmp/` sweep; reference-swap as commit point for moves |
| A user-visible move never rewrites content | moves are projection operations; the content-addressed object never moves |
| Historical versions never surface as duplicate current assets | the object store is never browsed; the UI resolves current via the DB pointer |
| Path traversal / symlink escape rejected | canonicalize + assert under `STORAGE_ROOT` on every access |
| Every stored file reconciles to an `AssetVersion` + hash | filename *is* the hash; `asset_version.content_hash` is unique/indexed |

## Decision

*[Proposed for acceptance — pending operator review. On acceptance: Option B with a content-addressed
BLAKE3 internal object store, as recommended above; the browsable projection and cross-filesystem move
protocol are directionally set here but finalized when their operators are built in M2/M3. M1 builds only
the internal object store, ingest, and integrity reconciliation.]*
