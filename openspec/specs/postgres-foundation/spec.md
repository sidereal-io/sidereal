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
- **THEN** the database contains the Sidereal identity row and two successful migration records
- **AND** its only application tables are `sidereal_metadata` and `_sqlx_migrations`

#### Scenario: Repeated startup
- **WHEN** Sam restarts against an initialized database
- **THEN** migration records, including their timestamps and checksums, remain identical

#### Scenario: Concurrent startup
- **WHEN** two servers start against the same empty database
- **THEN** both complete startup and exactly two successful migration records exist

#### Scenario: Incompatible history or schema
- **WHEN** a fixture contains a newer version, changed checksum, missing history entry, failed record, or altered expected table
- **THEN** startup fails and database objects and rows remain unchanged

#### Scenario: Migration failure
- **WHEN** a migration fails partway through its transaction
- **THEN** startup fails and that migration leaves no committed schema changes or successful migration record

#### Scenario: Upgrade preserves the original migration

- **GIVEN** a database with the original foundation migration and saved metadata
- **WHEN** the new binary applies the storage pairing migration
- **THEN** the original migration record and metadata remain identical
- **AND** the existing metadata row gains a null storage UUID before storage initialization

### Requirement: Separate liveness and readiness

`GET /healthz` SHALL return `200 {"status":"ok"}` while HTTP is running, independent of database availability.
A `/readyz` probe that runs its own check SHALL return `200 {"status":"ready"}` only when the database is reachable and compatible.
Otherwise that probe SHALL return `503 {"status":"not_ready"}`.
Readiness SHALL finish within two seconds and expose no database details.
Both endpoints SHALL permit unauthenticated health probes without enabling operational mutations.
Readiness SHALL allow at most one active database check.
A probe that arrives while a check is active SHALL immediately return the response of the most recent finished check, without a database check of its own.
That response can therefore be 200 for a short time after the database becomes unusable, until the active check finishes.
Before the first check finishes, that response SHALL be the ready response, because startup has just proved the database usable.
Readiness SHALL use database capacity separate from operational requests.

#### Scenario: Healthy database
- **WHEN** startup succeeds and the database remains reachable and compatible
- **THEN** `/healthz` returns its documented 200 response
- **AND** `/readyz` returns its documented 200 response

#### Scenario: Database outage and recovery
- **WHEN** PostgreSQL stops after startup and a caller probes `/readyz` while no other check is active
- **THEN** `/healthz` still returns 200 and that probe returns its documented 503 response within two seconds
- **AND** probes attempted once per second return ready within five seconds after PostgreSQL accepts connections again
- **AND** this bound applies when the compatible database can answer within the two-second probe deadline

#### Scenario: Database compatibility changes
- **WHEN** database migration history changes to an unsupported version after startup
- **THEN** `/readyz` returns its documented 503 response without modifying the database

#### Scenario: HTTP surface
- **WHEN** a caller sends `POST`, `PUT`, `PATCH`, or `DELETE` to either health route
- **THEN** the server returns 405 and database objects and rows remain unchanged

#### Scenario: Overlapping probes on a usable database
- **GIVEN** a ready server whose database stays usable
- **WHEN** one readiness check is active and another caller probes `/readyz`
- **THEN** the other caller receives `200 {"status":"ready"}` immediately

#### Scenario: Overlapping probes during an outage
- **GIVEN** a ready server whose last finished check found the database usable
- **WHEN** PostgreSQL stops, caller A probes `/readyz`, and caller B probes while A's check is active
- **THEN** caller B receives `200 {"status":"ready"}` immediately
- **AND** caller A receives `503 {"status":"not_ready"}`
- **AND** a probe that arrives after A's check finishes, while another check is active, receives `503 {"status":"not_ready"}`

### Requirement: Retained fixture data and database regression tests

Root recipes SHALL provide a PostgreSQL fixture with retained data.
Stopping the fixture SHALL preserve its volume. Only an explicitly named cleanup recipe SHALL delete fixture data.
Database integration tests SHALL run in the server gate and SHALL fail when their database prerequisite is unavailable.

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
- **THEN** the additional callers immediately receive the most recent finished check's response without acquiring operational database connections

### Requirement: Connection failures name their cause

When startup cannot open its database connection, the server SHALL report which of these causes applies:

- **unreachable**: the host name does not resolve, the connection is refused or reset, the host does not answer before the connection deadline, or PostgreSQL reports that it is starting up or shutting down;
- **login rejected**: PostgreSQL rejects the user, the password, or the client's address;
- **TLS failed**: certificate or hostname verification fails in a verified `sslmode`, including a failure reported during the TLS handshake;
- **database missing**: the server accepts the login but the configured database does not exist.

Each message SHALL start with a fixed phrase for its cause and SHALL name one recovery action.
The fixed phrases SHALL be `database unreachable`, `database login rejected`, `database TLS verification failed`, and `database does not exist`.
A connection failure that matches none of these causes SHALL keep the existing `database connection failed` message.
A deadline that expires after the connection opens SHALL keep the existing `database startup deadline exceeded` message.
A connection lost while startup checks ownership or migration history SHALL be reported as unreachable, not as an ownership or history fault. That includes a session that PostgreSQL ends because it is shutting down.
A verified TLS mode with a trusted certificate SHALL still start.
Every message SHALL keep the existing redaction: no password, user name, connection string, certificate detail, or raw database error.
A failed connection SHALL leave database objects and rows unchanged.

#### Scenario: PostgreSQL is not running

