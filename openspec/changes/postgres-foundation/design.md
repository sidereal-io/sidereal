## Context

See `proposal.md` for the outcome and scope. Core currently exposes only an ABI version.
The server builds a stateless axum router and reads `PORT` without database configuration.
Its integration test exercises `/healthz` without binding a socket.
The root server gate runs formatting, clippy, tests, and architecture checks. CI currently provides no database fixture.

## Goals / Non-Goals

**Goals:** Keep database policy in core and HTTP wiring in the server. Prove startup safety against real PostgreSQL.

**Non-Goals:** No generic persistence framework, database creation API, schema repair, or migration downgrade command.
UI tokens added: none.

## Decisions

### Configuration and transport

The server parses required `DATABASE_URL` into PostgreSQL connection options before passing them to core.
Core reads `server_version_num` and accepts PostgreSQL 18, including patch releases.
Supporting one major version initially keeps the compatibility claim equal to the tested scope.
Unsupported-version errors name PostgreSQL 18 and direct operators to a supported server without downgrading data.
Existing `PORT` configuration remains available.

Use sqlx with PostgreSQL, Tokio, migrations, and TLS support. Embed migrations in the binary.
Track migration files in build inputs so SQL edits trigger recompilation.

The server overrides sqlx's default `Prefer` mode; that mode allows silent plaintext fallback.
Without explicit `sslmode`, literal loopback addresses, `localhost`, and Unix sockets use `disable`.
Other hosts default to `verify-full`, including container service names such as `db`.
Evaluate literal hosts before DNS lookup; a name resolving to loopback does not receive the local exception.
For multiple hosts, choose verified TLS unless every host meets the local exception.
Explicit modes accepted are `disable`, `verify-ca`, and `verify-full`.
Reject `allow`, `prefer`, and `require`: these permit fallback or omit server identity verification.
`disable` is an operator's explicit opt-in, including for a trusted Compose network.
The fixture uses `disable`; packaging must also set its intended mode explicitly.
Verified modes retain sqlx certificate configuration and fail closed when validation fails.
Test TLS using a local PostgreSQL TLS fixture and a test CA, with both valid and invalid certificates.
Only the test process trusts that CA. Tests inject connection options, not a runtime transport bypass.

Map configuration and database failures into fixed categories and operator actions.
Never display connection options, database-supplied messages, or raw sqlx errors.
This avoids relying on password substitution, which can miss escaped secrets.

### Validate before writing

Acquire Sidereal's session lock with `pg_advisory_lock(int4, int4)` on a dedicated, unpooled connection.
Use fixed keys `1397310533` and `1`, identifying Sidereal's startup namespace.
PostgreSQL separates this two-key namespace from sqlx's single-bigint migration lock namespace.
Keep the same connection through preflight, bootstrap, sqlx migration execution, and final verification.
Retain sqlx migration locking on this connection; acquire Sidereal's lock first.
Explicitly unlock and close the connection after success. Close it after cancellation or failure to release session locks.

Read PostgreSQL catalogs before invoking sqlx, which can create its migration table.
An empty database contains only system objects and an empty default `public` schema.
Allow objects supplied by the standard empty PostgreSQL template, including its built-in procedural language.
Reject other schemas, relations, functions, types, sequences, triggers, and extensions outside the expected schema.
Startup uses fixed, qualified identifiers and parameterized values.

The initial transaction creates one identity row in `public.sidereal_metadata` with product `sidereal` and format version 1.
The only application tables are `public.sidereal_metadata` and `public._sqlx_migrations`.
Core accepts only successful migration records forming an exact prefix of the bundled migrations.
Unknown versions, checksum differences, missing records, failed records, and altered schema objects fail before writes.
An identity row without complete records, or a migration table without its identity row, is invalid.

