## 1. Error categories

- [x] 1.1 Add `rustls` to `core` with `default-features = false` and no features. Verify that `cargo tree -i rustls -e features` lists no `aws-lc-rs` and that `Cargo.lock` gains no new crate.
- [x] 1.2 Add the `Unreachable`, `LoginRejected`, `Tls`, `DatabaseMissing`, and `CertificateFile` variants to `DatabaseError`, each with its fixed phrase and one recovery step. Verify with a unit test that each message starts with its phrase from the spec.
- [x] 1.3 Classify `sqlx::Error` in `From<sqlx::Error>` in the order of design decision 1, including SQLSTATE class `08` and `57P01`, `57P02`, and `57P03`. Verify with unit tests for an `Io` error that wraps a `rustls::Error`, a plain `Io` error, and `PoolTimedOut`.
- [x] 1.4 Change the ownership and migration-history queries in preflight to convert with `From` first. Pass `Unreachable`, `Tls`, and `Permission` through, and keep `Ownership` or `History` for any other error. Verify that the existing ownership and history tests still pass.

## 2. Certificate file check

- [x] 2.1 In `server/src/config.rs`, find the `sslrootcert`, `sslcert`, and `sslkey` values through `to_url_lossy()`, and read only values with the `file: ` prefix. Verify with config tests that a path in the URL and a path in `PGSSLROOTCERT` are both checked, and that inline PEM is not.
- [x] 2.2 Reject a path that is not a regular file, and read at most 1 MiB, returning `CertificateFile`. Verify with config tests for a missing file, a directory, a named pipe that returns within 1 second, and a file over 1 MiB. Each test also checks that the message does not contain the path.

## 3. Test relay

- [x] 3.1 Add a TCP relay to `test_support` that can refuse connections, accept them without replying, forward them, and close the connections it holds open. Verify with a test that a connection through the relay works while forwarding and fails in each other mode.

## 4. Startup diagnostics

- [x] 4.1 Give the startup connect step its own timeout that maps to `Unreachable`, and keep `StartupDeadline` for later steps. Verify with a test that a silent relay and a 1-second deadline fail with `database unreachable` within 1.25 seconds.
- [x] 4.2 Add server tests for each connection cause: a closed loopback port, a `.invalid` host, a wrong password, an untrusted certificate, a certificate for another host, and a missing database. Verify that each output starts with its phrase and holds no password or connection string.
- [x] 4.3 Add a test that starts with `sslmode=verify-ca` and a trusted CA. Verify that startup succeeds and `/readyz` returns 200.

## 5. Readiness outcome and log lines

- [x] 5.1 Add `Database::probe()` returning `Ready`, `NotReady(Cause)`, or `Busy`, with the last result stored in `Database` and starting as ready. Set the readiness pool's acquire timeout to 1.5 seconds, and set a flag after `acquire()` so that an expired outer deadline maps to `Timeout` only after the connection was acquired. Verify that the existing readiness tests pass.
- [x] 5.2 Make the `/readyz` handler answer a busy probe from the last result and print each change to standard error as `readiness: not ready (<cause>)` or `readiness: ready`. Verify that the response bodies and status codes match the existing tests.
- [x] 5.3 Add readiness tests for the spec scenarios: outage and recovery through the relay, a held lock on the migration table, 20 probes with no line, a connection closed during the history query, a session ended with `pg_terminate_backend`, redaction, and overlapping probes on a usable database and during an outage. Verify that each test passes and asserts the exact lines written.

## 6. Container health

- [x] 6.1 Change the `Dockerfile` health check to probe `/readyz`, keeping its interval, timeout, start period, and retries. Verify with `deli -- docker build .` and `docker inspect` that the image's health check names `/readyz`.

## 7. Operator guide

- [x] 7.1 Add a `Troubleshoot` section to `docs/install.md` that covers every point in the guide requirement, including each message phrase, `docker compose ps`, `docker compose logs app`, the up-to-40-second lag, and the warnings against deleting volumes and editing migration records. Verify that the section contains each phrase from the spec scenarios.

## 8. Deployment verification

- [x] 8.1 Add the outage pass to `scripts/instance-verify.sh`: save migration records and pairing, stop `db`, wait for `unhealthy`, check `/healthz` and the outage line, start `db`, wait for `healthy`, and compare `RestartCount`, records, and pairing. Verify that `deli -- just instance-verify` exits 0.

## 9. Gate

- [x] 9.1 Run `deli -- just check` and verify that it exits 0.
