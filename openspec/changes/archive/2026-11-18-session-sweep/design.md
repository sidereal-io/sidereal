## Context

- `Database::session` in `server/crates/core/src/access/session.rs` deletes an expired row when its token is presented. `session_alive` and `sign_out` leave expired rows alone or delete only their own. Its doc comment already says "A periodic sweep removes the ones nobody presents again."
- The server has one background task today: the security event `relay` in `server/crates/server/src/access/mod.rs`. It is spawned from `main.rs` after startup, writes through a `Log`, writes its own `database unavailable:` line with `eprintln!`, and ends when `Database::is_closed()` answers true.
- `main.rs` closes the database after `axum::serve` returns from a graceful shutdown, then returns from `#[tokio::main]`. Dropping the runtime cancels every spawned task, so a task asleep for an hour does not hold the process open.
- `Database::pool()` is the operational pool, `pub(crate)` to `core`. A server-crate task cannot run SQL itself; it calls a `Database` method.
- The `sessions` table has no index on `expires_at`. One administrator holds a handful of rows.

## Goals / Non-Goals

**Goals:**

- One SQL statement that deletes every expired row and returns how many it deleted.
- One server task that runs it on a fixed schedule and writes the two lines the spec names.
- A schedule that tests can shorten without making it configurable at run time.

**Non-Goals:**

- A reusable job runner or scheduler abstraction (D6).
- An index on `expires_at`. The table stays small.
- Changing the on-use delete in `Database::session`.

## Test Seams

- **Primary test seam:** the `sweep` task, driven in process from the server crate's integration tests against a `TestDatabase`. A test passes a short `Schedule` and a capturing `Log`, then reads the `sessions` table and the captured lines. This covers deletion, the hourly repeat, the two log lines, and the outage.
- **Existing seams reused:**
  - `server/crates/server/tests/siderealctl.rs`: the `Lines` capture pattern for a `Log`, and `ALTER DATABASE … CONNECTION LIMIT 0` with backend termination for an outage.
  - `server/crates/core/src/access/session.rs` tests: `claimed`, `on_sessions` and `sessions` helpers for the delete itself.
  - `server/crates/server/tests/access.rs`: the spawned-binary `Server::terminate` harness for the shutdown scenario.

The scenarios that name 1 minute and 1 hour are proved in two parts. The task test proves the order of events with a short schedule. A unit test proves `Schedule::HOURLY` is 60 seconds then 3600 seconds, and `main.rs` passes `Schedule::HOURLY`.

## Scrutiny Tier & Blast Radius

- **Tier:** medium
- **Blast radius:** `core::access::session` gains one method; `server::access` gains one task; `main.rs` starts it. The task deletes rows from `sessions` only. A wrong `WHERE` clause would sign the administrator out early; it cannot grant access. No schema, route, `plugin-abi` or `siderealctl` change.

## Decisions

### The delete is a `Database` method in `core`

`Database::sweep_sessions(&self) -> Result<u64, Unavailable>` runs `DELETE FROM public.sessions WHERE expires_at <= now()` and returns `rows_affected()`. It sits beside the other session queries, uses the operational pool, and uses the same `expires_at <= now()` test as `Database::session`, so the two agree on what "expired" means (D1).

Alternative: make `pool()` public and run the SQL from the server crate. Rejected: it spreads session SQL across two crates and widens `core`'s surface for one query.

### The task lives in the server crate, beside `relay`

A new `server/crates/server/src/access/sweep.rs` holds `pub fn sweep(database: Database, schedule: Schedule, log: Log) -> JoinHandle<()>`. `main.rs` calls it once, after `relay` succeeds and before the listener binds. `siderealctl` never calls it (D7). Scheduling belongs to the process, not to the reusable `core` library that `siderealctl` also links.

### A `Schedule` value fixes the timing in code

`pub struct Schedule { pub first: Duration, pub every: Duration }`, with `pub const HOURLY: Schedule = Schedule { first: 60 s, every: 3600 s }` (D2). Nothing reads it from the environment. Tests pass milliseconds.

Alternative: `tokio::time::pause` with the real constants. Rejected: a paused clock advances whenever the runtime is idle, including while a query waits on PostgreSQL, so pool and test timeouts would fire early and make the database tests flaky.

### The loop uses `interval_at` with `MissedTickBehavior::Delay`

`tokio::time::interval_at(Instant::now() + first, every)`, then `tick().await`, sweep, repeat. `Delay` means a slow sweep, a long outage or a suspended host gives one late sweep, not a burst of catch-up sweeps.

### The task writes both of its lines through `Log`

The count line and the `database unavailable:` line go through the `Log` it is given, so a test captures both. This needs a crate-visible way to write a plain line through `Log`; `relay` already calls `(log.0)(&line)` for its relayed lines. The `Log` doc comment widens from "one line for each security event" to "the server's log lines". The count line is not built from `SecurityEvent`, so it can never take the `security event:` prefix (D3).

Alternative: `eprintln!` as `relay` does for its outage line. Rejected: the spec's outage scenario asserts the line, and a spawned binary would need an hour to reach a second sweep.

### Each failure writes one line, and shutdown writes none

When `sweep_sessions` answers `Unavailable`, the task first checks `database.is_closed()`. If the database is closed, a graceful shutdown is under way, and the task returns without writing a line. Otherwise it writes `database unavailable: cannot sweep expired sessions; trying again in an hour` and waits for the next tick (D4). The task also returns at the top of each tick when the database is closed (D5).

### Line wording

`session sweep: deleted <n> expired sessions`, with `session` when `<n>` is 1. Nothing is written when `<n>` is 0 (D3). The outage line follows the relay's pattern: the `database unavailable:` prefix, what failed, then what happens next. The ledger fixed the prefix and the count. The exact remaining words and the singular form are this design's choice.

## Risks / Trade-offs

- [The `main.rs` wiring is not covered by a test that waits a real minute.] → The wiring is one call with `Schedule::HOURLY`, a unit test pins the constant, and the SIGTERM test starts the real binary with the task running.
- [A sweep during an outage waits for the operational pool's acquire timeout before it fails.] → It runs in its own task, so no request waits on it. `Delay` stops a late tick from bunching up.
- [Deleting a row while its events socket is open.] → The socket's 30-second recheck already closes on a missing or expired row, so the sweep changes nothing there.
- [A full-table scan for each sweep.] → Accepted. One administrator holds few rows; an index can come with a multi-user release.

## Migration & Rollback

N/A. No schema or data shape changes. Rolling back the code stops the sweep; expired rows then wait for their token again, as they do today.
