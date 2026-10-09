## 1. Database fixture and test setup

- [x] 1.1 Add the database-only Compose fixture and single PostgreSQL 18 image pin; verify the fixture resolves the exact patch and digest from `server/postgres-image.env`.
- [x] 1.2 Create separate runtime, demo, and test roles; verify only the test role has `CREATEDB` and initialization finishes before fixture readiness succeeds.
- [x] 1.3 Add worktree-isolated `db-up`, `db-down`, and `db-clean` recipes; verify configurable loopback ports, retained volumes, and deletion limited to the selected fixture.
- [x] 1.4 Add `db-url` and `db-test-url` recipes; verify distinct fixture credentials, the selected port, explicit `sslmode=disable`, and no production URL disclosure.
- [x] 1.5 Add temporary test databases and supervisor cleanup; verify unique names, cleanup after failures, and actionable failure when `TEST_DATABASE_URL` is missing or unreachable.

## 2. Configuration and transport

- [x] 2.1 Add sqlx dependencies within the existing crate boundaries and track embedded migration inputs; verify compilation, migration rebuilds, and the architecture check.
- [x] 2.2 Parse required database configuration and positive startup and migration timeouts; verify missing, malformed, and invalid values fail before connecting, with 30-second defaults.
- [x] 2.3 Implement the approved local exceptions and verified TLS defaults; verify loopback, sockets, named hosts, multiple hosts, explicit modes, and rejection of unsafe modes.
- [x] 2.4 Add the local TLS test fixture and test CA; verify valid certificates connect and untrusted or wrong-host certificates fail without plaintext fallback or database writes.
- [x] 2.5 Map startup errors to safe categories and operator actions; verify captured output excludes distinctive credentials, connection strings, and raw database messages.

## 3. Ownership and migration safety

- [x] 3.1 Implement PostgreSQL 18 policy and catalog preflight checks; verify unrelated objects and injected unsupported versions leave catalog snapshots and rows unchanged.
- [x] 3.2 Implement identity, migration-prefix, and checksum validation; verify foreign history, damaged identity, gaps, failed records, and unknown versions fail without writes.
- [x] 3.3 Add the initial migration and atomic bootstrap before invoking sqlx; verify identity, migration table, and record commit together, while injected failure rolls everything back.
- [x] 3.4 Hold both migration locks on the dedicated startup connection in the approved order; verify concurrent startup applies once and failure or cancellation releases locks.
- [x] 3.5 Apply later migrations transactionally and reject nontransactional sources; verify injected failure leaves no committed objects or record and repeated startup preserves every record field.
- [x] 3.6 Generate normalized schema descriptions with `db-schema`; verify deterministic regeneration, altered-object rejection, and compatibility with the committed initial migration-table baseline.
- [x] 3.7 Check required privileges separately from schema structure; verify equivalent restores under different owners succeed and insufficient privileges fail safely without writes.
- [x] 3.8 Enforce separate connection, startup-lock, and migration deadlines; verify shorter test deadlines within 250 milliseconds of tolerance and closed connections after failure.

## 4. Server startup and health

- [x] 4.1 Wire core initialization before HTTP binding and close pools during shutdown; verify failed startup leaves HTTP unbound and successful startup preserves configurable `PORT` behavior.
- [x] 4.2 Preserve `/healthz` and add fixed `/readyz` responses; verify healthy status, compatibility failures, and 405 responses for mutation methods without database changes.
- [x] 4.3 Add the separate one-connection readiness pool and semaphore; verify excess probes return 503 immediately and checks finish within two seconds without using operational capacity.
- [x] 4.4 Run readiness checks in read-only consistent transactions with safe cancellation; verify database outages preserve liveness and readiness recovers within the specified five-second bound.

## 5. Retained-state demonstration and guidance

- [x] 5.1 Add `db-demo` using only the fixture demo database; verify identical migration records after server and PostgreSQL restarts, bounded readiness waits, and nonzero failure results.
- [x] 5.2 Add demo interruption cleanup and fixture-only orphan-test cleanup; verify spawned servers exit, scratch files stay in `.workspace/`, and cleanup cannot select production databases.
- [x] 5.3 Update server guidance, root recipe descriptions, and CONTRIBUTING's version table; verify documented development and direct-cargo setup includes both database URLs and fixture prerequisites.

## 6. CI and complete verification

- [x] 6.1 Start the shared fixture in server CI and export its test URL without logging credentials; verify CI uses the single image pin and runs the existing server gate.
- [x] 6.2 Run `deli -- just check` with the prepared test database; verify all server and web checks pass and database prerequisites never cause silent skips.
- [ ] 6.3 Run the retained-state demo and documented development checks through `deli`; verify retained records, fixture stop/start behavior, process cleanup, and all modified development scenarios.
