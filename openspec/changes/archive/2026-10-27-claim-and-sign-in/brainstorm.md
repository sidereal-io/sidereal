## Story

#447 claim-and-sign-in — Sam claims the instance, signs in and out, and the claim stays closed ⭐ walking skeleton
https://github.com/sidereal-io/sidereal/issues/447

## Decisions

| # | Question | Answer | Source |
|---|---|---|---|
| D1 | Does a successful claim also sign Sam in? | No. `POST /api/claim` creates the administrator and answers 201 with no cookie; Sam then signs in with the same credentials. | User |
| D2 | How does the screen learn whether the claim is open? | A public read, `GET /api/claim` → `{"open": true\|false}`, which reads PostgreSQL on every call. Immich does the same with `isInitialized` on its public config route. | User |
| D3 | What rules apply to the claimed credentials? | Username 1–64 characters with no leading or trailing whitespace; password 8–256 bytes. A violation is a 400 before any write. | User |
| D4 | How does the ADR-007 amendment land? | In place: the Decision states that each prerequisite gates the surface it protects and that the claim is the one unauthenticated mutation, permitted only while no administrator exists; Consequences says at-rest key custody is decided by the first feature that stores a provider secret; the front matter dates the amendment. | User |
| D5 | When is the cookie `Secure`? | Never in this story: the server listens on plain TCP only. The cookie builder takes an "arrived over HTTPS" input that is always false here, and the trusted-proxy story feeds it. | User |
| D6 | Where does the access section live? | On the home screen beside the health status, with no router. It shows the claim form while unclaimed, the sign-in form while signed out, and the username with a sign-out button while signed in. | User |
| D7 | Where do the `users` and `sessions` tables live? | In `core`: it owns `migrations/` and the schema baselines, so this is migration 3 with a new `schema/3.json` baseline. | Evidence: `server/crates/core/src/database/startup.rs` |
| D8 | What is a session token? | 32 random bytes; PostgreSQL stores its SHA-256 hash and the browser holds the token in the cookie. | Evidence: #388 Shared decisions |
| D9 | How does a session expire? | 30 days after last use, refreshed on use. An expired row met on use is deleted and the request is treated as signed out. The design may throttle the last-used write, as Immich does with one bump per hour, as long as the 30-day window still moves with use. A periodic sweep is #456. | Evidence: #388 Shared decisions; throttle and split: User |
| D10 | What does a failed sign-in look like? | 401 with one fixed body for unknown username and wrong password alike; an unknown username still verifies against a fixed dummy hash. | Evidence: #447 Scope |
| D11 | What does a closed claim answer? | 409 when an administrator exists; 503 when PostgreSQL cannot be read, and nothing is created. | Assumption |
| D12 | What do the session routes answer? | Sign-in 200 and who-am-I 200 with `{"username","role"}`; who-am-I 401 without a valid session; sign-out 204, deletes the row and expires the cookie. | Assumption |
| D13 | What is the cookie? | Named `sidereal_session`, `HttpOnly`, `SameSite=Lax`, `Path=/`, `Max-Age` of 30 days. | Evidence: #447 Scope; name: Assumption |
| D14 | How are usernames matched? | Exactly, case-sensitive, unique. | Assumption |
| D15 | What does the start warning say? | One stderr line on every start while unclaimed, telling Sam to open the web shell and claim the instance. | Evidence: #447 Scope; wording: Assumption |
| D16 | How does the dev web shell reach `/api`? | The Vite proxy forwards `/api` to the server, as it does `/healthz`. | Evidence: `web/vite.config.ts` |
| D17 | What tests does this story carry? | Server integration tests on the test database for the claim race, restart, unavailable persistence, expired session and the cookie attributes; Vitest unit tests for the three screen states with a mocked fetch. No Playwright test, because a claim would leave the shared fixture claimed. | Evidence: #447 Scope; e2e: Assumption |

## Changes from the story

- Added `GET /api/claim`, a public read of the claim state (D2). The story listed only the mutation, and the screen needs the state.
- `Secure` on the cookie is unreachable in this story (D5). Nothing is cut: the input exists, and the trusted-proxy story sets it.
- Split out #456 `session-sweep`: a periodic delete of expired session rows (D9). This story only refuses and deletes an expired session when a browser presents it, which leaves rows nobody presents again. Immich pairs the on-use check with a nightly cleanup job.

Confirmed by user: yes