- **WHEN** Sam starts the server with `DATABASE_URL` naming a loopback port where nothing listens
- **THEN** the process exits unsuccessfully
- **AND** its output starts with `database unreachable`

#### Scenario: The host name does not resolve

- **WHEN** Sam starts the server with a `DATABASE_URL` host under the `.invalid` domain
- **THEN** the process exits unsuccessfully and its output starts with `database unreachable`

#### Scenario: The host never answers

- **WHEN** Sam starts the server against a listener that accepts TCP connections but never replies, with a 1-second startup deadline
- **THEN** the process exits unsuccessfully within 1.25 seconds and its output starts with `database unreachable`

#### Scenario: The password is wrong

- **WHEN** Sam starts the server with a distinctive wrong password for an existing role
- **THEN** the process exits unsuccessfully and its output starts with `database login rejected`
- **AND** the output contains neither that password nor the connection string

#### Scenario: The certificate does not verify

- **WHEN** the database presents an untrusted certificate, or a certificate for another host, with `sslmode=verify-full`
- **THEN** startup fails with output that starts with `database TLS verification failed`
- **AND** database objects and rows remain unchanged

#### Scenario: The database name is wrong

- **WHEN** Sam starts the server with valid credentials and a database name that does not exist
- **THEN** the process exits unsuccessfully and its output starts with `database does not exist`

#### Scenario: A trusted CA certificate verifies

- **WHEN** Sam starts the server with `sslmode=verify-ca` and `sslrootcert` naming the CA that signed the database's certificate
- **THEN** startup succeeds and `/readyz` returns `200 {"status":"ready"}`

### Requirement: Certificate files are checked before connecting

Before it connects, startup SHALL read every certificate or key file that the configuration names.
That includes `sslrootcert`, `sslcert`, and `sslkey` in `DATABASE_URL`, and the `PGSSLROOTCERT`, `PGSSLCERT`, and `PGSSLKEY` environment variables.
A certificate given inline as PEM text needs no file.
The check SHALL accept only regular files of at most 1 MiB, and it SHALL NOT wait on a file that never finishes reading.
When a named file is missing, cannot be read, is not a regular file, or is too large, startup SHALL fail without opening a database connection.
Its message SHALL start with `database TLS certificate file unreadable` and SHALL tell Sam to check the certificate paths in the configuration.
The message SHALL NOT contain the path or any file content.

#### Scenario: The CA file is missing

- **WHEN** Sam starts the server with `sslmode=verify-full` and `sslrootcert` naming a file that does not exist
- **THEN** the process exits unsuccessfully and its output starts with `database TLS certificate file unreadable`
- **AND** the output does not contain the configured path

#### Scenario: The CA path cannot be read

- **WHEN** Sam starts the server with `sslrootcert` naming a directory
- **THEN** the process exits unsuccessfully and its output starts with `database TLS certificate file unreadable`

#### Scenario: The CA path is a named pipe

- **WHEN** Sam starts the server with `sslrootcert` naming a named pipe that nothing writes to
- **THEN** the process exits unsuccessfully within 1 second and its output starts with `database TLS certificate file unreadable`

### Requirement: Readiness changes are logged

The server SHALL write one line to standard error when a readiness check finds the database unusable after the previous check found it usable.
The line SHALL name a safe cause, using the connection causes above or another startup category such as permission or incompatible schema.
A check that cannot open a connection within its deadline SHALL name the cause unreachable.
A check that opens a connection but does not finish its queries within its deadline SHALL name the cause timed out.
A check whose connection is lost during its queries SHALL name the cause unreachable.
The server SHALL write one more line when a later check finds the database unusable for a different cause.
The server SHALL write one line when a check finds the database usable again.
Further checks with the same result SHALL write nothing.
A probe answered without its own check, because another check is active, SHALL write nothing.
The server SHALL treat the database as usable when startup finishes, so a ready start writes no readiness line.
Readiness lines SHALL hold no password, user name, connection string, or raw database error.
The server SHALL observe changes only when a caller probes `/readyz`.

#### Scenario: PostgreSQL stops and starts again

- **GIVEN** a ready server whose readiness is probed once per second
- **WHEN** PostgreSQL stops accepting connections for 10 seconds and then accepts them again
- **THEN** standard error gains exactly one line that reports the database not ready and names it unreachable
- **AND** it then gains exactly one line that reports the database ready again

#### Scenario: Schema queries stall

- **GIVEN** a ready server
- **WHEN** another session holds an exclusive lock on the migration table and a caller probes `/readyz`
- **THEN** standard error gains exactly one line that reports the database not ready and names it timed out

#### Scenario: A ready server writes no readiness lines

- **GIVEN** a ready server whose database stays usable
- **WHEN** a caller probes `/readyz` 20 times
- **THEN** standard error gains no line

#### Scenario: The connection drops during a check

- **GIVEN** a ready server
- **WHEN** a readiness check has opened its connection and the connection closes while the check reads migration history
- **THEN** `/readyz` returns its documented 503 response
- **AND** standard error gains exactly one line that reports the database not ready and names it unreachable

#### Scenario: PostgreSQL ends the session during a check

- **GIVEN** a ready server
- **WHEN** a readiness check is reading migration history and PostgreSQL ends its session with SQLSTATE `57P01`
- **THEN** `/readyz` returns its documented 503 response
- **AND** standard error gains exactly one line that names the cause unreachable

#### Scenario: Readiness lines stay redacted

- **GIVEN** a server started with a distinctive password
- **WHEN** PostgreSQL stops and readiness reports not ready
- **THEN** standard error contains neither the password nor the connection string
