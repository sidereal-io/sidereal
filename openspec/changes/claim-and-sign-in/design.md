## Context

- The server is two layers: `sidereal-core` owns PostgreSQL startup, the two migrations under `crates/core/migrations/`, the schema baselines `schema/0.json` to `schema/2.json`, and the `Database` handle with its pools. `sidereal-server` is a thin axum binary: `app()` serves `/healthz`, `app_with_database()` adds `/readyz` with `State<Database>`, and `main.rs` initializes the database before binding the listener. Nothing in either crate knows about users.
- `Database`'s pools are private. Every query the HTTP layer needs goes through a method on `Database` or a function in `core` that takes the pool.
- Startup verifies the live schema against the baseline for the number of applied migrations, so a new migration needs a new baseline file, regenerated with `just db-schema`.
- Existing integration tests in `crates/server/tests/` use `TestDatabase` from `core/src/database/test_support.rs` for a fresh database per test, drive the router with `tower::ServiceExt::oneshot`, and spawn the real binary with `CARGO_BIN_EXE_sidereal-server` when they need stdout or stderr.
- The web shell is one screen: `App.tsx` renders a heading and `HealthStatus`. Unit tests stub `fetch` with `vi.stubGlobal`. The Vite dev server proxies `/healthz` only. `base.css` uses tokens through `var(--sr-…)` custom properties.
- The server has no `serde` derive, no password hashing, no random source and no cookie handling among its dependencies.
- Constraints: `core` stays domain-agnostic and never depends on `server`; `packs/astro` is untouched; PostgreSQL only; `just check` must stay green; DESIGN.md governs the forms' look.

## Goals / Non-Goals

**Goals:**

