## MODIFIED Requirements

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
