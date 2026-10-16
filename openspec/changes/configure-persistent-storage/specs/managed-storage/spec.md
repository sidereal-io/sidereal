## Purpose

Sam can explicitly initialize managed storage and detect missing or unrelated storage before the instance serves HTTP.
Storage identity survives container replacement and changes to the host path.

## ADDED Requirements

### Requirement: Storage configuration is explicit

The server SHALL accept `STORAGE_ROOT` as an absolute path to an existing directory.
An explicitly empty or relative setting SHALL fail before database writes.
An unset setting SHALL preserve direct database-only health serving.
The initialization command SHALL require `STORAGE_ROOT` and SHALL NOT create its directory.

#### Scenario: Direct database-only operation

- **WHEN** a tester starts the server with valid database configuration and no `STORAGE_ROOT`
- **THEN** the server reaches readiness without accessing a managed storage directory

#### Scenario: Invalid storage configuration

- **WHEN** a tester sets `STORAGE_ROOT` to an empty value, relative path, missing path, or regular file
- **THEN** startup exits unsuccessfully before database writes
- **AND** startup creates no storage directory

### Requirement: Initialization pairs one store with one database

`sidereal-server storage init` SHALL create a durable marker and record the same opaque identity in PostgreSQL.
The marker SHALL be named `.sidereal-storage.json` and contain format version 1 and a UUID storage identity.
The command SHALL use the existing database migration path and SHALL open no HTTP listener.
Initialization SHALL create a new identity only when both the database pairing and marker are absent.
Before creating a marker, initialization SHALL require an otherwise empty storage root.
Initialization SHALL NOT adopt an existing marker into an unpaired database.
A completed, matching pairing SHALL make repeated initialization succeed without changing either identity.
Initialization SHALL NOT replace any existing marker or ready database pairing.

#### Scenario: First initialization

- **GIVEN** an empty writable storage directory and an unpaired dedicated database
- **WHEN** Sam runs `sidereal-server storage init`
- **THEN** the command exits successfully without starting HTTP
- **AND** the marker and ready database pairing contain the same UUID
- **AND** the root contains only the marker

#### Scenario: Repeat initialization

- **GIVEN** a completed matching pairing and saved marker and database values
- **WHEN** Sam runs initialization again
- **THEN** the command exits successfully and every saved identity value remains identical

#### Scenario: Existing unrelated contents

- **GIVEN** an unpaired database and storage containing an unrelated file
- **WHEN** Sam runs initialization
- **THEN** initialization exits unsuccessfully and preserves every existing file
- **AND** it creates no marker or pairing row

#### Scenario: Existing marked storage with a different database

- **GIVEN** valid marked storage and an unpaired database
- **WHEN** Sam runs initialization
- **THEN** initialization exits unsuccessfully without adopting the marker or inserting a pairing row

### Requirement: Interrupted initialization cannot become automatic replacement

Initialization SHALL reserve its identity in a pending database pairing before creating the marker.
It SHALL durably sync the marker and its parent directory before marking the pairing ready.
Normal startup SHALL refuse a pending pairing.
An explicit retry SHALL reuse a pending identity with an absent marker or a complete matching marker.
Marker creation SHALL remain exclusive, and an absent-marker retry SHALL require an otherwise empty root.
A retry SHALL reject malformed or mismatched markers without overwriting them.
Concurrent initialization SHALL produce at most one ready identity for a database and at most one marker per root.
Lock acquisition SHALL use the configured database startup deadline and return a redacted error when it expires.

#### Scenario: Interrupted before marker creation

- **GIVEN** a pending pairing and an empty root after an interrupted command
- **WHEN** Sam explicitly retries initialization
- **THEN** initialization creates a marker with the reserved identity and completes the pairing

#### Scenario: Interrupted after marker creation

- **GIVEN** a pending pairing and a complete matching marker
- **WHEN** Sam explicitly retries initialization
- **THEN** initialization completes the pairing without changing the identity

#### Scenario: Partial marker

- **GIVEN** a pending pairing and a truncated marker
- **WHEN** Sam retries initialization or starts the server
- **THEN** the process exits unsuccessfully and leaves marker bytes and pairing state unchanged

#### Scenario: Two initializers race

