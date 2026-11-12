## Context

- Every `/api` route passes two `route_layer`s in `server/crates/server/src/access/mod.rs`. The origin check runs first, then the session guard. The guard looks the session up, puts the `User` in the request's extensions, and sets the cookie again on the response. A route added there is guarded with no code of its own.
- The origin check in `access/guard.rs` lets every `GET`, `HEAD` and `OPTIONS` through. A WebSocket upgrade is a `GET`.
- `Database::session` in `server/crates/core/src/access/session.rs` is the only session lookup. It records a use when the last one was over an hour ago, and it deletes an expired row. The socket's recheck must not record a use (D3), so it cannot reuse this as it is.
- axum 0.8 is in the workspace without its `ws` feature. tokio already has `time`.
- The server's integration tests drive the `Router` with `tower::ServiceExt::oneshot`, with no listener. An upgrade needs a real connection, so socket tests must serve on a port. `tests/access.rs` already binds `127.0.0.1:0` for its child-process harness.
- The web shell's `Access.tsx` holds the view state. `Access.test.tsx` stubs `fetch` with `vi.stubGlobal`. `web/vite.config.ts` proxies `/api` with `changeOrigin: false`, so the browser's `Host` matches its `Origin`.
- `docs/install.md` shows a Caddy proxy and says any proxy that sends the forwarded headers works. Caddy passes WebSocket upgrades by default. Others, such as nginx, need it configured.

## Goals / Non-Goals

**Goals:**

- The socket reuses both existing guards unchanged in order and behavior, and adds only the upgrade case to the origin check.
- The heartbeat and silence intervals are fixed in production and injectable in tests, so socket tests run in well under a second each.
- The web shell's socket logic is testable with stubbed `WebSocket`, `fetch` and fake timers, with no server.

**Non-Goals:**

- Closing open sockets with 1001 on server shutdown. A shutdown drops the connection, and the shell's retry path (D8) handles it.
- A limit on sockets per session or per address.
- Any event other than the heartbeat. No general event bus or broadcast channel is built yet.
- Pushing revocation to the socket at once (D11).

## Test Seams

- **Primary test seam:** a new `server/crates/server/tests/events.rs`. Each test serves the router on `127.0.0.1:0` with short intervals and talks to it with a WebSocket client. It covers the upgrade, origin, recheck, close-code, heartbeat and silence scenarios.
- **Existing seams reused:** `tests/common` (`TestDatabase`) and the SQL helpers in `tests/access.rs` for deleting or ageing session rows. The `Access.test.tsx` harness covers the web shell, with a fake `WebSocket` added beside its `fetch` stub. One test in the child-process harness in `tests/access.rs` checks that the real binary upgrades and that SIGTERM with a socket open still exits. The harness only kills its child today, so it gains a method that sends SIGTERM with `libc::kill`. `libc` is already a dependency of `sidereal-server`.
- **By hand:** the dev proxy scenario. Run `just dev`, sign in, and see the socket and a heartbeat in the browser's network panel. The existing dev-proxy scenarios are checked the same way.

## Scrutiny Tier & Blast Radius

- **Tier:** high. The change adds a new authenticated surface, extends the origin check (a trust boundary), and adds a wire contract between the server and the web shell.
- **Blast radius:** `server/crates/server/src/access/` (routes, guard, a new events module), `server/crates/server/src/lib.rs` (wiring), `server/crates/core/src/access/session.rs` (a read-only lookup), `server/Cargo.toml`, `web/src/` (access section and a new socket module), `web/vite.config.ts`, and `docs/install.md`. No schema change.

## Decisions

