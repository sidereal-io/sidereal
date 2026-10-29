## Context

- `app_with_database` in `server/crates/server/src/lib.rs` merges `/readyz` and `access::routes()` into one `Router<Database>`. The only session check is inside `current_session` and `sign_out`, each reading the cookie and calling `Database::session` or `Database::sign_out` itself.
- `Database::session(token)` already does everything a guard needs: it refuses an unknown or expired token, deletes an expired row, slides the expiry at most once an hour, and returns the `User` with its `Role`. `Role` has one variant, `Admin`.
- `AccessError` gives every refusal a fixed body. `cookie::session` and `cookie::cleared` build the two `Set-Cookie` values; `cookie::token` reads the request cookie.
- Configuration helpers in `config.rs` take `Option<&OsStr>`, return typed errors whose `Display` never carries the value, and are unit-tested in place. `main.rs` reads them before the database.
- The server has no CORS layer and no `tower-http` features beyond `fs`. `url` is already a dependency of the server crate. `tower` is a dev dependency for `oneshot`.
- Integration tests in `tests/access.rs` drive the `Router` with `oneshot` through a `send` helper that sets no `Origin`, and a `spawned` module runs the real binary with an environment.
- #450 will add trusted proxies and may let `X-Forwarded-Host` stand in for `Host`. #451 will apply the same guard to a WebSocket upgrade. Both need the guard as a reusable layer, not code inside handlers.

## Goals / Non-Goals

**Goals:**
- One place decides who may call `/api`. Adding a route adds no code to be protected.
- One database read per guarded request. The guard looks the session up and hands the `User` to the handler.
- The origin check costs no database read and runs before the session check.
- Parsing of `SIDEREAL_ALLOWED_ORIGINS` fails at start, with the same secret-free error style as `WEB_ROOT`.
- Existing tests keep passing without edits other than the new router constructor argument.

