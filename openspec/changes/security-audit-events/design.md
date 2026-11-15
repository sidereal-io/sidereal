## Context

- Every security event already passes through one `core` method: `Database::claim`, `sign_in`, `sign_out` and `reset_admin_password` in `server/crates/core/src/access/`. The HTTP handlers in `server/crates/server/src/access/mod.rs` and `siderealctl` in `server/crates/server/src/siderealctl/mod.rs` call them.
- `claim` inserts the administrator with one statement on the pool. `reset_admin_password` already runs one transaction that updates the hash and deletes every session.
- The server writes its log with `eprintln!`. The only access line today is `sign-in limited:`, written by `limited()` on the first 429 of a run (`Refusal::new`). `siderealctl::run` already writes to an `err: &mut impl Write` that tests capture.
- In the container image, `sidereal-server` is PID 1, and `docker compose logs app` shows its stderr. `siderealctl` runs beside it through `docker compose exec`, so its own stderr reaches the operator's terminal, not that log. Both reach the same PostgreSQL.
- The client address comes from the `Forwarded` request extension. It is `None` when the nearest untrusted `X-Forwarded-For` entry is not an address.
- `core` is domain-agnostic. Access and its audit are not astronomy, so they stay in `core`.
- Branch history: this change first kept some events as rows in a `security_events` table, through migration `4_security_events.sql`. That was dropped on 2026-11-15 (brainstorm D1) before it merged. The rework removes the migration, its baseline and every row write.

## Goals / Non-Goals

**Goals:**

- One type describes an event, and one `Display` writes its line, so the server and `siderealctl` cannot drift apart.
- Every event reaches the server's log, including one from `siderealctl`.
- A line is written only after the act takes effect.

**Non-Goals:**

- No table, no migration and no separate log file (D1).
- No logging framework. The server's other `eprintln!` lines stay as they are.
- No delivery guarantee for a `siderealctl` line while no server is listening. The command's own stderr still shows it.

## Test Seams

- **Primary test seam:** the in-process integration suites. `server/crates/server/tests/access.rs` drives the router built by `app_with_tuning`, with a capturing log sink in `Tuning` (D-5). `server/crates/server/tests/siderealctl.rs` drives `siderealctl::run` with a `Scripted` terminal and captured `out` and `err`, and runs the relay (D-10) against the same database with a capturing sink.
- **Existing seams reused:** the `spawned` server in `tests/access.rs`, for one test that a line reaches the real process stderr. The `core` unit tests in `access/claim.rs` and `access/recovery.rs`, for the returned event and the notification sent with the act.

## Scrutiny Tier & Blast Radius

- **Tier:** medium. A new log contract operators may filter on, and a notification sent inside the claim and reset transactions. No schema change.
- **Blast radius:** `core::access` (claim, reset, a new event module), the server's access handlers, `Tuning` and start-up, and `siderealctl`'s create and reset paths. The web interface, the schema, and the cookie and session contracts are unchanged.

## Decisions

### D-1: One `SecurityEvent` value in `core` makes the line

`core::access` gains a `security_event` module with:

- `SecurityEvent { kind, outcome, source, username: Option<String>, sessions_ended: Option<u64>, retry_after: Option<u64> }`;
- `Kind` (`InstanceClaim`, `SignIn`, `SignOut`, `AccessRecovery`) and `Outcome` (`Success`, `WrongCredentials`, `Limited`), each with `as_str`;
- `Source::Http { client_address: Option<IpAddr>, user_agent: Option<String> }` and `Source::Siderealctl`. The enum makes "a `siderealctl` event names no client" a fact of the type.

Its `Display` writes the whole line, `security event:` and the fields in the spec's order, leaving out each `None`. Both the server and `siderealctl` print `event.to_string()`, so the format exists once.

Alternative: format the line in the server and in `siderealctl` separately. Rejected: two formats drift, and the forge-proof escaping must be right in both.

### D-2: Escape `username` and `user_agent` as quoted strings

A quoted value writes `\` as `\\`, `"` as `\"`, and every `char::is_control()` character as `\u{<hex>}`. Rust's `char::escape_unicode` gives that form. Every other character, non-ASCII included, is written as is. `client_address`, `kind`, `outcome`, `source` and the numbers are never caller-supplied, so they are written bare.

Alternative: `{:?}` (Rust `Debug`) for strings. Rejected: it writes `\n` and `\t` rather than one rule for all control characters, and its output is not a contract Rust promises to keep.

### D-3: The user agent is read once, in the server

A `user_agent(&HeaderMap) -> Option<String>` helper in the server's access module reads the first `User-Agent` value. It turns the bytes into text with `String::from_utf8_lossy` and keeps the first 256 characters. The value goes into `Source::Http`.

### D-4: The claim and the reset return their event

