## MODIFIED Requirements

### Requirement: The session cookie is scoped to the browser

The `sidereal_session` cookie SHALL carry the attributes `HttpOnly`, `SameSite=Lax`, `Path=/` and `Max-Age=2592000`. It SHALL carry `Secure` when and only when the request's resolved scheme is HTTPS, as the requirement "The scheme and the client address come from a trusted proxy" defines it. This applies to every `sidereal_session` cookie the server sets: after sign-in, on the refresh of a guarded request, on a cleared stale cookie, and on sign-out.

#### Scenario: Cookie attributes after sign-in
- **WHEN** Sam signs in from a peer that is not a trusted proxy, with no forwarded headers
- **THEN** the `Set-Cookie` header names `sidereal_session`, contains `HttpOnly`, `SameSite=Lax`, `Path=/` and `Max-Age=2592000`, and does not contain `Secure`

#### Scenario: Cookie is Secure behind a trusted proxy
- **WHEN** the server started with `SIDEREAL_TRUSTED_PROXIES` naming the peer, and Sam signs in with `X-Forwarded-Proto: https`
- **THEN** the `Set-Cookie` header contains `Secure`, along with `HttpOnly`, `SameSite=Lax`, `Path=/` and `Max-Age=2592000`

#### Scenario: Trusted proxy over plain HTTP
- **WHEN** the server started with `SIDEREAL_TRUSTED_PROXIES` naming the peer, and Sam signs in with `X-Forwarded-Proto: http`
- **THEN** the `Set-Cookie` header does not contain `Secure`

#### Scenario: Sign-out behind a trusted proxy
- **WHEN** the server started with `SIDEREAL_TRUSTED_PROXIES` naming the peer, and Sam sends `DELETE /api/session` with the cookie from sign-in and `X-Forwarded-Proto: https`
- **THEN** the status is 204 and the cleared `sidereal_session` cookie contains `Secure` and `Max-Age=0`

## ADDED Requirements

### Requirement: Trusted proxies come from the environment

`SIDEREAL_TRUSTED_PROXIES` is optional. When set, it SHALL hold a comma-separated list of IPv4 or IPv6 addresses and CIDR ranges, spaces around an item ignored. Unset or empty SHALL mean no peer is a trusted proxy. A peer is a trusted proxy when its TCP address equals a listed address or lies in a listed range. An item that is not an address or a CIDR range SHALL stop the server at start, before it touches the database. The message SHALL name `SIDEREAL_TRUSTED_PROXIES` and not the item's value.

#### Scenario: Unset
- **WHEN** the server starts with `SIDEREAL_TRUSTED_PROXIES` unset
- **THEN** it starts, and no peer is a trusted proxy

#### Scenario: A list
- **WHEN** the server starts with `SIDEREAL_TRUSTED_PROXIES=127.0.0.1, 172.16.0.0/12, ::1, fd00::/8`
- **THEN** it starts, and a connection from `127.0.0.1`, from `172.18.0.1` or from `::1` is from a trusted proxy, while one from `192.168.1.20` is not

#### Scenario: A malformed item
- **WHEN** the server starts with `SIDEREAL_TRUSTED_PROXIES=proxy.example`
- **THEN** it exits with a non-zero status before serving, and stderr names `SIDEREAL_TRUSTED_PROXIES` and does not contain `proxy.example`

### Requirement: The scheme and the client address come from a trusted proxy

For every request, the server SHALL resolve a scheme and a client address.

When the peer is not a trusted proxy, the scheme SHALL be HTTP and the client address SHALL be the peer's address.

When the peer is a trusted proxy, the scheme SHALL be HTTPS when `X-Forwarded-Proto` is exactly `https`, compared without regard to case. Any other value, a list of values, or no header SHALL mean HTTP.

When the peer is a trusted proxy, the server SHALL read `X-Forwarded-For` from its last entry to its first. The first entry that is not a trusted proxy SHALL be the client address. An entry that is not an address SHALL count as not trusted, and so SHALL be the client address. When every entry is a trusted proxy, or the header is absent, the client address SHALL be the peer's address.