For an empty database, core owns one bootstrap transaction using the bundled sqlx migration text and checksum.
It creates the sqlx-compatible migration table, executes the initial SQL, and inserts its migration record before committing.
Do not invoke sqlx's default migrator before bootstrap; separate migration-table creation could leave an unowned table after interruption.
Later startup uses sqlx's migrator on the same dedicated connection after preflight succeeds.
A crash either commits the complete initialized state or rolls it back.
The spec requires a transaction per bundled migration. Reject migration sources that disable transactions before executing them.
Long-running or nontransactional migrations need a separate design before adding domain tables.
This foundation does not choose how domain packs contribute migrations; settle that when the first pack table is proposed.

Using sqlx validation alone was rejected because it does not establish ownership before creating its migration table.
Sharing a database or adopting arbitrary existing objects was rejected because this story requires a dedicated database.
The startup role needs schema privileges, not PostgreSQL superuser access.

### Define and generate schema checks

Core compares a normalized schema description for each supported migration prefix.
The description includes schema and relation names, relation kinds, and ordered column names.
It includes PostgreSQL type names, nullability, defaults, generated or identity properties, constraints, and index definitions.
It also checks unexpected functions, types, sequences, triggers, and extensions.
Normalize implicit names and qualified expressions; never include OIDs, physical file identifiers, timestamps, owner names, or access lists.
Owner names and grants vary across restores. Check required runtime privileges separately rather than requiring identical roles or grants.

A root `db-schema` recipe generates descriptions by applying each migration prefix to isolated databases on the pinned PostgreSQL image.
Commit the deterministic output alongside core's migration fixtures. People do not hand-write schema descriptions.
A gate test regenerates and compares them, including sqlx's migration table.
Keep the initial migration-table SQL and its generated baseline fixture across dependency updates.
A sqlx upgrade must prove that its migrator accepts that baseline without changing existing migration records.
A changed sqlx table requirement needs an explicit forward migration; never recreate or silently adopt an incompatible table.
Test a restore with different owners and equivalent required privileges.
Test missing privileges independently; matching schema descriptions cannot substitute for working database access.

### Startup and health checks

Startup follows configuration, connection, lock, preflight, migrations, verification, pool creation, and HTTP binding.
Use a 30-second connection deadline and a separate 30-second startup-lock deadline.
`DATABASE_STARTUP_TIMEOUT_SECONDS` configures both deadlines with a positive value; invalid values fail before connecting.
`DATABASE_MIGRATION_TIMEOUT_SECONDS` sets positive statement and database-lock deadlines, defaulting to 30 seconds.
These settings permit slow hardware without introducing an infinite wait.
Deadline tests use shorter internal durations and allow 250 milliseconds of scheduling tolerance.
HTTP remains unbound on startup failure. Close every startup connection on failure.

`/healthz` keeps its existing body and never queries PostgreSQL.
Create a separate readiness pool with at most one connection.
Permit only one active readiness check using a semaphore; excess requests immediately return the fixed 503 body.
The two-second probe deadline covers connection acquisition, schema checks, and migration-record checks.
Use a read-only repeatable-read transaction so one probe observes a consistent schema and migration history.
Set a database-side two-second statement timeout and discard a connection whose timed-out work cannot be safely cancelled.
Readiness never uses the future operational pool or performs migrations.
After PostgreSQL accepts connections again, probes attempted once per second must report ready within five seconds.
This recovery bound assumes a compatible database can answer each check within the probe deadline.
Graceful shutdown closes both pools.

Keeping HTTP unbound until initialization succeeds avoids exposing a partially initialized server.
Testing only `SELECT 1` was rejected because connectivity alone does not prove compatibility.
Keep the existing Docker healthcheck on `/healthz`; readiness failure is not a command to restart the process.

### Fixture, pin, and demonstration

Add a database-only Compose fixture under `server/tests/fixtures/`.
`server/postgres-image.env` sets its single `POSTGRES_IMAGE` value, with an exact PostgreSQL 18 patch and image digest.
Fixture recipes pass this pin file through Compose's `--env-file` option.
CI starts the same fixture through root recipes after checkout; it does not repeat the image tag in a service definition.
Add the pin to CONTRIBUTING's version table during implementation.

