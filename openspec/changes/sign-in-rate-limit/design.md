## Context

- `sign_in` in `server/crates/server/src/access/mod.rs` parses the body into `Credentials`, then calls `Database::sign_in` in `sidereal-core`. Core verifies an Argon2 hash for a known and an unknown username alike, so each attempt costs real CPU. It answers 200, 401 (`AccessError::WrongCredentials`), 400 or 503.
- `access/forwarded.rs` puts a `Forwarded` extension on every request when `with_trusted_proxies` wraps the router. `Forwarded.client` is `None` when the nearest untrusted `X-Forwarded-For` entry is not an address. A router built without that layer, as some tests build it, has no `Forwarded` at all. Addresses are already canonical, so an IPv4-mapped IPv6 address arrives as IPv4.
- The server logs with `eprintln!`; there is no logging library. Integration tests that check stderr spawn the server binary (`Server::ready_with` in `tests/access.rs`).
- Tests inject timing already: `app_with_event_timing` takes an `EventTiming` so the events tests need not wait 30 s.
- `AccessError` variants carry no data today, and every refusal has a fixed body.
- `web/src/Access.tsx` maps a refused sign-in through `refusal(status)`, which never shows server text.

## Goals / Non-Goals

**Goals:**

- One small limiter module in the `server` crate, used only by `sign_in`.
- Exact, fast tests of every tier through the existing router seam, with no real waiting.
- No password-hash work for a limited address, and no way to beat the limit by sending attempts in parallel.

**Non-Goals:**

- A general rate-limiting layer for other routes.
- Anything in `sidereal-core` or `plugin-abi`. Core stays unaware of HTTP clients.
- Configurable limits, persistence, or a per-username key.

## Test Seams

- **Primary test seam:** the router, driven in process by `send_from` in `server/crates/server/tests/access.rs`, with a manual clock passed in through the app's constructor. It covers every tier, grouping, the shared unresolved count, the reset on success, concurrency, the 429 answer and the restart case (a second router on the same database).
- **Existing seams reused:** `Server::ready_with` in the same file, for the two warning-line scenarios, which need captured stderr and use the minute tier so real time is enough. `web/src/Access.test.tsx`, with `fetch` stubbed, for the web messages. A unit test module in the limiter for the `Retry-After` arithmetic and key grouping.

## Scrutiny Tier & Blast Radius

- **Tier:** medium
- **Blast radius:** the `POST /api/session` handler and a new module beside it in `server/crates/server/src/access/`; the app constructors in `server/crates/server/src/lib.rs`, with their test callers; the sign-in form in `web/src/Access.tsx`; `docs/install.md`. No schema, no new contract in `plugin-abi`, no change to the trust boundary. It touches authentication, which is why it is not low.

## Decisions

**D-1. The limiter lives in `server/crates/server/src/access/limit.rs`.** It is HTTP policy keyed by client address, next to `forwarded.rs` that resolves the address. Putting it in core would teach core about clients and transports. A tower middleware layer was rejected: it cannot see whether the handler answered 401 without inspecting responses, and the reset on success needs the handler anyway.

**D-2. The key is an enum: `V4(Ipv4Addr)`, `V6(u64)` holding the top 64 bits, and `Unresolved`.** `Forwarded.client == None` and a missing `Forwarded` extension both map to `Unresolved`. A request without `ConnectInfo` already resolves to `0.0.0.0` in `forwarded::layer` and is counted under that address.

**D-3. State is `std::sync::Mutex<HashMap<Key, Entry>>`.** `Entry` holds a `VecDeque<Instant>` of counted attempts in the last hour, at most 20, and a `refused: bool` for the log rule. The lock is never held across an `.await`, so a blocking mutex is correct and cheaper than `tokio::sync::Mutex`. One lock for all keys is enough: each operation is a few deque steps, and every admitted attempt then spends far longer hashing.

**D-4. Three steps in `sign_in`, so a limited address does no work and parallel attempts cannot slip through.**

1. `limiter.refused(key)`: if any tier is full, answer 429 before the body is checked. This makes a malformed body from a limited address a 429.
2. Parse the body. A 400 here never touches the limiter, so it never counts.
3. `limiter.admit(key)`: under one lock, check the tiers again and, if there is room, push `now` onto the deque. It returns an `Attempt` guard, or the wait. The hash runs only after the push, so ten parallel attempts see each other's pushes and nine are refused.