The server SHALL NOT read the `Forwarded` header. The server SHALL NOT read `X-Forwarded-Host`: the origin check compares against `Host` alone.

#### Scenario: No proxy
- **WHEN** a request arrives from `192.168.1.20`, which is not a trusted proxy, with no forwarded headers
- **THEN** the scheme is HTTP and the client address is `192.168.1.20`

#### Scenario: One trusted proxy
- **WHEN** the server started with `SIDEREAL_TRUSTED_PROXIES=172.18.0.1` and a request arrives from `172.18.0.1` with `X-Forwarded-Proto: https` and `X-Forwarded-For: 203.0.113.9`
- **THEN** the scheme is HTTPS and the client address is `203.0.113.9`

#### Scenario: A spoofed entry behind the proxy
- **WHEN** the server started with `SIDEREAL_TRUSTED_PROXIES=172.18.0.1` and a request arrives from `172.18.0.1` with `X-Forwarded-For: 10.0.0.1, 203.0.113.9`
- **THEN** the client address is `203.0.113.9`, not `10.0.0.1`

#### Scenario: Two trusted proxies in a chain
- **WHEN** the server started with `SIDEREAL_TRUSTED_PROXIES=172.18.0.0/16` and a request arrives from `172.18.0.1` with `X-Forwarded-For: 203.0.113.9, 172.18.0.5`
- **THEN** the client address is `203.0.113.9`

#### Scenario: Every entry is trusted
- **WHEN** the server started with `SIDEREAL_TRUSTED_PROXIES=172.18.0.0/16` and a request arrives from `172.18.0.1` with `X-Forwarded-For: 172.18.0.5`
- **THEN** the client address is `172.18.0.1`

#### Scenario: Proto that is not https
- **WHEN** the server started with `SIDEREAL_TRUSTED_PROXIES=172.18.0.1` and a request arrives from `172.18.0.1`, once with `X-Forwarded-Proto: http`, once with `X-Forwarded-Proto: https, http`, and once with no `X-Forwarded-Proto`
- **THEN** the scheme is HTTP each time

#### Scenario: Case of the proto value
- **WHEN** the server started with `SIDEREAL_TRUSTED_PROXIES=172.18.0.1` and a request arrives from `172.18.0.1` with `X-Forwarded-Proto: HTTPS`
- **THEN** the scheme is HTTPS

### Requirement: Forwarded headers from an untrusted peer are refused

A request on any route, web shell files and health probes included, whose peer is not a trusted proxy and which carries an `X-Forwarded-For` or `X-Forwarded-Proto` header SHALL be answered 400 with the body `{"error":"untrusted proxy: set SIDEREAL_TRUSTED_PROXIES"}`. The server SHALL do so before the origin check and the session check, without reading the database, and without setting a cookie. The response SHALL hold nothing taken from the request. The server SHALL write nothing to its log for such a request.

#### Scenario: Forwarded headers with no trusted proxies
- **WHEN** the server started with `SIDEREAL_TRUSTED_PROXIES` unset and a caller sends `POST /api/session` with correct credentials and `X-Forwarded-Proto: https`
- **THEN** the status is 400, the body is `{"error":"untrusted proxy: set SIDEREAL_TRUSTED_PROXIES"}`, no cookie is set, and the `sessions` table is unchanged

#### Scenario: Forwarded headers from a peer outside the list
- **WHEN** the server started with `SIDEREAL_TRUSTED_PROXIES=172.18.0.1` and a caller at another address sends `GET /healthz` with `X-Forwarded-For: 203.0.113.9`
- **THEN** the status is 400 and the body is `{"error":"untrusted proxy: set SIDEREAL_TRUSTED_PROXIES"}`

#### Scenario: Refusal wins over the origin check
- **WHEN** the server started with `SIDEREAL_TRUSTED_PROXIES` unset and a caller sends `POST /api/session` with `Origin: http://evil.example` and `X-Forwarded-Proto: https`
- **THEN** the status is 400, not 403

#### Scenario: No forwarded headers, no refusal
- **WHEN** the server started with `SIDEREAL_TRUSTED_PROXIES` unset and a caller sends `GET /healthz` with no forwarded headers
- **THEN** the status is 200
