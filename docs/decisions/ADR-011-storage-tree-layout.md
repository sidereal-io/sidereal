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

*(Sketched to frame the decision; to be developed with the operator when M1 storage design opens.)*

### Option A: Single user-meaningful tree

Originals and current renditions live in one tree organised by target/session/date; superseded versions
are marked in the database but not surfaced as separate current files.

### Option B: Split browsable + protected internal subtree

User-visible originals live in a browsable tree; derived renditions and superseded versions live in a
separate internal subtree the UI never presents as current assets.

Cross-filesystem moves, under either option, are likely **copy → verify hash → atomically swap the
reference → reclaim the source**, rather than a non-atomic `rename` across devices — but the exact
protocol and its crash-recovery steps are part of what this ADR must decide.

## Recommendation

*(To be filled in with the operator during M1 design.)*

## Decision

*[Open — must be worked and Accepted before M1 storage design begins, per the STOP-on-open-ADR rule.]*