Containers provide the same PostgreSQL process and restart behavior with or without Nix.
A Nix-only service would introduce a second test setup and would not exercise container-volume restart behavior.
This follows ADR-013's optional Nix policy; it does not change how build tools are pinned.
Document Docker Engine or Docker Desktop with Compose v2 as fixture prerequisites.
An existing PostgreSQL 18 test service can supply `TEST_DATABASE_URL` for database tests without a local container runtime.
The retained-state demonstration and schema-generation recipe require the fixture runtime.

Root `db-up` and `db-down` manage a named volume without deleting it.
`db-clean` explicitly deletes only that fixture volume.
A worktree-specific project name and configurable loopback port isolate concurrent worktrees.
The fixture creates distinct runtime, demo, and test roles. Only the test role receives `CREATEDB`.
Readiness waits for initialization of those roles, including in CI.
`db-url` prints a fixture runtime connection URL; `db-test-url` prints the test-role URL.
Both use the chosen fixture project and port and fixed test-only credentials.
The documented setup is `just db-up`, then export `DATABASE_URL` from `just db-url`.
Export `TEST_DATABASE_URL` from `just db-test-url` before the server gate or direct `cargo test`.
Never use runtime `DATABASE_URL` as a fallback for tests.

Each test creates a uniquely named temporary database under the test role.
A supervisor connection drops test databases on success, assertion failure, and unwind.
Killed processes may leave test databases; fixture-only cleanup removes databases with that worktree's reserved test prefix.
Missing test configuration fails with instructions, rather than skipping tests.
CI exports the test URL without printing credentials into its logs, then runs the unchanged root server gate.
Update direct-cargo examples to include the same database prerequisite.

Core tests inject migration sources and deadlines through private test helpers.
Test unsupported versions through the pure version-policy function used by preflight.
Test the preflight read-before-write ordering with an injected unsupported result and before/after catalog snapshots.
A test-only migration source creates an object and then fails, proving bootstrap and later-migration rollback against real PostgreSQL.
Production has no environment setting that injects a server version or migration source.

`db-demo` builds the server once and uses only the fixture's dedicated demo database.
It snapshots every migration-record field in stable order, then restarts the server and PostgreSQL separately.
After each restart, it compares records and polls readiness with a bounded wait.
Track server processes and remove them on failure or interruption.
Scratch snapshots live under `.workspace/`. The demo never accepts an arbitrary production database URL.

### Security scope

The [recorded scope decision](https://github.com/sidereal-io/sidereal/issues/394#issuecomment-6071730280) permits this limited database foundation before authentication.
It exposes only unauthenticated liveness and readiness probes with fixed status bodies.
The HTTP surface permits no operational mutations, library reads, plugins, providers, or asset access.
Migrations run only during local startup under the configured database role.
Auth epic #388 owns ADR-007 wording and access controls before protected capabilities become available.
Compose story #389 records the same boundary. This change does not amend ADR-007 or implement authentication.

## Risks / Trade-offs

- Strict ownership checks reject custom database additions. Document the dedicated-database requirement and test unrelated contents explicitly.
- An operator can change schemas outside Sidereal's lock. Require exclusive application ownership; the lock coordinates Sidereal startups only.
- PostgreSQL 18-only support excludes other majors. Publish this limit and expand support only with real version coverage.
- Database tests add an infrastructure prerequisite. Supply root fixture recipes and the same setup in CI.
- Forward-only migration prevents automatic rollback. Older binaries reject newer history; operators retain data and use a compatible binary.
- Full readiness checks cost more than a connectivity probe. Bound concurrency, reserve a separate connection, and verify the deadline in tests.

## Migration Plan

Prepare an empty dedicated database and set `DATABASE_URL`. Start the server and check `/readyz`.
Run the retained-state demonstration before consuming this foundation in packaging work.
There is no import from v0 databases and no downgrade path.
