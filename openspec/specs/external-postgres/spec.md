# external-postgres Specification

## Purpose

Sam can connect the packaged Sidereal app to an existing PostgreSQL service through Docker Compose.
This spec owns the entry files, connection settings, mounts, and separation of app and database resources.
Server transport, startup, migrations, and readiness belong to [postgres-foundation](../postgres-foundation/spec.md).
Storage validation and pairing belong to [managed-storage](../managed-storage/spec.md).

## Requirements

### Requirement: External mode starts only the app

The repo SHALL provide `compose.external-db.yml` as an alternative entry file to `compose.yml`.
Sam SHALL select it through `COMPOSE_FILE` or explicit `-f` arguments.
Both entries SHALL extend the shared app definition in `compose.app.yml`.
Except for `DATABASE_URL` and the bundled database dependency, their resolved app settings SHALL match.
External mode SHALL NOT require `POSTGRES_PASSWORD`, define a database service, or declare a PostgreSQL data volume.
It SHALL use the image's readiness health check without a database service dependency and retain Compose 2.20 compatibility.

#### Scenario: Only external credentials are present

- **GIVEN** a valid external `DATABASE_URL` and no `POSTGRES_PASSWORD`
- **WHEN** Sam resolves the external entry with `docker compose config`
- **THEN** the command exits successfully and the services list contains only `app`
- **AND** the volumes list contains only `storage`
- **AND** the app has no database service dependency

#### Scenario: App settings match

- **WHEN** a tester resolves bundled and external configurations with valid settings
- **THEN** their complete app configurations match after excluding `DATABASE_URL` and the bundled database dependency

### Requirement: Compose requires and forwards the external connection URL

External mode SHALL require a nonempty `DATABASE_URL` during Compose configuration validation, before creating containers.
It SHALL forward the configured URL unchanged, including encoded credentials and query parameters.
It SHALL NOT add or override transport settings in that URL.

#### Scenario: External credentials are missing or empty

- **WHEN** Sam resolves external mode with `DATABASE_URL` unset or empty
- **THEN** `docker compose config` exits unsuccessfully and names `DATABASE_URL`

#### Scenario: The URL contains encoded credentials and TLS settings

- **GIVEN** a `DATABASE_URL` containing encoded credentials and certificate query parameters
- **WHEN** Sam resolves the external entry
- **THEN** the app's resolved `DATABASE_URL` equals the supplied value

### Requirement: External mode permits the existing storage override

Sam SHALL be able to combine `compose.external-db.yml` with `compose.storage-bind.yml`, including when the TLS override is selected.
The storage override SHALL mount the selected host directory at `/var/lib/sidereal/storage` without creating a missing source directory.
External mode SHALL NOT use `compose.database-bind.yml` to manage the separate service's database files.
Combining that override with the external entry SHALL fail Compose configuration validation.

#### Scenario: Choose managed storage independently

- **GIVEN** a directory selected through `SIDEREAL_STORAGE_PATH`
- **WHEN** Sam resolves external mode with the storage and TLS overrides
- **THEN** the app's storage mount resolves to that host directory at `/var/lib/sidereal/storage`
- **AND** its bind settings disable source-directory creation
- **AND** the services list contains only `app`

#### Scenario: An external installation selects the bundled database bind override

- **WHEN** Sam resolves the external entry with `compose.database-bind.yml` and all required variables set
- **THEN** `docker compose config` exits unsuccessfully because the resulting `db` service has no image or build

### Requirement: Operators can mount database certificates read-only

The repo SHALL provide the optional override `compose.database-tls.yml`.
It SHALL require a nonempty `SIDEREAL_DATABASE_TLS_PATH` and mount that host directory read-only at `/etc/sidereal/postgres`.
It SHALL disable source-directory creation and refuse a missing host directory.
Sam SHALL name certificate files using container-visible paths in `DATABASE_URL`.
The override SHALL NOT change the TLS mode, elevate privileges, or modify certificate permissions.
Without this override, the app SHALL NOT require a certificate directory setting.

