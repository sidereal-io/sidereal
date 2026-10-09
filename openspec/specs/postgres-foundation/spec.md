# postgres-foundation Specification

## Purpose

Sam can start Sidereal against a dedicated PostgreSQL database and retain its migration state across restarts.

## Requirements

### Requirement: Explicit database configuration

The server SHALL require `DATABASE_URL` for a dedicated PostgreSQL 18 database. It SHALL reject missing, malformed, or unsupported configuration before database writes.
The server SHALL exit unsuccessfully when startup exceeds its configured connection deadline, defaulting to 30 seconds.
`DATABASE_STARTUP_TIMEOUT_SECONDS` and `DATABASE_MIGRATION_TIMEOUT_SECONDS` SHALL accept positive seconds and default to 30.
Invalid timeout values SHALL fail before connecting.
Errors SHALL identify the failure category without exposing credentials, connection strings, or raw database errors.

#### Scenario: Missing configuration
- **WHEN** Sam starts the server without `DATABASE_URL`
- **THEN** the process exits unsuccessfully and reports missing database configuration

#### Scenario: Unsupported PostgreSQL
- **WHEN** Sam connects to PostgreSQL with a major version other than 18
- **THEN** startup fails and leaves database objects and rows unchanged

#### Scenario: Unavailable database
- **WHEN** the configured database refuses connections or never responds
- **THEN** startup fails by the configured connection deadline, allowing 250 milliseconds of scheduling tolerance

#### Scenario: Secret-bearing failures
- **WHEN** connection parsing or authentication fails with a distinctive password in the configuration
- **THEN** captured stdout and stderr contain neither that password nor the connection string

### Requirement: Explicit database transport security

Without `sslmode`, connections to literal loopback addresses, `localhost`, or Unix sockets SHALL use unencrypted local transport.
Other hosts SHALL default to `sslmode=verify-full`.
Explicit `sslmode` SHALL accept only `disable`, `verify-ca`, or `verify-full`.
`disable` SHALL represent an explicit operator opt-in to unencrypted transport, including on a Compose network.
Verified modes SHALL fail when certificate validation fails; they SHALL NOT retry using plaintext.

#### Scenario: Named database host without sslmode
- **WHEN** Sam configures a named database host without `sslmode`
- **THEN** the connection requires TLS with certificate and hostname verification

#### Scenario: Loopback or socket without sslmode
- **WHEN** Sam configures a literal loopback address, `localhost`, or Unix socket without `sslmode`
- **THEN** the connection uses local unencrypted transport

#### Scenario: Explicit unencrypted Compose connection
- **WHEN** Sam configures the `db` service host with `sslmode=disable`
- **THEN** the server connects without TLS and applies the same ownership checks

#### Scenario: Verified certificate accepted
- **WHEN** the database certificate matches the configured hostname and chains to the configured trusted CA
- **THEN** startup connects successfully with `sslmode=verify-full`

#### Scenario: Verified certificate rejected
- **WHEN** the database presents an untrusted or wrong-host certificate with `sslmode=verify-full`
- **THEN** startup fails without retrying plaintext or changing database contents

#### Scenario: Unsafe explicit transport mode
- **WHEN** Sam configures `sslmode=allow`, `prefer`, or `require`
- **THEN** startup rejects the configuration before connecting

### Requirement: Dedicated database ownership

The server SHALL initialize only an empty application database. System objects and the default empty `public` schema SHALL NOT count as application contents.
The server SHALL reject unrelated objects, unrelated migration history, and a damaged Sidereal identity row before database writes.

#### Scenario: Unrelated contents
- **WHEN** the configured database contains an unrelated table, view, function, extension, or custom schema
- **THEN** startup fails and a before-and-after comparison shows identical database objects and rows

#### Scenario: Foreign migration table
- **WHEN** the database contains the migration table without a valid Sidereal identity row
- **THEN** startup fails without changing the migration table or creating a Sidereal identity row

### Requirement: Forward-only startup migrations

The server SHALL apply pending migrations automatically before serving HTTP.
It SHALL accept only successful migration history matching a prefix of the bundled history.
It SHALL reject unknown versions, changed migration checksums, missing history entries, or mismatched schema objects before writes.
The server SHALL never reset the database or execute downgrade migrations.
Each bundled migration SHALL run in one transaction, including its essential migration record fields.
The `execution_time` field SHALL be best-effort timing metadata; sqlx MAY update it after commit or leave its sentinel after interruption.
Compatibility checks SHALL NOT reject an otherwise valid migration because of its timing metadata.
The initial transaction SHALL also create the migration table and identity row.
The server SHALL reject migrations that disable transactions before applying them.
Concurrent server startups SHALL apply each migration exactly once.

