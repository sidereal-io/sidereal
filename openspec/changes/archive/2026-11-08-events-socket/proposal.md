## Why

ADR-007 requires WebSocket access that derives from the session and rechecks expiry and revocation, but the server has no WebSocket yet, so the rule is untested. A minimal `/api/events` socket proves the rule. It also gives the web shell a way to learn that Sam was signed out somewhere else, such as by `siderealctl reset-admin-password` or a sign-out in another tab. Later live features reuse it.

## What Changes

- `GET /api/events` with a WebSocket upgrade opens a socket for a caller with an administrator session. It sits behind the existing session guard, so a caller without a session gets the same 401. The upgrade counts as a use of the session (D3).
- The origin check also covers the upgrade, though it is a `GET`. An upgrade that names an origin that is not allowed gets 403 (D2).
- While the socket is open, the server rechecks the session every 30 s and on every client message. A recheck does not extend the session (D3, D5). A revoked or expired session closes the socket with code 1008 (D6). A recheck that cannot reach PostgreSQL closes it with 1011 (D7).
- A heartbeat runs in both directions. Every 30 s the server sends `{"type":"heartbeat"}`. The server closes a client that has been silent for 60 s, and the web shell treats 60 s of silence as a dead socket (D4).
- The web shell opens the socket while it shows a signed-in user and closes it on sign-out (D9). A 1008 close shows the sign-in form. Any other close makes it ask `GET /api/session` every 5 s until it can decide to show sign-in or reopen the socket (D8).
- The web shell's dev server forwards WebSocket upgrades under `/api` (D12).
- The health check is unchanged (D10). Revocation is detected by polling, not pushed (D11).

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `instance-access`: adds the `/api/events` socket's requirements: upgrade, recheck, close codes and heartbeat. Extends "A state-changing request comes from an allowed origin" to cover the WebSocket upgrade.
- `web-shell`: adds requirements for holding the socket in the signed-in view and reacting to its close or silence. "The web shell reaches the server through its own origin" adds forwarding WebSocket upgrades. "The access section has unit tests" adds the socket's states.

## Impact

- **Server:** the `ws` feature on axum in `server/Cargo.toml`. A new events route under `server/crates/server/src/access/`, behind the existing guards in `access/mod.rs`. The origin check in `access/guard.rs`. A read-only session lookup in `server/crates/core/src/access/session.rs`, because today's lookup always records a use. New integration tests in `server/crates/server/tests/`, which need a WebSocket client as a dev dependency.
- **Web:** `web/src/Access.tsx` and its tests, and `web/vite.config.ts` (`ws: true` on the `/api` proxy).
- **API:** a new route, `GET /api/events` (WebSocket). No existing route changes its answers.
- **Not breaking:** a non-browser client that sends no `Origin` still passes the origin check.
