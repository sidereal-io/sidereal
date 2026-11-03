## Story

#450 proxy-and-secure-cookie — a trusted reverse proxy makes the session cookie Secure and the guide shows how. https://github.com/sidereal-io/sidereal/issues/450

## Decisions

| # | Question | Answer | Source |
|---|---|---|---|
| D1 | Do Compose and `.env.example` default `SIDEREAL_ALLOWED_ORIGINS`? | No. #448 made it optional and same-origin by default. `.env.example` mentions it only as optional, for a proxy that rewrites `Host`. The guide's "set the allowed origin" line becomes an optional note for that case. | Evidence: #450 comment (andrewduckett, 2026-10-29) |
| D2 | Does this story resolve the client address, though nothing reads it until #452 and #453? | Yes. One check of the connecting peer decides both the scheme and the client address. `X-Forwarded-For` is walked right to left, and the client address is the first entry that is not a trusted proxy. | User |
| D3 | Where does the guide's Caddy example run? | A Caddy that Sam runs: a Caddyfile snippet, a required step to put the address the app sees Caddy connect from in `SIDEREAL_TRUSTED_PROXIES`, and how to find it: the Compose network's gateway for Caddy on the same host, or the proxy machine's address. No Compose file for Caddy. | User |
| D4 | Does the server log forwarded headers from an untrusted peer, to help Sam find the right address? | No. The server logs nothing triggered by an untrusted source. The guide explains how to find the address. | User |
| D5 | Does the origin check honour `X-Forwarded-Host` from a trusted proxy? | No. Caddy keeps `Host`, and `SIDEREAL_ALLOWED_ORIGINS` covers a proxy that rewrites it. | User |
| D6 | Does the origin check now compare `Origin`'s scheme with the resolved scheme? | No. It still compares host and port only. Comparing the scheme would refuse every sign-in for Sam behind HTTPS without `SIDEREAL_TRUSTED_PROXIES`. | User |
| D7 | Can Sam close the plain-HTTP port behind a proxy on the same machine? | Yes. A new `SIDEREAL_BIND_ADDRESS`, default `0.0.0.0`, sets the host address the app port is published on. The guide says to set it to `127.0.0.1` behind a proxy on the same machine. | User |
| D8 | Does the glossary define "client address"? | Yes, under Access, as agreed and committed on this branch. | User |
| D9 | What does `SIDEREAL_TRUSTED_PROXIES` accept, and what does unset mean? | A comma-separated list of IP addresses and CIDR ranges, IPv4 and IPv6. Unset or empty trusts nobody, as Home Assistant does; Immich's default of trusting private ranges was weighed and dropped, because a trusted proxy is one named in configuration. A bad item stops the server at start, before it touches the database, with a message that names the variable and not the item. | User |
| D10 | When is the resolved scheme HTTPS? | Only when the peer is a trusted proxy and its `X-Forwarded-Proto` is `https`, ignoring case. Any other value, a list, or no header means HTTP. | User |
| D11 | Is the standard `Forwarded` header read? | No. Only `X-Forwarded-Proto` and `X-Forwarded-For`, which Caddy sends by default. | User |
| D12 | Does Compose set `SIDEREAL_TRUSTED_PROXIES`? | It passes the value from `.env` through, with no default. `.env.example` shows it commented out. | User |
| D13 | What replaces the guide's "The app has no sign-in yet" section? | "Claim your instance": open the page right after the first start, which shows `healthy` and the claim form, or run `siderealctl reset-admin-password` first. The `compose-instance` guide requirement and its `no sign-in` scenario change to match. | Evidence: `web/src/App.tsx`, `openspec/specs/compose-instance/spec.md` |
| D14 | Which existing requirements change? | `instance-access` "The session cookie is scoped to the browser" gains `Secure` over HTTPS. `compose-instance` "Only the app port is published" gains the bind address. | Evidence: `openspec/specs/instance-access/spec.md`, `openspec/specs/compose-instance/spec.md` |
| D15 | What happens to forwarded headers from a peer that is not a trusted proxy? | The request is refused on every route, as Home Assistant does: a request carrying `X-Forwarded-For` or `X-Forwarded-Proto` from an untrusted peer answers a fixed 400 that names `SIDEREAL_TRUSTED_PROXIES` and holds nothing from the request. A misconfigured proxy fails at setup instead of running quietly weaker. | User |

## Changes from the story

- **Cut**: the `SIDEREAL_ALLOWED_ORIGINS` default in `compose.app.yml` and `.env.example`, and the guide's required "set the allowed origin" step (D1). Both became optional when #448 shipped.
- **Added**: `SIDEREAL_BIND_ADDRESS`, so the plain-HTTP port can stay off the LAN behind a proxy on the same machine (D7).
- **Added**: the glossary entry **Client address** (D8).
- **Added**: forwarded headers from an untrusted peer are refused with 400, not just ignored (D15). The story said they are ignored; refusing makes a missing `SIDEREAL_TRUSTED_PROXIES` visible without a log line.

Confirmed by user: yes
