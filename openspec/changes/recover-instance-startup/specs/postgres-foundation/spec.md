## ADDED Requirements

### Requirement: Connection failures name their cause

When startup cannot open its database connection, the server SHALL report which of these causes applies:

- **unreachable**: the host name does not resolve, the connection is refused or reset, or the host does not answer;
- **login rejected**: PostgreSQL rejects the user, the password, or the client's address;
- **TLS failed**: certificate or hostname verification fails in a verified `sslmode`;
- **database missing**: the server accepts the login but the configured database does not exist.

Each message SHALL start with a fixed phrase for its cause and SHALL name one recovery action.
The fixed phrases SHALL be `database unreachable`, `database login rejected`, `database TLS verification failed`, and `database does not exist`.
A connection failure that matches none of these causes SHALL keep the existing `database connection failed` message.
Every message SHALL keep the existing redaction: no password, user name, connection string, or raw database error.
A failed connection SHALL leave database objects and rows unchanged.

#### Scenario: PostgreSQL is not running

- **WHEN** Sam starts the server with `DATABASE_URL` naming a loopback port where nothing listens
- **THEN** the process exits unsuccessfully
- **AND** its output starts with `database unreachable`

#### Scenario: The host name does not resolve

- **WHEN** Sam starts the server with a `DATABASE_URL` host under the `.invalid` domain
- **THEN** the process exits unsuccessfully and its output starts with `database unreachable`

#### Scenario: The password is wrong

- **WHEN** Sam starts the server with a distinctive wrong password for an existing role
- **THEN** the process exits unsuccessfully and its output starts with `database login rejected`
- **AND** the output contains neither that password nor the connection string

#### Scenario: The certificate does not verify

- **WHEN** the database presents an untrusted certificate with `sslmode=verify-full`
- **THEN** startup fails with output that starts with `database TLS verification failed`
- **AND** database objects and rows remain unchanged

#### Scenario: The database name is wrong

- **WHEN** Sam starts the server with valid credentials and a database name that does not exist
- **THEN** the process exits unsuccessfully and its output starts with `database does not exist`

### Requirement: Readiness changes are logged

The server SHALL write one line to standard error when a readiness check finds the database unusable after the previous check found it usable.
The line SHALL name a safe cause from the startup categories, such as unreachable, login rejected, permission, or incompatible schema.
The server SHALL write one more line when a later check finds the database unusable for a different cause.
The server SHALL write one line when a check finds the database usable again.
Further checks with the same result SHALL write nothing.
A probe refused because another readiness check is active SHALL NOT count as a result and SHALL write nothing.
The server SHALL treat the database as usable when startup finishes, so a ready start writes no readiness line.
Readiness lines SHALL hold no password, user name, connection string, or raw database error.
The server SHALL observe changes only when a caller probes `/readyz`.

#### Scenario: PostgreSQL stops and starts again

- **GIVEN** a ready server whose readiness is probed once per second
- **WHEN** PostgreSQL stops for 10 seconds and then accepts connections again
- **THEN** standard error gains exactly one line that reports the database not ready and names it unreachable
- **AND** it then gains exactly one line that reports the database ready again

#### Scenario: A ready server writes no readiness lines

- **GIVEN** a ready server whose database stays usable
- **WHEN** a caller probes `/readyz` 20 times
- **THEN** standard error gains no line

#### Scenario: Readiness lines stay redacted

- **GIVEN** a server started with a distinctive password
- **WHEN** PostgreSQL stops and readiness reports not ready
- **THEN** standard error contains neither the password nor the connection string
