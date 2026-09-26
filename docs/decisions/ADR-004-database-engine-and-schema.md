---
id: adrs-adr004
date: 2026-08-18
status: accepted
title: 'ADR004: Database Engine and Schema Strategy'
description: Architecture Decision Record (ADR) for the v2 database engine and schema approach — PostgreSQL only, with JSONB+GIN facets and sqlx.
---

# ADR-004: Database Engine and Schema Strategy

## Context

The sidereal datastore has three core requirements:

- Efficient indexed lookups of semi-structured facet data (e.g., treating calibration-master matching as a generic facet query rather than dedicated columns).
- Recursive traversal of lineage graphs.
- An initial goal to avoid requiring a full database server for single-user deployments, which constitute most installations.

While both SQLite (leveraging JSON1 and recursive CTEs) and PostgreSQL fulfill the functional requirements for facet queries and lineage traversal, selecting an engine comes down to concurrency during ingest, operational maintenance, and deployment complexity.

Maintaining two separate hand-written schemas in v0.10.x introduced clear operational friction: migration paths diverged, and PostgreSQL runtime parity went unverified. Experience demonstrates that supporting multiple database dialects imposes a significant ongoing overhead.

## Decision

**Standardizing on PostgreSQL**. Utilizing a single server-grade database engine delivers optimal JSONB and GIN facet indexing, peak recursive-CTE lineage performance, and robust concurrency during heavy ingest operations. This strategy eliminates dialect fragmentation, unverified runtime parity, and redundant indexing overhead. Key implementation directives include:

- **JSONB Column Storage with GIN Indexing**: Store facets directly in JSONB columns utilizing GIN indexes rather than maintaining separate key-value tables.
- **Query Abstraction via `sqlx`**: Dynamic facet querying makes static ORM builders such as `diesel` or `SeaORM` impractical. Adopting `sqlx` enables asynchronous PostgreSQL access using runtime-built SQL combined with optional compile-time checking.
- **Strict Forward-Only Migrations**: Schema downgrades are disabled and blocked; updated schemas will refuse execution against legacy application binaries. A unified migration pipeline replaces separate schema definitions.

Deployment overhead for single-tenant setups is addressed via container orchestration rather than database architecture. Release v2 provides a unified single image or a pre-configured Docker Compose setup running PostgreSQL alongside the main application.
gle-user deployment cost is neutralised at the packaging layer, not the schema. v2 ships an all-in-one image and/or a docker-compose bundle that stands Postgres up beside the app.

## Consequences

- Facet indexing, lineage traversal, and concurrent ingest are all on their strongest footing, with one dialect to test and one migration chain.
- Every self-hoster now runs a database server. "Effectively one command" still hides real operational surface. Postgres bundled inside a single container needs correct shutdown ordering to checkpoint cleanly, since a hard container kill risks an unclean stop. The compose bundle is cleaner, but it is genuinely two services the user must understand and back up. Backup guidance must cover the Postgres volume and `pg_dump`, not a file copy.
- Existing Postgres and SQLite installs both reach v2 through the one-way importer, not through in-place dialect migration.

## Alternatives Considered

### Alternative 1: SQLite only
- **Pros:** one dialect, one migration chain, and zero-config deployment that matches the single-user reality; adequate for a bursty single-writer workload; removes untested-parity bugs.
- **Cons:** drops a currently advertised capability; caps the multi-user and remote-database future.
- **Why not:** it ceilings facet indexing and concurrency at exactly the workload v2 leans on, and the server-database cost it avoids is recoverable at the packaging layer.

### Alternative 2: SQLite default, PostgreSQL optional (status quo)
- **Pros:** feature parity with v0.10.x; keeps the door open for larger deployments.
- **Cons:** carries forward the exact dual-dialect maintenance burden that produced the unverified-parity problem; doubles facet-indexing work where JSON1 and JSONB/GIN diverge most; needs a real CI matrix against both engines, or the parity claim is again unchecked.
- **Why not:** a rewrite that reintroduces untested dual-dialect support has learned nothing from the debt it is escaping.
