---
id: adrs-adr004
date: 2026-08-18
status: accepted
title: 'ADR004: Database Engine and Schema Strategy'
description: Architecture Decision Record (ADR) for the v2 database engine and schema approach — PostgreSQL only, with JSONB+GIN facets and sqlx.
---

# ADR-004: Database Engine and Schema Strategy

## Context

The v2 store must do three things:

1. Look up semi-structured facet values by index. Calibration-master matching is a facet query, not a set of bespoke columns.
2. Traverse lineage graphs recursively.
3. The original hope: avoid a hard dependency on a server database for single-user installs, which are the majority.

Both SQLite (JSON1 + recursive CTEs) and PostgreSQL satisfy the first two, so the choice turns on deployment, concurrency under ingest, and maintenance.

The v0.10.x codebase carried two hand-written schema files kept in sync by convention. PostgreSQL parity was never verified at runtime, and the migration chain had drifted. Dual-dialect support has a demonstrated cost.

## Decision

**PostgreSQL only.** One server-grade engine gives the strongest facet indexing (JSONB + GIN), the best recursive-CTE performance for lineage, and real concurrency under bursty ingest. It also removes the entire class of unverified-parity, dialect-divergence, and doubled-indexing problems. Three coupled calls follow:

- **Facets live in JSONB columns with GIN indexes,** not in a key-value side table.
- **The query layer is `sqlx`.** Facet queries are dynamic, which argues against compile-time builders like `diesel` or `SeaORM`; sqlx gives async Postgres with runtime-composed SQL and optional static checking.
- **Migrations are forward-only.** Downgrade is unsupported and guarded — a newer schema refuses to start against an older binary. One canonical chain replaces the two hand-maintained files.

The single-user deployment cost is neutralised at the packaging layer, not the schema. v2 ships an all-in-one image and/or a docker-compose bundle that stands Postgres up beside the app.

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
