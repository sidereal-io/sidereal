## Context

- Every security event already passes through one `core` method: `Database::claim`, `sign_in`, `sign_out` and `reset_admin_password` in `server/crates/core/src/access/`. The HTTP handlers in `server/crates/server/src/access/mod.rs` and `siderealctl` in `server/crates/server/src/siderealctl/mod.rs` call them.
- `claim` inserts the administrator with one statement on the pool. `reset_admin_password` already runs one transaction that updates the hash and deletes every session.
- The server writes its log with `eprintln!`. The only access line today is `sign-in limited:`, written by `limited()` on the first 429 of a run (`Refusal::new`). `siderealctl::run` already writes to an `err: &mut impl Write` that tests capture.
- The client address comes from the `Forwarded` request extension. It is `None` when the nearest untrusted `X-Forwarded-For` entry is not an address.
- Migrations are forward-only and checked against committed schema baselines (`server/crates/core/schema/<n>.json`, listed in `baselines()`). `siderealctl` refuses a database whose schema is not the current one.
- `core` is domain-agnostic. Access and its audit are not astronomy, so they stay in `core`.

## Goals / Non-Goals

**Goals:**

- One type describes an event. The row and the line are both made from it, so they cannot drift apart.
- A claim and a reset commit their row with the act or not at all.
- The line is written by the caller after the act takes effect, never by `core`.

**Non-Goals:**

- No index on `security_events`: nothing reads the table yet. The screen story adds the index its queries need.
- No logging framework. The server's other `eprintln!` lines stay as they are.
- No session id column. Issue #453 asked for one, but no event that keeps a row names a session once D1 cut the sign-in and sign-out rows.

## Test Seams

- **Primary test seam:** the in-process integration suites. `server/crates/server/tests/access.rs` drives the router built by `app_with_tuning`, with a capturing log sink in `Tuning` (D-5), and reads `security_events` over its own connection. `server/crates/server/tests/siderealctl.rs` drives `siderealctl::run` with a `Scripted` terminal and captured `out` and `err`. Its `before_read` hook installs a refusing trigger while the command waits, after the command's schema check.
- **Existing seams reused:** the `spawned` server in `tests/access.rs`, for one test that a line reaches the real process stderr. The `core` unit tests in `access/claim.rs` and `access/recovery.rs`, for the transaction holding the row. The schema baseline test in `core/src/database/startup/tests.rs`, regenerated with `SIDEREAL_GENERATE_SCHEMA=1`.

## Scrutiny Tier & Blast Radius

- **Tier:** high. A new migration, a new log contract operators may filter on, and a change inside the claim and reset transactions.
- **Blast radius:** `core::access` (claim, reset, a new event module), the server's access handlers and `Tuning`, `siderealctl`'s create and reset paths, the migration history and its baselines. The web interface and the cookie and session contracts are unchanged.

## Decisions

### D-1: One `SecurityEvent` value in `core` makes both the row and the line

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

A `user_agent(&HeaderMap) -> Option<String>` helper in the server's access module reads the first `User-Agent` value. It turns the bytes into text with `String::from_utf8_lossy` and keeps the first 256 characters. The value goes into `Source::Http`, so the row and the line hold the same text.

### D-4: The claim and the reset insert their row in their own transaction

- `Database::claim(username, password, source)` opens a transaction, inserts the user, inserts the event, and commits. A unique violation on the user insert stays `ClaimError::Claimed`, with no event. Any other failure, the event insert included, is `ClaimError::Unavailable`, and the transaction rolls back. It returns `Claimed { user, event }`.
- `Database::reset_admin_password(password)` inserts an `access_recovery` event with `Source::Siderealctl` and `sessions_ended` after the session delete, in its existing transaction. `Reset` gains the `event`. Only `siderealctl` resets, so the method takes no source.
- An insert sets `occurred_at` to `now()`, the transaction's start time, which is close enough for an audit row.

The handler and `siderealctl` print the returned event once the call returns `Ok`. A failed call returns no event, so no line can be printed for an act that did not happen.

Alternative: insert the row after the act commits, best effort. Rejected by D4: a claim without a row is the case the audit exists to catch.

### D-5: The server's log is a sink in `Tuning`

`Tuning` gains `log: Log`. `Log` wraps an `Arc<dyn Fn(&str) + Send + Sync>`, and `Log::default()` writes the line and a line feed to stderr with `eprintln!`. `access::routes` adds it as an `Extension`, as it does the limiter. In-process tests pass a sink that pushes into an `Arc<Mutex<Vec<String>>>`.

