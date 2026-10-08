## Purpose

Sam can start Sidereal against a dedicated PostgreSQL database and retain its migration state across restarts.

## ADDED Requirements

### Requirement: Explicit database configuration

The server SHALL require `DATABASE_URL` for a dedicated PostgreSQL 18 database. It SHALL reject missing, malformed, or unsupported configuration before database writes.
The server SHALL exit unsuccessfully when startup cannot connect within 30 seconds.
Errors SHALL identify the failure category without exposing credentials, connection strings, or raw database errors.

#### Scenario: Missing configuration
- **WHEN** Sam starts the server without `DATABASE_URL`
- **THEN** the process exits unsuccessfully and reports missing database configuration

#### Scenario: Unsupported PostgreSQL
- **WHEN** Sam connects to PostgreSQL with a major version other than 18
- **THEN** startup fails and leaves database objects and rows unchanged

#### Scenario: Unavailable database
- **WHEN** the configured database refuses connections or never responds
- **THEN** startup fails within 30 seconds after connection begins

#### Scenario: Secret-bearing failures
- **WHEN** connection parsing or authentication fails with a distinctive password in the configuration
- **THEN** captured stdout and stderr contain neither that password nor the connection string

### Requirement: Dedicated database ownership

The server SHALL initialize only an empty application database. System objects and the default empty `public` schema SHALL NOT count as application contents.
The server SHALL reject unrelated objects, unrelated migration history, and damaged Sidereal metadata before database writes.

#### Scenario: Unrelated contents
- **WHEN** the configured database contains an unrelated table, view, function, extension, or custom schema
- **THEN** startup fails and a before-and-after comparison shows identical database objects and rows

#### Scenario: Foreign migration history
- **WHEN** the database contains migration bookkeeping without valid Sidereal identity metadata
- **THEN** startup fails without changing bookkeeping or creating Sidereal metadata

### Requirement: Forward-only startup migrations

The server SHALL apply pending migrations automatically before serving HTTP.
It SHALL accept only successful migration history matching a prefix of the bundled history.
It SHALL reject unknown versions, changed migration checksums, missing history entries, or mismatched schema objects before writes.
The server SHALL never reset the database or execute downgrade migrations.
Concurrent server startups SHALL apply each migration exactly once.

#### Scenario: First startup
- **WHEN** Sam starts the server against an empty dedicated database
- **THEN** the database contains Sidereal identity metadata and one successful initial migration record
- **AND** the database contains no authentication or library tables

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

#### Scenario: Healthy database
- **WHEN** startup succeeds and the database remains reachable and compatible
- **THEN** `/healthz` returns its documented 200 response
- **AND** `/readyz` returns its documented 200 response

#### Scenario: Database outage and recovery
- **WHEN** PostgreSQL stops after startup
- **THEN** `/healthz` still returns 200 and `/readyz` returns its documented 503 response within two seconds
- **AND** after PostgreSQL returns, the next successful probe returns ready without restarting the server

#### Scenario: Database compatibility changes
- **WHEN** database migration history changes to an unsupported version after startup
- **THEN** `/readyz` returns its documented 503 response without modifying the database

#### Scenario: HTTP surface
- **WHEN** a caller sends `POST`, `PUT`, `PATCH`, or `DELETE` to either health route
- **THEN** the server returns 405 and database objects and rows remain unchanged

### Requirement: Retained-state demonstration

Root recipes SHALL provide a disposable persistent PostgreSQL fixture and a repeatable retained-state demonstration.
The demonstration SHALL restart both the server and PostgreSQL against the same fixture volume.
It SHALL compare migration records and return nonzero on any mismatch or readiness failure.
Stopping the fixture SHALL preserve its volume. Only an explicitly named cleanup recipe SHALL delete fixture data.
Database integration tests SHALL run in the server gate and SHALL fail when their database prerequisite is unavailable.

#### Scenario: Retained migration state
- **WHEN** a contributor runs `just db-demo`
- **THEN** the command exits zero after initial and repeated startup, server restart, and PostgreSQL restart
- **AND** its migration-record comparison confirms identical versions, checksums, and timestamps

#### Scenario: Fixture cleanup
- **WHEN** a contributor stops and starts the database fixture without invoking its cleanup recipe
- **THEN** the fixture retains identical migration records

#### Scenario: Database tests cannot run
- **WHEN** the server gate cannot reach its configured test database
- **THEN** the gate exits nonzero instead of skipping database tests
