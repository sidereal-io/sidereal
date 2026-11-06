## Why

Sam usually runs Sidereal behind a reverse proxy that terminates TLS. The server sees only plain HTTP from that proxy, so today the session cookie is never `Secure`, and the server cannot tell the client's address from the proxy's. Rate limiting (#452) and audit events (#453) need that address. The server must know which proxies to believe before either can be trusted.

## What Changes

- New setting `SIDEREAL_TRUSTED_PROXIES`: a list of IP addresses and CIDR ranges. Unset trusts nobody, and a bad item stops the server at start (D9).
- From a trusted proxy, `X-Forwarded-Proto: https` makes the request HTTPS (D10), and `X-Forwarded-For` gives the client address (D2). The `Forwarded` header is not read (D11).
- The session cookie carries `Secure` when the request is HTTPS. This applies to sign-in, the guard's refresh, a cleared cookie, and sign-out.
- **BREAKING**: a request on any route that carries `X-Forwarded-For` or `X-Forwarded-Proto` from a peer that is not a trusted proxy is refused with a fixed 400 that names `SIDEREAL_TRUSTED_PROXIES` (D15). An existing proxy in front of Sidereal stops working until its address is listed. The server logs nothing about such requests (D4).
- The origin check is unchanged. It still compares host and port only (D6), and it does not read `X-Forwarded-Host` (D5).
- New Compose setting `SIDEREAL_BIND_ADDRESS`, default `0.0.0.0`, sets the host address the app port is published on (D7). Compose passes `SIDEREAL_TRUSTED_PROXIES` through with no default (D12). `.env.example` shows both, and mentions `SIDEREAL_ALLOWED_ORIGINS` as optional (D1).
- `docs/install.md` gains "Put Sidereal behind a reverse proxy" with a Caddy example and a required trusted-proxy step (D3). "The app has no sign-in yet" becomes "Claim your instance" (D13).
- The glossary gains **Client address** (D8). That is already committed on this branch.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `instance-access`: the session cookie carries `Secure` over HTTPS. New requirements cover the trusted-proxy setting, how the scheme and client address are resolved, and the refusal of forwarded headers from untrusted peers.
- `compose-instance`: the app port is published on `SIDEREAL_BIND_ADDRESS`. Compose passes `SIDEREAL_TRUSTED_PROXIES` through. The guide requirement replaces the "no sign-in" point with claiming the instance, and adds the reverse-proxy section.

## Impact

- **Server**:
  - `server/crates/server/src/config.rs` parses the new setting.
  - `src/main.rs` reads it at start and serves with the peer's address.
  - `src/lib.rs` and `src/access/` resolve each request and feed the cookie's `Secure` flag. This replaces the `ARRIVED_OVER_HTTPS` constant.
  - Integration tests go in `server/crates/server/tests/`.
  - No new crate is expected. No database change.
- **Compose**: `compose.app.yml`, `.env.example`, and a bind-address case in `scripts/compose-config-check.sh`.
- **Docs**: `docs/install.md`.
- **Operators**: a deployment already behind a proxy must set `SIDEREAL_TRUSTED_PROXIES` after upgrading. `main` has not released, so no released install is affected.
- **Later stories**: #452 and #453 read the resolved client address that this change provides.
