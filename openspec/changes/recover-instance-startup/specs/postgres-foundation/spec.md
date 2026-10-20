## ADDED Requirements

### Requirement: Connection failures name their cause

When startup cannot open its database connection, the server SHALL report which of these causes applies:

- **unreachable**: the host name does not resolve, the connection is refused or reset, or the host does not answer before the connection deadline;
- **login rejected**: PostgreSQL rejects the user, the password, or the client's address;
- **TLS failed**: certificate or hostname verification fails in a verified `sslmode`, including a failure reported during the TLS handshake;
- **database missing**: the server accepts the login but the configured database does not exist.

Each message SHALL start with a fixed phrase for its cause and SHALL name one recovery action.
The fixed phrases SHALL be `database unreachable`, `database login rejected`, `database TLS verification failed`, and `database does not exist`.
A connection failure that matches none of these causes SHALL keep the existing `database connection failed` message.
A deadline that expires after the connection opens SHALL keep the existing `database startup deadline exceeded` message.
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

### Requirement: Readiness changes are logged

The server SHALL write one line to standard error when a readiness check finds the database unusable after the previous check found it usable.
The line SHALL name a safe cause, using the connection causes above or another startup category such as permission or incompatible schema.
A check that cannot open a connection within its deadline SHALL name the cause unreachable.
A check that opens a connection but does not finish its queries within its deadline SHALL name the cause timed out.
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

#### Scenario: Readiness lines stay redacted

- **GIVEN** a server started with a distinctive password
- **WHEN** PostgreSQL stops and readiness reports not ready
- **THEN** standard error contains neither the password nor the connection string

## MODIFIED Requirements

### Requirement: Separate liveness and readiness

`GET /healthz` SHALL return `200 {"status":"ok"}` while HTTP is running, independent of database availability.
`GET /readyz` SHALL return `200 {"status":"ready"}` only when the database is reachable and compatible.
Otherwise it SHALL return `503 {"status":"not_ready"}`.
Readiness SHALL finish within two seconds and expose no database details.
Both endpoints SHALL permit unauthenticated health probes without enabling operational mutations.
Readiness SHALL allow at most one active database check.
A probe that arrives while a check is active SHALL immediately return the response of the most recent finished check, without a database check of its own.
Before the first check finishes, that response SHALL be the ready response, because startup has just proved the database usable.
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

#### Scenario: Overlapping probes on a usable database
- **GIVEN** a ready server whose database stays usable
- **WHEN** one readiness check is active and another caller probes `/readyz`
- **THEN** the other caller receives `200 {"status":"ready"}` immediately

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
