## MODIFIED Requirements

### Requirement: The home screen shows the server's health

The web shell's home screen SHALL check the server's readiness at `/readyz`.
It SHALL show exactly one of four health states:

- **checking**, before the first health check finishes;
- **healthy**, when the last check got status 200 and a JSON body whose `status` field is `"ready"`;
- **not ready**, when the last check got status 503 and a JSON body whose `status` field is `"not_ready"`;
- **unreachable**, when the last check got any other result, including a network error, another status, a body that is not that JSON, or no answer within 5 seconds.

The screen SHALL show the state's name as visible text. The screen SHALL NOT show any text taken from the server's response.
The `not ready` state SHALL say only that the server cannot use its database. It SHALL NOT name a cause or suggest that any other feature exists.

#### Scenario: The server is running

- **WHEN** the server is ready and a user opens the web shell's home page
- **THEN** the page shows the text `healthy` within 5 seconds

#### Scenario: The server's database is unusable

- **WHEN** the server is running, its database is stopped, and a user opens the web shell's home page
- **THEN** the page shows the text `not ready` within 5 seconds

#### Scenario: The server is not running

- **WHEN** nothing listens on the server's port and a user opens the web shell's home page
- **THEN** the page shows the text `unreachable` within 5 seconds

#### Scenario: The server answers with an unexpected body

- **WHEN** a stub on the server's port answers `GET /readyz` with status 200 and the body `{"status":"<b>degraded</b>"}`
- **AND** a user opens the web shell's home page
- **THEN** the page shows the text `unreachable`
- **AND** the page text does not contain `degraded`

#### Scenario: The server does not answer

- **WHEN** a stub on the server's port accepts connections to `GET /readyz` but never answers
- **AND** a user opens the web shell's home page
- **THEN** the page shows the text `unreachable` within 10 seconds

### Requirement: The home screen keeps checking while it is open

The home screen SHALL check the server's health again 5 seconds after each check finishes. It SHALL keep checking for as long as the page is open. It SHALL update the shown state after each check, with no reload.

#### Scenario: The server starts after the page opens

- **WHEN** the page shows `unreachable`, and the server then starts
- **THEN** the page shows `healthy` within 10 seconds of the server listening, without a reload

#### Scenario: The server stops while the page is open

- **WHEN** the page shows `healthy`, and the server then stops
- **THEN** the page shows `unreachable` within 10 seconds, without a reload

#### Scenario: The database recovers while the page is open

- **WHEN** the page shows `not ready`, and the server's database then accepts connections again
- **THEN** the page shows `healthy` within 15 seconds, without a reload

### Requirement: The web shell reaches the server through its own origin

In development, the web shell SHALL request the server's readiness at `/readyz` on its own origin. The web shell's dev server SHALL forward `/healthz` and `/readyz` to the server at `http://localhost:5000`. The server therefore needs no cross-origin settings for the web shell.

#### Scenario: The browser asks the web shell's origin

- **WHEN** a user opens the web shell's home page at `http://localhost:5173`
- **THEN** every health request the browser sends goes to `http://localhost:5173/readyz`

#### Scenario: The dev server forwards to the server

- **WHEN** the server and the web shell are both running
- **AND** a tester runs `curl -s http://localhost:5173/healthz` and `curl -s http://localhost:5173/readyz`
- **THEN** the outputs are `{"status":"ok"}` and `{"status":"ready"}`

### Requirement: The health screen has unit tests

The web shell SHALL have unit tests for its health screen. The tests SHALL check that the screen shows `healthy` for status 200 with the body `{"status":"ready"}`. They SHALL check that it shows `not ready` for status 503 with the body `{"status":"not_ready"}`. They SHALL also check that it shows `unreachable` for any other status.

The tests SHALL run without a server, without a browser, and without network access. They SHALL replace the screen's health request with a stub.

#### Scenario: The tests run with no server

- **WHEN** nothing listens on port 5000, and a contributor runs `pnpm test` in `web/`
- **THEN** the command exits with status 0
- **AND** the test report lists at least three passing tests for the health screen

#### Scenario: The healthy check breaks

- **WHEN** a tester changes the health screen so that status 200 with `{"status":"ready"}` shows `unreachable`
- **AND** runs `pnpm test` in `web/`
- **THEN** the command exits with a non-zero status

#### Scenario: The not-ready check breaks

- **WHEN** a tester changes the health screen so that status 503 with `{"status":"not_ready"}` shows `unreachable`
- **AND** runs `pnpm test` in `web/`
- **THEN** the command exits with a non-zero status

#### Scenario: The unreachable check breaks

- **WHEN** a tester changes the health screen so that any status shows `healthy`
- **AND** runs `pnpm test` in `web/`
- **THEN** the command exits with a non-zero status
