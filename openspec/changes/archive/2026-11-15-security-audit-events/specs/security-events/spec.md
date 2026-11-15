## Purpose

Sam can see who claimed the instance, who recovered access from the box, and who tried to sign in and failed: every security event writes one log line, and the ones that matter after the fact also leave a row in PostgreSQL.

## ADDED Requirements

### Requirement: Each security event writes one log line

Each of these acts SHALL write exactly one line that starts with `security event:`:

- an instance claim from the browser or from `siderealctl`;
- a sign-in answered 200;
- a sign-in answered 401;
- a sign-in that makes an address limited;
- a sign-out answered 204;
- a reset by `siderealctl reset-admin-password`.

The server SHALL write its lines to its stderr. `siderealctl` SHALL write its line to its own stderr. A line SHALL be written only once the act has taken effect, and SHALL be written even when what follows the act fails.

#### Scenario: A claim from the browser
- **WHEN** no administrator exists and Sam sends `POST /api/claim` with username `sam` and a valid password, answered 201
- **THEN** captured server stderr holds exactly one line that starts with `security event:`

#### Scenario: A sign-in answered 200
- **WHEN** Sam signs in with correct credentials
- **THEN** captured server stderr holds exactly one new line that starts with `security event:`

#### Scenario: A sign-in answered 401
- **WHEN** a caller signs in as `sam` with a wrong password
- **THEN** captured server stderr holds exactly one new line that starts with `security event:`

#### Scenario: A sign-out answered 204
- **WHEN** Sam sends `DELETE /api/session` with the cookie from sign-in, answered 204
- **THEN** captured server stderr holds exactly one new line that starts with `security event:`

#### Scenario: A reset from the box
- **WHEN** an administrator exists and Sam runs `siderealctl reset-admin-password --password-stdin` with a valid password, exit 0
- **THEN** the command's stderr holds exactly one line that starts with `security event:`

#### Scenario: The generated password cannot be shown
- **WHEN** Sam resets the password from the box, leaves `New password:` empty, and stdout cannot be written, so the command exits 1
- **THEN** the command's stderr holds exactly one line that starts with `security event:`, because the reset took effect

### Requirement: The log line names the event in fixed fields

After `security event:`, the line SHALL hold `key=value` fields separated by one space, in this order: `kind`, `outcome`, `source`, `username`, `client_address`, `user_agent`, `sessions_ended`, `retry_after`. A field with no value for the event SHALL be left out. The values are:

- `kind`: `instance_claim`, `sign_in`, `sign_out` or `access_recovery`;
- `outcome`: `success` for every event except a failed sign-in, which is `wrong_credentials` when answered 401 and `limited` when the address becomes limited. Only a `wrong_credentials` sign-in counts toward the sign-in limit, as "Failed sign-ins are counted per client address" in `instance-access` says;
- `source`: `http` for an event from an HTTP request, `siderealctl` for one from the command;
- `username`: the user the event is about, as the requirement "A failed sign-in names no username" limits it;
- `client_address`: the request's resolved client address, as the requirement "The scheme and the client address come from a trusted proxy" in `instance-access` defines it, left out when it is unresolved;
- `user_agent`: the request's `User-Agent` header, left out when the request has none;
- `sessions_ended`: for an access recovery, the number of sessions the reset ended;
- `retry_after`: for a `limited` sign-in, the seconds of the 429's `Retry-After` header.

#### Scenario: A failed sign-in from a browser
- **WHEN** a caller at `192.168.1.20` with `User-Agent: Firefox/140` signs in as `sam` with a wrong password
- **THEN** the new line is `security event: kind=sign_in outcome=wrong_credentials source=http client_address=192.168.1.20 user_agent="Firefox/140"`

#### Scenario: A sign-in answered 200
- **WHEN** Sam, at `192.168.1.20` with no `User-Agent` header, signs in with correct credentials
- **THEN** the new line is `security event: kind=sign_in outcome=success source=http username="sam" client_address=192.168.1.20`

#### Scenario: A reset from the box
- **WHEN** Sam is signed in twice and runs the reset from the box with a valid password
- **THEN** the command's line is `security event: kind=access_recovery outcome=success source=siderealctl username="sam" sessions_ended=2`

#### Scenario: An unresolved client address
- **WHEN** the server started with `SIDEREAL_TRUSTED_PROXIES=172.18.0.1` and a request from `172.18.0.1` with `X-Forwarded-For: garbage` fails sign-in
- **THEN** the new line holds `kind=sign_in` and `outcome=wrong_credentials` and has no `client_address` field

### Requirement: Caller-supplied values cannot forge a line

