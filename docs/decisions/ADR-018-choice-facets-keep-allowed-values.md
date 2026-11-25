---
id: adrs-adr018
date: 2026-11-25
status: accepted
title: 'ADR-018: Choice Facets Keep Their Allowed Values with the Schema Version'
description: Architecture decision record (ADR) for the choice value type. A choice facet's schema lists its allowed values; core checks every proposal against them, keeps them in the facet schema row, and a pack must raise the schema version to change them.
---

# ADR-018: Choice Facets Keep Their Allowed Values with the Schema Version

## Context

A facet is a typed fact about an Asset or an Asset version. One pack owns each facet's schema, plugins propose values, and core checks every value against the schema before it stores it. Core keeps one row in `facet_schemas` for each facet name and schema version, never changes it, and refuses a run when the running declaration differs from the kept row.

Some facts take one value from a small, known set, such as an image's format: `PNG` or `JPEG`. A whole-number count cannot hold them. Free text could, but then core could not refuse a misspelled or unsupported value, and a search for one format could miss values a plugin wrote differently.

[ADR-017](ADR-017-facet-value-rows.md) keeps each value in a JSONB column and expected new value types to need no migration, because the schema row only names its type. A choice is the first type that carries parameters of its own, its allowed values, and core needs them to check and read values.

## Decision

Add a choice value type. A choice facet's schema lists its allowed values, in order, and a value is valid only when it equals one of them exactly, letter case included. Core keeps the allowed values in a nullable `allowed_values TEXT[]` column of `facet_schemas`, which is set for choices only. They are part of the kept declaration: adding, removing, renaming or reordering an allowed value needs a new schema version, as a new label or unit does.

## Consequences

- Core refuses any value outside the declared set, so every stored choice is one a person can search for by its exact text.
- A value is always read with the allowed values it was checked against, because the schema row of its version keeps them.
- Supporting a new value, such as a new image format, means a new schema version. Values under the old version stay readable with their own schema row.
- The stored value is also the text a person reads, so changing how a value reads needs a new schema version too.
- `facet_schemas` now holds one type's parameters in a typed column. A later type with parameters of its own needs another migration, which ADR-017 had expected to avoid.
- The public plugin contract gains a value type that holds text, so its facet type and value types no longer implement `Copy`. Packs borrow or move them, and clone one only when they need a second owned value.

## Alternatives Considered

### Alternative 1: Free text
- **Pros**: no allowed values to declare or keep; a new value needs no schema version.
- **Cons**: core cannot refuse an unknown or misspelled value; two producers can write the same fact differently, so a search misses some.
- **Why not**: the set of formats Sidereal supports is known, and core's job is to refuse what does not fit the schema.

### Alternative 2: Allowed values only in the running declaration
- **Pros**: no migration; the schema row stays as ADR-017 described it.
- **Cons**: a value could be read back under allowed values it was never checked against, and a pack could change the list without a new version and without core noticing.
- **Why not**: every other part of a declaration is kept and compared so that a stored value always reads with the schema it was written under. The allowed values belong to that declaration.

### Alternative 3: A general JSONB column for any type's parameters
- **Pros**: a later type with parameters, such as a decimal with a precision, would need no migration.
- **Cons**: the database could no longer check the shape of the parameters, and core would parse JSON to compare declarations.
- **Why not**: no second parameterized type exists yet. A typed column keeps the database's check that only choices carry allowed values.
