---
id: adrs-adr003
date: 2026-08-18
status: accepted
title: 'ADR003: Asset Identity and Content Revisions'
description: Architecture Decision Record (ADR) for the identity model of the v2 store — a stable Asset plus immutable, content-hashed AssetVersions.
---

# ADR-003: Asset Identity and Content Revisions

## Context

Sidereal serves as the system of record for the files it renames, moves, and organizes, meaning asset identity cannot be derived from file paths.

A single mutable Asset record is also not enough. If an operation rewrites bytes under a stable id, the previous hash and state vanish, and lineage cannot tell before from after.

The model must be settled before the core spine is built, and it must answer three things:

- What stable identity do users, collections, and external mappings refer to?
- How are immutable byte states identified and retained?
- Which changes create a new revision?

On-disk layout and move semantics are a separate decision, taken in [ADR-011 — Storage tree layout & cross-filesystem moves](ADR-011-storage-tree-layout.md).

## Decision

Implement a dual-layer entity architecture combining a **persistent `Asset` record with immutable `AssetVersion` instances**.

### Entity Definitions & References
- **`Asset.id`**: Serves as a persistent, opaque surrogate identifier, decoupled from both file path and underlying binary content.
- **`AssetVersion.id`**: An opaque identifier tied to a mandatory content hash (indexed for deduplication and verification, but not exposed as the primary key), along with size in bytes, format specification, and origin provenance.
- **Explicit Lineage**: All lineage links and input/output definitions for Operation Runs strictly bind to specific **versions**.

### Lifecycle Rules & Mutation Boundaries
- Renames & Relocations: Path modifications are treated purely as location updates and do not generate a new version.
- Byte Modifications: Any change to binary content forces the creation of a distinct immutable version.
- Derived Output: Secondary scientific artifacts (such as stacks, thumbnails, masters, or exported files) are created as separate Assets by default.

### Core Platform Responsibilities & Retention
System core retains exclusive responsibility for minting IDs, calculating content hashes, advancing version pointers, and logging lineage. Garbage collection and retention are 
fully lineage-aware: core will refuse to delete any entity currently bound to lineage history, active runs, legal holds, or migration audit records.

### State Tracking & Version Navigation
The active version is referenced through a dedicated `Asset.current_version_id` pointer rather than a moving `v0` target, while `isLatest` is evaluated dynamically. Auxiliary 
attributes—such as the version_seq counter alongside optional `label`, `aliases`, or `note` fields—act purely as descriptive metadata. Pointer updates rely on optimistic 
concurrency control, using compare-and-swap validation backed by an `Asset.revision` sequence.

### Reconciliation & Out-of-Band Changes
External or out-of-band byte modifications will never trigger automatic version updates. Instead, reconciliation flags an integrity mismatch requiring deliberate resolution 
(adopt, restore, or ignore). If an asset's file path disappears, it is flagged as unavailable while preserving its identity, version history, and lineage untamed.

## Consequences

- Stable links, collections, and source mappings survive moves and byte revisions. Lineage records exact content, not a mutable placeholder.
- Byte-editing operations consume additional storage until retention safely reclaims old revisions. A current-state query joins Asset to its current version.
- Whether an Operator's output is a *new version of an input* or a *new Asset* rests on "Operator intent," which every Operator must declare correctly. A mis-declaration corrupts lineage semantics — for example, a stack recorded as a version of one of its 187 inputs. This classification rule is load-bearing and must be pinned down when the Operator contract is specified.

## Alternatives Considered

### Alternative 1: Mutable Asset with a surrogate id (one current hash; byte rewrites update the row)
- **Pros:** paths and references stay stable; simplest model.
- **Cons:** it destroys byte history — a self-edge cannot say which pre- or post-operation bytes participated.
- **Why not:** lineage is the whole point of the rewrite, and this model cannot express it.

### Alternative 2: Content hash as Asset identity (every byte change is a new Asset)
- **Pros:** strong integrity by construction.
- **Cons:** logical identity becomes unstable; mechanical revisions mix with scientifically meaningful assets; collections and external mappings must chase replacements.
- **Why not:** it sacrifices the stable user-facing identity that links and collections depend on.
