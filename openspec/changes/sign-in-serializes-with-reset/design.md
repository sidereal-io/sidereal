## Context

`Database::sign_in` in `server/crates/core/src/access/session.rs` runs three steps, each on its own pooled connection in autocommit:

1. `SELECT id, password_hash, role FROM public.users WHERE username = $1`
2. `password::verify` on `spawn_blocking`: argon2, hundreds of milliseconds
3. `INSERT INTO public.sessions ...`

`Database::reset_admin_password` in `server/crates/core/src/access/recovery.rs` runs `UPDATE public.users SET password_hash = ...` and then `DELETE FROM public.sessions` in one transaction, at PostgreSQL's default isolation, `READ COMMITTED`.

The `UPDATE` changes no key column, so it takes a `FOR NO KEY UPDATE` row lock on the administrator's row. The session insert's foreign key takes only `FOR KEY SHARE` on that row, which does not conflict. So nothing makes step 3 wait for the reset. An insert that runs after the reset's `DELETE` but before its commit is invisible to that `DELETE`, and survives it.

The pool has 5 connections (`server/crates/core/src/database/startup.rs`). The sign-in limiter already counts an attempt before its check starts.

## Goals / Non-Goals

**Goals:**

- The session insert waits for any open reset, and saves nothing when the hash it checked has been replaced.
- No lock or transaction is held during the argon2 check (brainstorm D6).
- The race is proved by a test that is deterministic, not by timing.

**Non-Goals:**

- Changing `reset_admin_password`.
- Changing the HTTP handler. It already turns `None` into a 401, a wrong-credentials security event line and a counted failed attempt.
- Serializable isolation or advisory locks in production code.

## Test Seams

- **Primary test seam:** the `core` database tests in `server/crates/core/src/access/session.rs`, which call `Database::sign_in` and `Database::reset_admin_password` against a throwaway PostgreSQL from `TestDatabase`.
- **Existing seams reused:** `TestDatabase`, the `claimed`, `connection` and `sessions` helpers in the same test module, and the test-only trigger pattern the recovery tests already use to interfere with a statement.

## Scrutiny Tier & Blast Radius

- **Tier:** medium. The change is one statement in one function, on a trust-relevant path. It adds no contract, schema or migration. The reviewer should check the locking claims below against PostgreSQL's documented behavior.
- **Blast radius:** `Database::sign_in` in `core`, and through it `POST /api/session`. No other caller.

## Decisions

### D-1. The insert takes a share lock on the user row and rechecks the hash

Step 3 becomes one statement that inserts only when the row still holds the hash step 1 read:

```sql
WITH checked AS (
  SELECT id FROM public.users WHERE id = $2 AND password_hash = $4 FOR SHARE
)
INSERT INTO public.sessions (token_hash, user_id, auth_type, created_at, last_used_at, expires_at)
SELECT $1, id, 'password', now(), now(), now() + make_interval(secs => $3) FROM checked
```

`$4` is the hash string step 1 read. `sign_in` answers `None` when the statement inserts no row.

Why this closes the race, case by case:

- **The reset has updated the row but not committed.** `FOR SHARE` conflicts with the reset's `FOR NO KEY UPDATE`, so the statement waits. When the reset commits, `READ COMMITTED` rechecks the `WHERE` clause against the new row version. The hash no longer matches, so `checked` is empty and nothing is inserted.
- **The reset has committed.** The statement sees the new hash, and inserts nothing.
- **The insert locks the row first.** The reset's `UPDATE` waits until the insert commits. The reset's `DELETE` then takes a new snapshot, sees the session and deletes it. The sign-in answered 200, and its session ended with the reset, as the spec allows.

Alternatives:

- **One transaction from step 1 to step 3, with `SELECT ... FOR SHARE` in step 1.** This is what the issue asked for. It closes the race the same way, but holds a pooled connection and a row lock through the argon2 check. Five attempts at once would hold every connection. The user chose the insert-only lock (brainstorm D6).
- **Recheck the hash in a plain `SELECT` before the insert.** It narrows the window but does not close it. The issue rules it out (D5).
- **Make the reset take `FOR UPDATE`.** Then the foreign key's `FOR KEY SHARE` would wait. But a sign-in that has already read the old hash would still insert after the reset commits. It also changes the reset, which this change leaves alone.

The comparison is on the stored hash string, not on a second argon2 check: the string is what the reset replaces, and comparing it costs nothing.

### D-2. The test holds the reset open with a trigger and an advisory lock

The test makes the reset pause between its `UPDATE` and its `DELETE`, so the order is fixed rather than timed:

1. A test connection takes `pg_advisory_lock(<key>)` and creates a statement trigger, `BEFORE DELETE ON public.sessions`, whose function waits on the same key with `pg_advisory_xact_lock_shared`.
2. Spawn `reset_admin_password(NEW_PASSWORD)`. It updates the hash, then waits in the trigger. Poll `pg_stat_activity` until a backend waits with `wait_event = 'advisory'`.
3. Spawn `sign_in("sam", PASSWORD)`. Its first `SELECT` reads the old hash, because the reset has not committed. Poll until the sign-in task has finished, or `pg_locks` shows an ungranted lock that is not an advisory lock.
4. Release the advisory lock. Await both tasks.
5. Assert: the reset succeeded, the sign-in returned `Ok(None)`, and `public.sessions` is empty.

Without D-1 the insert does not wait, so the sign-in returns `Some` and the test fails on that assertion. That is the red step. Each poll has a 10-second limit, so a regression fails instead of hanging.

The opposite order gets no new test (brainstorm D7).

### D-3. No ADR

The change applies ADR-015's opaque sessions and the reset's existing rule. It sets no new direction, and undoing it is a one-statement revert.

## Risks / Trade-offs

- [`READ COMMITTED` recheck works differently than expected inside `INSERT ... SELECT` with a CTE] → The D-2 test exercises exactly that path. A wrong assumption makes the test fail on the `Ok(None)` assertion.
- [A sign-in waits for a slow reset, with no lock timeout on the operational pool] → The wait lasts only while a reset's transaction is open. That transaction runs a few short statements and no argon2 work, because the reset hashes the new password before it begins. Any other writer of the users row already waits the same way.
- [A sign-in that loses the race reads as a wrong password in the log and the limiter] → That is accurate: the password was no longer valid when the session would have started.
- [The test trigger outlives the test] → `TestDatabase` gives each test its own database, so the trigger dies with it.

## Migration & Rollback

N/A. No schema change. Rolling back is reverting the one statement.
