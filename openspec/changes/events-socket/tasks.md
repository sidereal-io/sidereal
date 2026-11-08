## 1. Server contracts

- [x] 1.1 [easy] `Database::session_alive` answers true for an unexpired session of an `admin` user, and false for a missing or expired row, and it updates and deletes no row (verify: cd server && cargo test -p sidereal-core session_alive) (✓ cargo test -p sidereal-core session_alive passed, 4 tests)
- [x] 1.2 [medium] The origin check treats a request whose `Upgrade` header names `websocket` as state-changing: a disallowed `Origin` or `Referer` gets 403 before the session check, while a same-origin upgrade and an upgrade with neither header pass (verify: cd server && cargo test -p sidereal-server --test access origin) (✓ cargo test -p sidereal-server --test access origin passed, 17 tests)
- [x] 1.3 [medium] `GET /api/events` behind both guards: a WebSocket upgrade with a session answers 101, one without a cookie or with a revoked session answers 401, and a plain `GET` with a session answers a 4xx. Adds axum's `ws` feature, `EventTiming`, `app_with_event_timing`, and `tests/events.rs` serving on `127.0.0.1:0` with `tokio-tungstenite` as a dev dependency (verify: cd server && cargo test -p sidereal-server --test events upgrade) (✓ cargo test -p sidereal-server --test events upgrade passed, 4 tests)

## 2. The socket loop [depends-on: 1]

- [ ] 2.1 [medium] Every heartbeat period after the upgrade, the server sends the text `{"type":"heartbeat"}` and a ping. A client text message gets no reply and leaves the socket open (verify: cd server && cargo test -p sidereal-server --test events heartbeat)
- [ ] 2.2 [complex] A recheck, on each tick and on each client text or binary message, closes with 1008 when the session row is gone or expired, and with 1011 when PostgreSQL cannot be read. The server waits for the client's close frame, up to 5 s, before it drops the connection. Tests cover revocation with a message, revocation with a silent client, expiry, access recovery, and closed database connections (verify: cd server && cargo test -p sidereal-server --test events recheck)
- [ ] 2.3 [easy] A recheck does not move the session's expiry: after the row is aged to 2 days old and the client sends a message, the socket stays open and `expires_at` is unchanged (verify: cd server && cargo test -p sidereal-server --test events not_a_use)
- [ ] 2.4 [medium] A client that sends nothing and answers no ping is dropped once the silence window passes, and a client that answers every ping stays open past it (verify: cd server && cargo test -p sidereal-server --test events silent)

## 3. The web shell's socket

- [ ] 3.1 [medium] `watchSession` opens a WebSocket to `/api/events` on the page's origin (`ws:` or `wss:` from the page). On a 1008 close it calls `onSignedOut` and opens nothing more. Its stop function closes the socket, clears every timer and aborts any in-flight request, and nothing fires after it (verify: cd web && pnpm test -- eventsSocket)
- [ ] 3.2 [medium] On any other close, a failed open, or 60 s with no message, `watchSession` waits 5 s and sends `GET /api/session`: 401 calls `onSignedOut`, 200 opens a new socket, and anything else or a network error waits 5 s and asks again (verify: cd web && pnpm test -- eventsSocket)
- [ ] 3.3 [medium] The access section holds the socket only in the signed-in view: a sign-in opens it, a sign-out closes it, a signed-out page opens none, and `onSignedOut` shows the sign-in form with no message. Signing out during the 5 s wait sends no `GET /api/session` and opens no socket (verify: cd web && pnpm test -- Access)

## 4. Integration [depends-on: 2, 3]

- [ ] 4.1 [medium] The real binary upgrades `/api/events` and exits on SIGTERM while a socket is open. The child-process harness in `tests/access.rs` gains a method that sends SIGTERM with `libc::kill` (verify: cd server && cargo test -p sidereal-server --test access spawned)
- [ ] 4.2 [easy] The dev proxy sets `ws: true` on `/api`, and `docs/install.md` says that Caddy passes WebSocket upgrades by itself and that another proxy must pass them for `/api/events`. Then check by hand with `just dev`: sign in, and see the socket open and a heartbeat arrive in the browser's network panel (verify: just check)

## Deviations Ledger

<!-- Dated notes on small departures from the plan, written during apply. Stop and ask the user about a large one. -->
