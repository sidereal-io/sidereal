## Why

Sidereal has no users and no sessions, so any network caller could become its administrator once mutating routes exist. ADR-007 forbids an unauthenticated mutation mode but leaves the setup flow undecided, which blocks every story under #388. This change is the walking skeleton: the thinnest path through claim and sign-in, with the tables, the token and the cookie every later access story builds on, and the ADR-007 amendment that lets implementation begin.

## What Changes

- A `users` table and a `sessions` table arrive as a third forward-only migration with its schema baseline (D7). A user has a username, an argon2 password hash and a role; a session holds the SHA-256 hash of an opaque token, its authentication type, and its created, last-used and expiry times (D8).
- `GET /api/claim` tells whether the instance claim is open. `POST /api/claim` creates the one administrator while no administrator exists, atomically, and answers 201 with no cookie (D1, D2). Credentials must meet the length rules (D3). The claim answers 409 once an administrator exists and 503 when PostgreSQL cannot be read (D11).
- `POST /api/session` signs in and sets the session cookie; `GET /api/session` says who is signed in; `DELETE /api/session` signs out, deletes the row and expires the cookie (D12). A failed sign-in is one fixed 401 and always runs a hash (D10).
- The cookie is `sidereal_session`, `HttpOnly`, `SameSite=Lax`, `Path=/`, with a 30-day `Max-Age` (D13). `Secure` has an "arrived over HTTPS" input that is always false in this change (D5).
- A session expires 30 days after last use and is refreshed on use; an expired session met on use is deleted and treated as absent (D9). The periodic sweep is #456.
- The server writes one warning line on every start while the instance is unclaimed (D15).
- The web shell's home screen gains an access section beside the health status: the claim form while unclaimed, the sign-in form while signed out, and the username with a sign-out button while signed in (D6). The dev server forwards `/api` to the server (D16).
- ADR-007 is amended in place (D4): each prerequisite gates the surface it protects, the claim is the one unauthenticated mutation and is permitted only while no administrator exists, and at-rest key custody is decided by the first feature that stores a provider secret.
- Out of scope, as the story says: guarding other routes, CSRF and allowed origins (#448), `sidectl` (#449), proxy awareness (#450), a router, rate limiting (#452) and audit rows (#453).

## Capabilities

### New Capabilities

- `instance-access`: the instance claim, sign-in, who-am-I and sign-out routes, the session and its cookie, the credential rules, the unclaimed-start warning, and the integration tests for the claim race, restart, unavailable persistence and expired sessions.

### Modified Capabilities

- `postgres-foundation`: the first-startup scenario counts three successful migration records and the application tables `sidereal_metadata`, `_sqlx_migrations`, `users` and `sessions`.
- `web-shell`: the home screen shows the access section in its three states beside the health status, with unit tests for each state, and the dev server forwards `/api` to the server.

## Impact

- `server/crates/core`: migration `3_users_and_sessions.sql`, baseline `schema/3.json`, and a new module for users, password hashing and sessions. New dependencies for argon2 password hashing, SHA-256 and random bytes.
- `server/crates/server`: the `/api/claim` and `/api/session` routes, the cookie builder, the start warning in `main.rs`, and new integration tests under `tests/`.
- `web/src`: `App.tsx` gains the access section and its components, with Vitest tests; `vite.config.ts` gains the `/api` proxy entry. Styling uses existing tokens; any new semantic token is named in the design.
- `docs/decisions/ADR-007-security-and-plugin-trust.md`: amended in place.
- No change to Compose files, `sidectl`, or the v0.x line.
