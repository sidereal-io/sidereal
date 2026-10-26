## Purpose

Sam can run the packaged Sidereal app against a dedicated database on an existing PostgreSQL service.
The installation preserves the bundled path's startup checks, verified transport, and storage pairing.

## ADDED Requirements

### Requirement: External mode starts only the app

The repo SHALL provide `compose.external-db.yml` as an alternative entry file to `compose.yml`.
Sam SHALL select it through `COMPOSE_FILE` or explicit `-f` arguments.
External mode SHALL require a nonempty `DATABASE_URL` before creating containers.
It SHALL NOT require `POSTGRES_PASSWORD`, define a bundled database service, or create a PostgreSQL data volume.
It SHALL retain Compose 2.20 compatibility and the existing app build, port, user, capability restrictions, restart policy, and managed-storage target.
External mode SHALL use the image's readiness health check without a database service dependency.

#### Scenario: Only external credentials are present

- **GIVEN** a valid external `DATABASE_URL` and no `POSTGRES_PASSWORD`
- **WHEN** Sam resolves the external entry file with `docker compose config --services`
- **THEN** the command exits successfully and lists only `app`
- **AND** `docker compose config --volumes` prints only `storage`

#### Scenario: External credentials are missing or empty

- **WHEN** Sam starts external mode with `DATABASE_URL` unset or empty
- **THEN** Compose exits unsuccessfully and names `DATABASE_URL`
- **AND** no container exists under that new Compose project

#### Scenario: App restrictions match

- **WHEN** a tester resolves bundled and external configurations with valid settings
- **THEN** their app build, ports, storage target, capability restrictions, and restart policy match
- **AND** both resolve `cap_drop: [ALL]` and `security_opt: [no-new-privileges:true]`
- **AND** both app images run as UID/GID `10001:10001` and probe `/readyz`

### Requirement: External mode preserves database and storage safeguards

External mode SHALL use the existing startup, migration, readiness, and storage-pairing behavior.
It SHALL accept only PostgreSQL 18 and dedicated empty databases or compatible Sidereal databases.
It SHALL reject unrelated objects and incompatible migration history before modifying application objects or rows.
It SHALL NOT provision a database, reset existing state, or run a separate migration command.
Recreating the app SHALL preserve migration records, database pairing, storage marker bytes, and managed files.
Stopping or removing the app SHALL NOT stop, remove, or delete data from the external database service.

#### Scenario: Initialize and recreate an external instance

- **GIVEN** an empty dedicated PostgreSQL 18 database and empty managed storage
- **WHEN** Sam starts external mode, saves migration records and pairing, then recreates the app without deleting volumes
- **THEN** `/readyz` returns `200 {"status":"ready"}` before and after recreation
- **AND** the saved records, pairing, storage marker, and a tester's sentinel file remain identical
- **AND** the external database container ID and start time remain unchanged

#### Scenario: The selected database contains unrelated data

- **GIVEN** an external database containing an unrelated table and saved rows
- **WHEN** Sam starts external mode against it
- **THEN** the app exits unsuccessfully before binding HTTP
- **AND** app logs report `database is not an empty or owned Sidereal database`
- **AND** the application schema and rows equal their saved values

#### Scenario: The selected database belongs to a newer release

- **GIVEN** a disposable Sidereal database with an unknown migration version
- **WHEN** Sam starts external mode against it
- **THEN** the app exits unsuccessfully and reports incompatible migration history
- **AND** its application schema and rows remain unchanged

### Requirement: External mode permits the existing storage override

Sam SHALL be able to combine `compose.external-db.yml` with `compose.storage-bind.yml`.
The override SHALL retain its existing directory, access, marker, and pairing checks.
External mode SHALL NOT use `compose.database-bind.yml` to manage the separate service's database files.
Combining that override with the external entry file SHALL fail Compose configuration validation.

#### Scenario: An external installation selects the bundled database bind override

- **WHEN** Sam resolves the external entry file with `compose.database-bind.yml` and all required variables set
- **THEN** `docker compose config` exits unsuccessfully because the resulting `db` service has no image or build

