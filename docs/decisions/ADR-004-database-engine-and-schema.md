# 004: Database Engine and Schema Strategy

**Status:** Proposed
**Date:** 2026-07-29
**Context:** M0 of [RFC #213](https://github.com/sidereal-io/sidereal/issues/213). The [facet mechanism](../architecture/README.md#the-mechanism-metadata-facets) imposes concrete requirements on the store; this ADR picks the engine and schema approach that satisfy them.

## Problem

v2's storage layer must support three things the architecture depends on:

1. **Indexed lookup on semi-structured facet values** — calibration-master matching is a facet query (`astro.fits.ccd_temp`, `gain`, `exptime`, `binning`), not bespoke columns.
2. **Recursive traversal** for lineage graphs — "everything transitively derived from this master."
3. **No hard dependency on a server database** for single-user installs, which are the overwhelming majority.

Both SQLite (JSON1 + recursive CTEs) and PostgreSQL satisfy all three, so the architecture does not force the choice. This ADR decides on other grounds: deployment simplicity, concurrency under ingest, and migration tooling.

Secondary but coupled: v0.10.x maintains two hand-written schema files (`sqlite-schema.ts`, `pg-schema.ts`) that must be kept in sync by convention, and the analysis package flags that **PostgreSQL parity was never runtime-verified** (`Q1`) and that the migration snapshot chain has drifted. Dual-dialect support has a real, demonstrated maintenance cost.

## Options

### Option A: SQLite-only

**Pros:**
- One dialect, one schema, one migration chain, one set of tests that actually run.
- Zero-configuration deployment; matches the self-hosted single-user reality.
- Adequate for the workload — ingest is bursty but single-writer, and WAL mode handles concurrent reads.
- Removes an entire class of untested-parity bugs.

**Cons:**
- Drops a currently advertised capability; existing Postgres users need a migration path.
- Caps the multi-user/remote-database future, which is unfunded but not formally excluded.

### Option B: SQLite default, PostgreSQL optional (status quo)

**Pros:**
- Feature parity with v0.10.x; no capability regression.
- Keeps the door open for larger deployments.

**Cons:**
- Carries forward the exact maintenance burden that produced `Q1`. A rewrite that reintroduces untested dual-dialect support has learned nothing.
- Doubles facet-indexing work, where the dialects diverge most (JSON1 vs. JSONB and GIN).
- Requires a real CI matrix running the full vector set against both, or the parity claim is again unverified.

### Option C: PostgreSQL-only

**Pros:**
- Strongest facet indexing (JSONB + GIN), best recursive-CTE performance, real concurrency.
- One dialect.

**Cons:**
- Forces every single-user self-hoster to run a database server. Directly against the deployment story that makes Sidereal easy to adopt.

## Recommendation

The RFC records a leaning toward **Option B** (keep SQLite-default / Postgres-optional).

Worth pushing back on during review: **Option A is the honest reading of the evidence.** Postgres support in v0.10.x is unverified at runtime, and the failure mode of unverified support is silent divergence rather than a clean error. If Postgres is kept, the decision should come with a committed CI matrix — otherwise the option costs maintenance and delivers a claim nobody has checked.

Also decide here:

- **ORM / query-layer choice** (`sqlx`, `SeaORM`, `diesel`, or hand-rolled). Facet queries are dynamic, which weighs against heavily-typed compile-time query builders.
- **Migration tooling and forward-only policy.** v0.10.x migrations are forward-only with undefined downgrade behaviour (`U2`); v2 should either support downgrade or document it as unsupported and guard against it.
- **Whether facets live in JSON columns or a key-value side table.** The side table indexes and joins predictably; the JSON column reads better and leans on dialect-specific features — which is only safe once the dialect question above is settled.

## Decision

[Filled in after review.]
