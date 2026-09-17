---
id: adrs-adr008
date: 2026-08-18
status: accepted
title: 'ADR008: Metadata Envelope, Facets, and Write Authority'
description: Architecture Decision Record (ADR) for the v2 metadata contract — a small core envelope plus namespaced facets, each schema owned by one pack with explicit producer grants.
---

# ADR-008: Metadata Envelope, Facets, and Write Authority

## Context

Sidereal needs a small set of fields shared by every Asset, plus an extensible way to carry rich, typed astrophotography facts. It must do this without two competing ways to represent selectable metadata.

The architecture needs one canonical boundary: which fields every Asset has, what belongs in a facet, where each value is scoped, and who may write it. It must also keep Selectors from coupling to whichever plugin produced a value.

(Backstage separates a common entity envelope from kind-specific spec/status. Sidereal borrows that split but makes facets its single extension mechanism.)

The envelope-plus-facets shape is not contested. The open choice is facet ownership.

## Decision

Use a **small core-owned envelope plus facets as the single extension mechanism, with exclusive schema ownership and explicit producer grants.**

**Envelope.** Every Asset has exactly three fields: `id` (stable opaque identity), `kind` (one normalized discriminator from a pack vocabulary, e.g. `astro.light`), and `name` (a mutable, non-unique display name). We omit `namespace` and a descriptor `apiVersion`. Paths and external IDs are scoped state, not identity.

**Facets** are the only other selectable metadata. A facet schema declares its type, units, nullability, and validation; its scope (Asset, AssetVersion, or immutable Collection snapshot); its mutability and invalidation; its owner and producer-grant rules; its index hints; and its version and migration rules. This keeps three kinds of fact distinct: mutable intent (`core.processing.mode=auto`), observed (`astro.fits.exptime`), and derived (`astro.solve.ra`).

**Ownership and writes.** Selectors reference schema names, not producer plugins. One pack owns each schema, and two plugins cannot declare the same one — but multiple granted producers may write values under it (Astrometry.net and ASTAP both writing canonical `astro.solve.*`). Values are never copied into a second mechanism to become selectable. Core is the only writer of envelope and facet state; plugins propose through `AssetContext`, and core validates every value and stores provenance (actor, plugin and version, producing run, schema version, and time).

**Out of scope for v2.0:** the value *history* of mutable facets. v2.0 keeps write-once observations plus last-write provenance on mutable intent facets. A full audit trail is deferred; if adopted, it reuses the immutable-version pattern from [ADR-003 — Asset identity & content revisions](ADR-003-asset-identity-and-content-revisions.md).

## Consequences

- Built-in policies and queries stay portable across competing producer plugins. Canonical schemas survive provider substitution, and removing a schema owner blocks new writes without deleting stored values.
- Every producer needs an install-time grant, and breaking a schema requires a new facet version or a declared migration.
- When multiple granted producers write the same schema, core "preserves observations rather than overwriting" and "domain policy selects the current value" — but that selection policy is unspecified. A Selector that predicates on a multi-valued facet (e.g. `astro.solve.ra`) has no defined single value to test until the current-value resolution rule (latest? highest-confidence? provider preference?) is pinned down. Until then, selector results over contested facets are ambiguous.

## Alternatives Considered

### Alternative 1: Producer-owned namespaces (each producer defines its own values)
- **Pros:** no grants needed; producers are fully independent.
- **Cons:** forces every Selector, query, and UI to understand provider-specific alternatives (`astrometry.ra` vs `astap.ra`).
- **Why not:** it pushes provider knowledge into every consumer, destroying the portability the whole model exists to provide.

### Alternative 2: Shared declaration by convention (any plugin may declare a matching facet)
- **Pros:** no ownership bookkeeping when definitions happen to agree.
- **Cons:** fails at schema evolution — load order and superficially similar declarations cannot establish which definition is canonical.
- **Why not:** it has no answer for who owns a schema when two declarations diverge, which is exactly when ownership matters.
