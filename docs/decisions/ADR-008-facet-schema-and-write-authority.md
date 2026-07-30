# 008: Facet Schema and Write Authority

**Status:** Proposed
**Date:** 2026-07-30
**Context:** M0 of [RFC #213](https://github.com/sidereal-io/sidereal/issues/213). Facets are the cross-plugin metadata contract used by search, calibration matching, and future routing rules.

## Problem

The architecture originally made namespace ownership exclusive and allowed plugins to emit only their
own facets. That prevents interoperable alternative implementations: if the astro pack owns
`astro.solve.ra`, an ASTAP Operator cannot publish canonical solve results without either claiming the
same schema or inventing an incompatible namespace.

Schema definition and write authority are different concerns:

- **Schema ownership** decides the canonical name, type, constraints, indexing, and evolution.
- **Write authority** decides which producer implementations may emit values conforming to it.

## Options

### Option A: Producer-owned namespaces

Every producer defines its own values, such as `astrometry.ra` and `astap.ra`.

This avoids grants but forces every query and UI to understand provider-specific alternatives.

### Option B: Shared declaration by convention

Multiple plugins may declare the same facet if their definitions happen to match.

This fails at schema evolution: load order and superficially similar declarations cannot establish
which definition is canonical.

### Option C: Exclusive schema owner with explicit producer grants

One pack owns each schema. Compatible producers request write grants and values retain producer
provenance.

This preserves canonical queries while allowing competing implementations.

## Recommendation

Choose **Option C**.

The registry stores, per facet:

- fully qualified name and schema owner;
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
