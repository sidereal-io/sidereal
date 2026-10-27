## Purpose

Sam claims an unclaimed instance as its one administrator, signs in to get a session, learns who is signed in, and signs out, with the claim closed forever once an administrator exists.

## ADDED Requirements

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

`POST /api/claim` SHALL reject, with status 400 and the body `{"error":"invalid credentials"}`, before any write: a body that is not a JSON object with string `username` and `password`; a username shorter than 1 or longer than 64 characters; a username with leading or trailing whitespace; a password shorter than 8 or longer than 256 bytes.

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

`POST /api/session` with the JSON body `{"username":"<u>","password":"<p>"}` SHALL, when the username exists and the password matches its hash, create a session row holding the SHA-256 hash of a new random 32-byte token, the authentication type `password`, and its created, last-used and expiry times, and answer 200 with the body `{"username":"<u>","role":"<role>"}` and a `Set-Cookie` header for `sidereal_session` whose value is the token. Otherwise it SHALL answer 401 with the body `{"error":"invalid credentials"}` and create nothing. It SHALL verify a password hash whether or not the username exists, so the two failures take the same work.

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

The `sidereal_session` cookie SHALL carry the attributes `HttpOnly`, `SameSite=Lax`, `Path=/` and `Max-Age=2592000`. It SHALL carry `Secure` only when the request arrived over HTTPS, which no request does while the server listens on plain TCP only.

#### Scenario: Cookie attributes after sign-in
- **WHEN** Sam signs in over the plain TCP listener
- **THEN** the `Set-Cookie` header names `sidereal_session`, contains `HttpOnly`, `SameSite=Lax`, `Path=/` and `Max-Age=2592000`, and does not contain `Secure`

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

A session SHALL expire 30 days after its most recent use. A use SHALL move the expiry to at least 30 days from that use; the server MAY record a use up to one hour late. A session SHALL stay valid across a server restart, because PostgreSQL holds it.

#### Scenario: Use refreshes the expiry
- **WHEN** a session row's last-used time and expiry are set as if last used 29 days ago, and its cookie is sent to `GET /api/session`
- **THEN** the status is 200 and the row's expiry is now at least 30 days away

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

No response body, log line or error SHALL contain a password, a password hash or a session token hash. The session token itself SHALL appear only in the `Set-Cookie` header of the sign-in response.

#### Scenario: Claim and sign-in with a distinctive password
- **WHEN** Sam claims and signs in with a distinctive password
- **THEN** no response body and no captured stdout or stderr contains that password or the stored hash
