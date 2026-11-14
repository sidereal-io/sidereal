# instance-access Specification

## Purpose
Sam claims an unclaimed instance as its one administrator, signs in to get a session, learns who is signed in, and signs out, with the claim closed forever once an administrator exists.

## Requirements

### Requirement: The claim state is readable

`GET /api/claim` SHALL answer 200 with the JSON body `{"open":true}` while no administrator exists and `{"open":false}` once one does. It SHALL need no session. It SHALL read the state from PostgreSQL on every request, never from memory, and SHALL answer 503 with the body `{"error":"unavailable"}` when PostgreSQL cannot be read.

#### Scenario: Unclaimed instance
- **WHEN** no user has the `admin` role and a caller sends `GET /api/claim`
- **THEN** the status is 200 and the body is `{"open":true}`

#### Scenario: Claimed instance
- **WHEN** an administrator exists and a caller sends `GET /api/claim`
- **THEN** the status is 200 and the body is `{"open":false}`

#### Scenario: The claim stays closed after a restart
- **WHEN** an administrator was created by an earlier server process and a new process serves `GET /api/claim` against the same database
- **THEN** the body is `{"open":false}`

#### Scenario: Persistence is unavailable
- **WHEN** the server's database connections are closed and a caller sends `GET /api/claim`
- **THEN** the status is 503 and the body is `{"error":"unavailable"}`

### Requirement: The claim creates the one administrator

`POST /api/claim` with the JSON body `{"username":"<u>","password":"<p>"}` SHALL create a user with that username, an argon2 hash of the password and the role `admin`, and answer 201 with the body `{"username":"<u>","role":"admin"}` and no `Set-Cookie` header. It SHALL be permitted only while no administrator exists: once one does it SHALL answer 409 with the body `{"error":"claimed"}` and create nothing. Concurrent claims SHALL produce exactly one administrator. When PostgreSQL cannot be read or written it SHALL answer 503 with the body `{"error":"unavailable"}` and create nothing.

#### Scenario: First claim
- **WHEN** no administrator exists and Sam sends `POST /api/claim` with username `sam` and password `correct-horse-battery`
- **THEN** the status is 201, the body is `{"username":"sam","role":"admin"}`, the response has no `Set-Cookie` header, and the `users` table holds one row with username `sam`, role `admin`, and a password hash that is not the password

#### Scenario: Second claim
- **WHEN** an administrator exists and a caller sends `POST /api/claim` with any credentials
- **THEN** the status is 409, the body is `{"error":"claimed"}`, and the `users` table is unchanged

#### Scenario: Concurrent claims
- **WHEN** no administrator exists and ten callers send `POST /api/claim` at the same time with different usernames
- **THEN** exactly one answer is 201, the others are 409, and the `users` table holds exactly one row

#### Scenario: Persistence is unavailable
- **WHEN** the server's database connections are closed and a caller sends `POST /api/claim`
- **THEN** the status is 503 and the body is `{"error":"unavailable"}`

### Requirement: Claimed credentials meet the length rules

`POST /api/claim` SHALL answer 400 with the body `{"error":"invalid credentials"}` and write nothing when the request breaks a rule. The rules are: the body is a JSON object with string `username` and `password`; the username is 1 to 64 characters; the username has no leading or trailing whitespace; the password is 8 to 256 bytes.

#### Scenario: Short password
- **WHEN** no administrator exists and a caller sends `POST /api/claim` with username `sam` and password `short`
- **THEN** the status is 400, the body is `{"error":"invalid credentials"}`, and the `users` table is empty

#### Scenario: Padded username
- **WHEN** no administrator exists and a caller sends `POST /api/claim` with username ` sam` and a valid password
- **THEN** the status is 400 and the `users` table is empty

#### Scenario: Not a credentials object
- **WHEN** no administrator exists and a caller sends `POST /api/claim` with the body `["sam"]`
- **THEN** the status is 400 and the `users` table is empty

### Requirement: Sign-in creates a session

