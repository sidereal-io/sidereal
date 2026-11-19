## ADDED Requirements

### Requirement: The server sweeps expired sessions every hour

The server SHALL delete every session row whose expiry has passed, and no other row, in a sweep. The first sweep SHALL run 1 minute after the server starts, and a sweep SHALL run every hour after that. Neither time SHALL be configurable. A sweep SHALL delete expired rows whether or not any browser presents their tokens again. `siderealctl` SHALL NOT sweep. A graceful shutdown SHALL NOT wait for the next sweep.

#### Scenario: An expired session nobody presents is deleted
- **WHEN** the server starts against a database that holds one session row whose expiry is one minute in the past and one unexpired session row, and 1 minute passes
- **THEN** the `sessions` table holds only the unexpired row

#### Scenario: A session that expires later goes at the next hourly sweep
- **WHEN** the first sweep has run, a session row's expiry is then set to one minute in the past, and one more hour passes
- **THEN** that row is gone from the `sessions` table

#### Scenario: No sweep before the first minute
- **WHEN** the server starts against a database that holds one session row whose expiry is one minute in the past, and 30 seconds pass
- **THEN** the row is still in the `sessions` table

#### Scenario: Shutdown before the first sweep
- **WHEN** the server receives `SIGTERM` before its first sweep
- **THEN** the process exits within 5 seconds with status 0

### Requirement: A sweep that deletes sessions writes one line

A sweep that deletes at least one row SHALL write one line to the server's stderr: `session sweep: deleted <n> expired sessions`, where `<n>` is the number of rows deleted. When it deletes one row, the line SHALL be `session sweep: deleted 1 expired session`. A sweep that deletes nothing SHALL write nothing. The line SHALL NOT be a security event: it SHALL NOT start with `security event:`, and it SHALL NOT name a user or hold any part of a token.

#### Scenario: Three expired sessions
- **WHEN** a sweep runs while the `sessions` table holds three rows whose expiry has passed
- **THEN** captured stderr gains exactly one line, `session sweep: deleted 3 expired sessions`

#### Scenario: One expired session
- **WHEN** a sweep runs while the `sessions` table holds one row whose expiry has passed
- **THEN** captured stderr gains exactly one line, `session sweep: deleted 1 expired session`

#### Scenario: Nothing to delete
- **WHEN** a sweep runs while every session row is unexpired
- **THEN** captured stderr gains no line

### Requirement: A sweep that cannot reach the database tries again at the next hour

When a sweep cannot read or write PostgreSQL, the server SHALL write one line to its stderr: `database unavailable: cannot sweep expired sessions; trying again in an hour`. The server SHALL run the next sweep at its usual time. A failed sweep SHALL NOT stop the server or change what any route answers.

#### Scenario: PostgreSQL is down at a sweep
- **WHEN** PostgreSQL cannot be reached when a sweep runs
- **THEN** captured stderr gains exactly one line, `database unavailable: cannot sweep expired sessions; trying again in an hour`, and `GET /healthz` still answers 200

#### Scenario: PostgreSQL is back at the next sweep
- **WHEN** a sweep fails because PostgreSQL cannot be reached, PostgreSQL is reachable again, a session row's expiry has passed, and one more hour passes
- **THEN** that row is gone from the `sessions` table
