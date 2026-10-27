<!-- Server tests need the fixture: `just db-up`, then `export TEST_DATABASE_URL=$(just db-test-url)`.
     Run cargo from the repo root with `--manifest-path server/Cargo.toml`, as `just check-server` does. -->

## 1. Storage contract

- [ ] 1.1 [medium] A fresh database holds `users` and `sessions` after startup, with the partial unique index that allows one `admin`, as migration `3_users_and_sessions.sql` with a regenerated `schema/3.json`; the first-startup, concurrent-startup and upgrade scenarios count three records and the new tables (verify: `cargo test --manifest-path server/Cargo.toml -p sidereal-core database`)
- [ ] 1.2 [medium] `Database::claim_open()` reads whether an administrator exists, and `Database::claim(username, password)` applies the length rules before any write, answers `Claimed` before hashing once an administrator exists, hashes with Argon2id on the blocking pool, and maps a unique violation from a concurrent first claim to `Claimed` (verify: `cargo test --manifest-path server/Cargo.toml -p sidereal-core access::claim`)
- [ ] 1.3 [medium] `Database::sign_in()` verifies a hash whether or not the username exists and creates a session row with the SHA-256 of a new 32-byte token; `Database::session(token)` returns the user for an unexpired row, deletes an expired row, and moves `last_used_at` and `expires_at` when the last use is over an hour old; `Database::sign_out(token)` deletes the row (verify: `cargo test --manifest-path server/Cargo.toml -p sidereal-core access::session`)

## 2. HTTP routes [depends-on: 1]

- [ ] 2.1 [easy] [parallel] The sign-in `Set-Cookie` value carries `HttpOnly`, `SameSite=Lax`, `Path=/` and `Max-Age=2592000`, adds `Secure` only when the HTTPS input is true, the sign-out value has an empty value and `Max-Age=0`, and the parser finds `sidereal_session` among other cookies separated by `; ` (verify: `cargo test --manifest-path server/Cargo.toml -p sidereal-server cookie`)
- [ ] 2.2 [medium] `GET /api/claim` answers `{"open":true}` while unclaimed, `{"open":false}` once claimed and after a second router on the same database, and 503 `{"error":"unavailable"}` after `database.close()`; `POST /api/claim` answers 201 without a cookie on the first claim, 400 `{"error":"invalid credentials"}` for a short password, a padded username or a body that is not a credentials object, 409 `{"error":"claimed"}` afterwards, exactly one 201 among ten concurrent claims, and 503 when the pool is closed (verify: `cargo test --manifest-path server/Cargo.toml -p sidereal-server --test access claim`)
- [ ] 2.3 [medium] `POST /api/session` answers 200 with `{"username","role"}` and the cookie on correct credentials, an identical 401 for a wrong password and an unknown username, and 400 for a malformed body; `GET /api/session` answers 200 with the cookie and 401 `{"error":"unauthenticated"}` with no cookie or an unknown token; `DELETE /api/session` answers 204 with the clearing cookie, after which the old cookie gets 401, and 401 without a session (verify: `cargo test --manifest-path server/Cargo.toml -p sidereal-server --test access session`)
- [ ] 2.4 [medium] A session whose expiry is in the past gets 401 and its row is deleted; a session last used 29 days ago gets 200 and an expiry at least 30 days away; a cookie from a first router is valid on a second router over the same database; and no response body or captured output from claim and sign-in contains a distinctive password or the stored hash (verify: `cargo test --manifest-path server/Cargo.toml -p sidereal-server --test access lifetime`)
- [ ] 2.5 [easy] The spawned binary writes exactly one stderr line starting with `instance unclaimed:` when it starts against a database with no administrator, and none when an administrator exists, using the `ready()` spawn-and-kill pattern of `tests/storage.rs` with stderr piped (verify: `cargo test --manifest-path server/Cargo.toml -p sidereal-server --test access warning`)

## 3. Web shell [depends-on: 1]

- [ ] 3.1 [medium] [parallel] The home screen's access section shows the claim form with a `Claim` button when `GET /api/claim` answers open, the sign-in form with a `Sign in` button when the claim is closed and `GET /api/session` answers 401, the username and a `Sign out` button when `GET /api/session` answers 200, nothing before the first answers, and `Something went wrong` with no form when `GET /api/claim` answers 503, with `fetch` stubbed in Vitest and the dev proxy forwarding `/api` (verify: `cd web && pnpm test -- Access`)
- [ ] 3.2 [medium] Submitting the claim form moves to the sign-in form on 201; a sign-in answered 401 with `{"error":"<b>nope</b>"}` shows `Invalid username or password`, keeps the form and never shows `nope`; a sign-in answered 200 shows the username; pressing `Sign out` moves to the sign-in form on 204; and the stylesheet uses only tokens, with the submit button computing to `rgb(124, 58, 237)` and a `4px` radius (verify: `cd web && pnpm test -- Access`)

## 4. Gate [depends-on: 2] [depends-on: 3]

- [ ] 4.1 [easy] The whole gate is green, including `cargo fmt --check`, `clippy -D warnings`, the arch lint, `design:lint`, `tokens:check` and the web unit tests (verify: `just check`)

## Deviations Ledger

<!-- Dated notes on small departures from the plan, written during apply. Stop and ask the user about a large one. -->