#### Scenario: The certificate directory setting is missing or empty

- **WHEN** Sam resolves the TLS override with `SIDEREAL_DATABASE_TLS_PATH` unset or empty
- **THEN** `docker compose config` exits unsuccessfully and names `SIDEREAL_DATABASE_TLS_PATH`

#### Scenario: A private CA is available to the app

- **GIVEN** a readable CA file in the selected host directory and a reachable PostgreSQL service with a matching certificate signed by that CA
- **WHEN** Sam starts external mode with the TLS override and `/etc/sidereal/postgres/ca.crt` in `sslrootcert`
- **THEN** the app reaches `/readyz` status 200 against the TLS-only service
- **AND** container inspection reports the selected directory mounted read-only at `/etc/sidereal/postgres`

#### Scenario: The host certificate directory is absent

- **WHEN** Sam starts with the TLS override and a nonexistent source directory
- **THEN** Compose fails and leaves that source directory absent
- **AND** no app container exists under that new Compose project

### Requirement: App lifecycle leaves external resources and retained state intact

The app SHALL be able to join an operator-provided network to reach a separately managed PostgreSQL service.
Stopping or removing the app project SHALL leave that service and its externally owned network intact.
A `docker compose down` followed by `up`, without deleting volumes, SHALL preserve migration records, database pairing, storage marker bytes, and managed files.
Compose SHALL run no separate database-provisioning or migration command.

#### Scenario: Recreate an app connected through an external network

- **GIVEN** a separate PostgreSQL fixture project that owns its database and network, with an empty dedicated database
- **AND** an app project configured to join that network and use its own managed-storage volume
- **WHEN** the app reaches readiness, a tester saves migration rows, pairing, marker bytes, and a sentinel file, then runs app `down` and `up`
- **THEN** `/readyz` returns `200 {"status":"ready"}` again and the saved state remains identical
- **AND** the external database container ID and start time remain unchanged
- **AND** the fixture's network still exists after app removal

### Requirement: The installation guide explains Compose configuration and ownership

`docs/install.md` SHALL explain entry-file selection, `.env` settings, and network access from the app container.
It SHALL describe URL encoding, literal `.env` quoting, shell-variable precedence, and credential exposure in `docker compose config` output.
It SHALL explain the optional certificate mount and use of container-visible paths in `DATABASE_URL`.
It SHALL describe storage selection, warn against the bundled database-bind override, and distinguish fresh installation from moving an existing database with its matching storage.
It SHALL require a separate project name or fresh managed-storage bind directory for a fresh external installation.
It SHALL state that external database lifecycle and backups remain the operator's responsibility.

#### Scenario: Select the external entry and its overrides

- **WHEN** Sam follows the guide to configure external mode with storage and TLS overrides
- **THEN** Compose resolves only the app service, the supplied URL, and the documented mount targets
- **AND** the guide identifies which resources the app project manages and which remain externally managed

### Requirement: Verification covers the Compose contract safely

The regular check gate and CI SHALL run daemon-free Compose configuration checks for the entry files, URL requirements, and overrides.
A separate deployment recipe SHALL use one disposable external PostgreSQL fixture to verify the missing certificate directory refusal, a positive TLS start through the mount, and app recreation with retained state.
It SHALL use generated credentials, isolated project names, locking, and scratch paths under `.workspace/`.
It SHALL ignore operator connection URLs, Compose selections, certificate paths, and storage paths from the caller's environment and root `.env`.
Cleanup SHALL remove only resources created by that verification run.

#### Scenario: Configuration checks need no daemon

- **GIVEN** the Compose CLI and `jq`, with no reachable Docker daemon
- **WHEN** a tester runs `just check-compose`
- **THEN** configuration checks pass without creating containers, networks, or volumes

#### Scenario: Run with operator settings in the environment

- **GIVEN** operator connection URLs, Compose selections, certificate paths, and storage paths in the environment and root `.env`
- **WHEN** the external verification recipe runs
- **THEN** no verification container uses those connection URLs or mounts those paths
- **AND** operator services, settings files, and data remain unchanged