1. **The route sits inside the guarded router.** `/api/events` is a `get` route in `access::routes`, so both `route_layer`s cover it. The handler takes axum's `WebSocketUpgrade` and the cookie's token. It hands the token, the `Database` and the timing to the socket task. When axum's extractor refuses a plain request, it answers 400, 405 or 426, which meets "a 4xx". *Alternative:* a separate router with its own auth. Rejected because it would copy the guard and drift from it.
2. **The origin check treats an upgrade as state-changing.** In `guard::origin`, a request whose `Upgrade` header holds the token `websocket` (case-insensitive) skips the safe-method shortcut. The rest of the check is unchanged, including passing a request with no `Origin` or `Referer` (D2). *Alternative:* a check inside the events handler. Rejected because the next socket route would need to remember it.
3. **The recheck is a new read-only lookup.** `Database::session_alive(&token) -> Result<bool, Unavailable>` runs one `SELECT` for an unexpired row whose user has the `admin` role. It never updates or deletes. An expired row is left for the guard or the sweep to delete. *Alternative:* a flag on `Database::session`. Rejected because one function would then hold two policies.
4. **One task per socket, one `select!` loop.** The loop waits on three things: a heartbeat tick, the next client frame, and the silence deadline. The tick is a `tokio::time::interval_at` whose first tick comes one period after the upgrade. On a tick, the server rechecks, then sends the heartbeat text and a ping. On a text or binary frame, it rechecks and sends nothing. Any frame, a pong included, moves the silence deadline. A recheck that finds no session sends a close frame with code 1008. A recheck that cannot read PostgreSQL sends 1011. After it sends a close frame, the server finishes the closing handshake: it keeps reading until the client's close frame arrives or the connection ends, for at most 5 s, and only then drops it. Dropping the connection with unread data would send a TCP reset, and the browser could then report 1006 instead of the code. A silent client or a read error ends the task and drops the connection at once. axum answers the client's pings by itself.
5. **The timing is a value, not a constant in the loop.** An `EventTiming { heartbeat, silence }` defaults to 30 s and 60 s. `app_with_database` uses the default. A new `app_with_event_timing` takes one, and only the tests call it. No environment variable sets it. *Alternative:* `tokio::time::pause`. Rejected because the tests run real sockets and PostgreSQL, and paused time does not mix well with real I/O.
6. **The test client is `tokio-tungstenite`, as a dev dependency.** It is the library axum's `ws` feature already builds on, so no new transport enters the build. It reports the close code and lets a test choose whether to answer pings.
7. **The web shell's socket lives in its own module.** A new `web/src/eventsSocket.ts` exports a `watchSession(onSignedOut)` function that returns a stop function. It opens the socket at `/api/events` on the page's origin, with `ws:` or `wss:` taken from the page. It resets a 60 s silence timer on each message. On a 1008 close it calls `onSignedOut`. On any other close, a failed open or silence, it waits 5 s and asks `GET /api/session`: 401 calls `onSignedOut`, 200 reopens the socket, and anything else waits again. `Access.tsx` calls it from an effect that runs while the view is the signed-in user. The effect's cleanup is the stop function, which covers both sign-out and leaving the view (D9). The stop function closes the socket, clears every timer, aborts an in-flight `GET /api/session`, and sets a stopped flag. After it runs, the module opens no socket and never calls `onSignedOut`. *Alternative:* a React hook. Rejected because a plain module is easier to test with fake timers and holds no view state.
8. **The dev proxy adds `ws: true` to `/api`.** `changeOrigin` stays `false`, so the upgrade keeps the browser's `Host` and passes the origin check.
9. **The install guide notes upgrades.** One sentence in `docs/install.md`: Caddy passes WebSocket upgrades by itself, and another proxy must be set to pass them for `/api/events`.

## Design tokens

None. The change adds no visible element. A 1008 close shows the existing sign-in form.

## Risks / Trade-offs

- [A revoked session keeps its socket for up to 30 s] → This is the agreed bound (D5). A client message triggers a recheck at once.
- [Each open socket costs one `SELECT` every 30 s] → With one administrator and a few tabs, this is negligible. The lookup uses the `token_hash` key.
- [An idle open tab is signed out 30 days after its last request, even though the tab looks active] → Agreed (D3). The upgrade and every reconnect count as uses, so a tab that reconnects after a restart extends the session.
- [Graceful shutdown could wait on upgraded connections] → hyper hands off the connection at the upgrade, so `serve` should not wait for it. The child-process test sends SIGTERM with a socket open and checks that the server exits within the harness's usual bound.
- [The cookie refresh on a 101 response is not applied by some client] → The guard already appends it, and browsers apply `Set-Cookie` on the handshake. A client that ignores it loses nothing, because rechecks never depend on the cookie.
- [A proxy that drops upgrades makes the shell retry every 5 s] → Each retry is one `GET /api/session` and one failed upgrade, and the shell stays usable. The install guide names the requirement. Backoff beyond a fixed retry is out of this story's scope.
- [While upgrades keep failing, each `GET /api/session` counts as a use, so an open tab keeps its session alive] → This loses D3's bound on an idle tab only on a misconfigured proxy. The guide note prevents it, and a later backoff can limit it.

## Migration & Rollback

N/A. No schema or data change. Rolling back removes the route. An older web shell opens no socket, and a newer one against an older server gets a failed upgrade. It then asks `GET /api/session` every 5 s and stays signed in.