`POST /api/session` takes the JSON body `{"username":"<u>","password":"<p>"}`. When the username exists and the password matches its hash, it SHALL create a session row and answer 200 with the body `{"username":"<u>","role":"<role>"}` and a `Set-Cookie` header for `sidereal_session` whose value is the token. The session row SHALL hold the SHA-256 hash of a new random 32-byte token, the authentication type `password`, and its created, last-used and expiry times. When the username does not exist or the password does not match, it SHALL answer 401 with the body `{"error":"invalid credentials"}` and create nothing. It SHALL verify a password hash whether or not the username exists, so the two failures take the same work. A body that is not a JSON object with string `username` and `password` SHALL answer 400 with the same body. Each of these answers applies only while the request's client address is not limited; a limited address is answered 429, as the sign-in limit requirements say.

#### Scenario: Correct credentials
- **WHEN** Sam signs in with the username and password of the claim
- **THEN** the status is 200, the body is `{"username":"sam","role":"admin"}`, the response sets `sidereal_session`, and the `sessions` table holds one row whose token hash is the SHA-256 of the cookie value, with authentication type `password`

#### Scenario: Wrong password
- **WHEN** a caller signs in with username `sam` and a wrong password
- **THEN** the status is 401, the body is `{"error":"invalid credentials"}`, no cookie is set, and the `sessions` table is unchanged

#### Scenario: Unknown username
- **WHEN** a caller signs in with username `nobody` and any password
- **THEN** the status, body and headers are identical to the wrong-password answer

#### Scenario: Malformed body
- **WHEN** a caller sends `POST /api/session` with a body that is not a JSON object with string `username` and `password`
- **THEN** the status is 400 and the body is `{"error":"invalid credentials"}`

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

### Requirement: Who-am-I needs a valid session

`GET /api/session` with a `sidereal_session` cookie whose token hash matches an unexpired session row SHALL answer 200 with the body `{"username":"<u>","role":"<role>"}`. Without a cookie, with a token that matches no row, or with a token whose row has expired, it SHALL answer 401 with the body `{"error":"unauthenticated"}`. An expired row met this way SHALL be deleted.

#### Scenario: Signed in
- **WHEN** Sam sends `GET /api/session` with the cookie from sign-in
- **THEN** the status is 200 and the body is `{"username":"sam","role":"admin"}`

#### Scenario: No cookie
- **WHEN** a caller sends `GET /api/session` with no cookie
- **THEN** the status is 401 and the body is `{"error":"unauthenticated"}`

#### Scenario: Unknown token
- **WHEN** a caller sends `GET /api/session` with a `sidereal_session` cookie that matches no row
- **THEN** the status is 401 and the body is `{"error":"unauthenticated"}`

#### Scenario: Expired session
- **WHEN** a session row's expiry is set to one minute in the past and its cookie is sent to `GET /api/session`
- **THEN** the status is 401 and the row is gone from the `sessions` table

### Requirement: A session lasts 30 days from its last use

A session SHALL expire 30 days after its most recent use. A use SHALL move the expiry to at least 30 days from that use; the server MAY record a use up to one hour late. A successful `GET /api/session` SHALL set the `sidereal_session` cookie again with the same value and attributes, so the browser keeps the cookie as long as the server keeps the row. A session SHALL stay valid across a server restart, because PostgreSQL holds it.

#### Scenario: Use refreshes the expiry
- **WHEN** a session row's last-used time and expiry are set as if last used 29 days ago, and its cookie is sent to `GET /api/session`
- **THEN** the status is 200, the row's expiry is now at least 30 days away, and the response sets `sidereal_session` to the same token with `Max-Age=2592000`

#### Scenario: Valid after a restart
- **WHEN** Sam signs in, the server process stops, and a new process serves `GET /api/session` with the same cookie against the same database
- **THEN** the status is 200 and the body is `{"username":"sam","role":"admin"}`

### Requirement: Sign-out ends the session

`DELETE /api/session` with a valid `sidereal_session` cookie SHALL delete that session row, answer 204, and set `sidereal_session` to an empty value with `Max-Age=0`. Without a valid session it SHALL answer 401 with the body `{"error":"unauthenticated"}`.

#### Scenario: Sign out
- **WHEN** Sam sends `DELETE /api/session` with the cookie from sign-in
- **THEN** the status is 204, the `Set-Cookie` header sets `sidereal_session` to an empty value with `Max-Age=0`, and the `sessions` table no longer holds that row

#### Scenario: Reuse after sign-out
- **WHEN** Sam signs out and then sends `GET /api/session` with the old cookie
- **THEN** the status is 401

