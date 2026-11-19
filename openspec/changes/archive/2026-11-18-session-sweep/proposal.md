## Why

The server deletes an expired session only when a browser presents its token again. A session that nobody presents, such as one from a browser that was cleared or thrown away, stays in the `sessions` table for the life of the instance. This change deletes those rows on a schedule, so the table holds only live sessions.

## What Changes

- The server process sweeps the `sessions` table: it deletes every row whose expiry has passed, and nothing else (D1).
- The first sweep runs 1 minute after the server starts, then one runs every hour. The timing is fixed in code (D2).
- A sweep that deletes rows writes one plain stderr line with the count, such as `session sweep: deleted 3 expired sessions`. It is not a security event (D3).
- A sweep that cannot reach PostgreSQL writes one line that starts `database unavailable:` and waits for the next interval. The server keeps serving (D4).
- The sweep stops with the server on a graceful shutdown (D5). Only the server runs it; `siderealctl` does not (D7).
- The on-use check that deletes an expired session stays as it is.

Out of scope (D6): a general background job runner, a configurable schedule, and sweeping anything other than sessions.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `instance-access`: adds a requirement that the server deletes expired sessions every hour, with its log line and its behavior when PostgreSQL cannot be reached.

## Impact

- `server/crates/core/src/access/session.rs`: a new `Database` method that deletes expired sessions and returns the count.
- `server/crates/server/src/`: a periodic task that calls it on the schedule, writes the lines, and ends when the database closes.
- `server/crates/server/src/main.rs`: starts the task after the database is ready.
- Tests: a core test for the delete, and server tests for the schedule, the log line and an outage.
- No schema change, no new dependency, and no change to the HTTP API or to `siderealctl`.