Alternative: assert lines only through the spawned server. Rejected: each line test would start a process, and the 10-at-once test needs the in-process router.

### D-6: A failed sign-in records its row before it answers, and ignores a failure

`Database::record(&SecurityEvent) -> Result<(), Unavailable>` inserts one row on the pool. The sign-in handler calls it for:

- a 401, after the password check and before it answers. Known and unknown usernames make the same insert, so the timing still does not reveal which it was;
- the first 429 of a run (`Refusal::new`), on both paths that refuse: `refused` before the body is read, and `admit`. `limited()` becomes an async function that builds the event, records it, writes the line, and returns the 429 error.

The handler discards an `Err` from `record`, then writes the line (D5). A 429 that is not new writes nothing, as today.

Alternative: spawn the insert and answer at once. Rejected: a test could not tell when the row exists, and the spec says the row exists when the server answers.

### D-7: A successful sign-in and a sign-out only write their line

`sign_in` writes a `sign_in`/`success` line with the matched user's username after `attempt.succeeded()`. `sign_out` takes the guard's `Extension<User>` and writes a `sign_out`/`success` line only when `Database::sign_out` answers `true`. Neither touches `security_events`.

### D-8: `siderealctl` writes its line right after the `core` call succeeds

In `reset_admin_password` in `siderealctl/mod.rs`, the `Ok` arm of each call writes `writeln!(err, "{event}")` before the `Generated password:` line. A failed stdout write that follows still leaves the line in place. A lost race (`ClaimError::Claimed`) has no event and writes only its refusal line.

### D-9: Migration `4_security_events.sql`

```sql
CREATE TABLE public.security_events (
    id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    occurred_at TIMESTAMPTZ NOT NULL,
    kind TEXT NOT NULL CHECK (kind IN ('instance_claim', 'sign_in', 'sign_out', 'access_recovery')),
    outcome TEXT NOT NULL CHECK (outcome IN ('success', 'wrong_credentials', 'limited')),
    source TEXT NOT NULL CHECK (source IN ('http', 'siderealctl')),
    username TEXT,
    client_address INET,
    user_agent TEXT,
    sessions_ended BIGINT,
    CHECK (source = 'http' OR (client_address IS NULL AND user_agent IS NULL))
);
```

- No foreign key: `username` is a copy, so a row outlives its user. No row names a session.
- `client_address` is `INET` so a later screen can filter by subnet. The insert binds the text form and casts it with `$n::inet`, so sqlx needs no new feature. Tests read it with `host(client_address)`.
- The `CHECK` lists follow `users.role`. A later epic that adds a kind changes its list in its own migration.
- `sign_out` stays in the `kind` list, though no row uses it today, so the column's values match the line's.

Alternative: `TEXT` for `client_address`. Rejected: `INET` costs nothing now and rejects a malformed address.

This decision is costly to reverse, because a forward-only migration ships it. The ADR step decides whether it needs a record.

## Risks / Trade-offs

- [Operators filtering on `sign-in limited:` lose their match] → The proposal marks the change **BREAKING**. The sign-in limits section of `docs/install.md` names the new line in place of the old, and the pull request description names the new prefix.
- [Many addresses can grow the table through failed sign-ins] → The sign-in limit allows at most 20 rows an hour for each IPv4 address or IPv6 /64, plus one `limited` row for each run. A user agent is at most 256 characters. Retention is a later story.
- [A failed sign-in row can be lost silently when only its insert fails] → The line is still written, and the 401 path read PostgreSQL moments before, so this needs PostgreSQL to fail between two statements. Accepted by D5.
- [The 429 path now writes to PostgreSQL] → Only the first refusal of a run writes, and no repeat 429 touches the database, so a flood of refused requests costs no writes.
- [A claim's two inserts change how a race ends] → The unique index still decides the race on the user insert, before the event insert runs. The core test of ten concurrent claims stays.

## Migration & Rollback

- **Deploy:** the server applies `4_security_events.sql` at start. It creates one empty table and touches no existing row.
- **Rollback:** migrations are forward-only, and an older binary refuses a database with migration 4. Roll forward with a fix. To return to the previous release, restore the database backup taken before the upgrade.
- **`siderealctl`:** a new `siderealctl` against a database not yet migrated says to start the server first, as it does for any older schema.
