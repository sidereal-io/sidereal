## 1. Shared contracts

- [x] 1.1 [medium] `config::allowed_origins` turns an unset or empty `SIDEREAL_ALLOWED_ORIGINS` into no extra origin, a comma-separated list into normalized origins (lowercase host, a port of 80 or 443 stored as none, spaces trimmed), and refuses an item with a path, trailing slash, wildcard, other scheme, credentials or no host with an error whose text names the variable and not the item (verify: cd server && cargo test -p sidereal-server allowed_origins) (✓ cd server && cargo test -p sidereal-server allowed_origins passed)
- [x] 1.2 [easy] `app_with_database(database, allowed_origins)` takes the parsed list, `main.rs` reads it before the database, and every caller in `main.rs`, `tests/access.rs` and `tests/database.rs` passes it (verify: cd server && cargo test -p sidereal-server --no-run) (✓ cd server && cargo test -p sidereal-server --no-run passed)
- [ ] 1.3 [medium] The spawned binary exits non-zero before readiness with `SIDEREAL_ALLOWED_ORIGINS` naming a malformed item, with stderr naming the variable and not the item, and becomes ready with a valid list (verify: cd server && cargo test -p sidereal-server --test access allowed_origins_at_start)

## 2. Session guard [depends-on: 1]

- [ ] 2.1 [medium] A dedicated `/api` router with `any`-bound catch-alls for `/api`, `/api/` and `/api/{*rest}` behind a `route_layer` session guard: without a cookie, `GET /api/does-not-exist`, `GET /api/`, `GET /api`, `PUT /api/claim` and `DELETE /api/session` answer 401 `{"error":"unauthenticated"}`, while `GET /api/claim`, `HEAD /api/claim`, `POST /api/claim`, `POST /api/session`, `/healthz`, `/readyz` and `/` with a web root do not (verify: cd server && cargo test -p sidereal-server --test access guard_)
- [ ] 2.2 [medium] A cookie that matches no row, an expired row, or a row deleted after sign-in answers 401 with `sidereal_session` set empty and `Max-Age=0`, and a 401 to a request with no cookie sets no cookie (verify: cd server && cargo test -p sidereal-server --test access stale_cookie)
- [ ] 2.3 [medium] The guard hands the `User` to handlers through request extensions and appends the refreshed `sidereal_session` cookie to every guarded response, so `GET /api/does-not-exist` with a valid cookie answers 404 with the same token and `Max-Age=2592000`, sign-out's cleared cookie still wins, and `current_session` and `sign_out` no longer look the session up themselves while the existing session tests stay green (verify: cd server && cargo test -p sidereal-server --test access session_)

## 3. Origin check [depends-on: 2]

- [ ] 3.1 [medium] An origin comparison unit: `Origin` or the `Referer`'s origin against `Host`, matching on lowercase host and port with 80 and 443 dropped on both sides and the scheme ignored, matching a listed origin on scheme, host and port, and treating an unparsable value or `null` as a mismatch (verify: cd server && cargo test -p sidereal-server origin_matches)
- [ ] 3.2 [medium] A `route_layer` origin check outside the session guard, for methods other than `GET`, `HEAD` and `OPTIONS`: same-origin sign-in answers 200, `Host: photos.example:443` with `Origin: https://photos.example` and the reverse both answer 200, a cross-site sign-in answers 403 `{"error":"forbidden"}` with no cookie and no session row, a cross-site sign-out with a valid cookie answers 403 and keeps the row, a matching `Referer` passes and a mismatched one is refused, neither header passes, `GET /api/claim` with a foreign `Origin` answers 200 with no `Access-Control-Allow-Origin`, and a listed origin passes with a different `Host` (verify: cd server && cargo test -p sidereal-server --test access origin_)
- [ ] 3.3 [easy] The origin check runs before the session check: a cross-site `DELETE /api/session` with an unknown cookie answers 403, not 401, and no database read happens (verify: cd server && cargo test -p sidereal-server --test access origin_before_session)

## 4. Integration [depends-on: 3]

- [ ] 4.1 [easy] The whole gate passes, and the spec scenarios in the delta each map to a test by name (verify: just check)

## Deviations Ledger