#### Scenario: Sign out without a session
- **WHEN** a caller sends `DELETE /api/session` with no cookie
- **THEN** the status is 401 and the body is `{"error":"unauthenticated"}`

### Requirement: The server warns on every start until the instance is claimed

After database initialization and before serving HTTP, the server SHALL write one line to stderr that starts with `instance unclaimed:` and tells Sam to open the web shell and claim the instance, when no administrator exists. It SHALL write no such line when an administrator exists.

#### Scenario: Unclaimed start
- **WHEN** the server starts against a database with no administrator
- **THEN** stderr holds exactly one line starting with `instance unclaimed:`

#### Scenario: Claimed start
- **WHEN** the server starts against a database with an administrator
- **THEN** stderr holds no line starting with `instance unclaimed:`

### Requirement: Credentials and tokens never leave the server

No response body, log line or error SHALL contain a password, a password hash or a session token hash. The session token itself SHALL appear only in a `Set-Cookie` header, and only in a response to the browser that created it at sign-in or presented it in its request.

#### Scenario: Claim and sign-in with a distinctive password
- **WHEN** Sam claims and signs in with a distinctive password
- **THEN** no response body and no captured stdout or stderr contains that password or the stored hash

#### Scenario: A token is returned only to its own browser
- **WHEN** Sam signs in and then sends `GET /api/session` with the cookie, and a second caller sends `GET /api/session` with no cookie
- **THEN** Sam's two responses set `sidereal_session` to the same token, and the second caller's response has no `Set-Cookie` header and no token in its body

### Requirement: Every API route needs an administrator session

Every request to `/api`, `/api/` or a path under `/api` SHALL need a `sidereal_session` cookie whose token hash matches an unexpired session row for a user with the `admin` role. The exceptions are `GET /api/claim`, `HEAD /api/claim`, `POST /api/claim` and `POST /api/session`. Without such a session the server SHALL answer 401 with the body `{"error":"unauthenticated"}` before any route-specific handling. An unknown `/api` path, or an unsupported method on a known one, therefore answers 401 without a session. A session whose row has been deleted SHALL be refused on its next request. `/healthz`, `/readyz`, `/` and the web shell's files SHALL need no session.

#### Scenario: A guarded route without a cookie
- **WHEN** a caller sends `DELETE /api/session` with no cookie
- **THEN** the status is 401 and the body is `{"error":"unauthenticated"}`

#### Scenario: An unknown API path without a cookie
- **WHEN** a caller sends `GET /api/does-not-exist`, `GET /api/`, `GET /api` and `PUT /api/claim`, each with no cookie
- **THEN** every status is 401 and every body is `{"error":"unauthenticated"}`

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

When the session check refuses a request that carried a `sidereal_session` cookie, the 401 response SHALL set `sidereal_session` to an empty value with `Max-Age=0`. When the refused request carried no `sidereal_session` cookie, the 401 response SHALL set no cookie. This covers only the session check's 401; a public route's own 401, such as a sign-in with wrong credentials, is unchanged.

#### Scenario: An unknown token is cleared
- **WHEN** a caller sends `GET /api/session` with a `sidereal_session` cookie that matches no row
- **THEN** the status is 401 and the `Set-Cookie` header sets `sidereal_session` to an empty value with `Max-Age=0`

#### Scenario: No cookie, no clearing
- **WHEN** a caller sends `GET /api/session` with no cookie
- **THEN** the status is 401 and the response has no `Set-Cookie` header

### Requirement: A guarded request is a use of the session

A request that passes the session check SHALL count as a use of that session. The expiry moves as the requirement "A session lasts 30 days from its last use" says. The response SHALL set the `sidereal_session` cookie again with the same value and attributes, whatever the route answers, unless the route sets that cookie itself. Sign-out does: its cleared cookie is the only `sidereal_session` cookie in its response.

#### Scenario: A guarded route refreshes the cookie
- **WHEN** Sam sends `GET /api/does-not-exist` with the cookie from sign-in
- **THEN** the status is 404 and the response sets `sidereal_session` to the same token with `Max-Age=2592000`

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

### Requirement: Extra allowed origins come from the environment