#### Scenario: Choose managed storage independently

- **GIVEN** an existing writable directory selected through `SIDEREAL_STORAGE_PATH`
- **WHEN** Sam resolves external mode with the storage bind override
- **THEN** the app mounts that directory at `/var/lib/sidereal/storage`
- **AND** the resolved services list contains only `app`

#### Scenario: A paired external database finds empty substitute storage

- **GIVEN** a paired external database and an empty directory replacing its managed storage
- **WHEN** Sam starts external mode with that directory
- **THEN** the app refuses HTTP and reports `storage marker is missing`
- **AND** it creates no marker and preserves the database pairing

### Requirement: External transport never silently becomes insecure

External mode SHALL pass the configured connection URL without adding or weakening its transport settings.
It SHALL retain the `postgres-foundation` transport policy, certificate checks, and redacted diagnostics.
Named remote hosts SHALL default to certificate and hostname verification when `sslmode` is absent.
Certificate verification failure SHALL prevent startup without retrying plaintext.
App logs SHALL contain neither connection URLs nor credentials, including on failed login and TLS failure.

#### Scenario: Remote transport defaults to verification

- **GIVEN** a named PostgreSQL host and matching certificate whose CA is supplied through `sslrootcert`
- **AND** the service rejects plaintext connections for the app login
- **WHEN** Sam connects without `sslmode`
- **THEN** the app reaches readiness over TLS
- **AND** an explicit `sslmode=disable` control using the same login fails with `database login rejected`

#### Scenario: An untrusted CA or wrong hostname fails

- **GIVEN** saved database schema and rows
- **WHEN** external startup uses an untrusted CA or a hostname absent from the certificate
- **THEN** the app reports `database TLS verification failed` and never binds HTTP
- **AND** its application schema and rows remain unchanged
- **AND** app logs contain neither the distinctive password nor the connection URL

### Requirement: Operators can mount database certificates read-only

The repo SHALL provide the optional override `compose.database-tls.yml`.
It SHALL require `SIDEREAL_DATABASE_TLS_PATH` to select an existing host directory.
It SHALL mount that directory read-only at `/etc/sidereal/postgres` without creating a missing source directory.
Sam SHALL name certificate files using container-visible paths in `DATABASE_URL`.
The override SHALL NOT change the TLS mode, elevate privileges, or modify certificate permissions.
Without this override, the app SHALL NOT require a certificate directory setting.

#### Scenario: A private CA is available to the app

- **GIVEN** a CA file readable by UID/GID `10001:10001` in the selected directory
- **WHEN** Sam selects the TLS override and names `/etc/sidereal/postgres/ca.crt` in `sslrootcert`
- **THEN** the app reaches readiness against a correctly named certificate signed by that CA
- **AND** container inspection reports that directory mount as read-only

#### Scenario: The host certificate directory is absent

- **WHEN** Sam starts with the TLS override and a nonexistent source directory
- **THEN** Compose fails and leaves that source directory absent
- **AND** no app container exists under that new Compose project

#### Scenario: A named certificate file is unreadable

- **WHEN** the configured CA file is missing or unreadable by the app user
- **THEN** the app reports `database TLS certificate file unreadable` before opening a database connection
- **AND** the fixture's authorization log records no new authenticated connection for the app login
- **AND** app logs contain no certificate path, file content, password, or connection URL

### Requirement: The installation guide explains external operation

