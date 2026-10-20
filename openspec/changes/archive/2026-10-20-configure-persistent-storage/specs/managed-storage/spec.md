## Purpose

Sam can initialize managed storage during startup and detect missing storage using its retained PostgreSQL pairing.
The marker identifies the store independently of its host path.

## ADDED Requirements

### Requirement: Storage configuration is explicit

The server SHALL accept `STORAGE_ROOT` as an absolute path to an existing directory.
An explicitly empty or relative setting SHALL fail before database writes.
An unset setting SHALL preserve direct database-only health serving.
The server SHALL NOT create the storage root directory.

#### Scenario: Direct database-only operation

- **WHEN** a tester starts the server with valid database configuration and no `STORAGE_ROOT`
- **THEN** the server reaches readiness without accessing a managed storage directory

#### Scenario: Invalid storage configuration

- **WHEN** `STORAGE_ROOT` is empty, relative, absent from disk, or a regular file
- **THEN** startup exits unsuccessfully before database writes and creates no storage directory

### Requirement: Startup initializes only unpaired storage

Storage-enabled startup SHALL evaluate the database UUID and directory contents before serving HTTP.
A paired database SHALL require a valid marker carrying the same UUID.
An unpaired database with an empty root SHALL create and durably sync a marker before persisting its UUID.
An unpaired database with only a valid marker SHALL persist that marker's UUID after syncing the marker and root directory.
Startup SHALL reject all other unpaired states and every paired state with a missing, malformed, or mismatched marker.
Startup SHALL NOT replace an existing marker or database UUID.
Repeated successful startup SHALL preserve both identities.
The server SHALL expose no separate storage initialization command or HTTP initialization endpoint.

#### Scenario: First startup

- **GIVEN** an empty writable storage root and an unpaired dedicated database
- **WHEN** Sam starts the instance
- **THEN** the app reaches readiness
- **AND** the marker and database contain the same UUID
- **AND** the root contains only the marker

#### Scenario: Interrupted first install leaves only a marker

- **GIVEN** an unpaired database and a root containing only a valid marker
- **WHEN** Sam starts the instance
- **THEN** startup persists that marker's UUID and reaches readiness without changing marker bytes

#### Scenario: Unpaired storage contains other files

- **GIVEN** an unpaired database and a root containing any file other than a valid marker
- **WHEN** Sam starts the instance
- **THEN** startup refuses HTTP, preserves existing files, and leaves the database UUID null

#### Scenario: Paired storage loses its marker

- **GIVEN** a paired database and an empty directory at the selected storage target
- **WHEN** Sam starts the instance
- **THEN** startup refuses HTTP and creates no file
- **AND** the database UUID remains unchanged

#### Scenario: Paired storage has another marker

- **GIVEN** a paired database and a valid marker carrying another UUID
- **WHEN** Sam starts the instance
- **THEN** startup refuses HTTP and preserves both existing identities

#### Scenario: Normal restart

- **GIVEN** a matching pairing and saved marker bytes and database UUID
- **WHEN** Sam recreates the containers
- **THEN** the app reaches readiness with the same marker bytes and UUID

### Requirement: Initialization remains safe across interruption and concurrent startup

The server SHALL sync the marker and parent directory before committing the database UUID.
Startup SHALL serialize pairing for one database and SHALL create markers exclusively without overwriting existing files.
Concurrent successful starts against one database SHALL observe the same UUID.
A failed or interrupted pairing SHALL leave the UUID null unless a concurrent successful startup committed it.
A subsequent startup SHALL recover a complete marker-only state without changing its identity.
A malformed partial marker SHALL cause refusal rather than automatic replacement.

#### Scenario: Marker write completes before database commit

- **GIVEN** a complete synced marker and a null database UUID after an interrupted startup
- **WHEN** startup runs again
- **THEN** it persists the same UUID and reaches readiness

#### Scenario: Marker write is incomplete

- **GIVEN** a truncated marker and an unpaired database
- **WHEN** startup runs again
- **THEN** it refuses HTTP and preserves the marker bytes and null UUID

#### Scenario: Concurrent starts

- **WHEN** two servers start concurrently against one unpaired database and one empty storage root
- **THEN** every successful startup observes the same marker UUID and database UUID
- **AND** the storage root contains at most one marker

### Requirement: Marker parsing permits compatible extensions

The marker SHALL be `.sidereal-storage.json` with required `format_version: 1` and UUID `storage_id` fields.
The server SHALL ignore unknown fields and reject unsupported versions, malformed JSON, or invalid required fields.
It SHALL accept only a regular non-symlink marker no larger than 4096 bytes.
Marker contents SHALL NOT select paths, commands, or database statements.

#### Scenario: Compatible additional field

- **GIVEN** a valid version 1 marker containing an unknown field and a matching database UUID
- **WHEN** startup validates the marker
- **THEN** startup accepts the marker without changing its bytes

#### Scenario: Invalid marker

- **WHEN** the marker is a symlink, directory, FIFO, socket, device file, oversized file, malformed JSON, or has unsupported or invalid required fields
- **THEN** startup refuses HTTP and leaves marker bytes and any symlink target unchanged

### Requirement: Storage access checks preserve operator data

The server SHALL validate create, write, sync, and delete access as the app user.
The server SHALL run the access probe only after pairing succeeds, while holding startup serialization.
Each probe SHALL use a unique name.
It SHALL use exclusive creation and remove its probe after successful validation.
A pairing refusal SHALL create no file.
A later probe failure SHALL NOT clear the committed pairing UUID.
Access failure SHALL prevent HTTP serving and leave existing storage files unchanged.
The server SHALL NOT change ownership, broaden permissions, or elevate privileges.

#### Scenario: Storage permission failure

- **GIVEN** storage that UID/GID `10001:10001` cannot write
- **WHEN** startup checks storage access
- **THEN** startup refuses HTTP and reports the failed operation
- **AND** storage ownership, permissions, and existing files remain unchanged

#### Scenario: Leftover probe

- **GIVEN** a paired store with a matching marker and a leftover probe file
- **WHEN** Sam starts the instance
- **THEN** the app reaches readiness and the leftover file remains unchanged

### Requirement: Storage diagnostics identify the failed path

Storage errors SHALL name the container-visible path, failing operation, and a safe recovery action.
The server SHALL escape control characters when rendering paths in logs.
It SHALL continue redacting database credentials, connection URLs, marker bodies, and raw nested database errors.
A paired database with no marker SHALL receive mount-recovery guidance rather than instructions to create a replacement marker.

#### Scenario: Missing paired marker

- **GIVEN** a paired database and a missing marker
- **WHEN** startup fails
- **THEN** server output includes the escaped storage path and mount-recovery guidance
- **AND** it contains neither the configured database password nor its connection URL