`SIDEREAL_ALLOWED_ORIGINS` is optional. When set, it SHALL hold a comma-separated list of origins, each `scheme://host` or `scheme://host:port` with a scheme of `http` or `https`, spaces around an item ignored. Unset or empty SHALL mean no extra origin. An item with a path, a trailing slash, a wildcard, another scheme, or no host SHALL stop the server at start, before it touches the database. The message SHALL name `SIDEREAL_ALLOWED_ORIGINS` and not the item's value. A listed port of 80 or 443 SHALL match an `Origin` that omits it.

#### Scenario: Unset
- **WHEN** the server starts with `SIDEREAL_ALLOWED_ORIGINS` unset
- **THEN** it starts, and only same-origin state-changing requests are allowed

#### Scenario: A list
- **WHEN** the server starts with `SIDEREAL_ALLOWED_ORIGINS=https://photos.example, http://localhost:5173`
- **THEN** it starts, and a `POST /api/session` with `Origin: http://localhost:5173` and `Host: app:5000` is not refused with 403

#### Scenario: A malformed item
- **WHEN** the server starts with `SIDEREAL_ALLOWED_ORIGINS=https://photos.example/`
- **THEN** it exits with a non-zero status before serving, and stderr names `SIDEREAL_ALLOWED_ORIGINS`

### Requirement: The recovery command follows the database's state

`siderealctl reset-admin-password` SHALL read from PostgreSQL whether an administrator exists, never from memory or a setting. When none exists the command SHALL be in the create state; otherwise in the reset state. Before it asks for anything, it SHALL print one line on stdout: `No administrator exists. Creating one.` in the create state, or `Administrator <username> exists. Setting a new password and ending every session.` in the reset state, naming the administrator.

#### Scenario: Unclaimed instance
- **WHEN** no administrator exists and Sam runs `siderealctl reset-admin-password`
- **THEN** the first line on stdout is `No administrator exists. Creating one.`

#### Scenario: Claimed instance
- **WHEN** an administrator named `sam` exists and Sam runs `siderealctl reset-admin-password`
- **THEN** the first line on stdout is `Administrator sam exists. Setting a new password and ending every session.`

### Requirement: The create state creates the one administrator

In the create state the command SHALL create a user with the given username, an argon2 hash of the given password and the role `admin`. It SHALL print `Created administrator <username>.` on stdout and exit 0. The claim SHALL be closed afterwards, and the administrator SHALL be able to sign in with those credentials. When an administrator appears between the state check and the write, the command SHALL write nothing and exit 1. Its one line on stderr SHALL say that an administrator now exists and to run the command again.

#### Scenario: Create from the box
- **WHEN** no administrator exists and Sam runs `siderealctl reset-admin-password --username sam --password-stdin` with `correct-horse-battery` on stdin
- **THEN** the exit status is 0, stdout holds `Created administrator sam.`, `GET /api/claim` answers `{"open":false}`, and `POST /api/session` with username `sam` and that password answers 200

#### Scenario: The claim closes between the check and the write
- **WHEN** no administrator exists when the command starts, and a `POST /api/claim` creates one before the command writes
- **THEN** the exit status is 1, stderr says an administrator now exists, and the `users` table holds only the claimed administrator

### Requirement: The reset state sets a new password and ends every session

In the reset state the command SHALL replace the administrator's password hash with an argon2 hash of the given password, and delete every row in the `sessions` table. Both SHALL happen in one transaction, so no outcome has one without the other. It SHALL print `Reset the password of administrator <username> and ended <n> sessions.` on stdout, with `<n>` the number of rows deleted, and exit 0. A request that carries the cookie of a deleted session SHALL be refused as the requirement "Every API route needs an administrator session" says. This holds whether or not the server was running during the reset.

#### Scenario: Reset while signed in
- **WHEN** Sam is signed in twice and runs `siderealctl reset-admin-password --password-stdin` with `new-horse-battery` on stdin
- **THEN** the exit status is 0, stdout holds `Reset the password of administrator sam and ended 2 sessions.`, the `sessions` table is empty, `GET /api/session` with either old cookie answers 401, `POST /api/session` with the old password answers 401, and `POST /api/session` with `new-horse-battery` answers 200

#### Scenario: Reset with no session
- **WHEN** an administrator exists, no session does, and Sam runs the reset with a valid password
- **THEN** the exit status is 0 and stdout holds `ended 0 sessions.`

### Requirement: Credentials come from the terminal or from the flags

