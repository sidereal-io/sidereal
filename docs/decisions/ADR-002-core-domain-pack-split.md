# 002: Core / Domain-Pack Seam

**Status:** Accepted
**Date:** 2026-07-29
**Context:** The core is domain-agnostic with astrophotography as the first domain pack; this ADR fixes where the seam actually falls and how packs are delivered. It builds on the plugin execution profiles in [ADR-001 — Plugin contract & execution profiles](ADR-001-plugin-boundary.md).

## Problem

The principle is settled: `kind` must not be a Rust enum containing `light | dark | flat`, and astro vocabulary belongs to a pack rather than to core. Two questions remain.

**1. Where exactly does the seam fall?** A first cut:

- *Core:* Asset, Collection, Lineage, Operation Run, plugin registry, storage layout, search/index, job queue, web shell.
- *Astro pack:* FITS/XISF readers, astro kind vocabulary, calibration sets and master matching, sessions, OpenNGC catalog, visibility math, targets and annotations, equipment, acquisitions, plate solving, sky map.

Several of those are genuinely ambiguous. **Equipment** is arguably general (any camera-based hobby has gear) but its spec fields are astro-shaped. **Sessions** are a Collection specialisation — does core know the concept and the pack fill it in, or does the pack define it wholesale? **Visibility math** is pure astronomy but drives generic UI sorting.

**2. Are packs compiled in or loaded at runtime?** The execution profiles separate semantic contracts
from transport. A pack containing a FITS reader is on the hot path for every ingest and may use the
built-in Rust profile without receiving a different semantic contract.

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
- Realistically means shipping nothing useful for a long time, when a working vertical slice is needed early.
- Risks the core being *too* thin — cross-pack concerns (search UI, collection views) end up duplicated per pack.

## Recommendation

Working position: **decide the seam now and compile the first-party astro pack into v2.0.** It uses
the built-in Rust execution profile and implements the same semantic contracts and conformance fixtures.
User-installed script plugins and external providers can extend its schemas and Operators through
explicit grants; dynamically replacing the entire domain pack is deferred.

This keeps FITS/XISF parsing and the initial UI contribution in the shipped artifact without making
astro vocabulary part of core. Whether a later domain pack can be dynamically installed remains
reversible; putting astro fields into core does not.

Specific calls to make explicit:

- Does core know the *concept* "session" (a Collection with a time span and a subject) with the pack supplying vocabulary, or is session entirely pack-defined?
- Equipment: core, pack, or core concept with pack-defined spec facets?
- Can a pack contribute frontend surface, or only API and facets? This gates whether the frontend workstream can proceed independently.

## Decision

The concrete crate skeleton makes this seam physical, so it is
decided now rather than deferred. All four calls below are taken in the
reversible-safe direction — compiled-in → loadable, pack-owned → core, and
API-only → UI-ABI are each additive later, while the reverse would be a
migration.

**1. Seam = Option A.** The first-party astro pack is compiled into v2.0 as the
`packs/astro` crate. It codes against the public `plugin-abi` contract — the same
Source/Operator/Sink traits and registry a third-party pack would use — and never
against `core` internals; that direction is enforced structurally (astro depends on
`plugin-abi` only) and by a dependency-direction lint. The pack uses the
built-in Rust execution profile and implements the same semantic contracts and
conformance fixtures a third party would. Dynamically replacing the whole domain
pack is **deferred, not designed out**: the contract boundary exists today; only the
dynamic loader is absent.

**2. Session = core concept, pack vocabulary.** Core knows the generic notion of a
time-bounded, subject-bearing Collection. The astro pack supplies the "session"
vocabulary and its facet values. Core does not hardcode astronomy, and the pack does
not reinvent Collections.

**3. Equipment = pack-owned.** Every equipment field is astro-shaped, so equipment
lives entirely in the astro pack and core stays domain-free. Promoting a concept to
core later is cheap and additive; pushing astro fields into core now would be a
migration to undo.

**4. Frontend = API + descriptive facet schemas only.** Packs contribute backend
behaviour and facet schemas carrying render metadata (type, unit, label,
filterability, render hint). The single TypeScript frontend renders facets
generically; first-party astro views (sky map, visibility) live in the shared React
app. There is **no dynamic frontend plugin ABI in v2.0**; dynamic UI contribution is
deferred until after cutover. This keeps the frontend an independently-moving React app
and is consistent with the caution against a published dynamic ABI. Adding a
UI-contribution mechanism later is purely additive.
