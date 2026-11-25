---
id: adrs-adr017
date: 2026-11-23
status: accepted
title: 'ADR-017: Facet Values as Rows with a JSONB Value'
description: Architecture Decision Record (ADR) for how PostgreSQL stores facet values. Each value is one row, with its provenance in typed columns and foreign keys, and the value itself in a JSONB column with a GIN index. This replaces ADR-004's rule to store facets in JSONB columns on the asset rows.
---

# ADR-017: Facet Values as Rows with a JSONB Value

## Context

A facet is a typed, namespaced fact about an Asset or an Asset version, such as a file's size in bytes. Plugins propose facet values, and core checks and stores them. Core is the only writer.

ADR-008 requires each stored value to carry its own provenance: the plugin and its version, the run that produced it, the schema version and the time. It also keeps every observation, so several granted producers can each hold a value for the same facet on the same Asset.

[ADR-004](ADR-004-database-engine-and-schema.md) chose PostgreSQL and said to store facets "directly in JSONB columns utilizing GIN indexes rather than maintaining separate key-value tables". Read literally, that means a `facets` column on `assets` and on `asset_versions`. The first facet showed three problems with that:

- **Version rows are immutable.** An Asset version records exact bytes and never changes. A fact learned later about those bytes would have to rewrite the row.
- **The database cannot check provenance.** A run reference inside a JSON document cannot have a foreign key, so a value could name a run that does not exist.
- **Many values share one document.** Several producers, each writing at a different time, would all update the same row, and would each rewrite the whole document.

## Decision

Store each facet value as one row in a `facet_values` table. The row holds:

- the facet's name and schema version, with a foreign key to that schema version's row;
- exactly one subject: an Asset or an Asset version, with a foreign key;
- the provenance, in typed columns: producer id, producer version, write time, and a foreign key to the operation run, which records the actor who started it;
- the value, in a `JSONB` column with a GIN index.

Keep one schema row for each facet name and schema version, and never change it. A value is then always read with the label, unit and type of the schema version it was written under.

Core checks each value against its facet schema before it stores the row. The database only refuses a JSON `null` value, because a row check cannot read the schema's value type from another table. The schema row's value type is free text that core checks, so the database lists no value types.

This replaces only ADR-004's rule on where facets live. ADR-004's choice of PostgreSQL, sqlx and forward-only migrations still applies.

## Consequences

- PostgreSQL checks what matters for trust: every value has a schema version, a subject and a run that exist. Asset and Asset version rows never change when a fact is learned.
- A pack that changes a facet's label, unit or type must raise the schema version. Core refuses to write under a kept version whose declaration changed.
- Several producers can hold values for one facet on one subject, each with its own provenance, with no shared document to merge.
- A new value type (text, a decimal number, coordinates) needs no migration. Core learns the type, the value column already holds any JSON value, and the schema table does not list types.
- The database does not check the value's type. A bug in core's checks could store a text value under a whole-number facet. Core's tests own that rule, and core is the only writer.
- Reading all of an Asset's facets is a join and many rows, not one column. With an index on each subject column, this stays cheap at the size of one person's library.
- The GIN index serves containment and JSON path matches. A range comparison on one facet, such as "exposure longer than 300 seconds", needs an expression index for that facet. The first feature that needs such a query adds it.

## Alternatives Considered

### Alternative 1: A `facets` JSONB column on `assets` and `asset_versions`
- **Pros**: matches ADR-004's wording; reading an Asset's facets reads one column; one GIN index covers every facet.
- **Cons**: rewrites Asset version rows that must stay as they were recorded; keeps provenance where the database cannot check it; makes every producer update the same document.
- **Why not**: ADR-008 needs each value's run and producer to be checkable, and Asset versions must stay immutable. This shape gives up both.

### Alternative 2: One row per value, with a typed column per value type
- **Pros**: PostgreSQL checks each value's type, for example a whole number of zero or more in a `BIGINT` column; ordinary B-tree indexes serve range comparisons.
- **Cons**: every new value type needs a core migration and widens a check that exactly one value column is set; queries must pick the column from the facet's type.
- **Why not**: value types will grow as packs add facts, and each would mean a schema change in core. The JSONB column keeps the database's checks on provenance, which matter more, and leaves value checks to core, which already checks every proposal.
