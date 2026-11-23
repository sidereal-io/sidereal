<!-- Plain language: Name the actor, condition and observable result. Explain necessary domain terms using the agreed glossary. Keep requirement and scenario headings and WHEN/THEN syntax exactly as required. -->

## MODIFIED Requirements

### Requirement: Forward-only startup migrations

The server SHALL apply pending migrations automatically before serving HTTP.
It SHALL accept only successful migration history matching a prefix of the bundled history.
It SHALL reject unknown versions, changed migration checksums, missing history entries, or mismatched schema objects before writes.
The server SHALL never reset the database or execute downgrade migrations.

#### Scenario: First startup
- **WHEN** Sam starts the server against an empty dedicated database
- **THEN** the database contains the Sidereal identity row and five successful migration records
- **AND** its only application tables are `sidereal_metadata`, `_sqlx_migrations`, `users`, `sessions`, `assets`, `asset_versions`, `facet_schemas`, `facet_values` and `operation_runs`

#### Scenario: Repeated startup
- **WHEN** Sam restarts against an initialized database
- **THEN** migration records, including their timestamps and checksums, remain identical

#### Scenario: Incompatible history or schema
- **WHEN** a fixture contains a newer version, changed checksum, missing history entry, failed record, or altered expected table
- **THEN** startup fails and database objects and rows remain unchanged

#### Scenario: Upgrade preserves the original migration

- **GIVEN** a database with the original foundation migration and saved metadata
- **WHEN** the new binary applies the storage pairing migration
- **THEN** the original migration record and metadata remain identical
- **AND** the existing metadata row gains a null storage UUID before storage initialization

#### Scenario: Upgrade adds the access tables
- **WHEN** a database holding the first two migrations starts under the new binary
- **THEN** the first two migration records remain identical, a third successful record exists, and the `users` and `sessions` tables are empty

#### Scenario: Upgrade adds the asset tables
- **WHEN** a database holding the first three migrations, with a claimed user, starts under the new binary
- **THEN** the first three migration records remain identical, a fourth successful record exists, the `assets` and `asset_versions` tables are empty, and the user row is unchanged

#### Scenario: Upgrade adds the facet tables
- **WHEN** a database holding the first four migrations, with a claimed user and one Asset and its version, starts under the new binary
- **THEN** the first four migration records remain identical, a fifth successful record exists, and the `facet_schemas`, `facet_values` and `operation_runs` tables are empty
- **AND** the user, Asset and version rows are unchanged

### Requirement: Each migration runs in one transaction

Each bundled migration SHALL run in one transaction, including its essential migration record fields.
The initial transaction SHALL also create the migration table and identity row.
The server SHALL reject migrations that disable transactions before applying them.
Concurrent server startups SHALL apply each migration exactly once.

#### Scenario: Migration failure
- **WHEN** a migration fails partway through its transaction
- **THEN** startup fails and that migration leaves no committed schema changes or successful migration record

#### Scenario: Concurrent startup
- **WHEN** two servers start against the same empty database
- **THEN** both complete startup and exactly five successful migration records exist