**Non-Goals:**
- Forwarded headers, `Secure` from the request scheme, `X-Forwarded-Host` (#450).
- WebSocket upgrades (#451). Rate limiting. Roles other than `admin`.
- A CSRF token or a custom request header. A CORS layer of any kind.
- Changes to the web shell, `justfile`, Compose files or the install guide.

## Test Seams

- **Primary test seam:** `server/crates/server/tests/access.rs`, driving the `Router` with `oneshot`. The `send` helper grows an optional header list so a test can set `Host`, `Origin` and `Referer`. Every refusal, the allowlist, the cookie clearing, the cookie refresh and the revoked session are asserted here, with `PgConnection` reads for row state as the lifetime tests already do.
- **Existing seams reused:** the `spawned::Server` module in the same file for `SIDEREAL_ALLOWED_ORIGINS` at start (a malformed value exits before readiness; a list is accepted). Unit tests in `config.rs` for the parser and the origin comparison, beside the `web_root` tests.

## Scrutiny Tier & Blast Radius

- **Tier:** high. This is the authorization boundary for the whole API.
- **Blast radius:** every `/api` route, now and later; the `sidereal_session` cookie contract; server startup, which gains a configuration error; `tests/access.rs`; `tests/database.rs` and `main.rs`, which call `app_with_database` and gain its new argument. Nothing in `core`, `plugin-abi`, the packs, the database schema, `web/` or the Compose files.

## Decisions

1. **A dedicated `/api` router with two `route_layer`s, merged into the app.** `access::routes()` becomes the `/api` router. It holds the access routes plus three catch-all routes bound with `axum::routing::any` to a 404 handler: `/api`, `/api/` and `/api/{*rest}`. `any` matters: with `get`, an authenticated `POST /api/unknown` would answer 405 instead of 404. All three are needed because axum's `{*rest}` never matches an empty segment, so `/api/` would otherwise fall through to the web root. The router gets `.route_layer(session_guard)` then `.route_layer(origin_check)`, so the origin check is the outer layer and runs first. `route_layer` applies only to matched routes, including each route's method fallback. That is why the catch-all is a route and not a fallback: an unknown `/api` path is matched, so it is guarded and answers 401 before 404. An unsupported method on a public path, such as `PUT /api/claim`, is also guarded and answers 401, not 405, which the compose-instance delta states. `/readyz`, `/healthz` and the web root are outside this router and untouched. *Alternative:* `nest("/api", …)` with a fallback. Rejected: a nested router sees a stripped URI, so the allowlist's path matching would depend on nesting depth. *Alternative:* an `Admin` extractor required by each handler. Rejected: that is the opt-in the story exists to remove.

2. **The session guard is `axum::middleware::from_fn_with_state(database, …)`.** It skips the public `(method, path)` pairs, compared against the full request path: `GET` and `HEAD` on `/api/claim`, `POST /api/claim` and `POST /api/session`. `HEAD` is listed because axum routes it to the `GET` handler while the request still says `HEAD`. Otherwise it reads the cookie, calls `Database::session`, and on `None` answers `AccessError::Unauthenticated`, adding `cookie::cleared` when a cookie was present. On `Some(user)` it requires `Role::Admin` and inserts the `User` into the request extensions. It then runs the inner service and appends `cookie::session(token)` to the response. It skips that when the response already carries a `sidereal_session` cookie, because sign-out clears the cookie and must win. *Alternative:* a `tower::Layer` struct. Rejected: `from_fn_with_state` is the project's first middleware and needs no reuse beyond #451, which can call the same function.

3. **Handlers take `Extension<User>`; `current_session` and `sign_out` stop looking the session up.** `sign_out` still needs the token to delete the row, so it keeps `cookie::token(&headers)`; the guard has already proven it valid. This keeps one database read per request and removes the duplicated cookie logic. The `Unavailable` path stays: a database error in the guard answers 503 through `AccessError`.

4. **The origin check is `from_fn_with_state(allowed_origins, …)` and compares normalized origins.** For a method outside `GET`, `HEAD`, `OPTIONS`, it takes `Origin`, else the `Referer`. It parses the header with `url::Url` and reduces it to scheme, lowercase host and port. A port of 80 or 443 becomes `None`, whatever the scheme, because the `Host` header carries no scheme to tell them apart. The `Host` header is parsed by prefixing `http://` and reduced the same way, so `Host: photos.example:443` and `Origin: https://photos.example` compare equal. Allowed when host and port match `Host`, or when scheme, host and port match a list entry. A header that fails to parse, including `Origin: null`, is a mismatch. Neither header present passes. Refusal is `AccessError::Forbidden`, a new variant with status 403 and body `{"error":"forbidden"}`. *Alternative:* compare the scheme against `Host` too. Impossible until #450 tells the server its scheme. *Alternative:* Frigate's custom-header rule. Rejected by the brainstorm (D1, D2).

5. **`AllowedOrigins` is a config type parsed in `config.rs`, passed into `app_with_database`.** `allowed_origins(value: Option<&OsStr>) -> Result<AllowedOrigins, AllowedOriginsError>` splits on commas, trims, and parses each item with `url::Url`. It refuses an item whose scheme is not `http` or `https`. It also refuses an item with no host, a non-root path, a query, a fragment, a trailing slash, credentials, or a `*`. Each accepted item is stored normalized like a request origin: lowercase host, and a port of 80 or 443 stored as `None`, so `https://photos.example:443` matches a browser's `Origin: https://photos.example`. The error's `Display` names the variable and the problem, never the item. `main.rs` reads it before the database, like `WEB_ROOT`. `app_with_database(database, allowed_origins)` gains the argument; tests pass `AllowedOrigins::default()`. The type is `Clone` and wraps an `Arc<[Origin]>` so the layer clones cheaply.

6. **No new dependency.** `axum::middleware`, `url` and the existing `cookie` module cover it. `tower-http`'s `cors` feature stays off, so no `Access-Control-Allow-Origin` header can appear.

7. **ADR-007 is amended, not superseded.** The decision that changes is one clause: "a CSRF token on state-changing requests" becomes the origin check with a same-origin default and an optional exact list. The ADR step writes the amendment; no new record is needed because the rest of ADR-007 stands and the trust model does not change.

## Risks / Trade-offs

- [A proxy that rewrites `Host` makes every browser mutation 403] → `SIDEREAL_ALLOWED_ORIGINS` lists the public origin; the 403 body is distinct from 401 so the cause is visible; #450's guide covers it. Caddy's default `reverse_proxy` preserves `Host`, so the common case needs nothing.
- [A browser that sends `Origin: null`, for example from a sandboxed iframe or a `file:` page] → refused, which is the safe answer for a cookie-bearing mutation.
- [The scheme is not compared against `Host`, so `http://photos.example` passes for an `https` instance] → an attacker on `http://photos.example` already controls DNS or the network; this is not a CSRF vector. #450 can tighten it once the scheme is known.
- [The guard appends a `Set-Cookie` to every guarded response, including errors from the handler] → the cookie value is the one the browser already holds; sign-out's cleared cookie is detected and left alone.
- [Existing tests send no `Origin` and would silently pass a broken origin check] → the new tests set `Host` and `Origin` explicitly for both the allow and the refuse paths.
- [`route_layer` ordering is easy to get backwards] → one test sends a cross-site request with an unknown cookie and asserts 403, not 401.
- [A catch-all route is missed, and a path under `/api` reaches the web root unguarded] → tests cover `/api`, `/api/` and a deeper unknown path, each answering 401 without a session.

## Migration & Rollback

N/A. No schema or data changes. Rolling back is redeploying the previous binary; `SIDEREAL_ALLOWED_ORIGINS`, if set, is ignored by it.

## Open Questions

- Whether #450 should let a trusted proxy's `X-Forwarded-Host` stand in for `Host` in the same-origin comparison. It does not change this change's specs or tasks.
