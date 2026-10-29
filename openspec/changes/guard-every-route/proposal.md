## Why

The walking skeleton (#447) protects one route by hand: `GET /api/session` checks its own cookie. Every route that later stories add would have to remember to do the same. This change makes protection the default for the whole `/api` surface, so a forgotten check cannot expose a route, and it settles the CSRF and origin behaviour that ADR-007 lists as a prerequisite for each protected surface.

## What Changes

- A guard on every `/api` route. A request needs a session whose user has role `admin`, or it answers 401 with `{"error":"unauthenticated"}` and clears a stale `sidereal_session` cookie (D6, D7). The public exceptions are `GET /api/claim`, `POST /api/claim` and `POST /api/session`. Health probes, `/` and the web shell's files stay public. An unknown `/api` path without a session answers 401, not 404 (D9).
- A guarded request counts as a use of the session: the expiry slides and the cookie is set again (D14). A revoked or expired session loses access on its next request (D13).
- An origin check on every state-changing `/api` request, public routes included (D4, D5). The `Origin` header, or the `Referer` as a fallback, must name the request's own host and port, or an origin listed in `SIDEREAL_ALLOWED_ORIGINS`. A mismatch answers 403 with `{"error":"forbidden"}` before any session lookup (D8). A request with neither header is allowed (D3). Default ports are dropped and the scheme is not compared (D10, D11).
- A new optional setting, `SIDEREAL_ALLOWED_ORIGINS`: a comma-separated list of extra exact origins (D2). A malformed entry stops the server at start (D12). No Compose, `justfile` or Vite change is needed (D15).
- ADR-007 is amended: the origin check replaces the CSRF token it named (D1).
- **BREAKING** for any caller of `/api` without a session: there are none today beyond the web shell, which already treats 401 as signed out (D16).

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `instance-access`: adds the requirements that every `/api` route needs an administrator session, that a refused session answers 401 and clears the cookie, that a guarded request refreshes the session, that state-changing requests pass the origin check, and that `SIDEREAL_ALLOWED_ORIGINS` is parsed at start.
- `compose-instance`: rewrites "The app exposes only static files and health probes", which still says "until authentication exists" (D18). The app answers below 400 to static files, health probes, the claim and sign-in routes, and otherwise only to an administrator session.

## Impact

- `server/crates/server/src/lib.rs`: the router gains the guard and origin layers around `/api`.
- `server/crates/server/src/access/`: the session check moves out of `current_session` into a shared extractor or layer; `AccessError` gains a `Forbidden` variant.
- `server/crates/server/src/config.rs`: `SIDEREAL_ALLOWED_ORIGINS` parsing, with the same secret-free error style as `WEB_ROOT`.
- `server/crates/server/tests/access.rs`: integration tests for each refusal, the allowlist, the origin check and a session revoked mid-use (D19 keeps the existing tests green).
- `docs/decisions/ADR-007-security-and-plugin-trust.md`: amendment.
- `openspec/glossary.md`: **Allowed origin**, already committed on this branch.
- No new crate dependencies are expected; `axum` middleware and `url` (already a dependency) suffice.
- Follow-on stories: #450 inherits the optional setting and may honour `X-Forwarded-Host` from a trusted proxy; #451 applies the same guard to the WebSocket upgrade.
