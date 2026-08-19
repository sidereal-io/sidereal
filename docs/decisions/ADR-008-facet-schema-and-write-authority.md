# 008: Metadata Envelope, Facets, and Write Authority

**Status:** Accepted
**Date:** 2026-08-01
**Context:** M0 of [RFC #213](https://github.com/sidereal-io/sidereal/issues/213). Selectors need a
portable metadata contract, while domain packs and alternative plugins need interoperable typed
values.

## Problem

Sidereal needs simple operational classification, rich astrophotography facts, and a small set of
fields shared by every Asset. Maintaining both untyped labels and typed facets creates two ways to
represent selectable metadata, requires rules for choosing between them, and invites duplication.

The architecture therefore needs one canonical boundary: which fields every Asset has, what belongs
in an extensible facet, where each value is scoped, and who may write it. It must also prevent
Selectors from becoming unnecessarily coupled to whichever plugin produced a value.

Backstage usefully separates a common entity envelope (`apiVersion`, `kind`, and common metadata such
as `name`, optional `namespace`, UID, and labels) from kind-specific `spec` and `status`. Sidereal
borrows the envelope/type-specific-data separation, not every field: facets are its single extension
mechanism in M0, so it does not also add labels. See the
[Backstage descriptor format](https://backstage.io/docs/features/software-catalog/descriptor-format/).

## Common Asset envelope

Every Asset has a small core-owned envelope:

| Field | Scope and semantics |
|---|---|
| `id` | Stable opaque Asset identity. Names, paths, Sources, and external IDs never determine it. |
| `kind` | One normalized discriminator from a domain pack vocabulary, such as `astro.light`. |
| `name` | Mutable display name. It is not required to be unique and is not the filesystem path. |

`namespace` is deliberately omitted until Sidereal has a concrete ownership or tenancy boundary that
Sources and Collections cannot express. Adding it later does not change opaque Asset IDs. Sidereal
also does not need a descriptor `apiVersion`: database migrations and independently versioned plugin
and facet contracts own schema evolution.

Paths are mutable storage state. External IDs are scoped Source relationships. Neither belongs in the
identity envelope.

## Facets are the extension metadata

Every additional selectable value is a namespaced facet. A facet schema declares:

- scalar or structured type, units, nullability, and validation constraints;
- scope: Asset, AssetVersion, or immutable Collection snapshot;
- mutability and invalidation rules;
- schema owner and producer grant requirements;
- index and query hints;
- schema version and compatible migration rules.

Facets may express mutable intent as well as observed or derived facts. The schema keeps those uses
precise:

| Example | Scope | Semantics |
|---|---|---|
| `core.processing.mode=auto` | Asset, mutable | User or Source-configured processing intent |
| `astro.fits.exptime=300 s` | AssetVersion, observed | Typed value extracted from exact bytes |
| `astro.solve.ra=10.6847°` | AssetVersion, derived | Typed result produced by a solver |
| `astro.session.integration=12 h` | Collection snapshot, derived | Aggregate over fixed membership |

Raw evidence and a normalized decision remain different facts: `astro.fits.image_type=LIGHT` may be
retained as an observed facet while the classifier sets the envelope `kind=astro.light`.

## Selector portability

Selectors reference facet schema names, not producer plugins. To keep built-in behavior portable:

- schemas used by built-in policies are owned by core or the relevant domain pack;
- alternative plugins receive grants to produce the same canonical schemas;
- plugin-owned schemas remain selectable for deliberate plugin-specific policies, but the UI exposes
  the dependency and a missing schema makes the policy explicitly `blocked`;
- values are never copied into a second metadata mechanism merely to make them selectable.

This means an Astrometry.net and an ASTAP Operator can both satisfy a policy selecting or requiring
`astro.solve.*`; the policy does not depend on either implementation.

## Write authority

Core is the only component that persists envelope or facet changes. Plugins propose changes through
their capability-limited `AssetContext`:

- A domain pack owns its `kind` vocabulary. Sources may set a configured default or propose a
  detected kind within their grants.
- Users and Source configuration may set mutable facets through the same schema validation and audit
  path as plugins.
- One pack owns each facet schema. Compatible producer plugins request grants to write values that
  conform to it.

Schema definition and facet write authority are separate concerns. If the astro pack owns
`astro.solve.ra`, an ASTAP Operator must be able to publish canonical solve results without either
claiming the schema or inventing an incompatible namespace.

## Facet schema options

### Option A: Producer-owned namespaces

Every producer defines its own values, such as `astrometry.ra` and `astap.ra`.

This avoids grants but forces every Selector, query, and UI to understand provider-specific
alternatives.

### Option B: Shared declaration by convention

Multiple plugins may declare the same facet if their definitions happen to match.

This fails at schema evolution: load order and superficially similar declarations cannot establish
which definition is canonical.

### Option C: Exclusive schema owner with explicit producer grants

One pack owns each schema. Compatible producers request write grants and values retain producer
provenance.

This preserves canonical queries while allowing competing implementations.

## Recommendation

Use the small envelope plus facets only, and choose **Option C** for facet ownership.

A producer manifest requests write access to named facets or an explicitly grantable namespace
pattern. Core approves the request during installation, validates every emitted value, and stores:

- producer or configuring actor;
- producer plugin and version where applicable;
- producing Operation Run where applicable;
- schema version;
- observation or mutation time.

Two plugins cannot declare the same schema. Multiple authorised plugins may write values under that
schema. When concurrent or contradictory values are possible, core preserves observations rather
than silently overwriting provenance; domain policy selects the preferred/current value.

Breaking schema changes require a new facet version or a declared migration. Removing a schema owner
does not delete stored values; it makes the schema unavailable for new writes until ownership is
restored or migrated.

## Open question: audit history for mutable facets (input from #217 / DataHub comparison)

DataHub versions *every* aspect update immutably (v0 latest + a positive-number audit trail). Our facet
values carry producer/version/time provenance but keep no value **history**: a new value for a mutable
*intent* facet (e.g. `core.processing.mode`) overwrites the prior value. For observation facets this is
fine (they are effectively write-once). For mutable intent facets, an audit trail ("was `auto`, set to
`manual` by user X at T") may be worth keeping. This is **not** required before cutover and is out of
scope for M1 (whose facets are write-once observations); flagged here so the Decision can state whether
mutable-facet history is in or out, and if in, whether it reuses the immutable-version pattern.

## Decision

Accepted 2026-08-18 (M0 of RFC #213). Adopt the **small core-owned envelope plus facets as the single
extension mechanism**, and **Option C — exclusive schema owner with explicit producer grants** — as
detailed in the sections above (envelope, facets, selector portability, write authority). Option A
(producer-owned namespaces) is rejected because it forces every Selector, query, and UI to understand
provider-specific alternatives; Option B (shared declaration by convention) is rejected because it
cannot establish a canonical definition under schema evolution.

**Resolving the #217 audit-history open question:** mutable-facet value **history is out of scope for
v2.0 / M1.** M1's facets are write-once observations, and a mutable *intent* facet (e.g.
`core.processing.mode`) overwrites in place, keeping only last-write provenance (actor, plugin, version,
run, schema version, time) — not a value trail. A full immutable audit trail for mutable intent facets
is deferred to **post-cutover**; if adopted then, it reuses the immutable-version pattern from
[ADR-003](ADR-003-storage-layout-and-asset-identity.md) rather than inventing a second mechanism.
