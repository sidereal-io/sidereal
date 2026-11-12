## Why

Sidereal publishes a port and has no identity provider to fall back on, so sign-in is only as strong as the administrator's password. Today `POST /api/session` answers every wrong guess at network speed. #450 resolves the real client address behind a trusted proxy, so the server can now slow failed sign-ins per address.

## What Changes

- `POST /api/session` counts failed sign-ins per client address (D1, D6). Each address gets three sliding-window tiers: 1 failure per second, 5 per minute and 20 per hour (D2).
- An address that would pass a tier is answered 429 with `{"error":"too many attempts"}` and `Retry-After` (D7). The answer comes before any credential check, even for the correct password. These 429s do not count and do not extend the wait (D3).
- A successful sign-in clears its address's count (D4).
- Requests with no resolved client address share one counter of their own (D5).
- The counters live in the server process, and a restart clears them (D11).
- The server writes one warning line when an address first passes a tier, with the address and the wait, never the username (D8).
- The web sign-in screen shows the wait for a 429 and keeps the form usable (D9).
- `docs/install.md` gains a short "Sign-in limits" section next to the reverse-proxy guide (D10).

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `instance-access`: adds the sign-in limit: what counts as a failure, how addresses are grouped and counted, the three tiers, the 429 answer, the reset on success, and the warning line.
- `web-shell`: the access section shows the wait for a 429 from `POST /api/session`, and its unit tests cover that answer.
- `compose-instance`: the installation guide states the fixed sign-in limits.

## Impact

- **Server:** `server/crates/server/src/access/` gains an in-process limiter. `sign_in` in `access/mod.rs` consults it, and `AccessError` gains the 429 answer. The limiter reads the client address from `Forwarded` in `access/forwarded.rs`. No database schema change.
- **Web:** `web/src/Access.tsx` and its tests in `web/src/Access.test.tsx`.
- **Docs:** `docs/install.md`.
- **Tests:** new integration tests in `server/crates/server/tests/access.rs`: the limit, separate addresses, a known and an unknown username limited alike, the reset on success, the shared unresolved counter, and IPv6 grouping.
- **API:** a new 429 answer on `POST /api/session`. No existing answer changes for a caller under the limit.
- **Dependencies:** none expected.
