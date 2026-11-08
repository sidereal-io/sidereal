## Story

#451 events-socket — a live connection lives and dies with the session
https://github.com/sidereal-io/sidereal/issues/451

## Decisions

| # | Question | Answer | Source |
|---|---|---|---|
| D1 | Does the story fit one change, or split? | One change: server guard and recheck, heartbeat, web shell socket and close handling, Vite proxy, integration tests. | Assumption |
| D2 | Does the origin check apply to the upgrade, which is a `GET`? | Yes. A named origin that is not allowed gets 403 before the session check. No `Origin` or `Referer` still passes, as for HTTP. | User |
| D3 | Does a periodic recheck count as a use of the session? | No. The upgrade is a guarded request and counts as a use. Rechecks only read, so an idle open tab still expires 30 days after its last real request. | User |
| D4 | What is the heartbeat, and who notices a dead connection? | Every 30 s the server rechecks the session and sends the text frame `{"type":"heartbeat"}`. It closes a client that has sent nothing, pongs included, for 60 s. The shell treats 60 s with no message as a dead socket. | User |
| D5 | How often does the server recheck, and on what? | Before each heartbeat, so at least every 30 s, and on every text or binary message the client sends. The client's message content is ignored. | Evidence: #451 Scope |
| D6 | How is a gone session reported? | Close code 1008 (policy violation), for both a revoked and an expired session. | Evidence: #451 Scope |
| D7 | What happens when a recheck cannot reach PostgreSQL? | Close with 1011 (internal error), never 1008. | User |
| D8 | How does the shell react to a close? | 1008 shows the sign-in form at once, with no extra message. Any other close, or a dead socket: wait 5 s, then `GET /api/session`. 401 shows sign-in, 200 reopens the socket, anything else waits 5 s and asks again. | User |
| D9 | When does the shell open and close the socket? | It opens the socket while it shows a signed-in user, and closes it on sign-out or when it leaves the signed-in view. | Assumption |
| D10 | Does the health check change? | No. `HealthStatus` keeps polling `/healthz`, which is public and works signed out. | Assumption |
| D11 | Is revocation pushed to the socket at once, for example with PostgreSQL `LISTEN`? | No. Polling at each recheck is enough for this story. | Evidence: #451 Scope |
| D12 | What does the server need that it lacks? | axum's `ws` feature, and `ws: true` on the Vite dev proxy for `/api`. | Evidence: server/Cargo.toml, web/vite.config.ts |
| D13 | Does the story add a glossary term? | No. Specs name the `/api/events` socket directly, and Session already covers the rest. | Assumption |

## Changes from the story

- The story's "rechecks at least once a minute" is tightened to every 30 s, tied to the heartbeat (D4, D5).
- Added: a recheck that cannot reach PostgreSQL closes with 1011, so the shell retries instead of signing out (D7).
- Added: after a close that is not 1008, the shell asks `GET /api/session` before it reconnects, because a browser cannot see the status of a refused upgrade (D8).

Confirmed by user: yes
