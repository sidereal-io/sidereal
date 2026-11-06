## Context

- `server/crates/server/src/main.rs` binds `0.0.0.0:PORT` and serves with `axum::serve(listener, app)`, so no handler can see the peer's address today.
- `src/access/mod.rs` hardcodes `ARRIVED_OVER_HTTPS = false`. `cookie::session` and `cookie::cleared` already take a `secure` flag; the guard, sign-in and sign-out pass that constant.
- `src/config.rs` holds the pattern for a list setting: `allowed_origins` parses `SIDEREAL_ALLOWED_ORIGINS`, refuses a bad item at start, and keeps the value out of the message. `AllowedOrigins` is an `Arc` slice passed as middleware state.
- `src/lib.rs` builds the router in layers: `app()` (health), `app_with_database` (readiness and `/api`), then `with_web_root` adds the file services last, as a route and a fallback.
- Integration tests in `tests/access.rs` drive the router in-process with `tower::ServiceExt::oneshot`. A second harness there spawns the real binary and talks TCP, for start-up behaviour.
- No crate in the workspace parses CIDR ranges. The Nix shell pins the toolchain; `IpAddr::to_canonical` and `Ipv6Addr::to_ipv4_mapped` are stable.
- `scripts/compose-config-check.sh` renders `docker compose config --format json` without a daemon and asserts on it with `jq`.

## Goals / Non-Goals

**Goals:**
- One place decides, per request, whether the peer is a trusted proxy, and from that the scheme and the client address. Everything else reads the result.
- The result reaches every route, including the web shell files and the health probes, so the refusal in the spec holds everywhere.
- No new runtime crate.