The guard settles the attempt. A 200 calls `succeeded()`, which removes the key's entry. A 503 calls `released()`, which pops this attempt's instant. A 401 needs nothing: the instant stays. Dropping the guard unsettled, as when the client disconnects mid-hash, keeps the instant, so cancelling a request cannot be used to guess for free.

The alternative of counting at settle time was rejected: every attempt in flight would pass the check, which turns the 1-per-second tier into "as many as you can send at once".

**D-5. Tiers are a constant table `[(1, 1 s), (5, 60 s), (20, 3600 s)]`.** A tier is full when the deque holds at least `limit` instants newer than `now - window`. The wait for a full tier is `window - (now - oldest_in_window)`, where `oldest_in_window` is the `limit`-th newest instant. The `Retry-After` value is the largest wait over full tiers, in whole seconds rounded up, at least 1. Instants older than an hour are popped on every access to the entry.

**D-6. Old entries are swept on a timer inside `admit`.** When at least 60 s have passed since the last sweep, `admit` runs `retain` over the map, keeping entries with an instant in the last hour. Memory is then bounded by the number of distinct keys with a counted attempt in the last hour, and each such attempt cost the sender one hash on the server. No cap or eviction policy is needed.

**D-7. The clock is injected.** `Limiter` takes a `Clock`, an `Arc<dyn Fn() -> Instant + Send + Sync>`, with the system clock as default. `app_with_event_timing` becomes `app_with_tuning(database, allowed_origins, Tuning)`, where `Tuning { events: EventTiming, clock: Clock }` implements `Default`. `app_with_database` passes `Tuning::default()`. The events tests switch to `Tuning { events: FAST, ..Tuning::default() }`. A third constructor parameter was rejected: each new knob would add another constructor. `tokio::time::pause` was rejected because the tests use real PostgreSQL connections, and paused time can fire sqlx's pool timeouts.

Each `app_with_tuning` call builds its own `Limiter`, which is how the restart scenario is tested: a second router has empty counts. The limiter reaches `sign_in` as an `Extension` on the router, so `sign_in` keeps `State<Database>`.

**D-8. `AccessError::TooManyAttempts { retry_after: u64 }`.** Its response is 429, `{"error":"too many attempts"}` and a `Retry-After` header. The body stays fixed, as the enum's doc comment promises.

**D-9. The warning line.** When `refused` or `admit` refuses a key whose `refused` flag is false, it sets the flag and the handler writes `sign-in limited: <client address> must wait <n> s` with `eprintln!`. For `Unresolved` the address reads `unresolved client address`. A successful `admit` clears the flag. The line names the request's own client address, not the /64 key, and never the username, which the limiter never sees.

**D-10. The web message.** `signIn` in `Access.tsx` handles a 429 before `refusal`, with a small pure function `limitedMessage(retryAfter: string | null): string`. It accepts only `/^\d+$/`. Under 60 it says seconds; otherwise minutes as `Math.ceil(seconds / 60)`; `1` takes the singular. Anything else says `Try again later.` The claim form keeps `refusal` unchanged, since the claim has no limit.

**D-11. Docs.** A `## Sign-in limits` section in `docs/install.md` straight after `## Put Sidereal behind a reverse proxy`. It states the three tiers, the 429 and `Retry-After`, that a correct password does not get through while limited, that success and restart clear the count, and that clients behind a proxy that sends no `X-Forwarded-For` share one limit.

**Tokens.** None. The 429 message reuses the existing `access-message` style.

## Risks / Trade-offs

- [Everyone behind one address, or with an unresolved address, shares a limit; an attacker there can keep Sam out for up to an hour] → Accepted in the brainstorm (D5). Sam can sign in from another address, and a restart clears the counts. The docs say so.
- [An attacker spread over many IPv6 /64s or IPv4 addresses is limited per address only] → Out of scope; the hash cost bounds the total rate, and a per-username limit was rejected because it would let anyone lock out the one administrator.
- [The 1-per-second tier refuses a quick retype after a typo] → The wait is under a second, and the screen says so in seconds.
- [Tests depend on exact arithmetic at window edges] → Scenarios sit well inside or at the edges D-5 defines, with a manual clock; the limiter's unit tests pin the rounding.
- [The log line names a client address, which is personal data] → The operator already sees addresses in proxy logs; the line holds nothing else from the request.

## Migration & Rollback

N/A. No schema or stored data changes. Rolling back removes the limit and the 429 answer; the web client's 429 handling is then unused.