Without `--password-stdin` the command SHALL prompt on the terminal. In the create state it SHALL ask `Username:` with the answer echoed, unless `--username` was given, and then `New password:` and `Repeat password:` with no echo. In the reset state it SHALL ask only the two password prompts. When the two answers differ it SHALL write nothing, print `passwords do not match` on stderr and exit 1. Prompts SHALL be written to stderr, so stdout holds only the state line, a generated password and the result. With `--password-stdin` the command SHALL never prompt. It SHALL read the first line of stdin as the password, without its line ending. In the create state the username SHALL then come from `--username`; without it, `--password-stdin` is a usage error. An empty or missing first line under `--password-stdin` SHALL be refused: one line on stderr saying a password is required, and exit 1. Without the flag, a stdin that closes before an answer to a prompt SHALL be refused: one line on stderr saying stdin closed before an answer, and exit 1. No refusal in this requirement changes the database.

#### Scenario: Script mode creates the administrator
- **WHEN** no administrator exists and a script runs `siderealctl reset-admin-password --username sam --password-stdin` with `correct-horse-battery` and a line feed on stdin
- **THEN** the exit status is 0 and the stored hash matches `correct-horse-battery`, not `correct-horse-battery` followed by a line feed

#### Scenario: Script mode without a username on an unclaimed instance
- **WHEN** no administrator exists and a script runs `siderealctl reset-admin-password --password-stdin`
- **THEN** the exit status is 2, stderr names `--username`, and the `users` table is empty

#### Scenario: Script mode with an empty line
- **WHEN** an administrator exists and a script runs `siderealctl reset-admin-password --password-stdin` with a single line feed on stdin
- **THEN** the exit status is 1, stderr says a password is required, the password hash is unchanged and the `sessions` table is unchanged

#### Scenario: Script mode with a closed stdin
- **WHEN** an administrator exists and a script runs `siderealctl reset-admin-password --password-stdin` with stdin closed and nothing on it
- **THEN** the exit status is 1, stderr says a password is required, the password hash is unchanged and the `sessions` table is unchanged

#### Scenario: A closed stdin without the flag
- **WHEN** an administrator exists and a script runs `siderealctl reset-admin-password` with stdin closed, as `docker compose exec -T` with no input does
- **THEN** the exit status is 1, stdout holds only the state line, stderr says stdin closed, the password hash is unchanged and the `sessions` table is unchanged

#### Scenario: The repeated password differs
- **WHEN** Sam answers the prompts `New password:` with `correct-horse-battery` and `Repeat password:` with `correct-horse-batery`
- **THEN** stderr holds `passwords do not match`, the exit status is 1, and the `users` and `sessions` tables are unchanged

### Requirement: A given username must name the administrator in the reset state

In the reset state, a `--username` that is not the administrator's username SHALL be refused before any change. The command SHALL print one line on stderr naming the administrator and exit 1. A `--username` equal to the administrator's username SHALL be accepted.

#### Scenario: The wrong username
- **WHEN** the administrator is `sam` and a script runs `siderealctl reset-admin-password --username alex --password-stdin` with a valid password
- **THEN** the exit status is 1, stderr names `sam`, the password hash is unchanged and the `sessions` table is unchanged

#### Scenario: The right username
- **WHEN** the administrator is `sam` and a script runs `siderealctl reset-admin-password --username sam --password-stdin` with a valid password
- **THEN** the exit status is 0

### Requirement: An empty answer at the password prompt is generated

When the answer to `New password:` is an empty line, the command SHALL generate a random password of 24 characters drawn from ASCII letters and digits. It SHALL skip `Repeat password:`. It SHALL print `Generated password: <password>` on stdout, before the line that says what it did, and use that password. This is the only output that ever holds a password. When that line cannot be written, the command SHALL exit 1 with one line on stderr that says the generated password could not be shown and to run the command again, because the password is already stored. Only the prompt on a terminal generates a password. When stdin is not a terminal, an empty answer to `New password:` SHALL be refused like an empty line under `--password-stdin`, and `--password-stdin` never generates one.

#### Scenario: Empty answer at the prompt
- **WHEN** an administrator exists and Sam answers `New password:` with an empty line
- **THEN** the command does not ask `Repeat password:`, the exit status is 0, stdout holds one line `Generated password: <p>` where `<p>` is 24 ASCII letters and digits, that password appears once in stdout and never in stderr, and `POST /api/session` with `<p>` answers 200