`docs/install.md` SHALL document entry-file selection and all external settings through `.env`.
It SHALL describe PostgreSQL 18 support and recommend current minor updates using official PostgreSQL guidance.
It SHALL explain dedicated database ownership, required privileges, automatic migrations, and matching managed storage.
It SHALL recommend a dedicated non-superuser login owning the application database and its objects.
The guidance SHALL cover `CONNECT`, schema `USAGE` and `CREATE`, migration-table access, and metadata updates for first storage pairing.
It SHALL explain that container `localhost` names the app container and that the selected database must be reachable from that container.
It SHALL describe URL encoding, literal `.env` quoting, and the credential exposure in `docker compose config` output.
It SHALL state that exported shell variables take precedence over `.env` settings.
It SHALL cover verified TLS, private CA mounts, client certificate paths, and explicit plaintext opt-in on trusted networks.
It SHALL explain that default database trust uses compiled public roots, which update only with rebuilt images.
It SHALL state that host trust stores and additions to `/etc/ssl/certs` do not change that database trust.
It SHALL require explicit `sslrootcert` configuration through the TLS override for other CAs.
It SHALL state that `allow`, `prefer`, and `require` are rejected and certificate failures never fall back to plaintext.
It SHALL explain that external database lifecycle and backups remain the operator's responsibility.
It SHALL distinguish fresh installation from moving an existing database and its matching storage.
For a fresh external database, it SHALL require a separate Compose project name or a fresh managed-storage bind directory.
It SHALL forbid reusing another installation's storage and explain that an unpaired database can adopt a marker-only directory.
It SHALL explain late-start retries, outage recovery, and app recreation after updating rotated credentials in `.env`.
It SHALL warn against combining external mode with the bundled database bind override.

#### Scenario: Follow the guide using disposable infrastructure

- **WHEN** a tester follows the guide's settings in the external verification recipe against disposable PostgreSQL 18
- **THEN** startup reaches readiness with only an app container in the installation project
- **AND** app recreation preserves migration records and storage pairing

#### Scenario: A fresh external installation uses separate storage

- **GIVEN** an existing bundled installation with saved storage marker bytes and database pairing
- **WHEN** a tester follows the fresh-install guide using a new external database and a separate project name
- **THEN** external mode creates a different storage volume and storage UUID
- **AND** the bundled installation's marker bytes and database pairing remain unchanged

### Requirement: External database availability recovers without resetting state

An unavailable external database at startup SHALL keep HTTP unbound and produce the existing redacted `database unreachable` diagnostic.
The app's restart policy SHALL retry failed startup and reach readiness once the compatible database becomes available.
A database outage after startup SHALL leave liveness available and make readiness fail.
The running app SHALL recover readiness without restart when its compatible database returns.
App logs SHALL report database readiness loss and recovery using the existing diagnostics.
Both recovery paths SHALL preserve migration records, pairing, and storage marker bytes.

#### Scenario: The external database starts late

- **GIVEN** an initialized external instance whose database service is stopped
- **WHEN** the app starts, fails a connection attempt, and the tester then starts the database service
- **THEN** app logs contain `database unreachable` from the failed attempt
- **AND** Docker records a startup retry and the app reaches `/readyz` status 200 within 180 seconds of database availability
- **AND** migration records, pairing, and marker bytes equal their saved values

#### Scenario: The external database stops during operation

- **GIVEN** a ready external instance with saved app restart count and retained records
- **WHEN** the tester stops the database for 60 seconds while probing readiness, then starts it again
- **THEN** `/healthz` stays 200 while `/readyz` returns 503 during the outage
- **AND** `/readyz` returns 200 within five seconds once PostgreSQL accepts connections and answers within the existing probe deadline
- **AND** app logs report `readiness: not ready (database unreachable)` followed by `readiness: ready`
- **AND** the app restart count, migration records, pairing, and marker bytes remain unchanged

### Requirement: Deployment verification covers external operation safely

A root verification recipe SHALL initialize and recreate external mode against its own separate disposable PostgreSQL service.
It SHALL verify the external scenarios above, including named-volume and bind-mounted managed storage.
It SHALL compare retained records and bytes and verify that app removal leaves the database service running.
It SHALL use generated test credentials, isolated project names, and scratch paths under `.workspace/`.
It SHALL ignore operator connection URLs, Compose selections, certificate paths, and storage paths from the caller's environment and root `.env`.
Cleanup SHALL remove only resources created by that verification run.

#### Scenario: Run with operator settings in the environment

- **GIVEN** distinctive operator connection URLs, Compose selections, certificate paths, and storage paths in the environment and root `.env`
- **WHEN** the external verification recipe runs
- **THEN** no verification container uses those connection URLs or mounts those paths
- **AND** operator services, settings files, and data remain unchanged