- `Database::claim(username, password, source)` inserts the user with one statement, as before this change, and returns `Claimed { user, event }`. A unique violation stays `ClaimError::Claimed`, with no event.
- `Database::reset_admin_password(password)` returns `Reset` with an `access_recovery` event, `Source::Siderealctl` and `sessions_ended`, from its existing transaction.
- When the source is `Source::Siderealctl`, the method also sends the line with `pg_notify` inside the act (D-10). The claim then runs its insert and the notification in one transaction. A browser claim sends nothing, because the server writes its line itself.

The handler and `siderealctl` print the returned event once the call returns `Ok`. A failed call returns no event, so no line can be printed for an act that did not happen.

### D-5: The server's log is a sink in `Tuning`

`Tuning` gains `log: Log`. `Log` wraps an `Arc<dyn Fn(&str) + Send + Sync>`, and `Log::default()` writes the line and a line feed to stderr with `eprintln!`. `access::routes` adds it as an `Extension`, as it does the limiter. In-process tests pass a sink that pushes into an `Arc<Mutex<Vec<String>>>`.

Alternative: assert lines only through the spawned server. Rejected: each line test would start a process, and the 10-at-once test needs the in-process router.

### D-6: A failed sign-in writes its line before it answers

The sign-in handler builds a `wrong_credentials` event for a 401, after the password check, and writes its line before it answers. Known and unknown usernames build the same event, so nothing in the line or the timing reveals which it was.

For the first 429 of a run (`Refusal::new`), on both paths that refuse, `refused` before the body is read and `admit`, `limited()` builds a `limited` event, writes its line, and returns the 429 error. It touches no database. A 429 that is not new writes nothing, as today.

### D-7: A successful sign-in and a sign-out write their line

`sign_in` writes a `sign_in`/`success` line with the matched user's username after `attempt.succeeded()`. `sign_out` takes the guard's `Extension<User>` and writes a `sign_out`/`success` line only when `Database::sign_out` answers `true`.

### D-8: `siderealctl` prints its line right after the `core` call succeeds

In `reset_admin_password` in `siderealctl/mod.rs`, the `Ok` arm of each call writes `writeln!(err, "{event}")` before the `Generated password:` line. A failed stdout write that follows still leaves the line in place. A lost race (`ClaimError::Claimed`) has no event and writes only its refusal line. The server's copy of the line comes from D-10, not from this print.

D-9, the `security_events` migration, was removed with the rows.

### D-10: A `siderealctl` line reaches the server through PostgreSQL

- `core` sends a `siderealctl` event's line with `SELECT pg_notify('sidereal_security_event', $1)` inside the act's transaction. PostgreSQL delivers a notification only when its transaction commits, so a rolled-back act sends nothing.
- `core` offers `Database::security_event_lines()`, a `sqlx::postgres::PgListener` on that channel. The server starts one task at start-up, after migrations and before it serves HTTP. The task writes each payload to the `Log` sink of D-5.
- The task writes a payload only when it starts with `security event:` and holds no control character. It drops any other payload, so a stray `NOTIFY` on the channel cannot forge a line.
- A payload is at most about 700 bytes: a username is at most 64 characters, each escaped to at most 9 bytes, and a `siderealctl` line has no user agent. PostgreSQL's limit is 8000 bytes.

Alternative: `siderealctl` writes to `/proc/1/fd/2`, the server's stderr. Rejected: it depends on the server being PID 1 in the same container, and outside a container it would write to init. Alternative: `siderealctl` calls a local HTTP route on the server. Rejected: it needs a new unauthenticated or secret-bearing route for a command that already reaches the database.

## Risks / Trade-offs

- [Operators filtering on `sign-in limited:` lose their match] → The proposal marks the change **BREAKING**. The sign-in limits section of `docs/install.md` names the new line in place of the old, and the pull request description names the new prefix.
- [A `siderealctl` line is lost while the server is stopped, or while its listener reconnects] → The command's own stderr always shows it. `PgListener::recv` reconnects on its own, and the gap is a dropped connection, not a normal state.
- [The relay holds one more PostgreSQL connection] → One connection for the server's life, outside the pool's count.
- [Security events leave no durable record] → Accepted by D1. The log is the trace, and Docker's log rotation decides how long it lasts. ADR-007's amendment records this.
- [A claim from the box now runs a transaction] → The unique index still decides the race on the user insert, before the notification. The core test of ten concurrent claims stays.

## Migration & Rollback

- **Deploy:** no migration. The schema stays at three migrations.
- **Rollback:** code only. A previous binary runs against the same database.
- **Development databases:** a database that applied this branch's earlier migration 4 is refused by the new binary as unknown history. Recreate it, for example with `just db-clean` then `just db-up`, or a fresh volume. No release shipped migration 4.
