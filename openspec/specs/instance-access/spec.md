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

`POST /api/session` takes the JSON body `{"username":"<u>","password":"<p>"}`. When the username exists and the password matches its hash, it SHALL create a session row and answer 200 with the body `{"username":"<u>","role":"<role>"}` and a `Set-Cookie` header for `sidereal_session` whose value is the token. The session row SHALL hold the SHA-256 hash of a new random 32-byte token, the authentication type `password`, and its created, last-used and expiry times. When the username does not exist or the password does not match, it SHALL answer 401 with the body `{"error":"invalid credentials"}` and create nothing. It SHALL verify a password hash whether or not the username exists, so the two failures take the same work. A body that is not a JSON object with string `username` and `password` SHALL answer 400 with the same body.

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

This applies to every request under `/api` whose method is not `GET`, `HEAD` or `OPTIONS`, public routes included. The server SHALL read the `Origin` header, or when there is none the origin part of the `Referer` header. When one is present and it is not an allowed origin, the server SHALL answer 403 with the body `{"error":"forbidden"}`. It SHALL do so before the session check and without reading the database. When neither header is present the request SHALL pass this check. An origin is allowed when its host and port equal the request's `Host` header, or when it is listed in `SIDEREAL_ALLOWED_ORIGINS`. Hosts compare case-insensitively. A port of 80 or 443 counts as no port, on both sides. The scheme is not compared against `Host`. The server SHALL send no `Access-Control-Allow-Origin` header.

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

`sidectl reset-admin-password` SHALL read from PostgreSQL whether an administrator exists, never from memory or a setting. When none exists the command SHALL be in the create state; otherwise in the reset state. Before it asks for anything, it SHALL print one line on stdout: `No administrator exists. Creating one.` in the create state, or `Administrator <username> exists. Setting a new password and ending every session.` in the reset state, naming the administrator.

#### Scenario: Unclaimed instance
- **WHEN** no administrator exists and Sam runs `sidectl reset-admin-password`
- **THEN** the first line on stdout is `No administrator exists. Creating one.`

#### Scenario: Claimed instance
- **WHEN** an administrator named `sam` exists and Sam runs `sidectl reset-admin-password`
- **THEN** the first line on stdout is `Administrator sam exists. Setting a new password and ending every session.`

### Requirement: The create state creates the one administrator

In the create state the command SHALL create a user with the given username, an argon2 hash of the given password and the role `admin`. It SHALL print `Created administrator <username>.` on stdout and exit 0. The claim SHALL be closed afterwards, and the administrator SHALL be able to sign in with those credentials. When an administrator appears between the state check and the write, the command SHALL write nothing and exit 1. Its one line on stderr SHALL say that an administrator now exists and to run the command again.

#### Scenario: Create from the box
- **WHEN** no administrator exists and Sam runs `sidectl reset-admin-password --username sam --password-stdin` with `correct-horse-battery` on stdin
- **THEN** the exit status is 0, stdout holds `Created administrator sam.`, `GET /api/claim` answers `{"open":false}`, and `POST /api/session` with username `sam` and that password answers 200

#### Scenario: The claim closes between the check and the write
- **WHEN** no administrator exists when the command starts, and a `POST /api/claim` creates one before the command writes
- **THEN** the exit status is 1, stderr says an administrator now exists, and the `users` table holds only the claimed administrator

### Requirement: The reset state sets a new password and ends every session

In the reset state the command SHALL replace the administrator's password hash with an argon2 hash of the given password, and delete every row in the `sessions` table. Both SHALL happen in one transaction, so no outcome has one without the other. It SHALL print `Reset the password of administrator <username> and ended <n> sessions.` on stdout, with `<n>` the number of rows deleted, and exit 0. A request that carries the cookie of a deleted session SHALL be refused as the requirement "Every API route needs an administrator session" says. This holds whether or not the server was running during the reset.

#### Scenario: Reset while signed in
- **WHEN** Sam is signed in twice and runs `sidectl reset-admin-password --password-stdin` with `new-horse-battery` on stdin
- **THEN** the exit status is 0, stdout holds `Reset the password of administrator sam and ended 2 sessions.`, the `sessions` table is empty, `GET /api/session` with either old cookie answers 401, `POST /api/session` with the old password answers 401, and `POST /api/session` with `new-horse-battery` answers 200

#### Scenario: Reset with no session
- **WHEN** an administrator exists, no session does, and Sam runs the reset with a valid password
- **THEN** the exit status is 0 and stdout holds `ended 0 sessions.`

### Requirement: Credentials come from the terminal or from the flags

Without `--password-stdin` the command SHALL prompt on the terminal. In the create state it SHALL ask `Username:` with the answer echoed, unless `--username` was given, and then `New password:` and `Repeat password:` with no echo. In the reset state it SHALL ask only the two password prompts. When the two answers differ it SHALL write nothing, print `passwords do not match` on stderr and exit 1. Prompts SHALL be written to stderr, so stdout holds only the state line, a generated password and the result. With `--password-stdin` the command SHALL never prompt. It SHALL read the first line of stdin as the password, without its line ending. In the create state the username SHALL then come from `--username`; without it, `--password-stdin` is a usage error. An empty or missing first line under `--password-stdin` SHALL be refused: one line on stderr saying a password is required, and exit 1. Without the flag, a stdin that closes before an answer to a prompt SHALL be refused: one line on stderr saying stdin closed before an answer, and exit 1. No refusal in this requirement changes the database.