#### Scenario: Empty answer through a pipe
- **WHEN** an administrator exists and a script runs `siderealctl reset-admin-password` without `--password-stdin`, with a single line feed piped to stdin
- **THEN** the exit status is 1, stderr says a password is required, no `Generated password:` line is printed, the password hash is unchanged and the `sessions` table is unchanged

#### Scenario: Empty answer in the create state
- **WHEN** no administrator exists and Sam answers `Username:` with `sam` and `New password:` with an empty line
- **THEN** the exit status is 0, stdout holds a `Generated password:` line and `Created administrator sam.`, and `POST /api/session` with `sam` and the generated password answers 200

#### Scenario: The generated password cannot be shown
- **WHEN** an administrator exists, Sam answers `New password:` with an empty line, and stdout stops accepting writes after the state line, as under `| head -1`
- **THEN** the exit status is 1, stderr says the generated password could not be shown and to run the command again, and `POST /api/session` with the old password answers 401

### Requirement: The recovery command applies the claim's credential rules

The command SHALL apply the rules of the requirement "Claimed credentials meet the length rules" to a username in the create state and to a password in either state. A violation SHALL be refused before any change: one line on stderr that names the rule broken and exit 1. A generated password meets the rules by construction.

#### Scenario: Short password on stdin
- **WHEN** an administrator exists and a script runs the reset with `short` on stdin
- **THEN** the exit status is 1, stderr names the password length, the hash is unchanged and the `sessions` table is unchanged

#### Scenario: Padded username
- **WHEN** no administrator exists and a script runs the command with `--username " sam"` and a valid password
- **THEN** the exit status is 1, stderr names the username, and the `users` table is empty

### Requirement: The recovery command uses the server's database settings and never changes the schema

The command SHALL read `DATABASE_URL`, the timeout settings and the TLS settings as the server does, with the same errors for a missing or invalid value. It SHALL check that the database holds the current Sidereal schema before it reads the `users` table. It SHALL apply no migration and create no table. When the database is empty or behind, the command SHALL print one line on stderr that names the cause and says to start the server first, and exit 1. For any other database fault, such as a database that is not Sidereal's or a rejected login, it SHALL print the same line the server prints for that fault, and exit 1.

#### Scenario: An empty database
- **WHEN** `DATABASE_URL` names an empty database that no server has started against, and a script runs the command with valid credentials
- **THEN** the exit status is 1, stderr says to start the server first, and the database still holds no `_sqlx_migrations` table

#### Scenario: A database that is not Sidereal's
- **WHEN** `DATABASE_URL` names a database that holds another application's tables, and a script runs the command with valid credentials
- **THEN** the exit status is 1, stderr starts with `database is not an empty or owned Sidereal database`, and the database is unchanged

#### Scenario: Missing configuration
- **WHEN** a script runs the command with `DATABASE_URL` unset
- **THEN** the exit status is 1 and stderr starts with `missing database configuration`

#### Scenario: A host that accepts the connection and never answers
- **WHEN** `DATABASE_URL` names a host that accepts the TCP connection and sends nothing, and a script runs the command
- **THEN** within the startup timeout the exit status is 1 and stderr starts with `database unreachable`, the line the server prints for the same host

### Requirement: The recovery command keeps secrets out of its output

No stdout or stderr line of the command SHALL hold a password it was given, a password hash, a session token or the database URL. A generated password appears exactly once, in its `Generated password:` line.

#### Scenario: A distinctive password on stdin
- **WHEN** a script runs the reset with a distinctive password on stdin against a database whose URL holds a distinctive database password
- **THEN** neither distinctive value appears in stdout or stderr, and no `$argon2` string does

### Requirement: The command line is checked before anything runs

`siderealctl --help` and `siderealctl reset-admin-password --help` SHALL print the usage on stdout and exit 0 without touching the database. No command, an unknown command, an unknown flag or a flag without its value SHALL print the usage on stderr and exit 2 without touching the database.

#### Scenario: Help
- **WHEN** a script runs `siderealctl --help` with `DATABASE_URL` unset
- **THEN** the exit status is 0 and stdout names `reset-admin-password`, `--username` and `--password-stdin`

#### Scenario: Unknown command
- **WHEN** a script runs `siderealctl rotate-keys`
- **THEN** the exit status is 2 and stderr holds the usage

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

