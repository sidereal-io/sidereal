## ADDED Requirements

### Requirement: The health screen has unit tests

The web shell SHALL have unit tests for its health screen. The tests SHALL check that the screen shows `healthy` for status 200 with the body `{"status":"ok"}`. They SHALL also check that it shows `unreachable` for a status other than 200.

The tests SHALL run without a v2 server, without a browser, and without network access. They SHALL replace the screen's health request with a stub.

#### Scenario: The tests run with no server

- **WHEN** nothing listens on port 5000, and a contributor runs `pnpm test` in `web/`
- **THEN** the command exits with status 0
- **AND** the test report lists at least two passing tests for the health screen

#### Scenario: The healthy check breaks

- **WHEN** a tester changes the health screen so that status 200 with `{"status":"ok"}` shows `unreachable`
- **AND** runs `pnpm test` in `web/`
- **THEN** the command exits with a non-zero status

#### Scenario: The unreachable check breaks

- **WHEN** a tester changes the health screen so that any status shows `healthy`
- **AND** runs `pnpm test` in `web/`
- **THEN** the command exits with a non-zero status
