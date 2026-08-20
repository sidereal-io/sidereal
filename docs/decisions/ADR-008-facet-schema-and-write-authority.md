# 008: Metadata Envelope, Facets, and Write Authority

**Status:** Accepted
**Date:** 2026-08-01
**Context:** M0 of [RFC #213](https://github.com/sidereal-io/sidereal/issues/213). Selectors need a
portable metadata contract, while domain packs and alternative plugins need interoperable typed values.

## Problem

Sidereal needs a small set of fields shared by every Asset plus an extensible way to carry rich, typed
astrophotography facts. Maintaining both untyped labels and typed facets would create two ways to
represent selectable metadata and invite duplication. The architecture needs one canonical boundary:
which fields every Asset has, what belongs in an extensible facet, where each value is scoped, and who
may write it — without coupling Selectors to whichever plugin produced a value.

(Backstage separates a common entity envelope from kind-specific `spec`/`status`; Sidereal borrows that
split but makes facets its single extension mechanism, so it does not also add labels. See the
[Backstage descriptor format](https://backstage.io/docs/features/software-catalog/descriptor-format/).)

## Options

The envelope-plus-facets shape is not contested; the open choice is **facet ownership**.

### Option A: Producer-owned namespaces

Every producer defines its own values (`astrometry.ra`, `astap.ra`). Avoids grants, but forces every
Selector, query, and UI to understand provider-specific alternatives.

### Option B: Shared declaration by convention

Multiple plugins may declare the same facet if their definitions happen to match. Fails at schema
evolution — load order and superficially similar declarations cannot establish which is canonical.

### Option C: Exclusive schema owner with explicit producer grants

One pack owns each schema; compatible producers request write grants and values retain producer
provenance. Preserves canonical queries while allowing competing implementations.

## Recommendation

Small core-owned envelope plus facets as the single extension mechanism, and **Option C** for ownership —
Option A leaks provider-specific alternatives into every Selector, query, and UI; Option B cannot
establish a canonical definition under schema evolution. The contract:

**Envelope.** Every Asset has exactly three core-owned fields: `id` (stable opaque identity, never
derived from name, path, Source, or external ID), `kind` (one normalized discriminator from a
domain-pack vocabulary, e.g. `astro.light`), and `name` (mutable, non-unique display name, not the
path). `namespace` and a descriptor `apiVersion` are deliberately omitted — migrations and
independently versioned plugin/facet contracts own schema evolution, and no tenancy boundary yet exists
that Sources and Collections cannot express. Paths and external IDs are mutable/scoped state, not
identity.

**Facets** are the only extension metadata. A schema declares type/units/nullability/validation, scope
(Asset / AssetVersion / immutable Collection snapshot), mutability and invalidation, owner and
producer-grant requirements, index/query hints, and version/migration rules. A facet may carry mutable
intent or an observed/derived fact, and the schema keeps those distinct:

| Example | Scope | Semantics |
|---|---|---|
| `core.processing.mode=auto` | Asset, mutable | User/Source processing intent |
| `astro.fits.exptime=300 s` | AssetVersion, observed | Extracted from exact bytes |
| `astro.solve.ra=10.6847°` | AssetVersion, derived | Produced by a solver |
| `astro.session.integration=12 h` | Collection snapshot, derived | Aggregate over fixed membership |

Raw evidence and a normalized decision stay separate facts (`astro.fits.image_type=LIGHT` observed vs.
envelope `kind=astro.light`).

**Selector portability.** Selectors reference schema names, not producer plugins. Built-in policies use
core- or pack-owned schemas; competing producers (Astrometry.net, ASTAP) receive grants to write the
same canonical `astro.solve.*`. A plugin-owned schema stays selectable for deliberate plugin-specific
policies, but the UI exposes the dependency and a missing schema makes the policy `blocked`. Values are
never copied into a second mechanism just to be selectable.

**Write authority.** Core is the only writer of envelope/facet state; plugins propose through a
capability-limited `AssetContext`. One pack owns each `kind` vocabulary and each facet schema — two
plugins cannot declare the same schema, but multiple granted producers may write values under it. A
producer manifest requests access to named facets or a grantable namespace pattern, approved at
install; core validates every value and stores provenance (actor, plugin + version, producing Operation
Run, schema version, observation/mutation time). Core preserves observations rather than overwriting
provenance; domain policy selects the current value. Breaking changes need a new facet version or a
declared migration; removing a schema owner makes it unavailable for new writes without deleting stored
values.

**Mutable-facet audit history (#217 / DataHub) is out of scope for v2.0 / M1.** M1's facets are
write-once observations; a mutable intent facet (`core.processing.mode`) overwrites in place, keeping
only last-write provenance — not a value trail. A full immutable audit trail is deferred post-cutover
and, if adopted, reuses [ADR-003](ADR-003-storage-layout-and-asset-identity.md)'s immutable-version
pattern rather than inventing a second mechanism.

## Decision

Accepted 2026-08-18 (M0 of RFC #213) — **small core-owned envelope plus facets, Option C**, as
recommended.
