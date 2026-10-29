## Story

#448 guard-every-route — every API route refuses callers without an administrator session
https://github.com/sidereal-io/sidereal/issues/448

## Decisions

| # | Question | Answer | Source |
|---|---|---|---|
| D1 | CSRF token, as ADR-007 says, or an origin check, as the story says? | An origin check. This change amends ADR-007 to say so. | User |
| D2 | What must a state-changing request's origin match? | The request's own `Host` by default: the `Origin` header's host and port equal the `Host` header's. `SIDEREAL_ALLOWED_ORIGINS` is an optional comma-separated list of extra exact origins (`scheme://host[:port]`) for a proxy that rewrites `Host`. Unset means same-origin only. Immich has no such check and Frigate checks against `Host`; neither requires a setting. | User |
| D3 | A state-changing request with neither `Origin` nor `Referer`? | Allowed, as Immich and Frigate do: browsers always send `Origin` on unsafe methods, so a request without one is not a browser. Refuse only when a header is present and does not match. | User |
| D4 | Does the origin check cover `POST /api/claim` and `POST /api/session`? | Yes. Every state-changing `/api` request is checked. The public allowlist skips only the session requirement. | User |
| D5 | Which methods are state-changing? | Every method other than `GET`, `HEAD` and `OPTIONS`. | Assumption |
| D6 | Which routes need a session, and which role? | Every `/api` route, any method, needs a session whose user has role `admin`. Public: `GET /api/claim`, `POST /api/claim`, `POST /api/session`. `/healthz`, `/readyz`, `/` and the web shell's files stay public. | Evidence: issue #448, `openspec/specs/instance-access/spec.md` |
| D7 | What does a refused session look like? | 401 with the body `{"error":"unauthenticated"}`, the same as `GET /api/session` today. When the request carried a `sidereal_session` cookie, the response sets it to empty with `Max-Age=0`. Without a cookie, no `Set-Cookie`. | Evidence: issue #448 scope, `server/crates/server/src/access/mod.rs` |
| D8 | What does a refused origin look like, and which check runs first? | 403 with the body `{"error":"forbidden"}`. The origin check runs before the session check, so a cross-site request costs no database read and its refusal does not depend on the cookie. | Assumption |
| D9 | An unknown `/api` path without a session? | 401, not 404. The guard covers the whole `/api` prefix, including `/api` and `/api/` themselves, so an unauthenticated caller learns nothing about which routes exist. | Assumption |
| D10 | When `Referer` stands in for `Origin`, what is compared? | Only the scheme, host and port of the `Referer` URL, as an origin. The path is ignored. | Assumption |
| D11 | How do `Host` and `Origin` compare across default ports? | Host and port, case-insensitive host, after dropping a port of 80 or 443 on either side. The scheme is not compared, because the server cannot know its own scheme until #450 adds trusted proxies. | Assumption |
| D12 | What if `SIDEREAL_ALLOWED_ORIGINS` holds a malformed entry? | The server refuses to start with a message naming the variable, as `WEB_ROOT` and `STORAGE_ROOT` do. Malformed means: not `scheme://host[:port]`, a path or trailing slash, a wildcard, or an empty item. | Assumption |
| D13 | Does a session revoked mid-use lose access at once? | Yes. A session is a row; the guard reads the row on every request, so deleting it (what `sidectl` will do in #449) makes the next request 401. Expiry is already enforced and deletes the row. | Evidence: `docs/decisions/ADR-015-opaque-database-sessions.md`, `server/crates/core/src/access/session.rs` |
| D14 | Does the guard refresh the session? | Yes, like `GET /api/session`: a guarded request's use slides the expiry in the database and the response sets the cookie again with the same token. | Evidence: `openspec/specs/instance-access/spec.md`, requirement "A session lasts 30 days from its last use" |
| D15 | Do `just dev`, the Vite proxy or Compose need a setting? | No. The Vite proxy forwards the browser's `Host` of `localhost:5173` unchanged (no `changeOrigin`), and Caddy preserves `Host` by default, so the same-origin default holds in both. | Evidence: `web/vite.config.ts`, `justfile` |
| D16 | Does the web shell change? | No. Its requests are same-origin, so the browser sends a matching `Origin`, and `Access.tsx` already treats 401 as signed out. | Evidence: `web/src/Access.tsx` |
| D17 | Does the server ever send `Access-Control-Allow-Origin`? | No, as today: there is no CORS layer, and the spec keeps the requirement. | Evidence: `openspec/specs/compose-instance/spec.md`, `server/crates/server/src/lib.rs` |
| D18 | Which existing spec text is now wrong? | The compose-instance requirement "The app exposes only static files and health probes", which says "until authentication exists". It becomes: static files, health probes, the claim and sign-in routes, and everything else only with an administrator session. | Evidence: `openspec/specs/compose-instance/spec.md` |
| D19 | Do existing tests keep passing? | Yes. The spawned-binary and router tests send no `Origin`, so D3 lets them through, and the sign-in and claim tests use public routes. | Evidence: `server/crates/server/tests/access.rs` |

## Changes from the story

- **Changed**: the CSRF mechanism amends ADR-007 (D1). ADR-007 said "a CSRF token on state-changing requests"; this change records the origin check in its place.
- **Changed**: `SIDEREAL_ALLOWED_ORIGINS` is optional, not the one source of truth (D2). This supersedes the epic's shared decision that Compose defaults it and `just dev` adds the Vite origin. To record on #388, and on #450, whose Compose default and "set the allowed origin to the public URL" guide line shrink to an optional note.
- **Cut**: the `justfile` `dev` recipe setting the Vite origin (D15).
- **Added**: rewriting the compose-instance requirement "The app exposes only static files and health probes" (D18).
- **Split out**: nothing.

Confirmed by user: yes