Each `username` and `user_agent` value SHALL be written in double quotes. Inside the quotes, a backslash SHALL be written as `\\`, a double quote as `\"`, and every control character, such as a line feed, as `\u{<hex>}` with its code point in lowercase hexadecimal. A line SHALL therefore hold no line break and no unescaped double quote inside a value.

#### Scenario: A line feed in a claimed username
- **WHEN** no administrator exists and a caller claims with a username that is `sam`, a line feed, and `security event: kind=sign_in`
- **THEN** captured stderr gains exactly one `security event:` line, and it holds `username="sam\u{a}security event: kind=sign_in"`

#### Scenario: A tab in the user agent
- **WHEN** a caller fails sign-in with a `User-Agent` header of `Firefox`, a tab, and `140`
- **THEN** the new line holds `user_agent="Firefox\u{9}140"`

#### Scenario: A quote and a backslash in a claimed username
- **WHEN** no administrator exists and a caller claims with username `sa"m\` and a valid password
- **THEN** the line holds `username="sa\"m\\"`

### Requirement: A claim, an access recovery and a failed sign-in leave a row

An instance claim, an access recovery, and a failed sign-in with outcome `wrong_credentials` or `limited` SHALL each add exactly one row to the `security_events` table. A row SHALL hold the time of the event and the same `kind`, `outcome`, `source`, `username`, `client_address`, `user_agent` and `sessions_ended` values as its line, with no value where the line leaves a field out. A row SHALL hold no `retry_after`. The row SHALL exist when the server answers, or when `siderealctl` exits.

#### Scenario: A claim from the browser
- **WHEN** no administrator exists and Sam, at `192.168.1.20` with `User-Agent: Firefox/140`, claims with username `sam`, answered 201
- **THEN** `security_events` holds one row with kind `instance_claim`, outcome `success`, source `http`, username `sam`, client address `192.168.1.20`, user agent `Firefox/140` and no sessions ended

#### Scenario: A claim from the box
- **WHEN** no administrator exists and Sam runs `siderealctl reset-admin-password --username sam --password-stdin` with a valid password, exit 0
- **THEN** `security_events` holds one row with kind `instance_claim`, outcome `success`, source `siderealctl` and username `sam`

#### Scenario: A failed sign-in
- **WHEN** a caller at `192.168.1.20` signs in as `sam` with a wrong password, answered 401
- **THEN** `security_events` gains one row with kind `sign_in`, outcome `wrong_credentials`, source `http` and client address `192.168.1.20`

#### Scenario: An address becomes limited
- **WHEN** a caller at `127.0.0.1` fails sign-in 5 times within a minute and then sends three more sign-ins, each answered 429
- **THEN** `security_events` holds five rows with outcome `wrong_credentials` and exactly one with outcome `limited`, all with client address `127.0.0.1`

#### Scenario: Attempts at the same time
- **WHEN** a caller at `192.168.1.20` sends 10 sign-ins with a wrong password at the same moment, and one answers 401 and nine answer 429
- **THEN** `security_events` holds exactly one row with outcome `wrong_credentials` and exactly one with outcome `limited`

### Requirement: A successful sign-in and a sign-out leave no row

A sign-in answered 200 and a sign-out answered 204 SHALL write their line and SHALL add no row to `security_events`.

#### Scenario: Sign in and sign out
- **WHEN** an administrator exists, `security_events` holds its claim row, and Sam signs in with correct credentials and then signs out
- **THEN** `security_events` still holds only the claim row, and captured stderr holds two new `security event:` lines, one with `kind=sign_in` and one with `kind=sign_out`

### Requirement: A failed sign-in names no username

A failed sign-in's line and row SHALL hold no username: neither the username in the request nor the username of a user it matched. Its line and row SHALL NOT hold any other part of the request body.

#### Scenario: A wrong password for a known user
- **WHEN** a caller signs in as `sam` with a wrong password
- **THEN** the new line and the new row hold no username, and the line does not contain `sam`

#### Scenario: An unknown username
- **WHEN** a caller signs in as `nobody-distinctive` with any password
- **THEN** neither the new line nor any `security_events` row contains `nobody-distinctive`

### Requirement: An event from `siderealctl` names no client

A `siderealctl` event's line and row SHALL hold source `siderealctl` and SHALL hold no client address and no user agent.

#### Scenario: A reset from the box
- **WHEN** Sam is signed in twice and runs the reset from the box with a valid password
- **THEN** `security_events` gains one row with kind `access_recovery`, outcome `success`, source `siderealctl`, username `sam`, 2 sessions ended, and no client address or user agent

### Requirement: A claim or an access recovery needs its row

A claim and an access recovery SHALL write their row in the same transaction as the act. When the row cannot be written, the act SHALL take no effect and SHALL fail as unavailable, and no line SHALL be written. From the browser, the claim SHALL answer 503 with the body `{"error":"unavailable"}`. From `siderealctl`, the command SHALL print its database-unavailable line on stderr and exit 1.

#### Scenario: A browser claim whose row is refused
- **WHEN** no administrator exists, a trigger makes PostgreSQL refuse every insert into `security_events`, and a caller claims with valid credentials
- **THEN** the status is 503, the body is `{"error":"unavailable"}`, the `users` table is empty, and stderr holds no `security event:` line

#### Scenario: A reset whose row is refused
- **WHEN** Sam is signed in once, a trigger makes PostgreSQL refuse every insert into `security_events`, and Sam runs the reset from the box with a valid password
- **THEN** the exit status is 1, the password hash is unchanged, the `sessions` table still holds the session, and stderr holds no `security event:` line

#### Scenario: A create from the box whose row is refused
- **WHEN** no administrator exists, a trigger makes PostgreSQL refuse every insert into `security_events`, and Sam runs `siderealctl reset-admin-password --username sam --password-stdin` with a valid password
- **THEN** the exit status is 1 and the `users` table is empty

### Requirement: A failed sign-in answers the same without its row

When the row of a failed sign-in cannot be written, the server SHALL still answer as the sign-in requirements in `instance-access` say. The answer SHALL be 401 or 429 with the same body and headers, and the server SHALL still write the line.

#### Scenario: A wrong password whose row is refused
- **WHEN** a trigger makes PostgreSQL refuse every insert into `security_events` and a caller signs in as `sam` with a wrong password
- **THEN** the status is 401, the body is `{"error":"invalid credentials"}`, and stderr gains one line with `outcome=wrong_credentials`

#### Scenario: A limited address whose row is refused
- **WHEN** a trigger makes PostgreSQL refuse every insert into `security_events`, and a caller at `127.0.0.1` fails sign-in 5 times within a minute and sends one more sign-in
- **THEN** the last answer is 429 with a `Retry-After` header, and stderr gains one line with `outcome=limited`

### Requirement: A refused request is not a security event

A request or command that the system refuses, other than a failed sign-in answered 401 or the first 429 of a run, SHALL write no `security event:` line and add no row. This covers:

- a sign-in answered 400;
- a 429 to an address whose previous attempt was also refused;
- a claim answered 400 or 409;
- a sign-out answered 401;
- any answer of 503;
- a `siderealctl` create that finds an administrator already created, exit 1.

#### Scenario: A malformed sign-in body
- **WHEN** a caller sends `POST /api/session` with a body that is not credentials, answered 400
- **THEN** stderr gains no `security event:` line and `security_events` gains no row

#### Scenario: A second claim
- **WHEN** an administrator exists and a caller sends `POST /api/claim` with any credentials, answered 409
- **THEN** stderr gains no `security event:` line and `security_events` gains no row

#### Scenario: A sign-out without a session
- **WHEN** a caller sends `DELETE /api/session` with no cookie, answered 401
- **THEN** stderr gains no `security event:` line

#### Scenario: A create from the box that loses the race
- **WHEN** no administrator exists when the command starts, and a `POST /api/claim` creates one before the command writes
- **THEN** the command's stderr holds no `security event:` line, and `security_events` holds only the browser claim's row

### Requirement: No secret reaches a row or a line

No `security_events` row and no `security event:` line SHALL hold a password, a password hash, a session token or a session token hash.

#### Scenario: A distinctive password
- **WHEN** Sam claims, signs in, fails a sign-in and signs out, each with a distinctive password, and then resets the password from the box with another distinctive password
- **THEN** no `security_events` row and no `security event:` line contains either password, a `$argon2` string, or the session cookie's value

### Requirement: The user agent is kept as sent, up to 256 characters

The `user_agent` value SHALL be the request's `User-Agent` header, cut to its first 256 characters. Bytes that are not valid UTF-8 SHALL each be kept as U+FFFD. The row and the line SHALL hold the same value, before the line's escaping.

#### Scenario: A long user agent
- **WHEN** a caller fails sign-in with a `User-Agent` header of 1000 `a` characters
- **THEN** the new row's user agent is 256 `a` characters

#### Scenario: A user agent that is not UTF-8
- **WHEN** a caller fails sign-in with a `User-Agent` header holding the bytes `ab\xffcd`
- **THEN** the new row's user agent is `ab\u{fffd}cd`
