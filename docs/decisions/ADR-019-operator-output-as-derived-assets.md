---
id: adrs-adr019
date: 2026-11-25
status: accepted
title: 'ADR-019: Operator Output Becomes a Derived Asset Linked by Lineage'
description: Architecture decision record (ADR) for how core records a file an Operator makes. The output is a new Asset marked derived, its version is linked to the version it was made from by a lineage row with a role, and the Operator streams its bytes through the Asset context into core's staging.
---

# ADR-019: Operator Output Becomes a Derived Asset Linked by Lineage

## Context

An Asset is a stable identity, and an Asset version is one immutable set of bytes. Until now only a Source made Assets, and an Operator could only propose facet values. The first Operator that makes a file is the preview Operator: it turns an uploaded image, such as a TIFF that browsers cannot show, into a JPEG the library can show. Later Operators will make thumbnails, master calibration frames and stacks.

[ADR-003](ADR-003-asset-identity-and-content-revisions.md) says derived output is a separate Asset by default, and that lineage links versions, never Assets. It left open how an Operator declares its output and how core records it. The answer has to work for a 100 MiB file, keep core the only writer of identities, hashes and records, and let the library tell what a person brought in from what Sidereal made.

## Decision

An Operator's output is a new Asset, which core marks as derived in an `origin` column of `assets` (`source` or `derived`). Its one version records the Operator's id and version as its producer. Core links the new version to the version the run read with a row in `asset_lineage`, which names the output version, the input version, the output's role (`preview` is the first) and the run. The Operator declares at registration which roles it makes, proposes only a kind and a media type, and streams the bytes through its Asset context into core's staging. Core checks, publishes and commits the Asset, its version, the lineage row and the run's success in one transaction.

## Consequences

- The library lists originals by reading one column. Listing, reading and repeat-upload matching ignore derived Assets without following lineage.
- Finding "the preview of this version" is one indexed lookup on `asset_lineage` by input version and role.
- A new kind of output, such as a thumbnail, adds a role value, a declaration in the contract and its checks. The tables keep their shape.
- An output can have several inputs later, such as a stack of 187 lights: the lineage table already holds one row per input.
- A derived Asset can be deleted only by a lineage-aware cleanup, as ADR-003 requires. None exists yet, so derived Assets are kept.
- `asset_versions.source_id` and `source_version` now hold the producer, which is a Source or an Operator. `assets.origin` says which, and readers must not assume a Source.
- An output never passes through the Operator's memory as one buffer, so a large file costs the size of one write, not the size of the file. In exchange, the contract gains a host-supplied writer, which every host that runs Operators must provide.
- Every derived output is stored as a full object, so the library uses more storage.

## Alternatives Considered

### Alternative 1: Renditions attached to a version, with no Asset
- **Pros**: one table keyed by version; no origin column; nothing to hide from the library.
- **Cons**: an output could carry no facets, no versions and no lineage of its own; a stack or a master frame, which people search for and reuse, could not be one.
- **Why not**: ADR-003 makes derived output a separate Asset, and the outputs that come after previews are things people work with as Assets.

### Alternative 2: The output as a new version of the input Asset
- **Pros**: no new Asset; the input's page could show it as a later version.
- **Cons**: the original's current version would point to bytes that are not the file the person uploaded, and a stack would become a version of one of its many inputs.
- **Why not**: ADR-003 keeps a version for a change to an Asset's own bytes. A preview or a stack is a different file made from it.

### Alternative 3: Derived-ness read from lineage alone, with no origin column
- **Pros**: one fact in one place; no column to keep in step with lineage.
- **Cons**: every library listing and every repeat-upload check would test for a lineage row, and an output whose lineage was lost would appear in the library as an original.
- **Why not**: whether an Asset is an original is fixed when core creates it, so a column records it once and the database checks it.

### Alternative 4: The Operator returns its output as one byte buffer
- **Pros**: the simplest contract; no writer to supply.
- **Cons**: the whole file sits in memory next to the decoded image, and core learns its size only after the run.
- **Why not**: a preview of a large image runs close to its memory limit already, and core must refuse a file over 100 MiB while it is written.