#### Scenario: First startup
- **WHEN** Sam starts the server against an empty dedicated database
- **THEN** the database contains the Sidereal identity row and one successful initial migration record
- **AND** its only application tables are `sidereal_metadata` and `_sqlx_migrations`

#### Scenario: Repeated startup
- **WHEN** Sam restarts against an initialized database
- **THEN** migration records, including their timestamps and checksums, remain identical

#### Scenario: Concurrent startup
- **WHEN** two servers start against the same empty database
- **THEN** both complete startup and exactly one successful initial migration record exists

#### Scenario: Incompatible history or schema
- **WHEN** a fixture contains a newer version, changed checksum, missing history entry, failed record, or altered expected table
- **THEN** startup fails and database objects and rows remain unchanged

#### Scenario: Migration failure
- **WHEN** a migration fails partway through its transaction
- **THEN** startup fails and that migration leaves no committed schema changes or successful migration record

### Requirement: Separate liveness and readiness

`GET /healthz` SHALL return `200 {"status":"ok"}` while HTTP is running, independent of database availability.
`GET /readyz` SHALL return `200 {"status":"ready"}` only when the database is reachable and compatible.
Otherwise it SHALL return `503 {"status":"not_ready"}`.
Readiness SHALL finish within two seconds and expose no database details.
Both endpoints SHALL permit unauthenticated health probes without enabling operational mutations.
Readiness SHALL allow at most one active database check; excess probes SHALL return the fixed 503 response immediately.
Readiness SHALL use database capacity separate from operational requests.

#### Scenario: Healthy database
- **WHEN** startup succeeds and the database remains reachable and compatible
- **THEN** `/healthz` returns its documented 200 response
- **AND** `/readyz` returns its documented 200 response

#### Scenario: Database outage and recovery
- **WHEN** PostgreSQL stops after startup
- **THEN** `/healthz` still returns 200 and `/readyz` returns its documented 503 response within two seconds
- **AND** probes attempted once per second return ready within five seconds after PostgreSQL accepts connections again
- **AND** this bound applies when the compatible database can answer within the two-second probe deadline

#### Scenario: Database compatibility changes
- **WHEN** database migration history changes to an unsupported version after startup
- **THEN** `/readyz` returns its documented 503 response without modifying the database

#### Scenario: HTTP surface
- **WHEN** a caller sends `POST`, `PUT`, `PATCH`, or `DELETE` to either health route
- **THEN** the server returns 405 and database objects and rows remain unchanged

### Requirement: Retained-state demonstration

Root recipes SHALL provide a PostgreSQL fixture with retained data and a repeatable retained-state demonstration.
The demonstration SHALL restart both the server and PostgreSQL against the same fixture volume.
It SHALL compare migration records and return nonzero on any mismatch or readiness failure.
Stopping the fixture SHALL preserve its volume. Only an explicitly named cleanup recipe SHALL delete fixture data.
Database integration tests SHALL run in the server gate and SHALL fail when their database prerequisite is unavailable.

#### Scenario: Retained migration state
- **WHEN** a contributor runs `just db-demo`
- **THEN** the command exits zero after initial and repeated startup, server restart, and PostgreSQL restart
- **AND** its migration-record comparison confirms identical versions, checksums, and timestamps

#### Scenario: Fixture stop preserves records
- **WHEN** a contributor stops and starts the database fixture without invoking its cleanup recipe
- **THEN** the fixture retains identical migration records

#### Scenario: Database tests cannot run
- **WHEN** the server gate cannot reach its configured test database
- **THEN** the gate exits nonzero instead of skipping database tests

#### Scenario: Test configuration is missing
- **WHEN** a contributor runs the server gate without `TEST_DATABASE_URL`
- **THEN** the gate exits nonzero and identifies the missing test configuration

### Requirement: Database checks survive equivalent restores and dependency updates

Database checks SHALL compare application schema structure rather than role names or access-list ordering.
The server SHALL accept an equivalent restored schema when its configured role has the required privileges.
Dependency updates SHALL preserve compatibility with the initial migration-table format and retained migration records.
An incompatible format change SHALL require an explicit forward migration rather than silently recreating the migration table.

#### Scenario: Restore under a different role
- **WHEN** the initialized database is restored with different owners and equivalent required runtime privileges
- **THEN** startup succeeds and preserves all migration records

#### Scenario: Missing runtime privileges
- **WHEN** the configured role cannot read required migration records or apply a pending migration
- **THEN** startup fails with a safe permission error without modifying database objects or rows

#### Scenario: Existing migration-table baseline
- **WHEN** startup uses the committed initial database fixture after a sqlx dependency update
- **THEN** its migration records remain identical and startup succeeds without recreating either application table

#### Scenario: Concurrent health probes
- **WHEN** one readiness check holds its reserved database connection and additional callers probe readiness
- **THEN** the additional callers receive the fixed 503 response without acquiring operational database connections