#### Scenario: Script mode creates the administrator
- **WHEN** no administrator exists and a script runs `sidectl reset-admin-password --username sam --password-stdin` with `correct-horse-battery` and a line feed on stdin
- **THEN** the exit status is 0 and the stored hash matches `correct-horse-battery`, not `correct-horse-battery` followed by a line feed

#### Scenario: Script mode without a username on an unclaimed instance
- **WHEN** no administrator exists and a script runs `sidectl reset-admin-password --password-stdin`
- **THEN** the exit status is 2, stderr names `--username`, and the `users` table is empty

#### Scenario: Script mode with an empty line
- **WHEN** an administrator exists and a script runs `sidectl reset-admin-password --password-stdin` with a single line feed on stdin
- **THEN** the exit status is 1, stderr says a password is required, the password hash is unchanged and the `sessions` table is unchanged

#### Scenario: Script mode with a closed stdin
- **WHEN** an administrator exists and a script runs `sidectl reset-admin-password --password-stdin` with stdin closed and nothing on it
- **THEN** the exit status is 1, stderr says a password is required, the password hash is unchanged and the `sessions` table is unchanged

#### Scenario: A closed stdin without the flag
- **WHEN** an administrator exists and a script runs `sidectl reset-admin-password` with stdin closed, as `docker compose exec -T` with no input does
- **THEN** the exit status is 1, stdout holds only the state line, stderr says stdin closed, the password hash is unchanged and the `sessions` table is unchanged

#### Scenario: The repeated password differs
- **WHEN** Sam answers the prompts `New password:` with `correct-horse-battery` and `Repeat password:` with `correct-horse-batery`
- **THEN** stderr holds `passwords do not match`, the exit status is 1, and the `users` and `sessions` tables are unchanged

### Requirement: A given username must name the administrator in the reset state

In the reset state, a `--username` that is not the administrator's username SHALL be refused before any change. The command SHALL print one line on stderr naming the administrator and exit 1. A `--username` equal to the administrator's username SHALL be accepted.

#### Scenario: The wrong username
- **WHEN** the administrator is `sam` and a script runs `sidectl reset-admin-password --username alex --password-stdin` with a valid password
- **THEN** the exit status is 1, stderr names `sam`, the password hash is unchanged and the `sessions` table is unchanged

#### Scenario: The right username
- **WHEN** the administrator is `sam` and a script runs `sidectl reset-admin-password --username sam --password-stdin` with a valid password
- **THEN** the exit status is 0

### Requirement: An empty answer at the password prompt is generated

When the answer to `New password:` is an empty line, the command SHALL generate a random password of 24 characters drawn from ASCII letters and digits. It SHALL skip `Repeat password:`. It SHALL print `Generated password: <password>` on stdout, before the line that says what it did, and use that password. This is the only output that ever holds a password. Only the prompt on a terminal generates a password. When stdin is not a terminal, an empty answer to `New password:` SHALL be refused like an empty line under `--password-stdin`, and `--password-stdin` never generates one.

#### Scenario: Empty answer at the prompt
- **WHEN** an administrator exists and Sam answers `New password:` with an empty line
- **THEN** the command does not ask `Repeat password:`, the exit status is 0, stdout holds one line `Generated password: <p>` where `<p>` is 24 ASCII letters and digits, that password appears once in stdout and never in stderr, and `POST /api/session` with `<p>` answers 200

#### Scenario: Empty answer through a pipe
- **WHEN** an administrator exists and a script runs `sidectl reset-admin-password` without `--password-stdin`, with a single line feed piped to stdin
- **THEN** the exit status is 1, stderr says a password is required, no `Generated password:` line is printed, the password hash is unchanged and the `sessions` table is unchanged

#### Scenario: Empty answer in the create state
- **WHEN** no administrator exists and Sam answers `Username:` with `sam` and `New password:` with an empty line
- **THEN** the exit status is 0, stdout holds a `Generated password:` line and `Created administrator sam.`, and `POST /api/session` with `sam` and the generated password answers 200

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

### Requirement: The recovery command keeps secrets out of its output

No stdout or stderr line of the command SHALL hold a password it was given, a password hash, a session token or the database URL. A generated password appears exactly once, in its `Generated password:` line.

#### Scenario: A distinctive password on stdin
- **WHEN** a script runs the reset with a distinctive password on stdin against a database whose URL holds a distinctive database password
- **THEN** neither distinctive value appears in stdout or stderr, and no `$argon2` string does

### Requirement: The command line is checked before anything runs

`sidectl --help` and `sidectl reset-admin-password --help` SHALL print the usage on stdout and exit 0 without touching the database. No command, an unknown command, an unknown flag or a flag without its value SHALL print the usage on stderr and exit 2 without touching the database.

#### Scenario: Help
- **WHEN** a script runs `sidectl --help` with `DATABASE_URL` unset
- **THEN** the exit status is 0 and stdout names `reset-admin-password`, `--username` and `--password-stdin`

#### Scenario: Unknown command
- **WHEN** a script runs `sidectl rotate-keys`
- **THEN** the exit status is 2 and stderr holds the usage
