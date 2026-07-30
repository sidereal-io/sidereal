# 002: Core / Domain-Pack Seam

**Status:** Proposed
**Date:** 2026-07-29
**Context:** M0 of [RFC #213](https://github.com/sidereal-io/sidereal/issues/213). The [architecture](../architecture/README.md#core-and-domain-packs) commits to a domain-agnostic core with astrophotography as the first domain pack. This ADR fixes where the seam actually falls and how packs are delivered.

## Problem

The principle is settled: `kind` must not be a Rust enum containing `light | dark | flat`, and astro vocabulary belongs to a pack rather than to core. Two questions remain.

**1. Where exactly does the seam fall?** The RFC's first cut:

- *Core:* Asset, Collection, Lineage, Operation Run, plugin registry, storage layout, search/index, job queue, web shell.
- *Astro pack:* FITS/XISF readers, astro kind vocabulary, calibration sets and master matching, sessions, OpenNGC catalog, visibility math, targets and annotations, equipment, acquisitions, plate solving, sky map.

Several of those are genuinely ambiguous. **Equipment** is arguably general (any camera-based hobby has gear) but its spec fields are astro-shaped. **Sessions** are a Collection specialisation — does core know the concept and the pack fill it in, or does the pack define it wholesale? **Visibility math** is pure astronomy but drives generic UI sorting.

**2. Are packs compiled in or loaded at runtime?** This interacts with [ADR-001](ADR-001-plugin-boundary.md): a pack that is a plugin under the chosen boundary gets that boundary's costs, and a pack containing the FITS reader is on the hot path for every ingest.

## Options

### Option A: Astro pack compiled in, other plugins loaded

**Pros:**
- No boundary cost on the hot ingest path (FITS parsing, facet extraction).
- Single-binary distribution; the shipped product is one artifact.
- Pack still coded against the public interface, so the seam is real even if delivery isn't dynamic.

**Cons:**
- The pack interface is only proven by one in-tree consumer, so it can drift the way the plugin ABI would without dogfooding.
- "Swap the astro pack for a photo pack" stays theoretical.

### Option B: Packs are ordinary loadable plugins

**Pros:**
- Seam proven by construction — if the astro pack can be loaded, so can a third-party pack.
- A future general-media pack needs no core change.
- Users install only what they use.

**Cons:**
- Boundary cost on every ingest, which under an out-of-process ADR-001 may be significant for large FITS files.
- Packs need to contribute UI, not just backend behaviour — a much larger contract than an Operation plugin.
- More moving parts in the default install.

### Option C: Thin core with facets only; everything else a pack

**Pros:**
- Maximally clean seam; core is small and genuinely domain-free.

**Cons:**
- Realistically means shipping nothing useful for a long time, and the RFC's milestones assume a working vertical slice at M1.
- Risks the core being *too* thin — cross-pack concerns (search UI, collection views) end up duplicated per pack.

## Recommendation

Left open pending [ADR-001](ADR-001-plugin-boundary.md), since the boundary's cost profile changes the answer.

Working position: **decide the seam now, defer the delivery mechanism.** Coding the astro pack against a public pack interface captures most of the architectural benefit; whether it is dynamically loaded is reversible later, whereas a wrong seam is a migration.

Specific calls to make explicit:

- Does core know the *concept* "session" (a Collection with a time span and a subject) with the pack supplying vocabulary, or is session entirely pack-defined?
- Equipment: core, pack, or core concept with pack-defined spec facets?
- Can a pack contribute frontend surface, or only API and facets? This gates whether M5 can proceed independently.

## Decision

[Filled in after review.]
