## Context

See proposal.md for the motivation. These facts about the current code shape the approach:

- `DatabaseError` in `core/src/database/mod.rs` holds the startup categories. `From<sqlx::Error>` maps SQLSTATE `42501` to `Permission` and every other error to `Connection`.
- `Database::ready()` in `core/src/database/startup.rs` returns a `bool`. It throws away the `DatabaseError` it computed. A refused probe (the one-permit semaphore is taken) also returns `false`.
- The server prints only the start line and fatal startup errors, through `println!` and `eprintln!`. The workspace has no logging library.
- The `Dockerfile` health check runs `curl -fsS http://localhost:${PORT}/healthz`. `curl -f` already fails on a 503.
- sqlx 0.9 uses `tls-rustls`. A failed certificate check surfaces as `sqlx::Error::Tls`. A refused connection, a reset, and a failed DNS lookup surface as `sqlx::Error::Io`.
- Today, readiness logic and its tests live in `core`. Process output lives in the `server` binary.

## Goals / Non-Goals

**Goals:**

- Keep a single mapping from database errors to categories, and use it for both startup and readiness.
- Keep `/healthz` and `/readyz` byte-for-byte identical. Only their callers and the logs change.
- Test the connection causes and the readiness log lines without Docker, inside the existing `just check` gate.

**Non-Goals:**

- No logging library, log levels, structured logs, or timestamps. Docker adds timestamps with `docker compose logs -t`.
- No background readiness poller. The server observes changes only when a caller probes `/readyz`, as the spec states.
- No retry loop at startup. Docker's `restart: unless-stopped` stays the retry mechanism.

## Decisions

### 1. Classify connection causes in the existing `From<sqlx::Error>`

`DatabaseError` gains four variants: `Unreachable`, `LoginRejected`, `Tls`, and `DatabaseMissing`. `From<sqlx::Error>` maps them as follows:

| sqlx error | Category |
|---|---|
| `Io(_)`, `PoolTimedOut` | `Unreachable` |
| `Tls(_)` | `Tls` |
| SQLSTATE `28P01` or `28000` | `LoginRejected` |
| SQLSTATE `3D000` | `DatabaseMissing` |
| SQLSTATE `42501` | `Permission` (unchanged) |
| anything else | `Connection` (unchanged fallback) |

Each new `Display` text starts with the spec's fixed phrase and names one fix.
`Connection` stays as a fallback, so an unexpected error never gets a wrong but confident message.

Mapping in `From` means every call site gains the split without changes. One side effect: an I/O error during a migration now reads `database unreachable`. Before, it read `database connection failed`. Both are true, and the new text is more specific.

*Alternative considered:* map only at the first `connect_with` call. We rejected it because readiness would need its own copy of the mapping. Readiness then reaches the database through a pool, where failures arrive as `PoolTimedOut` or `Io`.

### 2. Readiness returns an outcome; the server writes the log line

`core` gains `Database::probe() -> Probe`, with `Probe::Ready`, `Probe::NotReady(Cause)`, and `Probe::Busy`.
`Cause` is a `DatabaseError` or `Timeout`, for the 2-second deadline.
`ready()` stays as a thin wrapper, so existing callers and tests keep working.

`Database` also holds `last: Arc<Mutex<Option<Cause>>>`. Its value starts as `None`, meaning ready, because startup has just proved the database usable.
`probe()` compares each result except `Busy` with `last`. When the two differ, it stores the new value and returns a `Change` beside the outcome.
The `/readyz` handler in `server` prints a `Change` to standard error:

```
readiness: not ready (database unreachable)
readiness: ready
```

Each cause has a short, fixed label, and the line never holds raw error text.
`core` therefore decides when a change happened, and the binary owns process output.
A `std::sync::Mutex` is enough here. The lock is held only to compare and swap, never across an `await`.

*Alternative considered:* a background task that probes every few seconds. We rejected it because it adds database load and a second timing source. Docker's health check already probes every 10 seconds, so a deployed instance is observed anyway.

*Alternative considered:* add `tracing`. We rejected it because two fixed lines don't justify a new dependency or a log format to support.

### 3. The image health check calls `/readyz`

The `Dockerfile` keeps its interval (10 s), timeout (5 s), start period (5 s), and retries (3). Only the path changes.
With these values, Docker marks the app unhealthy about 30 seconds into an outage, and healthy at the first good probe after recovery.
The spec's 60-second bounds leave room for this.
`docker compose up --wait` still returns once the app is healthy. The server binds only after a successful start, so its first probe is ready.

### 4. The home screen reads `/readyz`

`checkHealth` requests `/readyz` and returns one of four states. Status 200 with `status: "ready"` gives `healthy`. Status 503 with `status: "not_ready"` gives `not ready`. Anything else gives `unreachable`.
The Vite dev server forwards `/readyz` as well as `/healthz`.
The screen still prints only its own state names inside the existing `<strong>`.

Tokens added: none. The screen shows plain text with no badge or color, so no status token applies.

### 5. Tests use an in-process TCP relay to simulate an outage

`core`'s `test_support` gains a small TCP relay in front of the test database. A test can close its listener and drop its connections, then open them again on the same port.
Readiness tests then cover outage, recovery, and log changes in milliseconds, with no Docker and no need to stop the shared PostgreSQL.
The relay also gives the unreachable case a port that is certain to be closed.
A real outage with Docker is covered once, in `instance-verify`.

The other causes use the existing fixtures:

- **login rejected:** extend `authentication_errors_are_safe` to check the output's first phrase;
- **TLS failed:** extend `verified_tls_rejects_untrusted_and_wrong_host_certificates_without_writes`;
- **database missing:** connect with the test role to a database name that does not exist;
- **unreachable, by host name:** use a `.invalid` host, which RFC 6761 guarantees never resolves.

### 6. `instance-verify` adds one outage pass

After the default-volume instance is ready, the script saves its migration records and pairing, runs `docker compose stop db`, and waits until the app turns `unhealthy`. It checks `/healthz` and the outage log line. Then it runs `docker compose start db`, waits until the app turns `healthy`, and compares `RestartCount`, records, and pairing.
It reuses the script's existing wait helper and 180-second deadline.

## Risks / Trade-offs

- **The home screen and Docker probe at the same moment, so one probe gets `Busy` and a 503** → The screen may show `not ready` for one 5-second cycle. A probe takes a few milliseconds, so collisions are rare. Docker needs three failed probes in a row, so its status is unaffected. `Busy` never changes `last`, so no log line follows.
- **The `.invalid` lookup can behave differently in a sandbox without DNS** → Any lookup failure is still `Io`, which maps to `Unreachable`, so the test passes either way.
- **A PostgreSQL release changes which SQLSTATE a login failure uses** → The fallback stays `database connection failed`, which is still safe and true. The test for login rejected catches the change.
- **`unhealthy` might lead an operator to expect an automatic restart** → The troubleshooting section says Docker does not restart an unhealthy app, and that the app recovers by itself once its database returns.
- **Behavior change for anyone who scripts around `docker compose ps`** → An app with an unusable database used to show `healthy` and now shows `unhealthy`. That is the purpose of this change, and nothing in the repo depends on the old meaning.

## Migration Plan

No data migration is needed. Rebuilding the image with `docker compose up -d --build --wait` applies the new health check.
Rolling back to the previous image restores the `/healthz` check. Nothing is stored, so nothing needs undoing.
