# 002: Core / Domain-Pack Seam

**Status:** Accepted · **Date:** 2026-07-29

**Context:** Core is domain-agnostic and the astrophotography pack is the first domain pack — that split is settled. This ADR fixes one thing: how the first-party astro pack is *delivered*, which in turn fixes where the seam physically falls. It builds on the plugin execution profiles in [ADR-001 — Plugin contract & execution profiles](ADR-001-plugin-boundary.md).

## Problem

The astro pack has to reach users somehow, and that choice determines how real the core/pack seam is. A pack compiled into the binary proves the seam only by convention; a pack loaded at runtime proves it by construction — but pays a boundary cost on every ingest and needs a much larger contract, because a pack contributes UI and schemas, not just backend behaviour like an Operator plugin.

This matters because the astro pack sits on the hot path. Its FITS/XISF readers run for every ingest, and large FITS files make any per-ingest boundary crossing expensive. The execution profiles from ADR-001 separate semantic contracts from transport: a pack can use the built-in Rust profile and cross no dynamic boundary, or a script/external-provider profile and cross one on every asset.

So the question is not *whether* astro vocabulary lives in a pack — it does — but whether that pack is **compiled in, loaded at runtime, or reduced to almost nothing** so the seam is trivially clean. Each answer draws the physical seam in a different place.

## Options

### Option A: Astro pack compiled in, other plugins loaded

The astro pack is a first-party crate compiled into the binary, coded against the public pack contract but shipped as part of the single artifact. Third-party Operators and Sources still load at runtime.

**Pros:**
- No boundary cost on the hot ingest path (FITS parsing, facet extraction).
- Single-binary distribution; the shipped product is one artifact.
- Pack still coded against the public interface, so the seam is real even if delivery isn't dynamic.

**Cons:**
- The pack interface is only proven by one in-tree consumer, so it can drift the way the plugin ABI would without dogfooding.
- "Swap the astro pack for a photo pack" stays theoretical.

### Option B: Packs are ordinary loadable plugins

The astro pack is a dynamically loaded plugin like any other — installed, not compiled in.

**Pros:**
- Seam proven by construction — if the astro pack can be loaded, so can a third-party pack.
- A future general-media pack needs no core change.
- Users install only what they use.

**Cons:**
- A dynamically loaded pack using the script or external-provider profile adds boundary cost on every ingest, which may be significant for large FITS files.
- Packs need to contribute UI, not just backend behaviour — a much larger contract than an Operator plugin.
- More moving parts in the default install.

### Option C: Thin core with facets only; everything else a pack

Core shrinks to facet storage and query; every capability, including cross-pack UI concerns, ships as a pack.

**Pros:**
- Maximally clean seam; core is small and genuinely domain-free.

**Cons:**
- Realistically means shipping nothing useful for a long time, when a working vertical slice is needed early.
- Risks the core being *too* thin — cross-pack concerns (search UI, collection views) end up duplicated per pack.

## Recommendation

**Option A.** The seam only has to be *real*, not *dynamic*, to keep astro vocabulary out of core — and A gets that for free while B pays a per-ingest boundary cost exactly where it hurts most (large FITS files) and C ships nothing useful for too long. Coding the pack against the public contract, not `core` internals, makes the boundary genuine; whether a pack can later be *loaded* rather than compiled in stays reversible, because compiled-in → loadable is additive. Putting astro fields into core would not be reversible, so A is the safe direction.

## Decision

Accepted 2026-07-29 — **Option A, as recommended.** The astro pack is the `packs/astro` crate, compiled into v2.0. It codes against the public `plugin-abi` contract — the same Source/Operator/Sink traits and registry a third-party pack would use — never against `core` internals, enforced structurally (astro depends on `plugin-abi` only) and by a dependency-direction lint. It uses the built-in Rust execution profile and implements the same semantic contracts and conformance fixtures a third party would. Dynamically replacing the whole domain pack is **deferred, not designed out**: the contract boundary exists today; only the dynamic loader is absent.

The seam being physical now settles three boundary questions in the same reversible-safe direction (each additive to reverse, so none needs its own ADR — they are recorded as design facts in [the architecture overview](../architecture/README.md#design-facts)):

- **Session** is a core concept with pack vocabulary — core knows the generic time-bounded, subject-bearing Collection; the astro pack supplies the "session" term and its facet values.
- **Equipment** is pack-owned — every equipment field is astro-shaped, so it lives in the astro pack and core stays domain-free.
- **Frontend contribution** is API + descriptive facet schemas only — packs contribute backend behaviour and facet schemas carrying render metadata (type, unit, label, filterability, render hint); the single React app renders facets generically, with first-party astro views (sky map, visibility) living in that app. There is no dynamic frontend plugin ABI in v2.0.