**Non-Goals:**
- Reading the `Forwarded` header, `X-Forwarded-Host`, or the `Via` header.
- Comparing the resolved scheme in the origin check (D6).
- A rate limiter or an audit row. They read the client address later (#452, #453).
- A Compose file that runs Caddy.

## Test Seams

- **Primary test seam:** `tests/access.rs`, in-process through `oneshot`, against a router that has had `with_trusted_proxies` applied. The test inserts `ConnectInfo<SocketAddr>` into the request's extensions to play the peer, the way `into_make_service_with_connect_info` does in production. That proves the refusal and every `Secure` scenario without a socket.
- **Unit seam for the resolver:** a pure function `resolve(peer: IpAddr, headers: &HeaderMap, trusted: &TrustedProxies) -> Forwarded` in a new `src/access/forwarded.rs`, unit-tested there. Nothing in a response reflects the client address, so the seven scheme-and-client-address scenarios in the spec are asserted here, directly on the returned value.
- **Existing seams reused:** `config.rs` unit tests for parsing `SIDEREAL_TRUSTED_PROXIES`; the spawned-binary harness in `tests/access.rs` for the malformed-item exit and for one end-to-end request over TCP from `127.0.0.1` as a trusted proxy; `scripts/compose-config-check.sh` for the bind address and the pass-through; `grep` scenarios for `docs/install.md`.

## Scrutiny Tier & Blast Radius

- **Tier:** high. It is a trust boundary: the server decides whom to believe about the scheme and the client address, and the refusal touches every route.
- **Blast radius:** `server/crates/server` only: `config.rs`, `main.rs`, `lib.rs`, `access/`. Compose wiring and the install guide. No database change, no `core` or `plugin-abi` change.

## Decisions

1. **Peer address through `ConnectInfo`.** `main.rs` serves with `app.into_make_service_with_connect_info::<SocketAddr>()`. Alternatives: a custom accept loop (more code, same result), or reading the peer from a header (that is the thing being decided). A request with no `ConnectInfo` reaches the resolver only from an in-process test or a wiring mistake. The resolver then treats the peer as untrusted with the unspecified address `0.0.0.0`: forwarded headers are refused, the scheme is HTTP, and nothing is trusted by accident. It never answers 500 and never guesses loopback, because loopback may be a listed proxy.
2. **One resolver as a `layer` on the finished router.** `lib.rs` gains `pub fn with_trusted_proxies(router, TrustedProxies) -> Router`. `main.rs` calls it last, after `with_web_root`, and the test helper in `tests/access.rs` calls it on the router it builds. `Router::layer` wraps every route and the fallback present at that point, so the file services and the probes are covered; `route_layer` would miss the fallback. It inserts a `Forwarded { https: bool, client: IpAddr }` extension, or returns the 400. The `/api` origin and session layers run inside it, so the refusal wins over 403 and 401 as the spec says.
3. **The cookie reads the extension, and tolerates its absence.** `ARRIVED_OVER_HTTPS` goes. Sign-in and sign-out take `Option<Extension<Forwarded>>`; the guard reads `request.extensions()` before `next.run`, because it needs the flag after the handler returns. A missing extension means the layer was not applied, which happens only in the existing in-process tests that build `app_with_database` alone; it reads as plain HTTP, so those tests keep passing unchanged. In production the layer is always present, and the spawned-binary test proves that path.
4. **Hand-rolled CIDR matching.** `TrustedProxies` holds a `Vec<(IpAddr, u8)>`: an address with its prefix length, a bare address getting 32 or 128. Every address is canonicalised with `to_canonical` before it is stored or compared: listed items, the peer, and each `X-Forwarded-For` entry. So `::ffff:172.18.0.1` anywhere matches an IPv4 range. After canonicalisation, an address matches a range only within its own family; an IPv4 address never matches an IPv6 range, `::/0` included. The mask is built with `checked_shl` or `u32::MAX >> prefix` style arithmetic so prefix 0 and the full length produce no overflow; unit tests cover 0, 1, 31, 32 and 0, 1, 127, 128. Alternative: the `ipnet` crate. It is small and well kept, but a dependency for forty lines is not worth the review and lock churn.
5. **`X-Forwarded-For` walk, right to left, stopping at the first untrusted entry.** Entries are split on commas and trimmed. An entry that fails to parse as an address is not trusted and so becomes the client address. This is the rule Caddy, Traefik and Express's `trust proxy` share. The leftmost-wins alternative is spoofable through the proxy and was rejected in the ledger (D2).
6. **The refusal body is fixed text.** `{"error":"untrusted proxy: set SIDEREAL_TRUSTED_PROXIES"}`, status 400. It names the setting so Sam can fix it, and nothing from the request, because behind a misconfigured proxy that response can reach the internet. It goes through `AccessError`, as a new variant, to keep every refusal body in one place.
7. **`SIDEREAL_BIND_ADDRESS` in Compose only.** `compose.app.yml` publishes `"${SIDEREAL_BIND_ADDRESS:-0.0.0.0}:${SIDEREAL_PORT:-5000}:5000"`. The server keeps binding `0.0.0.0` inside the container. Reusing `SIDEREAL_PORT` for an address was rejected (D7).
8. **Compose passes `SIDEREAL_ALLOWED_ORIGINS` through too.** The brainstorm's D1 has `.env.example` mention it, but `compose.app.yml` does not forward it today, so setting it would do nothing. Passing it through, like `SIDEREAL_TRUSTED_PROXIES`, makes the note true. This was not put to the user; the review step should check it.

Nothing here is costly to reverse except the trust model itself: named proxies only, and forwarded headers from anyone else refused. That is a security policy choice, and the ADR step decides whether it amends ADR-007.

## Risks / Trade-offs

- [A working proxy setup stops working on upgrade, with a 400 on every request] → `main` has not released, so nobody upgrades into it. The 400 body names the setting, and the guide's proxy section makes it a required step.
- [Sam finds the right address only by reading docs, because nothing is logged (D4)] → The guide gives a `docker network inspect` command that prints the Compose network's gateway, and tells Sam to list that one address. It does not offer a private range: a range such as `172.16.0.0/12` would trust every container on the host, which is the ambient trust ADR-007 rules out.
- [A LAN client behind a forwarding proxy such as Squid is refused] → Accepted, as Home Assistant accepts it. Documented in the guide.
- [The resolver runs on every request, including static files] → It reads two headers and compares a few addresses. No allocation on the common path.
- [The existing `oneshot` tests send no `ConnectInfo` and build routers without the layer] → Decisions 1 and 3: a missing extension reads as plain HTTP, and a missing peer is untrusted. The `send_with` helper in `tests/access.rs` applies the layer and inserts a peer; the tests that need a trusted peer choose its address.
- [Hand-rolled CIDR has an off-by-one] → Unit tests cover prefix 0, the boundary prefix (31, 127), and the full length, for both families.

## Migration & Rollback

No schema change. Deploy: set `SIDEREAL_TRUSTED_PROXIES` in `.env` before starting a proxied instance. Rollback: revert the commits; the cookie returns to never `Secure`, and forwarded headers are ignored again.

## Open Questions

- Whether the guide's "find the gateway" command should name the Compose network by its default name (`<project>_default`). It depends on how the guide introduces project names. The tasks step can leave it to the doc task.
