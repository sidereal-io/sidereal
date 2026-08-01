# 008: Metadata Envelope, Labels, Facets, and Write Authority

**Status:** Proposed
**Date:** 2026-08-01
**Context:** M0 of [RFC #213](https://github.com/sidereal-io/sidereal/issues/213). Selectors need a
small common classification model, while domain packs and alternative plugins need an interoperable
typed-metadata contract.

## Problem

Sidereal needs both simple selection metadata and rich astrophotography facts. Treating everything as
an untyped label loses units, validation, range queries, revision scope, and producer provenance.
Treating everything as a schema-owned facet makes ordinary user and Source classification expensive
and forces operational intent into domain schemas.

The architecture therefore needs a canonical boundary: which fields every Asset has, what belongs in
a label, what belongs in a facet, where each value is scoped, and who may write it. Without that
boundary, the same fact will drift into several representations and Selectors will become ambiguous.

Backstage usefully separates a common entity envelope (`apiVersion`, `kind`, and common metadata such
as `name`, optional `namespace`, UID, and labels) from kind-specific `spec` and `status`. Sidereal
borrows that separation, not Backstage's name/namespace identity semantics. See the
[Backstage descriptor format](https://backstage.io/docs/features/software-catalog/descriptor-format/).

## Common Asset envelope

Every Asset has a small core-owned envelope:

| Field | Scope and semantics |
|---|---|
| `id` | Stable opaque Asset identity. Names, paths, Sources, and external IDs never determine it. |
| `kind` | One normalized discriminator from a domain pack vocabulary, such as `astro.light`. |
| `name` | Mutable display name. It is not required to be unique and is not the filesystem path. |
| `labels` | Mutable namespaced string key/value map for intent and coarse classification. |

`namespace` is deliberately omitted until Sidereal has a concrete ownership or tenancy boundary that
Sources, Collections, and labels cannot express. Adding it later does not change opaque Asset IDs.
Sidereal also does not need a descriptor `apiVersion`: database migrations and the independently
versioned plugin/facet contracts own schema evolution.

Paths are mutable storage state. External IDs are scoped Source relationships. Neither belongs in the
identity envelope.

## Labels versus facets

| Dimension | Label | Facet |
|---|---|---|
| Purpose | Intent, opt-in/out, and coarse grouping | Observed, extracted, or derived domain facts |
| Type | String key and string value | Schema-defined scalar or structure with units and constraints |
| Typical scope | Stable Asset | Declared by schema; usually AssetVersion or Collection snapshot |
| Ownership | Namespaced key prefix plus write grant | Exclusive schema owner plus producer write grants |
| Provenance | Origin and last mutation retained | Producer, version, Operation Run, schema version, and observation time |
| Queries | Exists, equals, in/not-in | Typed equality, range, structure-aware predicates |
| Examples | `processing.sidereal.io/mode=auto` | `astro.fits.exptime=300 s`, `astro.solve.ra=10.6847°` |

Facts have one canonical home. The system must not mirror a value into a label merely to make it
selectable because Selectors can query typed facets directly. Raw evidence and a normalized decision
are different facts: `astro.fits.image_type=LIGHT` may be retained as an observed facet while the
classifier sets the envelope `kind=astro.light`.

Label cardinality should remain low enough for indexing and UI comprehension. File paths, content
hashes, timestamps, coordinates, and arbitrary extracted headers are facets or core state, not labels.

## Write authority

Core is the only component that persists envelope or facet changes. Plugins propose changes through
their capability-limited `AssetContext`:

- A domain pack owns its `kind` vocabulary. Sources may set a configured default or propose a
  detected kind within their grants.
- Users and Source configuration may write labels. Plugins request write access to explicit label
  key prefixes; origin is retained for audit and selector explanations.
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

Choose **Option C** together with the envelope and label/facet boundary above.

The facet registry stores, per schema:

- fully qualified name, scope, and schema owner;
- scalar or structured type, units, nullability, and validation constraints;
- index and query hints;
- schema version and compatible migration rules;
- producer grant requirements.

A producer manifest requests write access to named facets or an explicitly grantable namespace
pattern. Core approves the request during installation, validates every emitted value, and stores:

- producer plugin and version;
- producing Operation Run;
- schema version;
- observation time where applicable.

Two plugins cannot declare the same schema. Multiple authorised plugins may write values under that
schema. When concurrent or contradictory values are possible, core preserves observations rather
than silently overwriting provenance; domain policy selects the preferred/current value.

Breaking schema changes require a new facet version or a declared migration. Removing a schema owner
does not delete stored values; it makes the schema unavailable for new writes until ownership is
restored or migrated.

## Decision

[Filled in after review.]
