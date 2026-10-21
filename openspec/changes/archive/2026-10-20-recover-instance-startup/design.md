## Context

See proposal.md for the motivation. These facts about the current code shape the approach:

- `DatabaseError` in `core/src/database/mod.rs` holds the startup categories. `From<sqlx::Error>` maps SQLSTATE `42501` to `Permission` and every other error to `Connection`.
- Startup opens one unpooled connection inside a `timeout(startup, …)`. An expired deadline there becomes `StartupDeadline`, whether or not the connection opened.
- `Database::ready()` returns a `bool` and discards the `DatabaseError` it computed. A probe refused by the one-permit semaphore also returns `false`, so it answers 503.
- The readiness pool's acquire timeout and the outer readiness deadline are both 2 seconds. In sqlx 0.9 (`pool/inner.rs`), the pool retries `ConnectionRefused` with backoff until its deadline, then returns `PoolTimedOut`. Other connect errors return at once.
- sqlx 0.9 with `tls-rustls` reports certificate rejection as `Error::Io`, not `Error::Tls`. The handshake's `complete_io().await?` converts the rustls error into an `io::Error` whose inner error is a `rustls::Error`.
- sqlx reads certificate files during the handshake. `CertificateInput::data()` returns a plain `io::Error`, so a missing CA file also arrives as `Error::Io`. It would look like an outage.
- sqlx turns every error message the server sends into `Error::Database` (`connection/stream.rs`). PostgreSQL ends a session at shutdown with SQLSTATE `57P01` or `57P02`. While it starts up, it refuses logins with `57P03`.
- Preflight wraps two queries in `map_err(|_| Ownership)` and `map_err(|_| History)` (`startup.rs`, near line 306). A connection lost during either query is reported as damaged data.
- In sqlx 0.9, `verify-ca` builds its verifier with `WebPkiServerVerifier::builder`, which picks the process's crypto provider. If two providers are compiled in and none is installed, rustls panics.
- The database configuration is parsed in `server/src/config.rs`.
- The server prints only its start line and fatal startup errors. The workspace has no logging library.
- The `Dockerfile` health check runs `curl -fsS http://localhost:${PORT}/healthz` every 10 seconds, with 3 retries. `curl -f` fails on a 503.
- The web home screen also reads `/healthz`. This change leaves it alone.

## Goals / Non-Goals

**Goals:**

- Use one classification of database errors for both startup and readiness.
- Keep the `/healthz` and `/readyz` response bodies and status codes as they are.
- Test every connection cause and every readiness log line inside `just check`, without Docker.

**Non-Goals:**

- No logging library, log levels, structured logs, or timestamps. `docker compose logs -t` adds timestamps.
- No background readiness poller.
- No startup retry loop. Docker's `restart: unless-stopped` remains the retry mechanism.

## Decisions

### 1. Classify connection causes in `From<sqlx::Error>`

`DatabaseError` gains `Unreachable`, `LoginRejected`, `Tls`, and `DatabaseMissing`. Decision 2 adds `CertificateFile`. The mapping checks these cases in order:

| sqlx error | Category |
|---|---|
| `Io(e)` whose inner error is a `rustls::Error` | `Tls` |
| `Tls(_)` | `Tls` |
| `Io(_)`, `PoolTimedOut` | `Unreachable` |
| SQLSTATE class `08`, or `57P01`, `57P02`, `57P03` | `Unreachable` |
| SQLSTATE `28P01` or `28000` | `LoginRejected` |
| SQLSTATE `3D000` | `DatabaseMissing` |
| SQLSTATE `42501` | `Permission` (unchanged) |
| anything else | `Connection` (unchanged fallback) |

The TLS check downcasts `io::Error::get_ref()` to `rustls::Error`. `core` therefore gains a direct `rustls` dependency, declared with `default-features = false` and no features of its own.
Cargo resolves it to the version sqlx already uses. With default features off, it enables no second crypto provider, so `verify-ca` keeps its single provider.
A test that starts with `verify-ca` and a trusted CA guards against that panic.
The check reads only the error's type. No error text reaches a message.

Two preflight call sites need changes: the ownership query and the migration-history query.
Each now converts the `sqlx::Error` with `From` first. A connection fault (`Unreachable`, `Tls`) or `Permission` passes through. Any other error keeps its `Ownership` or `History` category.
One test closes the relay's connection while readiness reads migration history and expects `Unreachable`.
Another test ends the readiness session with `pg_terminate_backend` during the same query. PostgreSQL then sends `57P01`, and the test expects `Unreachable`.

Class `08` covers connection exceptions. `57P01` and `57P02` mean the server ended the session because it is shutting down or crashed. `57P03` means it is still starting up. Sam's fix for each is the same as for an outage: wait for the database or start it.

*Alternative considered:* treat `io::ErrorKind::InvalidData` as TLS. We rejected it because other protocol faults use that kind too, and a wrong TLS diagnosis would send Sam to the wrong fix.

### 2. Certificate files are read during configuration

`DatabaseConfig` reads every certificate and key file before the first connection. It finds them by serializing the parsed options with `to_url_lossy()` and reading `sslrootcert`, `sslcert`, and `sslkey`. That covers both the URL and the `PGSSL*` environment variables, which sqlx folds into the options.
sqlx writes a file input as `file: <path>` and an inline PEM input as its text. Only values with the `file: ` prefix are read.
The serialized URL contains the password, so the check reads only those three values and never prints or keeps it.
The check first reads the path's metadata, which never blocks. A path that is not a regular file is rejected, so a named pipe or a device cannot stall startup.
The check then opens the file and reads at most 1 MiB. A larger file is rejected. Real certificate bundles are far smaller.
Any failure returns `DatabaseError::CertificateFile`. That includes a missing file, a denied file, a directory, a named pipe, and a file over the limit.
Its message is `database TLS certificate file unreadable: check sslrootcert, sslcert, and sslkey paths`. It names no path.
A file that disappears between this check and the handshake still reaches the `Io` mapping. That window is too small to matter.

