## MODIFIED Requirements

### Requirement: Retained fixture data and database regression tests

Root recipes SHALL provide a PostgreSQL fixture with retained data.
Stopping the fixture SHALL preserve its volume. Only an explicitly named cleanup recipe SHALL delete fixture data.
Database integration tests SHALL run in the server gate and SHALL fail when their database prerequisite is unavailable.
When a database integration test ends, its test database SHALL be dropped, even while a PostgreSQL autovacuum worker is processing it, using a test role that holds only `CREATEDB`.

#### Scenario: Fixture stop preserves records
- **WHEN** a contributor stops and starts the database fixture without invoking its cleanup recipe
- **THEN** the fixture retains identical migration records

#### Scenario: Database tests cannot run
- **WHEN** the server gate cannot reach its configured test database
- **THEN** the gate exits nonzero instead of skipping database tests

#### Scenario: Test configuration is missing
- **WHEN** a contributor runs the server gate without `TEST_DATABASE_URL`
- **THEN** the gate exits nonzero and identifies the missing test configuration

#### Scenario: Autovacuum is processing a test database at cleanup
- **WHEN** a database integration test's assertions pass and an autovacuum worker is processing its test database as the test ends
- **THEN** the test passes and its test database no longer exists

#### Scenario: A test database cannot be dropped
- **WHEN** a test database still cannot be dropped after cleanup's bounded retries
- **THEN** the test fails with a message that names the fixture-only orphan cleanup
