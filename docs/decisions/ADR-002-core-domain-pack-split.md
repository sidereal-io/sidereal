---
id: adrs-adr002
date: 2026-08-18
status: accepted
title: 'ADR002: Core / Domain-Pack Seam'
description: Architecture Decision Record (ADR) for where the domain-agnostic core ends and the astro domain pack begins, and how packs are delivered.
---

# ADR-002: Core / Domain-Pack Seam

## Context

Domain-specific terms such as astronomy vocabulary belong in dedicated packs rather than core logic; for instance, `kind` should avoid hardcoding a Rust enum of `light | dark | flat`. Consequently, two primary structural decisions must be addressed:

- Defining the exact boundary seam where ambiguous concepts—such as equipment, sessions, and visibility calculations—are split between domain packs and core.
- Determining if domain packs ought to be compiled directly into the binary or loaded dynamically at runtime.

This approach expands upon the execution strategies outlined in  [ADR-001 — Plugin contract & execution profiles](ADR-001-plugin-boundary.md). For example, a pack equipped with a FITS reader operates along the critical ingest path and can leverage the native built-in Rust execution profile without altering its underlying semantic contract.

Establishing a concrete crate structure gives physical form to this boundary, requiring an immediate decision rather than postponement. We consistently opt for reversible and additive choices: compiled-in components can later transition to runtime loading, pack-level features can migrate into core, and API-only models can subsequently receive a UI ABI—whereas reversing these choices later would necessitate complex migrations.

## Decision

The **first-party astro pack will be compiled directly into sidereal as a crate using the public plugin contract** — leveraging the identical Source, Operator, and Sink traits and registry available to third parties, avoiding direct reliance on core internals. Structural design and dependency-direction lints will strictly enforce this isolation. This decision rests on four key architectural choices:

1. **Seam: Static compilation against public traits**. Dynamic replacement of the entire domain pack is postponed rather than ruled out; the architectural boundary is established now, with only the dynamic loading mechanism left for future implementation.
2. **Session: Core abstraction, pack-provided terminology**. The core system defines a generic, time-bounded, subject-bearing Collection. Specific terminology ("session") and facet definitions are contributed by the astro pack.
3. **Equipment: Managed entirely within the pack**. Because equipment metadata is tailored to astronomy, equipment models reside strictly within the pack to keep core domain-agnostic.
4. **Frontend: API and declarative facet schemas**. Domain packs supply backend logic along with facet schemas containing rendering metadata (such as type, unit, display label, filterability, and render hints). A unified React application renders these facets generically, while specialized first-party astronomy views (e.g., sky maps, visibility graphs) are bundled directly into the application, foregoing a dynamic frontend plugin ABI.

## Consequences

- FITS/XISF parsing and the initial UI contribution ship in one artifact, with no boundary cost on the hot path and no astro vocabulary in core.
- Promoting a concept to core, adding a dynamic pack loader, and adding a UI-contribution mechanism are all additive later. None is foreclosed.
- Exactly one in-tree consumer proves the pack contract, so its genericity is unproven. The first third-party or general-media pack will likely surface abstractions the astro pack never needed, and the plugin contract will version in response.

## Alternatives Considered

### Alternative 1: Packs are ordinary loadable plugins from day one
- **Pros:** the seam is proven by construction; a future general-media pack needs no core change; users install only what they use.
- **Cons:** a dynamically loaded pack on the script or external-provider profile adds boundary cost on every ingest — significant for large FITS. Packs must also contribute UI, a much larger contract, and the default install has more moving parts.
- **Why not:** it pays the dynamic-loading and UI-ABI cost up front for a swap-the-pack capability nothing needs yet. The compiled-in seam keeps that reversible.

### Alternative 2: Thin core with facets only; everything else a pack
- **Pros:** maximally clean seam; core is small and genuinely domain-free.
- **Cons:** it ships nothing useful for a long time, when an early working vertical slice is required. It also risks a core so thin that cross-pack concerns (search UI, collection views) get duplicated per pack.
- **Why not:** it trades delivery and cohesion for a purity the compiled-in seam already achieves structurally.
