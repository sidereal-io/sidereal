---
id: adrs-adr004
date: 2026-08-18
status: accepted
title: 'ADR004: Database Engine and Schema Strategy'
description: Architecture Decision Record (ADR) for the v2 database engine and schema approach — PostgreSQL only, with JSONB+GIN facets and sqlx.
---

# ADR-004: Database Engine and Schema Strategy

## Context

The v2 store must do three things: indexed lookup on semi-structured facet values (calibration-master matching is a facet query, not bespoke columns), recursive traversal for lineage graphs, and — the original hope — no hard dependency on a server database for the single-user installs that are the majority. Both SQLite (JSON1 + recursive CTEs) and PostgreSQL satisfy the first two, so the choice turns on deployment, concurrency under ingest, and maintenance. The v0.10.x codebase carried two hand-written schema files kept in sync by convention, with PostgreSQL parity never runtime-verified and a drifted migration chain — dual-dialect support has a demonstrated cost.

## Decision

**PostgreSQL only.** One server-grade engine gives the strongest facet indexing (JSONB + GIN), the best recursive-CTE performance for lineage, and real concurrency under bursty ingest, and it removes the entire unverified-parity / dialect-divergence / doubled-indexing class of problems. Coupled calls: facets live in **JSONB columns with GIN indexes** (not a key-value side table); the query layer is **`sqlx`** (facet queries are dynamic, arguing against compile-time builders like `diesel`/`SeaORM`; sqlx gives async Postgres with runtime-composed SQL and optional static checking); migrations are **forward-only**, downgrade unsupported and guarded (a newer schema refuses to start against an older binary), one canonical chain replacing the two hand-maintained files. The single-user deployment cost is neutralised at the packaging layer, not the schema: v2 ships an all-in-one image and/or a docker-compose bundle that stands Postgres up beside the app.

## Consequences

- Facet indexing, lineage traversal, and concurrent ingest are all on their strongest footing, with one dialect to test and one migration chain.
- Every self-hoster now runs a database server. "Effectively one command" still hides real operational surface: Postgres bundled inside a single container needs correct shutdown ordering to checkpoint cleanly (a hard container kill risks an unclean stop), while the compose bundle is cleaner but is genuinely two services the user must understand and back up. Backup guidance must cover the Postgres volume and `pg_dump`, not a file copy.
- Existing Postgres and SQLite installs both reach v2 through the one-way importer, not in-place dialect migration.

## Alternatives Considered

### Alternative 1: SQLite only
- **Pros:** one dialect, one migration chain, zero-config deployment matching the single-user reality; adequate for a bursty single-writer workload; removes untested-parity bugs.
- **Cons:** drops a currently advertised capability; caps the multi-user/remote-database future.
- **Why not:** it ceilings facet indexing and concurrency at exactly the workload v2 leans on, and the server-database cost it avoids is recoverable at the packaging layer.

### Alternative 2: SQLite default, PostgreSQL optional (status quo)
- **Pros:** feature parity with v0.10.x; keeps the door open for larger deployments.
- **Cons:** carries forward the exact dual-dialect maintenance burden that produced the unverified-parity problem; doubles facet-indexing work where JSON1 and JSONB/GIN diverge most; needs a real CI matrix against both or the parity claim is again unchecked.
- **Why not:** a rewrite that reintroduces untested dual-dialect support has learned nothing from the debt it is escaping.
