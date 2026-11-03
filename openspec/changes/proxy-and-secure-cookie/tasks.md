## 1. The trusted-proxy setting and the resolver

- [ ] 1.1 [medium] `config::trusted_proxies` parses `SIDEREAL_TRUSTED_PROXIES` into `TrustedProxies`: a comma-separated list of IPv4 and IPv6 addresses and CIDR ranges, canonicalised, spaces trimmed; unset or empty trusts nobody; a malformed item is `TrustedProxiesError` whose message names the variable and not the item; `contains(IpAddr)` matches only within a family and does not overflow at prefix 0, 32 or 128 (verify: `cd server && cargo test -p sidereal-server config::tests::trusted_proxies`)
- [ ] 1.2 [medium] `access::forwarded::resolve(peer, headers, &trusted) -> Forwarded { https, client }` follows the spec: an untrusted peer is HTTP with the peer as client; a trusted peer is HTTPS only for `X-Forwarded-Proto: https` ignoring case; the client is the rightmost `X-Forwarded-For` entry that is not trusted, an unparsable entry counting as untrusted, and the peer when every entry is trusted or the header is absent (verify: `cd server && cargo test -p sidereal-server forwarded::tests`)

## 2. The layer and the cookie [depends-on: 1]

- [ ] 2.1 [medium] `with_trusted_proxies(router, TrustedProxies)` layers the finished router: it inserts `Forwarded` from `ConnectInfo<SocketAddr>` (a missing `ConnectInfo` is an untrusted peer at `0.0.0.0`), and answers 400 `{"error":"untrusted proxy: set SIDEREAL_TRUSTED_PROXIES"}` with no cookie and no database read when an untrusted peer sends `X-Forwarded-For` or `X-Forwarded-Proto`, on `/api/session`, `/healthz` and an unknown path, winning over the origin check's 403; the `tests/access.rs` helper applies the layer and injects a peer (verify: `cd server && cargo test -p sidereal-server --test access untrusted_proxy`)
- [ ] 2.2 [medium] The session cookie is `Secure` when `Forwarded.https`: sign-in, the guard's refresh, the cleared stale cookie and sign-out all read it, `ARRIVED_OVER_HTTPS` is removed, and a missing extension reads as plain HTTP so the existing router tests pass unchanged (verify: `cd server && cargo test -p sidereal-server --test access secure_cookie`)
- [ ] 2.3 [easy] `main.rs` reads `SIDEREAL_TRUSTED_PROXIES` before the database, serves with `into_make_service_with_connect_info::<SocketAddr>()` and applies `with_trusted_proxies` last; the spawned binary exits non-zero on `SIDEREAL_TRUSTED_PROXIES=proxy.example` with stderr naming the variable and not the item, and with `SIDEREAL_TRUSTED_PROXIES=127.0.0.1` a sign-in over TCP with `X-Forwarded-Proto: https` sets a `Secure` cookie (verify: `cd server && cargo test -p sidereal-server --test access spawned_trusted_proxy`)

## 3. Compose and the guide [depends-on: 1]

- [ ] 3.1 [parallel] [easy] `compose.app.yml` publishes `${SIDEREAL_BIND_ADDRESS:-0.0.0.0}:${SIDEREAL_PORT:-5000}:5000` and passes `SIDEREAL_TRUSTED_PROXIES` and `SIDEREAL_ALLOWED_ORIGINS` through with no default; `.env.example` shows all three commented out; `scripts/compose-config-check.sh` asserts the default `host_ip`, `host_ip` `127.0.0.1` when set, both variables absent when unset and present when set (verify: `just check-compose`)
- [ ] 3.2 [parallel] [medium] `docs/install.md` replaces "The app has no sign-in yet" with "Claim your instance" (open the page right after the first start, or run `siderealctl reset-admin-password` first), adds "Put Sidereal behind a reverse proxy" with a Caddyfile, the required `SIDEREAL_TRUSTED_PROXIES` step with the `docker network inspect` gateway command for a same-host proxy and the machine address for a remote one, the refusal of forwarded headers from an unlisted address, `SIDEREAL_BIND_ADDRESS=127.0.0.1`, and an optional note on `SIDEREAL_ALLOWED_ORIGINS` for a proxy that rewrites `Host`; the settings table gains the new rows (verify: `grep -q '^## Claim your instance' docs/install.md && grep -qi '^## Put Sidereal behind a reverse proxy' docs/install.md && grep -q 'SIDEREAL_TRUSTED_PROXIES' docs/install.md && grep -q 'SIDEREAL_BIND_ADDRESS' docs/install.md && ! grep -qi 'no sign-in' docs/install.md`)

## 4. Integration [depends-on: 2, 3]

- [ ] 4.1 [easy] The whole gate is green (verify: `just check`)

## Deviations Ledger

