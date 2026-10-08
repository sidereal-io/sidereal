## Context

See `proposal.md` for the outcome and scope. Core currently exposes only an ABI version.
The server builds a stateless axum router and reads `PORT` without database configuration.
Its integration test exercises `/healthz` without binding a socket.
The root server gate runs formatting, clippy, tests, and architecture checks. CI currently provides no database service.

## Goals / Non-Goals

**Goals:** Keep database policy in core and HTTP wiring in the server. Prove startup safety against real PostgreSQL.

**Non-Goals:** No generic persistence framework, database creation API, schema repair, or migration downgrade command.
UI tokens added: none.

## Decisions

### Configuration and supported server

The server parses required `DATABASE_URL` into PostgreSQL connection options before passing them to core.
Core checks `server_version_num` and accepts PostgreSQL 18, including patch releases.
Supporting one major version initially keeps the compatibility claim equal to the tested scope.
Broader version support needs separate tests rather than an untested minimum-version promise.
Existing `PORT` configuration remains available.

Use sqlx with PostgreSQL, Tokio, migrations, and TLS support. Embed migrations in the binary.
Track migration files in build inputs so SQL edits trigger recompilation.
TLS options follow the connection configuration; remote connections must not silently fall back from verified TLS.
The local fixture explicitly uses an unencrypted loopback connection.

Map configuration and database failures into fixed categories and operator actions.
Never display connection options, database-supplied messages, or raw sqlx errors.
This avoids relying on password substitution, which can miss escaped secrets.

### Validate before writing

Acquire a database-scoped PostgreSQL advisory lock on a dedicated connection before inspecting application objects.
Use a fixed Sidereal lock key shared by all server instances.
Keep this connection and lock through validation, migration execution, and final verification.
Retain sqlx migration locking as well; all Sidereal startup paths use the outer lock first.
Discard the connection on cancellation or failure so session locks cannot remain inside the pool.

Read catalogs before calling the sqlx migrator, which can create its bookkeeping table.
An empty database contains only PostgreSQL system objects and an empty default `public` schema.
Reject custom schemas and application tables, views, functions, types, sequences, and extensions outside the expected manifest.
Ignore built-in objects supplied by PostgreSQL's standard empty database template.

The initial transaction creates a singleton `sidereal_metadata` identity row with product value `sidereal` and format version 1.
Sqlx supplies `_sqlx_migrations` bookkeeping. This is the complete initial application schema.
Core validates both table shapes, constraints, indexes, identity, and migration records on later startup.
Expected catalog signatures belong beside the migration source and cover each supported migration prefix.
Parameterized queries carry values; qualified, fixed identifiers prevent `search_path` from selecting foreign objects.

Only successful records forming an exact prefix of bundled migrations are compatible.
Unknown versions, checksum differences, gaps, failed records, and unexpected catalog objects fail before writes.
A missing identity row alongside migration bookkeeping is invalid, including an otherwise empty bookkeeping table.
For an empty database, core owns one bootstrap transaction using the bundled sqlx migration text and checksum.
It creates sqlx-compatible bookkeeping, executes the initial SQL, and inserts the successful record before committing.
Do not call the default migrator before bootstrap; its separate bookkeeping creation could leave an unowned table after interruption.
Verify the bookkeeping format against the pinned sqlx release through a repeated-startup test using its migrator.
Later startup uses the sqlx migrator only after ownership validation succeeds.
A crash either commits the complete initialized state or rolls it back.
Future migrations must remain transactional under this pipeline. Nontransactional migrations require a separate design change.

Using sqlx validation alone was rejected because it does not establish ownership before creating bookkeeping.
Sharing a database or adopting arbitrary existing objects was rejected because Sam agreed to a dedicated database.
The startup connection requires schema privileges, not PostgreSQL superuser access.

### Startup and health checks

Startup follows configuration, connection, lock, read-only validation, migrations, verification, pool creation, and HTTP binding.
Connection acquisition has a 30-second deadline. Lock acquisition also has a 30-second deadline with a distinct safe error.
Set database-side lock and statement timeouts for migration operations; initially use 30 seconds per statement.
Startup failure closes database connections and exits nonzero without binding HTTP.

`/healthz` keeps its existing body and never queries PostgreSQL.
`/readyz` performs a read-only query through the pool under one two-second deadline.
The query checks identity and complete expected migration history, including checksums and success flags.
It also checks the expected catalog signatures. This detects incompatible changes while the process remains alive.
Pool acquisition and query execution share the deadline. No connection details appear in the response.
Successful probes recover automatically after a database outage.
Graceful shutdown closes the pool; migrations never run from HTTP handlers.

Keeping HTTP unbound until initialization succeeds avoids exposing a partially initialized server.
Testing only `SELECT 1` was rejected because connectivity alone does not prove schema compatibility.

### Fixture, checks, and demonstration

Add database-only fixture configuration under `server/tests/fixtures/` using a pinned PostgreSQL 18 image.
It binds a configurable loopback port and uses a named volume with a worktree-specific project name.
Root `db-up` and `db-down` recipes manage that fixture without deleting its volume.
`db-clean` explicitly deletes only this fixture's named volume.
The fixture is test infrastructure; it does not package Sidereal or define another installation path.

Real database tests use isolated temporary databases under a test-only PostgreSQL role with `CREATEDB`.
`TEST_DATABASE_URL` configures that role. Tests never use the runtime `DATABASE_URL` as an implicit fallback.
Missing test configuration fails with instructions to start the fixture. Tests never silently skip.
CI supplies the same PostgreSQL major version to `just check-server` through a service container.

`db-demo` builds the server once and uses a separate database inside the persistent fixture.
It saves an ordered snapshot of every migration-record field before restarting the server and PostgreSQL.
It compares snapshots after each restart and polls readiness with a bounded wait.
The script tracks its server processes and cleans them up on failure or interruption.
Scratch snapshots live under `.workspace/`. The demo never accepts an arbitrary production database URL.
Server documentation explains runtime configuration, test prerequisites, fixture cleanup, and the demonstration.

### Security scope

This story makes no auth prerequisite: the user confirmed that database work proceeds independently.
It adds only unauthenticated liveness and readiness probes. These return fixed status bodies and permit no HTTP mutations.
Database migrations run locally during process startup, under the configured database role.
Auth epic #388 owns clarification of ADR-007's broad implementation wording and the later access controls.
This change does not amend ADR-007 or add authentication, plugins, providers, or asset access.

## Risks / Trade-offs

- Strict ownership checks reject custom database additions. Document the dedicated-database requirement and test unrelated contents explicitly.
- Another operator can change schemas outside Sidereal's lock. Require exclusive application ownership; the lock coordinates Sidereal startups only.
- PostgreSQL 18-only support excludes other majors. Publish this limit and widen support only with real version coverage.
- Database tests add an infrastructure prerequisite. Supply a root fixture recipe and the same service in CI.
- Forward-only migration prevents automatic rollback. Older binaries reject newer history; operators retain data and use a compatible binary.

## Migration Plan

Prepare an empty dedicated database and set `DATABASE_URL`. Start the server and check `/readyz`.
Run the retained-state demonstration before consuming this foundation in packaging work.
There is no import from v0 databases and no downgrade path.
