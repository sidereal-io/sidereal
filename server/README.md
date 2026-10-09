# Sidereal server

The Rust workspace keeps database policy in `core`, public contracts in `plugin-abi`,
and astronomy in `packs/astro`. The thin axum server wires these crates and serves
`GET /healthz` and `GET /readyz`.
See [architecture](../docs/architecture.md) and [ADR-002](../docs/decisions/ADR-002-core-domain-pack-split.md).

## Prepare development and tests

Install rustup with the stable toolchain, a C linker, `just`, Node.js, OpenSSL, and Docker Engine
or Docker Desktop with Compose v2 or newer. The optional [Nix shell](../CONTRIBUTING.md#development-environment)
provides the build tools; Docker runs separately.

From the repository root:

```sh
just db-up
export DATABASE_URL=$(just db-url)
export TEST_DATABASE_URL=$(just db-test-url)
just server
```

The fixture binds only to loopback. Its Compose project, named volume, and default
port derive from the checkout path. Set `SIDEREAL_DB_PORT` before every recipe when
another process occupies that port. Database credentials are fixed test-only fixtures. TLS credentials are generated
per checkout under gitignored `.workspace/db-fixture-tls/`; no private key enters Git. Runtime and demo roles cannot create databases; the test role can.
The image's exact patch and digest live only in `server/postgres-image.env`.

Check liveness and database readiness:

```sh
curl localhost:5000/healthz  # 200 {"status":"ok"}
curl localhost:5000/readyz   # 200 {"status":"ready"}, or 503 {"status":"not_ready"}
```

`just dev` runs the server and web shell in parallel using native `just` recipes and the same exported `DATABASE_URL`.
Press Ctrl+C to stop both. If either recipe exits, the other keeps running until you stop it.
Prepare the fixture and exports before running it.

## Verify changes and retained state

```sh
just check          # Complete server and web gates
just db-demo        # Restart server and PostgreSQL; compare every migration-record field
just db-schema      # Regenerate committed normalized schema descriptions
just db-down        # Stop this fixture while retaining its volume
just db-test-clean  # Remove this fixture's killed-test leftovers only
just db-clean       # Explicitly delete this fixture's volume and all its data
```

The demo uses only the fixture's demo database, regardless of an inherited
`DATABASE_URL`. It cleans up spawned servers after failure or interruption.
Scratch files stay under `.workspace/`.

Database tests use unique temporary databases with a separate supervisor for cleanup
on success and assertion failure. Missing or unreachable `TEST_DATABASE_URL` fails
the gate; tests never fall back to `DATABASE_URL`.

A Rust-only contributor can run cargo directly after preparing the same fixture:

```sh
export DATABASE_URL=$(just db-url)
export TEST_DATABASE_URL=$(just db-test-url)
cd server
cargo run -p sidereal-server
cargo test
```

An existing PostgreSQL 18 service can replace Docker for database tests. Its test
role needs `CREATEDB`, and the default empty template must match PostgreSQL's standard
template. The TLS tests also expect the service on the URL's port to present the generated
`.workspace/db-fixture-tls/server.crt` and key (run `just db-tls` first): `localhost` must verify against the generated `ca.crt`, while
`127.0.0.1` must fail hostname verification. Configure those files with PostgreSQL's
`ssl`, `ssl_cert_file`, and `ssl_key_file` settings. Container-free setup requires these
same test capabilities; prerequisites are never silently skipped. The demo and schema
recipe require the pinned container fixture.

## Configure an instance

Use a dedicated, empty PostgreSQL 18 database. Sidereal does not adopt unrelated
objects, import v0 databases, reset state, or downgrade migrations. Keep other
applications and manual schema additions out of this database.

| Variable | Default | Purpose |
|---|---|---|
| `DATABASE_URL` | Required | Dedicated PostgreSQL connection URL |
| `DATABASE_STARTUP_TIMEOUT_SECONDS` | `30` | Positive connection and startup-lock deadlines |
| `DATABASE_MIGRATION_TIMEOUT_SECONDS` | `30` | Positive migration, statement, and database-lock deadlines |
| `PORT` | `5000` | HTTP listen port |
| `TEST_DATABASE_URL` | Required for tests | Separate test role with `CREATEDB`; never a production URL |
| `SIDEREAL_DB_PORT` | Derived from checkout | Loopback port for fixture recipes |

Without explicit `sslmode`, literal loopback, `localhost`, and Unix sockets use
unencrypted local transport. Other hosts require `verify-full`. Explicit modes are
`disable`, `verify-ca`, and `verify-full`. `disable` is an operator's opt-in, including
for a trusted Compose network. `allow`, `prefer`, and `require` are rejected.
Provide `sslrootcert` when verified TLS needs a private CA. Certificate failures
never fall back to plaintext.

Startup validates version, ownership, migration history, logical schema, and privileges
before writes. It applies forward migrations before binding HTTP. Compatibility
checks ignore owners and grants when comparing structure; required privileges are
checked separately. The configured role needs database `CONNECT`, schema `USAGE`,
read access to both tables, and schema `CREATE` plus migration-table `INSERT` and
`UPDATE` when a migration is pending. Later DDL may require ownership of objects it alters.

Migration SQL and essential records commit together. sqlx's `execution_time` is best
effort and can retain its sentinel after interruption; startup accepts and preserves
it. Failed startup keeps HTTP unbound and reports a safe category and operator action.

Readiness uses separate capacity and a read-only consistent snapshot, with one active
check and a two-second deadline. Concurrent probes can receive 503 immediately.
Database outages leave liveness healthy. A compatible service that answers within
the probe deadline becomes ready within five seconds of accepting connections again.
Use `/healthz` for process restart decisions; readiness failure alone should not restart
the server. Both pools close during graceful shutdown.