### Requirement: Failed sign-ins are counted per client address

The server SHALL count failed sign-ins for each client address. A failed sign-in SHALL be a `POST /api/session` answered 401. An answer of 400, 429 or 503 SHALL NOT count. An IPv4 client address SHALL be counted by its full address. An IPv6 client address SHALL be counted by its /64 prefix, so every address in one /64 shares one count. Every request whose client address the server could not resolve SHALL share one count, kept apart from every resolved address. The counts SHALL live only in the server process: a restart SHALL clear them, and the server SHALL write nothing about them to PostgreSQL.

#### Scenario: A malformed body is not a failure
- **WHEN** a caller at `192.168.1.20` sends 25 `POST /api/session` requests whose body is not credentials, each answered 400, and then signs in with correct credentials
- **THEN** the sign-in answers 200

#### Scenario: Two IPv4 addresses are counted apart
- **WHEN** a caller at `192.168.1.20` is limited and a caller at `192.168.1.21` signs in with correct credentials
- **THEN** the second caller's sign-in answers 200

#### Scenario: One IPv6 /64 shares a count
- **WHEN** the server started with `SIDEREAL_TRUSTED_PROXIES=172.18.0.1`, a request from `172.18.0.1` with `X-Forwarded-For: 2001:db8:1:2::1` fails sign-in, and within the same second a request from `172.18.0.1` with `X-Forwarded-For: 2001:db8:1:2::ffff` signs in with correct credentials
- **THEN** the second request answers 429

#### Scenario: Another IPv6 /64 is counted apart
- **WHEN** the server started with `SIDEREAL_TRUSTED_PROXIES=172.18.0.1`, a request from `172.18.0.1` with `X-Forwarded-For: 2001:db8:1:2::1` fails sign-in, and within the same second a request from `172.18.0.1` with `X-Forwarded-For: 2001:db8:1:3::1` signs in with correct credentials
- **THEN** the second request answers 200

#### Scenario: Unresolved addresses share one count
- **WHEN** the server started with `SIDEREAL_TRUSTED_PROXIES=172.18.0.1`, a request from `172.18.0.1` with `X-Forwarded-For: garbage` fails sign-in, and within the same second a request from `172.18.0.1` with `X-Forwarded-For: other-garbage` signs in with correct credentials
- **THEN** the second request answers 429

#### Scenario: Unresolved addresses do not limit a resolved one
- **WHEN** the server started with `SIDEREAL_TRUSTED_PROXIES=172.18.0.1`, the unresolved count is limited, and a request from `172.18.0.1` with `X-Forwarded-For: 203.0.113.9` signs in with correct credentials
- **THEN** that request answers 200

#### Scenario: A restart clears the counts
- **WHEN** a caller at `192.168.1.20` is limited and a new server process, against the same database, receives a sign-in from `192.168.1.20` with correct credentials
- **THEN** the sign-in answers 200

### Requirement: Sign-in failures are limited in three tiers

For each counted address the server SHALL apply three sliding windows: at most 1 failed sign-in in any 1 second, 5 in any 60 seconds, and 20 in any 3600 seconds. A tier is full while its window holds as many failures as its limit. While any tier is full, the address SHALL be limited. A tier SHALL have room again once its oldest counted failure is as old as its window. An attempt with well-formed credentials SHALL count from the moment the server starts to check its password, so attempts from one address that are checked at the same time count against the tiers together. Such an attempt that then answers 503 SHALL stop counting.

#### Scenario: A second attempt within the second
- **WHEN** a caller at `192.168.1.20` fails sign-in once and, within the same second, signs in with correct credentials
- **THEN** the second attempt answers 429

#### Scenario: The sixth attempt within a minute
- **WHEN** a caller at `192.168.1.20` fails sign-in 5 times, 2 seconds apart, and 2 seconds after the fifth failure signs in with correct credentials
- **THEN** the attempt answers 429 with `Retry-After: 50`

#### Scenario: The minute tier has room again
- **WHEN** a caller at `192.168.1.20` fails sign-in 5 times, 2 seconds apart, and 61 seconds after the first failure signs in with correct credentials
- **THEN** the sign-in answers 200

