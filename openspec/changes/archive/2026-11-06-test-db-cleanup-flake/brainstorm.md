**Why:** About 2 in 3 `just check` runs fail after every assertion has passed, because `DROP DATABASE … WITH (FORCE)` cannot terminate an autovacuum worker, so the gate no longer tells us whether the code works.

## Story

#474 test-db-cleanup-flake — server tests stop failing while dropping their test database: https://github.com/sidereal-io/sidereal/issues/474

## Decisions

| # | Question | Answer | Source |
|---|---|---|---|
| D1 | Where is the fix? | One file: `test_support.rs` in `sidereal-core`, which the server's integration tests include by `#[path]`. | Evidence: `server/crates/core/src/lib.rs`, `server/crates/server/tests/database.rs` |
| D2 | Why does a plain drop succeed where `FORCE` fails? | With `FORCE`, PostgreSQL's `dropdb` calls `TerminateOtherDBBackends`, which refuses roleless autovacuum workers. A plain drop calls `CountOtherDBBackends`, which stops them without a privilege check and waits up to 5 s while holding the database lock. | Evidence: PostgreSQL 18 `dbcommands.c` and `procarray.c` |
| D3 | Can the fixture grant `pg_signal_backend` instead? | No: `init.sql` runs only on a new volume, and the fix must work with a role that has only `CREATEDB`. | Evidence: #474 Scope |
| D4 | How does cleanup drop the database? | Run `DROP … WITH (FORCE)` first. If it fails with SQLSTATE 42501, run a plain `DROP DATABASE`. Repeat for a bounded number of rounds, then panic with today's message. | User |
| D5 | How many rounds? | 3. | User |
| D6 | How is the fallback tested? | Move the drop sequence into a small function that takes an injected statement runner. A unit test with a fake runner checks that 42501 leads to a plain drop and that the retries stop at the bound. The existing integration tests still cover real drops. | User |
| D7 | Does the spec change? | Yes. MODIFY postgres-foundation's "Retained fixture data and database regression tests" with a scenario: a test database is dropped when its test ends, even while autovacuum is working on it. | User |
| D8 | What is the acceptance evidence? | Five consecutive `just check` runs pass locally against the fixture, and the pull request's CI is green. | User |
| D9 | Does `orphan-clean` change? | No: it runs as the superuser `fixture_admin`, so `FORCE` can terminate autovacuum workers there. | Evidence: `server/scripts/db-fixture.sh` |

## Changes from the story

None

Confirmed by user: yes
