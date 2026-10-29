## ADDED Requirements

### Requirement: Every API route needs an administrator session

Every request to a path under `/api` SHALL need a `sidereal_session` cookie whose token hash matches an unexpired session row for a user with the `admin` role, except `GET /api/claim`, `POST /api/claim` and `POST /api/session`. Without such a session the server SHALL answer 401 with the body `{"error":"unauthenticated"}`, before any route-specific handling, so an unknown `/api` path without a session also answers 401. A session whose row has been deleted SHALL be refused on its next request. `/healthz`, `/readyz`, `/` and the web shell's files SHALL need no session.

#### Scenario: A guarded route without a cookie
- **WHEN** a caller sends `DELETE /api/session` with no cookie
- **THEN** the status is 401 and the body is `{"error":"unauthenticated"}`

#### Scenario: An unknown API path without a cookie
- **WHEN** a caller sends `GET /api/does-not-exist` with no cookie
- **THEN** the status is 401 and the body is `{"error":"unauthenticated"}`

#### Scenario: An unknown API path with a session
- **WHEN** Sam sends `GET /api/does-not-exist` with the cookie from sign-in
- **THEN** the status is 404

#### Scenario: A session revoked mid-use
- **WHEN** Sam signs in, the session row is deleted from the `sessions` table, and Sam sends `GET /api/session` with the same cookie
- **THEN** the status is 401 and the body is `{"error":"unauthenticated"}`

#### Scenario: The public routes need no session
- **WHEN** a caller with no cookie sends `GET /api/claim`, `POST /api/claim` with valid credentials on an unclaimed instance, and `POST /api/session` with correct credentials
- **THEN** no answer is 401

#### Scenario: Health probes and the web shell stay public
- **WHEN** a caller with no cookie sends `GET /healthz`, `GET /readyz` and `GET /` to a server with a web root
- **THEN** no answer is 401

### Requirement: A refused session clears the stale cookie

When a request under `/api` carries a `sidereal_session` cookie that names no unexpired session and the server answers 401, the response SHALL set `sidereal_session` to an empty value with `Max-Age=0`. A 401 to a request that carried no `sidereal_session` cookie SHALL set no cookie.

#### Scenario: An unknown token is cleared
- **WHEN** a caller sends `GET /api/session` with a `sidereal_session` cookie that matches no row
- **THEN** the status is 401 and the `Set-Cookie` header sets `sidereal_session` to an empty value with `Max-Age=0`

#### Scenario: No cookie, no clearing
- **WHEN** a caller sends `GET /api/session` with no cookie
- **THEN** the status is 401 and the response has no `Set-Cookie` header

### Requirement: A guarded request is a use of the session

A request that passes the session check SHALL count as a use of that session: the expiry moves as the requirement "A session lasts 30 days from its last use" says, and the response SHALL set the `sidereal_session` cookie again with the same value and attributes, whatever the route answers.

#### Scenario: A guarded route refreshes the cookie
- **WHEN** Sam sends `GET /api/does-not-exist` with the cookie from sign-in
- **THEN** the status is 404 and the response sets `sidereal_session` to the same token with `Max-Age=2592000`

### Requirement: A state-changing request comes from an allowed origin

For every request under `/api` whose method is not `GET`, `HEAD` or `OPTIONS`, public routes included, the server SHALL read the `Origin` header, or when there is none the origin part of the `Referer` header. When one is present and it is not an allowed origin, the server SHALL answer 403 with the body `{"error":"forbidden"}`, before the session check and without reading the database. When neither header is present the request SHALL pass this check. An origin is allowed when its host and port equal the request's `Host` header, or when it is listed in `SIDEREAL_ALLOWED_ORIGINS`. Hosts compare case-insensitively, a default port (80 for `http`, 443 for `https`) counts as no port, and the scheme is not compared against `Host`. The server SHALL send no `Access-Control-Allow-Origin` header.

#### Scenario: Same-origin sign-in
- **WHEN** a caller sends `POST /api/session` with correct credentials, `Host: localhost:5000` and `Origin: http://localhost:5000`
- **THEN** the status is 200

#### Scenario: Same host, default port
- **WHEN** a caller sends `POST /api/session` with correct credentials, `Host: photos.example` and `Origin: https://photos.example:443`
- **THEN** the status is 200

#### Scenario: Cross-site sign-in
- **WHEN** a caller sends `POST /api/session` with correct credentials, `Host: localhost:5000` and `Origin: http://evil.example`
- **THEN** the status is 403, the body is `{"error":"forbidden"}`, no cookie is set, and the `sessions` table is unchanged

#### Scenario: Cross-site sign-out with a valid cookie
- **WHEN** Sam sends `DELETE /api/session` with the cookie from sign-in, `Host: localhost:5000` and `Origin: http://evil.example`
- **THEN** the status is 403 and the session row still exists

#### Scenario: Referer stands in for Origin
- **WHEN** a caller sends `POST /api/session` with correct credentials, `Host: localhost:5000`, no `Origin` header and `Referer: http://localhost:5000/sign-in?next=/`
- **THEN** the status is 200

#### Scenario: Mismatched Referer
- **WHEN** a caller sends `POST /api/session` with correct credentials, `Host: localhost:5000`, no `Origin` header and `Referer: http://evil.example/`
- **THEN** the status is 403

#### Scenario: Neither header
- **WHEN** a caller sends `POST /api/session` with correct credentials and neither an `Origin` nor a `Referer` header
- **THEN** the status is 200

#### Scenario: Reads are not checked
- **WHEN** a caller sends `GET /api/claim` with `Origin: http://evil.example`
- **THEN** the status is 200 and the response has no `Access-Control-Allow-Origin` header

#### Scenario: A listed origin
- **WHEN** the server started with `SIDEREAL_ALLOWED_ORIGINS=https://photos.example` and a caller sends `POST /api/session` with correct credentials, `Host: app:5000` and `Origin: https://photos.example`
- **THEN** the status is 200

### Requirement: Extra allowed origins come from the environment

`SIDEREAL_ALLOWED_ORIGINS` is optional. When set, it SHALL hold a comma-separated list of origins, each `scheme://host` or `scheme://host:port` with a scheme of `http` or `https`, spaces around an item ignored. Unset or empty SHALL mean no extra origin. An item with a path, a trailing slash, a wildcard, another scheme, or no host SHALL stop the server at start, before it touches the database, with a message that names `SIDEREAL_ALLOWED_ORIGINS` and not the item's value.

#### Scenario: Unset
- **WHEN** the server starts with `SIDEREAL_ALLOWED_ORIGINS` unset
- **THEN** it starts, and only same-origin state-changing requests are allowed

#### Scenario: A list
- **WHEN** the server starts with `SIDEREAL_ALLOWED_ORIGINS=https://photos.example, http://localhost:5173`
- **THEN** it starts, and a `POST /api/session` with `Origin: http://localhost:5173` and `Host: app:5000` is not refused with 403

#### Scenario: A malformed item
- **WHEN** the server starts with `SIDEREAL_ALLOWED_ORIGINS=https://photos.example/`
- **THEN** it exits with a non-zero status before serving, and stderr names `SIDEREAL_ALLOWED_ORIGINS`
