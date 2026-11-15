## Why

Sam cannot tell today whether someone else claimed the instance, reset its password from the box, or tried to guess a password. ADR-007 says trust-relevant events are audited, and every source of these events has now shipped: the claim and sessions, `siderealctl`, and the sign-in limit.

## What Changes

- Every security event writes one `security event:` line to stderr, as `key=value` fields with every caller-supplied value escaped (D14). The events are a claim, a sign-in that succeeds or fails, a sign-out that ends a session, and an access recovery (D6).
- A claim from the browser or from `siderealctl`, a `siderealctl reset-admin-password` reset, a sign-in that fails on wrong credentials, and an address becoming limited each also leave one row in a new `security_events` table (D1, D2). A successful sign-in and a sign-out leave no row (D1).
- Every row records its source, `http` or `siderealctl` (D7). A failed sign-in names no username, only the client address and the user agent (D9). A reset row records how many sessions the reset ended (D13).
- A claim or a reset writes its row in its own transaction. When the row cannot be written, the act fails as unavailable: 503 from the browser, exit 1 from `siderealctl` (D4). A 401 or 429 sign-in answer does not change when its row cannot be written (D5).
- Refused requests are not security events and write neither a row nor a line: a malformed sign-in body (400), a 429 after the first of a run, a refused claim (409), and a `siderealctl` create that loses the race to a browser claim (D10, D11, D12).
- **BREAKING** for anyone who watches the log: the `security event:` line replaces the `sign-in limited:` line, so each event writes exactly one line (D3).
- A forward-only migration `4_` adds the table, with no foreign keys, so a row outlives the user and session it names.
- Out of scope: a screen for the events, retention or pruning, events from later epics, and a logging framework (D15).

This narrows issue #453, whose Scope also asks for rows for a successful sign-in and a sign-out (D1).

## Capabilities

### New Capabilities

- `security-events`: which acts are security events, the row each kept event leaves and its fields, the `security event:` log line and its escaping, what happens when a row cannot be written, and which refused requests write nothing.

### Modified Capabilities

- `instance-access`: "The server warns when an address becomes limited" asks for the `security event:` line in place of `sign-in limited:`. "Failed sign-ins are counted per client address" keeps the counts out of PostgreSQL but no longer reads as forbidding the failed sign-in row.
- `postgres-foundation`: "Forward-only startup migrations" counts four migration records and lists `security_events` among the application tables, and an upgrade from three migrations adds the empty table.

## Impact

- **Migrations and schema:** a new `server/crates/core/migrations/4_security_events.sql`, a new baseline `server/crates/core/schema/4.json`, and its entry in `baselines()` in `server/crates/core/src/database/startup.rs`.
- **Core:** `server/crates/core/src/access/` gains the event record and its log line, so the server and `siderealctl` share one format. `claim` and `reset_admin_password` take the event and write it in their transaction. The failed sign-in row is a separate write.
- **Server:** the claim, sign-in and sign-out handlers and `limited()` in `server/crates/server/src/access/mod.rs` pass the client address and user agent and write the line. `siderealctl` in `server/crates/server/src/siderealctl/mod.rs` writes its line to its own stderr.
- **Tests:** `server/crates/server/tests/access.rs` and `siderealctl.rs` gain per-path row and line checks. The existing `sign-in limited:` tests change to the new line. The startup migration tests count four migrations.
- **Operators:** a log filter on `sign-in limited:` must change to `security event:`. No API, cookie or web change.
