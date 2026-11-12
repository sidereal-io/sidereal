## 1. The limiter

- [x] 1.1 [medium] A new `access::limit` limiter with an injected clock refuses a key while any of the tiers 1 per 1 s, 5 per 60 s and 20 per 3600 s is full, with a wait that is the longest over full tiers in whole seconds, rounded up and at least 1, and admits it again once the window passes. Unit tests pin the spec's edges: the sixth attempt 2 s after five failures 2 s apart waits 50, the 21st 15 s apart waits 3300, and 999 ms after one failure waits 1 (verify: cd server && cargo test -p sidereal-server --lib limit) (✓ `cargo test -p sidereal-server --lib limit` passed)
- [x] 1.2 [easy] An IPv4 client address keys by its full address, an IPv6 one by its /64 prefix, and a `Forwarded` whose client is `None`, or no `Forwarded` at all, keys as the one unresolved key (verify: cd server && cargo test -p sidereal-server --lib limit) (✓ `cargo test -p sidereal-server --lib limit` passed)
- [x] 1.3 [medium] An admitted attempt's guard settles it: success removes the key's entry, a release removes only that attempt's own instant, even when a later attempt from the same key settled first, and dropping the guard unsettled keeps the instant. A refusal adds no instant (verify: cd server && cargo test -p sidereal-server --lib limit) (✓ `cargo test -p sidereal-server --lib limit` passed)
- [x] 1.4 [easy] The limiter marks only the first refusal of a run as new, until a check finds room again, as before a 400, and an admit 60 s or more after the last sweep drops every key with no instant in the last hour (verify: cd server && cargo test -p sidereal-server --lib limit) (✓ `cargo test -p sidereal-server --lib limit` passed)

## 2. The sign-in route [depends-on: 1]

- [x] 2.1 [medium] `app_with_tuning(database, allowed_origins, Tuning)` replaces `app_with_event_timing`, with `Tuning { events, clock }` and a `Default`, and builds one limiter per call. The events tests use it and still pass (verify: cd server && cargo test -p sidereal-server --test events) (✓ `cargo test -p sidereal-server --test events` passed, 30 tests)
- [x] 2.2 [complex] From a limited address, `POST /api/session` answers 429 with `{"error":"too many attempts"}` and `Retry-After`, sets no cookie and creates no session, for correct credentials and a malformed body alike, before any password check. Router tests with a manual clock cover the three tiers, a known and an unknown username limited alike, refusals that do not extend the wait, 25 malformed bodies that do not count, and `GET /api/claim` still answering 200 (verify: cd server && cargo test -p sidereal-server --test access sign_in_limit) (✓ `cargo test -p sidereal-server --test access sign_in_limit` passed, 11 tests)
- [x] 2.3 [medium] A 200 clears its address's count and no other, a 503 from closed database connections does not count, two IPv4 addresses and two IPv6 /64s behind a trusted proxy are counted apart, one /64 shares a count, unresolved `X-Forwarded-For` entries share one count apart from resolved ones, and a second router on the same database starts with empty counts (verify: cd server && cargo test -p sidereal-server --test access sign_in_limit) (✓ `cargo test -p sidereal-server --test access sign_in_limit` passed, 21 tests)
- [x] 2.4 [medium] Ten wrong sign-ins sent at the same moment from one address get exactly one 401 and nine 429s (verify: cd server && cargo test -p sidereal-server --test access sign_in_limit_concurrent) (✓ `cargo test -p sidereal-server --test access sign_in_limit_concurrent` passed)
- [x] 2.5 [medium] The spawned server writes exactly one stderr line starting `sign-in limited:` for five failures and then three refusals from `127.0.0.1`, and the line names `127.0.0.1` and the wait and not the username. Five failures alone write no such line (verify: cd server && cargo test -p sidereal-server --test access spawned) (✓ `cargo test -p sidereal-server --test access spawned` passed, 6 tests)

## 3. The sign-in screen

- [x] 3.1 [medium] A sign-in answered 429 shows `Too many failed sign-ins. Try again in N seconds.` under 60 s and `… N minutes.` rounded up from 60 s, with the singular for 1, and `Try again later.` when `Retry-After` is missing or not a whole number. The form stays usable and no server text appears. The claim form is unchanged (verify: cd web && pnpm test -- Access) (✓ `pnpm test -- Access` passed, 129 tests)

## 4. Docs and integration [depends-on: 2, 3]

- [x] 4.1 [easy] `docs/install.md` gains a `## Sign-in limits` section straight after the reverse-proxy section. It states the three tiers, the 429 and `Retry-After`, that a correct password does not get through while limited, that success and a restart clear the count, and that clients behind a proxy that sends no `X-Forwarded-For` share one limit (verify: grep -q '^## Sign-in limits' docs/install.md && grep -q '429' docs/install.md && grep -q 'Retry-After' docs/install.md) (✓ the three `grep -q` checks passed)
- [ ] 4.2 [easy] The whole gate passes. Then check by hand with `just dev`: five wrong sign-ins within a minute, then a sixth, show `Too many failed sign-ins. Try again in N seconds.` and leave the form usable (verify: just check)

## Deviations Ledger

<!-- Dated notes on small departures from the plan, written during apply. Stop and ask the user about a large one. -->

- 2026-11-11, task 2.2: two existing tests sent a failed sign-in and then another sign-in from one address at once, which the 1-per-second tier now refuses. `session_refusals_do_not_reveal_whether_the_username_exists` in `tests/access.rs` now sends its second attempt from `192.168.1.21`. `spawned_reset_state_sets_the_password_and_ends_every_session` in `tests/siderealctl.rs` signs in with the new password on a new router. Each test still checks what it checked before.
- 2026-11-11, task 2.2: `sign_in_limit_refuses_before_reading_the_database` closes the pool after limiting the address and expects 429, not 503. It is the observable form of "before any password check".