- One place for user and session storage that `sidectl` (#449) and the route guard (#448) can reuse without HTTP.
- A claim that the database makes atomic, so no process-level lock or in-memory flag exists to drift after a restart.
- A session check that later stories turn into a middleware without changing the session table or the cookie.
- Every secret-bearing failure maps to a fixed body, in the style of `DatabaseError`.

**Non-Goals:**

- Guarding `/readyz`, the static files or any route other than the four access routes.
- CSRF tokens, allowed origins, a trusted proxy, rate limiting, audit rows, `sidectl`.
- A session sweep (#456), a router in the web shell, or new design tokens.

## Test Seams

- **Primary test seam:** the axum router driven with `oneshot` against a `TestDatabase`, in a new `crates/server/tests/access.rs`. It proves the claim race, the closed claim, sign-in, who-am-I, sign-out, expiry, refresh, restart (a second `app_with_database` on the same database) and unavailable persistence (`database.close().await` first).
- **Existing seams reused:** the spawned binary with captured stderr in `tests/database.rs`, for the unclaimed-start warning; `schema_baselines_are_generated_and_stable` in `core` for the new baseline; `core`'s migration tests for the three-record count; Vitest with a stubbed `fetch`, as in `HealthStatus.test.tsx`, for the access section.

## Scrutiny Tier & Blast Radius

- **Tier:** high
- **Blast radius:** a new migration and baseline in `core`; a new `core::access` module and new public methods on `Database`; four new routes, a cookie builder and a cookie parser in `server`; `main.rs` start warning; `App.tsx`, a new `Access` component with its stylesheet and tests, `vite.config.ts`; ADR-007; three dependency additions. This is the first trust boundary in the server.

## Decisions

1. **Users and sessions live in `core::access`; HTTP lives in `server::access`.** `core::access` holds the types `User`, `Role`, `Session`, the credential rules, password hashing and the queries, exposed as methods on `Database`: `claim_open()`, `claim(username, password)`, `sign_in(username, password)`, `session(token)`, `sign_out(token)`. `server::access` holds the handlers, the JSON bodies, the error mapping and the cookie. Alternative: everything in `server`. Rejected because `sidectl` needs the same create and reset without HTTP, and the migration already belongs to `core`.

2. **The database enforces one administrator.** Migration `3_users_and_sessions.sql` creates `users` with a partial unique index on `(role) WHERE role = 'admin'`. The claim is a single `INSERT`; a unique violation (SQLSTATE `23505`) maps to 409. Alternative: a transaction with `SELECT … FOR UPDATE` or an advisory lock. Rejected because the index needs no lock protocol and cannot be bypassed by a second process. When a later release allows several administrators, a migration drops the index.

3. **Argon2id through the `argon2` crate, run on the blocking pool.** The hash is stored as a PHC string in `users.password_hash`. Hashing and verification run in `tokio::task::spawn_blocking`, because the default parameters take tens of milliseconds. For an unknown username, sign-in verifies the password against one fixed PHC string computed once at first use, so both failures do the same work. Alternative: bcrypt as Immich uses. Rejected: argon2 is the story's named choice and the current recommendation.

4. **Token: 32 random bytes from `rand`, base64url without padding in the cookie, SHA-256 digest from `sha2` in `sessions.token_hash BYTEA UNIQUE`.** Alternative: hex. Rejected only for cookie length; either would do.

5. **Session expiry is set by code, not a column default.** `sessions.expires_at` is `last_used_at + 30 days`, written on sign-in and on refresh. The constant lives once, in `core::access`. The lookup is one `SELECT` joined to `users`; an expired row is deleted and treated as absent; a row last used over an hour ago gets one `UPDATE` moving `last_used_at` and `expires_at`. Alternative: refresh on every request. Rejected to keep one write per hour per session, as Immich does, within the ledger's allowance (D9).

6. **Cookie handling is hand-written.** One function builds the `Set-Cookie` value for sign-in, one for sign-out, and one reads the `sidereal_session` value out of the `Cookie` header by splitting on `;` and the first `=`. `session_cookie(token, secure: bool)` takes the HTTPS flag; the handlers pass `false` with a comment pointing at #450. Alternative: `axum-extra` with the `cookie` feature. Rejected: one cookie, no signing, and the parser is a few lines with unit tests.

7. **Fixed error bodies from one `AccessError` enum that implements `IntoResponse`.** `InvalidCredentials` → 400 on claim and 401 on sign-in, `Claimed` → 409, `Unauthenticated` → 401, `Unavailable` → 503. Every `sqlx::Error` that is not the unique violation maps to `Unavailable`, which is what a closed pool returns. Request bodies are parsed with `Result<Json<Credentials>, JsonRejection>` and every rejection maps to `InvalidCredentials`, so a malformed body is 400 rather than axum's 415 or 422. `serde` with `derive` is added for the bodies.

8. **The start warning is a `Database` call in `main.rs`.** After initialization, `database.claim_open().await` decides whether to write `instance unclaimed: open the web shell and claim it` to stderr. A database error there does not stop startup; the server starts and the claim route reports 503.

9. **The web access section is one component with a small state machine.** `Access.tsx` owns `state: "loading" | "unclaimed" | "signed-out" | { user }` and `message: string | null`. On mount it calls `GET /api/claim`, then `GET /api/session` when the claim is closed. A `CredentialsForm` renders both forms with a `label` prop. The forms use primitives directly under DESIGN.md's allowance until semantic tokens exist: button `violet-600` with `white` label and `radius-4`; fields `obsidian-900` background, `obsidian-800` border at `border-width-1`, `radius-4`; the refusal message is body text in `obsidian-400`, not a status color, because DESIGN.md allows a status only inside a badge. **Tokens added: none.** No existing semantic token covers these roles, and the user chose to defer the semantic tokens and Penpot drawing to a later UI story.

10. **The Vite proxy gains `/api`.** Alternative: a proxy entry per route. Rejected: one prefix is enough and matches the server's route layout.

11. **ADR-007 is amended in place.** The ADR step edits the Decision and Consequences and dates the amendment.

## Risks / Trade-offs

- [Hashing on the request path can be slow under the default Argon2 parameters] → run it on the blocking pool; the defaults are acceptable for one administrator, and rate limiting is #452.
- [A regenerated `schema/3.json` can drift from the migration if someone edits the SQL later] → the baseline test fails in `just check` on any drift; `just db-schema` regenerates.
- [`database.close()` as the "unavailable" seam closes both pools, so the test cannot reuse the handle] → each such test takes its own `TestDatabase`.
- [Hand-written cookie parsing can mis-split a value with `=`] → base64url has no `=`, and the parser splits on the first `=` only; unit tests cover a cookie among others.
- [An unclaimed instance on an open port can be claimed by a stranger] → the start warning and the install guide say to claim at once; `sidectl` (#449) restores control.
- [The `users` partial index assumes one administrator] → stated in the glossary for this release; a later migration lifts it.

## Migration & Rollback

- `3_users_and_sessions.sql` creates `users` and `sessions` and the partial unique index, forward-only, applied at startup under the existing lock. `schema/3.json` is regenerated with `just db-schema`. The `First startup` and `Concurrent startup` counts in `core` and `server` tests move from two to three records.
- Rollback: none, as the foundation spec forbids downgrades. An older binary refuses to start against the migrated database with `incompatible migration history`, which is the existing behavior for any newer schema.
