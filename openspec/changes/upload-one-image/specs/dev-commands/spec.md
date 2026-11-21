## ADDED Requirements

### Requirement: The end-to-end suite runs on its own throwaway stack

`mise run e2e` SHALL run the browser tests against a stack it starts for that
run only: a new database made through the fixture's test role, named with the
prefix `sidereal_test_e2e_`, a new temporary storage folder, and its own server
and web shell on ports other than 5000 and 5173. It SHALL NOT reuse a running
server or web shell, and SHALL ignore an inherited `DATABASE_URL` and
`STORAGE_ROOT`.

#### Scenario: A contributor's running stack is left alone

- **GIVEN** a contributor has `mise run dev` running against the fixture's runtime database, with a claimed instance and one uploaded image
- **WHEN** the contributor runs `mise run e2e` and it exits with status 0
- **THEN** the runtime database's users, sessions, Assets and Asset versions are the same as before the run
- **AND** the `mise run dev` server and web shell are still the processes that answer on ports 5000 and 5173

#### Scenario: An inherited database URL is ignored

- **WHEN** a contributor runs `DATABASE_URL=<the fixture's runtime URL> mise run e2e`
- **THEN** the server that the run starts uses a database whose name starts with `sidereal_test_e2e_`, not the runtime database

### Requirement: The end-to-end suite removes its stack and refuses a stack it cannot make

When a `mise run e2e` run ends, whether its tests pass or fail, it SHALL stop
the server it started, remove the storage folder it made, and drop the
database it made, attempting each step even when an earlier one failed. When
the tests pass but cleanup fails, the run SHALL exit with a non-zero status
and name what it left behind. When it cannot make the database or the folder,
it SHALL exit with a non-zero status before any browser test runs.

#### Scenario: The run cleans up after a failure

- **WHEN** a browser test fails during `mise run e2e`
- **THEN** the command exits with a non-zero status
- **AND** afterwards the fixture holds no database that the run created, and the run's storage folder no longer exists

#### Scenario: The throwaway database cannot be made

- **WHEN** the database fixture is not running and a contributor runs `mise run e2e`
- **THEN** the command exits with a non-zero status before Playwright starts any browser test

### Requirement: The end-to-end suite talks only to the server it started

When anything already accepts connections on a port that `mise run e2e`
needs, it SHALL exit with a non-zero status before any browser test runs. It
SHALL send HTTP requests to its server only after the server it started has
reported that it is listening on the chosen port.

#### Scenario: Another process holds the server port

- **GIVEN** another process already accepts connections on the server port that `mise run e2e` chooses for this checkout
- **WHEN** a contributor runs `mise run e2e`
- **THEN** the command exits with a non-zero status before Playwright starts any browser test
- **AND** the other process receives no HTTP request from the run
