---
id: adrs-adr002
date: 2026-08-18
status: accepted
title: 'ADR002: Core / Domain-Pack Seam'
description: Architecture Decision Record (ADR) for where the domain-agnostic core ends and the astro domain pack begins, and how packs are delivered.
---

# ADR-002: Core / Domain-Pack Seam

## Context

`kind` must not be a Rust enum containing `light | dark | flat` — astro vocabulary belongs to a pack, not core. Two questions remain: exactly where the seam falls (equipment, sessions, and visibility math are genuinely ambiguous between "general" and "astro-shaped"), and whether packs are compiled in or loaded at runtime. This builds on the execution profiles in [ADR-001 — Plugin contract & execution profiles](ADR-001-plugin-boundary.md): a pack with a FITS reader is on the hot ingest path and can use the built-in Rust profile without a different semantic contract. The concrete crate skeleton makes the seam physical, so it is decided now rather than deferred, and every call is taken in the reversible-safe direction (compiled-in → loadable, pack-owned → core, and API-only → UI-ABI are each additive later; the reverse would be a migration).

## Decision

Compile the first-party **astro pack into v2.0 as a crate that codes against the public plugin contract** — the same Source/Operator/Sink traits and registry a third party uses — never against core internals, enforced structurally and by a dependency-direction lint. Four specific calls:

1. **Seam = compiled-in pack against the public contract.** Dynamically replacing the whole domain pack is deferred, not designed out: the boundary exists today; only the dynamic loader is absent.
2. **Session = core concept, pack vocabulary.** Core knows the generic time-bounded, subject-bearing Collection; the astro pack supplies the "session" term and its facet values.
3. **Equipment = pack-owned.** Every equipment field is astro-shaped, so equipment lives entirely in the pack and core stays domain-free.
4. **Frontend = API + descriptive facet schemas only.** Packs ship backend behaviour and facet schemas carrying render metadata (type, unit, label, filterability, render hint); the single React app renders facets generically, with first-party astro views (sky map, visibility) in that app. There is no dynamic frontend plugin ABI in v2.0.

## Consequences

- FITS/XISF parsing and the initial UI contribution ship in one artifact with no boundary cost on the hot path, without astro vocabulary entering core.
- Promoting a concept to core, adding a dynamic pack loader, or adding a UI-contribution mechanism are all additive later; none is foreclosed.
- The pack contract is proven by exactly one in-tree consumer, so its genericity is unproven — the first third-party or general-media pack will likely surface abstractions the astro pack never needed, and the plugin contract will version in response.

## Alternatives Considered

### Alternative 1: Packs are ordinary loadable plugins from day one
- **Pros:** the seam is proven by construction; a future general-media pack needs no core change; users install only what they use.
- **Cons:** a dynamically loaded pack on the script or external-provider profile adds boundary cost on every ingest (significant for large FITS); packs must contribute UI, a much larger contract; more moving parts in the default install.
- **Why not:** it pays dynamic-loading and UI-ABI cost up front for a swap-the-pack capability nothing needs yet; the compiled-in seam keeps that reversible.

### Alternative 2: Thin core with facets only; everything else a pack
- **Pros:** maximally clean seam; core is small and genuinely domain-free.
- **Cons:** ships nothing useful for a long time when an early working vertical slice is required; risks a core so thin that cross-pack concerns (search UI, collection views) duplicate per pack.
- **Why not:** it trades delivery and cohesion for purity the compiled-in seam already achieves structurally.
