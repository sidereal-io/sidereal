# 003: Storage Layout and Asset Identity

**Status:** Proposed
**Date:** 2026-07-29
**Context:** M0 of [RFC #213](https://github.com/sidereal-io/sidereal/issues/213). Not purely a mechanism choice — the outcome constrains the [Lineage](../architecture/README.md#lineage) model and determines whether an Operation may mutate bytes in place. Decide before M1 builds the core spine.

## Problem

Two coupled questions.

**1. What is an Asset's identity?** v2 makes Sidereal the system of record for files on disk — it renames and moves them. So identity cannot be path-derived, or the system reorganising a tree destroys its own references. Beyond that, the choice is content hash vs. surrogate ID.

**2. What is the on-disk tree shape?** v0.10.x uses `{STORAGE_PATH}/processed/{id % 1000}/{id}/{id}_{size}.jpg` — a sharded surrogate-ID layout holding three fixed renditions. v2 assets are FITS/XISF originals with derived renditions, in user-meaningful arrangements, so the layout is open.

### The tension that makes this load-bearing

Lineage edges point at asset identities. If identity **is** the content hash, any operation that rewrites an asset's bytes changes its identity and silently orphans every edge pointing at it. `stacked ← [187 lights] + [master_dark_v3]` becomes a dangling reference the moment something touches `master_dark_v3`.

That is not a bug to fix later; it is a property of the choice. Whichever option wins must state explicitly whether in-place byte mutation is legal.

## Options

### Option A: Surrogate identity, content hash as a property

Assets get a stable opaque ID. The content hash is recorded for dedup and integrity checking but is not the primary key.

**Pros:**
- Lineage edges survive any byte-level change; no orphaning.
- In-place mutation is legal, so rename/retag/metadata-write operations stay cheap.
- Matches what already works in v0.10.x, and what the importer must map onto.
- Dedup still available via a hash index.

**Cons:**
- Identity is not verifiable from the file alone — a moved-out-of-band file needs reconciliation to re-associate.
- Two identity-ish concepts to keep straight in the model.

### Option B: Content-hash identity plus strict immutability

The hash *is* the identity. Operations never mutate bytes; any byte-level change produces a new asset with a lineage edge to its predecessor.

**Pros:**
- Content-addressed integrity for free; verification is recomputing a hash.
- Dedup is structural, not a separate index.
- Full history of every byte-level change, expressed in the lineage graph the model already has.

**Cons:**
- Every metadata-embedding operation (FITS header write, XMP, tagging into the file) forks a full copy — expensive at FITS sizes.
- Disk growth on operations users think of as edits.
- Lineage graphs fill with mechanical fork edges alongside the scientifically meaningful ones, which is a UX problem as much as a storage one.
- The importer must synthesise identities for existing rows, so v0.10.x IDs stop being stable references.

### Option C: Surrogate identity with content-addressed blob storage

Assets have surrogate IDs; bytes live in a content-addressed store that assets point into.

**Pros:**
- Both properties: stable lineage plus structural dedup and verifiable bytes.
- Cheap "same bytes, two logical assets."

**Cons:**
- **Directly conflicts with a north-star goal** — a content-addressed blob store is not a user-meaningful tree, and "Sidereal renames, moves, and organises your files" is the point. Would need a projection layer (symlinks or an export view), and now there are two trees to keep consistent.
- Most complex option; heaviest migration.

## Recommendation

**Leaning Option A**, primarily because Option B's cost lands on the most common operations and Option C fights the north star.

Explicitly decide and record:

- **Is in-place byte mutation legal?** Under A it can be, but "operations that rewrite bytes must declare it, and core records a lineage edge" is a defensible middle ground worth considering.
- **Reconciliation behaviour** for files changed out of band — v0.10.x has an orphan sweep; v2 needs a documented answer since it owns the tree.
- **Tree shape.** User-meaningful arrangement (by target, by session, by date) is the north-star direction, but that is a *layout* decision separable from identity, and derived renditions may want a separate internal tree from originals.

Also worth deciding here: whether the content hash is recorded for **every** asset regardless of option. Dedup and integrity both want it, and it is cheap at ingest.

## Decision

[Filled in after review.]
