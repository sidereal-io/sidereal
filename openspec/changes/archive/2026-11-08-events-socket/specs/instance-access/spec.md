## ADDED Requirements

### Requirement: The events socket opens only with an administrator session

`GET /api/events` with a WebSocket upgrade SHALL open a socket when the request passes the session check, and SHALL answer 101. It sits behind the session check, so without an administrator session it SHALL answer 401 with the body `{"error":"unauthenticated"}` and open no socket. A request to `/api/events` that passes the session check but is not a WebSocket upgrade SHALL get a 4xx status and open no socket. The upgrade is a guarded request, so it counts as a use of the session.

#### Scenario: Upgrade with a session
- **WHEN** Sam sends a WebSocket upgrade to `GET /api/events` with the cookie from sign-in
- **THEN** the status is 101 and the socket opens

#### Scenario: Upgrade without a cookie
- **WHEN** a caller sends a WebSocket upgrade to `GET /api/events` with no cookie
- **THEN** the status is 401, the body is `{"error":"unauthenticated"}`, and no socket opens

#### Scenario: Upgrade with a revoked session
- **WHEN** Sam signs in, the session row is deleted from the `sessions` table, and Sam sends a WebSocket upgrade to `GET /api/events` with the same cookie
- **THEN** the status is 401 and no socket opens

#### Scenario: A plain request to the socket's path
- **WHEN** Sam sends `GET /api/events` with the cookie from sign-in and no upgrade headers
- **THEN** the status is between 400 and 499

### Requirement: The open socket closes when its session is gone

While the socket is open, the server SHALL check its session again every 30 seconds, and each time the client sends a text or binary message. When the session's row is gone or has expired, the server SHALL close the socket with code 1008. A revoked session therefore closes its socket within 30 seconds. The server SHALL ignore what a client message says and SHALL send no reply to it.

#### Scenario: A session revoked mid-connection
- **WHEN** Sam opens the socket, the session row is deleted from the `sessions` table, and Sam sends any text message
- **THEN** the server closes the socket with code 1008

#### Scenario: A session expired mid-connection
- **WHEN** Sam opens the socket, the session row's expiry is set to one minute in the past, and Sam sends any text message
- **THEN** the server closes the socket with code 1008

#### Scenario: Revocation with a silent client
- **WHEN** Sam opens the socket, sends nothing, and the session row is deleted from the `sessions` table
- **THEN** the server closes the socket with code 1008 within 30 seconds of the deletion

#### Scenario: Access recovery ends the socket
- **WHEN** Sam has the socket open and an operator runs `siderealctl reset-admin-password`
- **THEN** the server closes the socket with code 1008 within 30 seconds

#### Scenario: A live session keeps its socket
- **WHEN** Sam opens the socket and sends a text message
- **THEN** the socket stays open and the server sends no reply

### Requirement: A recheck is not a use of the session

A session check made while the socket is open SHALL NOT move the session's expiry. Only the upgrade and other guarded requests count as uses.

#### Scenario: An open socket does not extend its session
- **WHEN** Sam opens the socket, the session row's last-used time and expiry are then set as if last used 2 days ago, and Sam sends a text message
- **THEN** the socket stays open and the row's expiry is unchanged

### Requirement: A recheck that cannot read the database closes with 1011

When a session check made while the socket is open cannot read PostgreSQL, the server SHALL close the socket with code 1011, never 1008.

#### Scenario: Persistence is unavailable mid-connection
- **WHEN** Sam opens the socket, the server's database connections are closed, and Sam sends a text message
- **THEN** the server closes the socket with code 1011

### Requirement: The socket carries a heartbeat

While the socket is open, the server SHALL send the text message `{"type":"heartbeat"}` and a WebSocket ping every 30 seconds, after it has checked the session. The server SHALL send no other text or binary message. When the client has sent nothing for 60 seconds, a pong included, the server SHALL end the connection.

#### Scenario: A heartbeat arrives
- **WHEN** Sam opens the socket and waits
- **THEN** within 35 seconds the client receives the text message `{"type":"heartbeat"}`

#### Scenario: A silent client is dropped
- **WHEN** a client opens the socket and then sends nothing, answers no ping, and keeps its connection open
- **THEN** the server ends the connection within 70 seconds of the upgrade

#### Scenario: A client that answers pings stays
- **WHEN** a client opens the socket and answers every ping with a pong, but sends no other message, for 90 seconds
- **THEN** the socket is still open

## MODIFIED Requirements

### Requirement: A state-changing request comes from an allowed origin

This applies to every request under `/api` whose method is not `GET`, `HEAD` or `OPTIONS`, public routes included. It also applies to every WebSocket upgrade request under `/api`, though its method is `GET`: a request whose `Upgrade` header names `websocket`, compared case-insensitively. The server SHALL read the `Origin` header, or when there is none the origin part of the `Referer` header. When one is present and it is not an allowed origin, the server SHALL answer 403 with the body `{"error":"forbidden"}`. It SHALL do so before the session check and without reading the database. When neither header is present the request SHALL pass this check. An origin is allowed when its host and port equal the request's `Host` header, or when it is listed in `SIDEREAL_ALLOWED_ORIGINS`. Hosts compare case-insensitively. A port of 80 or 443 counts as no port, on both sides. The scheme is not compared against `Host`. The server SHALL send no `Access-Control-Allow-Origin` header.

#### Scenario: Same-origin sign-in
- **WHEN** a caller sends `POST /api/session` with correct credentials, `Host: localhost:5000` and `Origin: http://localhost:5000`
- **THEN** the status is 200

#### Scenario: Same host, default port
- **WHEN** a caller sends `POST /api/session` with correct credentials, once with `Host: photos.example` and `Origin: https://photos.example:443`, and once with `Host: photos.example:443` and `Origin: https://photos.example`
- **THEN** both statuses are 200

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

#### Scenario: Cross-site WebSocket upgrade
- **WHEN** Sam sends a WebSocket upgrade to `GET /api/events` with the cookie from sign-in, `Host: localhost:5000` and `Origin: http://evil.example`
- **THEN** the status is 403, the body is `{"error":"forbidden"}`, and no socket opens

#### Scenario: Same-origin WebSocket upgrade
- **WHEN** Sam sends a WebSocket upgrade to `GET /api/events` with the cookie from sign-in, `Host: localhost:5000` and `Origin: http://localhost:5000`
- **THEN** the status is 101 and the socket opens