- **WHEN** two initialization commands concurrently target one database and one empty root
- **THEN** at most one marker identity exists and the database contains at most one pairing row
- **AND** every successful command observes the same ready identity

#### Scenario: Different databases race for one root

- **WHEN** two initialization commands with different dedicated databases target one empty root
- **THEN** neither command overwrites the other's marker
- **AND** at most one database reaches ready with its own newly reserved identity

### Requirement: Startup validates the paired store

Storage-enabled startup SHALL require a valid marker and a matching ready database pairing before binding HTTP.
Startup SHALL validate the local root and marker before application database initialization.
Startup SHALL NOT create a marker, repair a pairing, or initialize replacement storage.
Startup SHALL accept moved storage when its marker still matches the retained database pairing.
Missing or mismatched storage SHALL remain unavailable even when the configured container path remains unchanged.

#### Scenario: Missing disk exposes an empty directory

- **GIVEN** a paired database and an empty directory beneath the configured mount path
- **WHEN** Sam starts the server
- **THEN** startup exits unsuccessfully before serving HTTP or changing application database contents
- **AND** the directory remains empty

#### Scenario: Another store appears at the same path

- **GIVEN** a paired database and a valid marker carrying another UUID at the configured path
- **WHEN** Sam starts the server
- **THEN** startup exits unsuccessfully before serving HTTP
- **AND** the existing marker and pairing remain unchanged

#### Scenario: Storage moves intact

- **GIVEN** storage moved intact to another host directory and its retained paired database
- **WHEN** Sam selects the new bind source and recreates the app container
- **THEN** the app reaches readiness without initialization
- **AND** marker bytes and database pairing remain identical

#### Scenario: Database data is missing

- **GIVEN** a valid existing storage marker and an empty dedicated database
- **WHEN** Sam starts the server
- **THEN** startup refuses HTTP because the database has no ready pairing
- **AND** startup inserts no pairing row and leaves the storage marker unchanged

### Requirement: Marker parsing and access checks preserve operator data

The server SHALL accept only a regular, non-symlink marker no larger than 4096 bytes.
It SHALL reject malformed JSON, unsupported versions, invalid UUIDs, and unexpected fields.
Marker contents SHALL NOT select paths, commands, or database statements.
Startup and initialization SHALL check create, write, sync, and delete access through an exclusively created temporary probe.
Successful checks SHALL remove their probe and preserve existing files.
The server SHALL NOT repair ownership, broaden permissions, or elevate privileges.

#### Scenario: Invalid marker inputs

- **WHEN** the marker is a symlink, directory, oversized file, malformed JSON, unsupported version, invalid UUID, or contains unexpected fields
- **THEN** validation fails without modifying the marker or its target

#### Scenario: Storage is not writable by the app user

- **GIVEN** a root that UID/GID `10001:10001` cannot write
- **WHEN** startup or initialization checks access
- **THEN** the process exits unsuccessfully without serving HTTP or changing ownership or permissions
- **AND** it reports a storage permission error

#### Scenario: Access probe preserves existing contents

- **GIVEN** marked storage with an existing sentinel
- **WHEN** startup validates access successfully
- **THEN** the marker and sentinel retain their bytes and no probe file remains

### Requirement: Storage errors guide recovery without exposing configuration values

Server errors SHALL name the storage failure category, operation, and a safe recovery action.
They SHALL name `STORAGE_ROOT` without printing its configured value.
They SHALL NOT expose credentials, database URLs, marker contents, or raw nested errors.
Missing markers on ready stores SHALL direct operators to check their mount, not recreate the marker.
The CLI SHALL expose no force-rebind or automatic-reset option.

#### Scenario: Secret-bearing configuration fails

- **GIVEN** distinctive secrets embedded in a configured path and database password
- **WHEN** startup or initialization fails
- **THEN** server stdout and stderr contain neither secret, database URL, nor marker body
- **AND** the message identifies the failing setting or operation and a safe next action

#### Scenario: Ready store loses its marker

- **GIVEN** a ready pairing whose marker is absent
- **WHEN** Sam runs initialization
- **THEN** the command exits unsuccessfully without recreating the marker
- **AND** the message tells Sam to check the intended mount