#### Scenario: The twenty-first attempt within an hour
- **WHEN** a caller at `192.168.1.20` fails sign-in 20 times, 15 seconds apart, and 15 seconds after the twentieth failure signs in with correct credentials
- **THEN** the attempt answers 429 with `Retry-After: 3300`

#### Scenario: Attempts at the same time are counted together
- **WHEN** a caller at `192.168.1.20` sends 10 sign-ins with a wrong password at the same moment
- **THEN** exactly one answers 401 and the other nine answer 429

#### Scenario: A known and an unknown username are limited alike
- **WHEN** a caller at `192.168.1.20` fails sign-in 5 times, 2 seconds apart, once as `sam` with a wrong password and otherwise as `nobody`, and 2 seconds later signs in as `sam` with correct credentials
- **THEN** the attempt answers 429 with `Retry-After: 50`, the same answer as when all 5 failures name `sam`

### Requirement: A limited address is answered 429 before any credential check

While a client address is limited, every `POST /api/session` from it SHALL answer 429 with the body `{"error":"too many attempts"}` and a `Retry-After` header. The server SHALL send this answer before it checks the body or a password: with correct credentials, with wrong ones and with a malformed body alike. It SHALL verify no password hash, create no session and set no cookie. `Retry-After` SHALL hold the seconds until every full tier has room again, rounded up to a whole number and at least 1. A 429 SHALL NOT count as a failure and SHALL NOT make the wait longer. Other routes SHALL be unaffected by the limit.

#### Scenario: Correct credentials from a limited address
- **WHEN** a caller at `192.168.1.20` is limited and signs in with correct credentials
- **THEN** the status is 429, the body is `{"error":"too many attempts"}`, the response has a `Retry-After` header holding a whole number of at least 1, no cookie is set, and the `sessions` table is unchanged

#### Scenario: A malformed body from a limited address
- **WHEN** a caller at `192.168.1.20` is limited and sends `POST /api/session` with a body that is not credentials
- **THEN** the status is 429 and the body is `{"error":"too many attempts"}`

#### Scenario: Refused attempts do not extend the wait
- **WHEN** a caller at `192.168.1.20` fails sign-in 5 times, 2 seconds apart, is then refused with 429 once a second for 40 seconds, and 61 seconds after the first failure signs in with correct credentials
- **THEN** the sign-in answers 200

#### Scenario: Retry-After is never zero
- **WHEN** a caller at `192.168.1.20` fails sign-in once and 999 milliseconds later sends a sign-in
- **THEN** the attempt answers 429 with `Retry-After: 1`

#### Scenario: Other routes ignore the limit
- **WHEN** a caller at `192.168.1.20` is limited and sends `GET /api/claim`
- **THEN** the status is 200

### Requirement: A successful sign-in clears its address's count

A sign-in answered 200 SHALL clear every tier's failures for its counted address. It SHALL NOT change any other address's count.

#### Scenario: Failures before a success are forgotten
- **WHEN** a caller at `192.168.1.20` fails sign-in 4 times, 2 seconds apart, signs in with correct credentials 2 seconds later, and then fails 4 more times, 2 seconds apart
- **THEN** every answer is 401 or 200, and none is 429

#### Scenario: A success does not clear another address
- **WHEN** a caller at `192.168.1.21` is limited and a caller at `192.168.1.20` signs in with correct credentials
- **THEN** the next sign-in from `192.168.1.21` still answers 429

### Requirement: The server warns when an address becomes limited

When the server answers 429 to an address whose previous sign-in attempt it did not refuse, it SHALL write one warning line to its log. The line SHALL start with `sign-in limited:`, SHALL name the client address, or say that the address is unresolved, and SHALL name the wait in seconds as `Retry-After` gives it. It SHALL NOT name the username or hold any part of the request body. Further 429s to the same address, with no attempt between them that was not refused, SHALL write nothing.

#### Scenario: One line for a run of refusals
- **WHEN** a caller at `127.0.0.1` fails sign-in as `sam` 5 times within a minute and then sends three more sign-ins as `sam`, each refused with 429
- **THEN** captured stderr holds exactly one line that starts with `sign-in limited:`, and that line contains `127.0.0.1` and does not contain `sam`

#### Scenario: A failure that fills a tier writes nothing
- **WHEN** a caller at `127.0.0.1` fails sign-in 5 times within a minute and sends nothing more
- **THEN** captured stderr holds no line that starts with `sign-in limited:`