*Alternative considered:* tell file errors apart by `io::ErrorKind`, such as `NotFound`. We rejected it because the kind does not say whether the error came from a file or a socket.

### 3. Timeouts are classified by stage

A deadline means different things before and after the connection opens:

- **At startup**, the connect step gets its own `timeout`. When it expires, the result is `Unreachable`. Later steps keep `StartupDeadline`, because there the likely cause is a lock or a slow migration.
- **In readiness**, the readiness pool's acquire timeout drops to 1.5 seconds, under the 2-second outer deadline. A refused or silent database then surfaces as `PoolTimedOut`, which maps to `Unreachable`, before the outer timer fires. When the outer timer fires after the connection was acquired, the cause is `Timeout`. A flag set right after `acquire()` tells the two cases apart.

The 2-second bound on `/readyz` stays unchanged.

*Alternative considered:* map every expired deadline to `Unreachable`. We rejected it because a held lock on the migration table would then read as a network fault.

### 4. Readiness returns an outcome, and the server writes the log line

`core` gains `Database::probe() -> Probe`, with `Probe::Ready`, `Probe::NotReady(Cause)`, and `Probe::Busy`.
`Cause` is a `DatabaseError` or `Timeout`.

`Database` holds `last: Arc<Mutex<Option<Cause>>>`. `None` means ready, and it starts as `None`, because startup has just proved the database usable.
`probe()` compares each finished check with `last`. When they differ, it stores the new value and returns a `Change` with the outcome.
A busy probe reads `last` without changing it. Its HTTP answer is the answer `last` implies: 200 for `None`, otherwise 503.
The `/readyz` handler in `server` prints each `Change` to standard error:

```
readiness: not ready (database unreachable)
readiness: ready
```

Each cause has a fixed short label, and no raw error text appears.
`core` decides when a change happened. The binary owns process output.
A `std::sync::Mutex` is enough, because the lock is held only to compare and swap, never across an `await`.

*Alternative considered:* keep the fixed 503 for busy probes. We rejected it because two overlapping probes, such as Docker's and an operator's `curl`, would then see a false outage.

*Alternative considered:* a background task that probes on a timer. We rejected it because it adds database load and a second source of timing. Docker's health check already probes every 10 seconds.

*Alternative considered:* add `tracing`. We rejected it because two fixed lines do not justify a new dependency and a log format to support.

### 5. The container health check reads `/readyz`

The `Dockerfile` changes only the probe path. Its interval, timeout, start period, and retries stay the same.

The web home screen keeps reading `/healthz`. The story serves Sam through logs and Docker, and the screen is a placeholder. Showing readiness there would add a status that DESIGN.md requires in a badge, and no badge exists yet. Tokens added: none.

### 6. Tests simulate an outage with an in-process TCP relay

`test_support` gains a TCP relay in front of the test database. A test can make it refuse connections, accept connections without replying, or forward them again, all on one port.
It can also close an open connection. That covers outage, recovery, a silent host, a connection lost mid-check, and log changes in milliseconds, without stopping the shared PostgreSQL.
The other causes use existing fixtures, plus a missing CA path and a directory used as a CA path. The two certificate fixtures in `verified_tls_rejects_untrusted_and_wrong_host_certificates_without_writes` check for `Tls`. `authentication_errors_are_safe` checks for the login-rejected phrase. A new test connects to a database name that does not exist. A `.invalid` host, which RFC 6761 reserves, covers DNS failure.
`instance-verify` covers a real outage with Docker once.

### 7. `instance-verify` adds one outage pass

On the ready default-volume instance, the script saves migration records and pairing, then stops `db`. It waits for `unhealthy` and checks `/healthz` and the outage log line. Then it starts `db`, waits for `healthy`, and compares `RestartCount`, records, and pairing. The pass reuses the script's wait helper and its 180-second deadline.

## Risks / Trade-offs

- **A busy probe returns the last finished result, which can be old** → Its age depends on how often callers probe, not on the 2-second deadline. The 2 seconds bounds only the check in progress, and that check's result replaces the old one when it finishes. Docker probes every 10 seconds, so in a running instance the result is rarely older than that.
- **Docker's health status lags an outage by up to 40 seconds** → The spec and the guide both state the lag. `/readyz` itself answers within 2 seconds.
- **A future sqlx release changes how it wraps TLS errors** → The certificate fixtures fail, and the message falls back to `database unreachable` or `database connection failed`. Both are still safe.
- **The `.invalid` lookup behaves differently in a sandbox without DNS** → Any lookup failure is still `Io`, which maps to `Unreachable`.
- **A PostgreSQL release changes the SQLSTATE for a rejected login** → The message falls back to `database connection failed`, and the login-rejected test catches the change.
- **An operator expects Docker to restart an unhealthy app** → The guide says that Docker does not, and that the app recovers by itself once the database returns.

## Migration Plan

No data migration is needed. `docker compose up -d --build --wait` rebuilds the image with the new health check.
Rolling back to an earlier image restores the `/healthz` check. Nothing is stored, so nothing needs undoing.
