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

**2. Are packs compiled in or loaded at runtime?** [ADR-001](ADR-001-plugin-boundary.md) separates
semantic contracts from execution profiles. A pack containing a FITS reader is on the hot path for
every ingest and may use the built-in Rust profile without receiving a different semantic contract.

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
- A dynamically loaded pack using the script or external-provider profile adds boundary cost on every
  ingest, which may be significant for large FITS files.
- Packs need to contribute UI, not just backend behaviour — a much larger contract than an Operator plugin.
- More moving parts in the default install.

### Option C: Thin core with facets only; everything else a pack

**Pros:**
- Maximally clean seam; core is small and genuinely domain-free.

**Cons:**
- Realistically means shipping nothing useful for a long time, and the RFC's milestones assume a working vertical slice at M1.
- Risks the core being *too* thin — cross-pack concerns (search UI, collection views) end up duplicated per pack.

## Recommendation

Working position: **decide the seam now and compile the first-party astro pack into v2.0.** It uses
the built-in Rust execution profile from ADR-001 and implements the same semantic contracts and
conformance fixtures. User-installed Rhai plugins and external providers can extend its schemas and
Operators through explicit grants; dynamically replacing the entire domain pack is deferred.

This keeps FITS/XISF parsing and the initial UI contribution in the shipped artifact without making
astro vocabulary part of core. Whether a later domain pack can be dynamically installed remains
reversible; putting astro fields into core does not.

Specific calls to make explicit:

- Does core know the *concept* "session" (a Collection with a time span and a subject) with the pack supplying vocabulary, or is session entirely pack-defined?
- Equipment: core, pack, or core concept with pack-defined spec facets?
- Can a pack contribute frontend surface, or only API and facets? This gates whether M5 can proceed independently.

## Decision

[Filled in after review.]
