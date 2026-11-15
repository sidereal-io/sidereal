## Purpose

Sam can see in the app's normal log who claimed the instance, who signed in and out, who recovered access from the box, and who tried to sign in and failed. Every security event writes one `security event:` line, and nothing is stored in PostgreSQL.

## ADDED Requirements

### Requirement: Each security event writes one log line

Each of these acts SHALL write exactly one line that starts with `security event:`:

- an instance claim from the browser or from `siderealctl`;
- a sign-in answered 200;
- a sign-in answered 401;
- a sign-in that makes an address limited;
- a sign-out answered 204;
- a reset by `siderealctl reset-admin-password`.

The server SHALL write its lines to its stderr. `siderealctl` SHALL write its line to its own stderr, and the server SHALL also write it, as "An event from `siderealctl` reaches the server's log" says. A line SHALL be written only once the act has taken effect, and SHALL be written even when what follows the act fails. No security event SHALL write to PostgreSQL beyond the act itself.

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

#### Scenario: Nothing is stored
- **WHEN** Sam claims, signs in, fails a sign-in, signs out, and resets the password from the box
- **THEN** the database's application tables are still `sidereal_metadata`, `_sqlx_migrations`, `users` and `sessions`

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

Each `username` and `user_agent` value SHALL be written in double quotes. Inside the quotes, a backslash SHALL be written as `\\` and a double quote as `\"`. Every control character, such as a line feed, SHALL be written as `\u{<hex>}`, with its code point in lowercase hexadecimal. A line SHALL therefore hold no line break and no unescaped double quote inside a value.

#### Scenario: A line feed in a claimed username
- **WHEN** no administrator exists and a caller claims with a username that is `sam`, a line feed, and `security event: kind=sign_in`
- **THEN** captured stderr gains exactly one `security event:` line, and it holds `username="sam\u{a}security event: kind=sign_in"`

#### Scenario: A tab in the user agent
- **WHEN** a caller fails sign-in with a `User-Agent` header of `Firefox`, a tab, and `140`
- **THEN** the new line holds `user_agent="Firefox\u{9}140"`

#### Scenario: A quote and a backslash in a claimed username
- **WHEN** no administrator exists and a caller claims with username `sa"m\` and a valid password
- **THEN** the line holds `username="sa\"m\\"`

### Requirement: An event from `siderealctl` reaches the server's log

A `siderealctl` claim or reset SHALL send its line with the act, so that it is sent only when the act takes effect. A running server SHALL write each line it receives this way to its stderr, exactly once, as it writes its own lines. The server SHALL write a received line only when all of these hold, and SHALL drop any other:

- it starts with `security event: kind=instance_claim outcome=success source=siderealctl` or `security event: kind=access_recovery outcome=success source=siderealctl`;
- it holds no control character.

When no server is running, the event SHALL still appear on the command's stderr.

#### Scenario: A reset while the server runs
- **WHEN** the server is running, Sam is signed in once, and Sam runs the reset from the box with a valid password, exit 0
- **THEN** captured server stderr holds exactly one new line that starts with `security event:`, and it is `security event: kind=access_recovery outcome=success source=siderealctl username="sam" sessions_ended=1`

#### Scenario: A create from the box while the server runs
- **WHEN** the server is running, no administrator exists, and Sam runs `siderealctl reset-admin-password --username sam --password-stdin` with a valid password, exit 0
- **THEN** captured server stderr holds exactly one new line, `security event: kind=instance_claim outcome=success source=siderealctl username="sam"`

#### Scenario: A line on the channel that `siderealctl` never sends
- **WHEN** the server is running and another database session sends `security event: kind=sign_in outcome=success source=http username="sam"` on the channel
- **THEN** captured server stderr gains no `security event:` line

#### Scenario: A reset that takes no effect
- **WHEN** the server is running, an administrator exists, and Sam runs the reset from the box but the database refuses the password update, so the command exits 1
- **THEN** captured server stderr gains no `security event:` line

### Requirement: A failed sign-in names no username

A failed sign-in's line SHALL hold no username: neither the username in the request nor the username of a user it matched. Its line SHALL NOT hold any other part of the request body.

#### Scenario: A wrong password for a known user
- **WHEN** a caller signs in as `sam` with a wrong password
- **THEN** the new line holds no `username` field and does not contain `sam`

#### Scenario: An unknown username
- **WHEN** a caller signs in as `nobody-distinctive` with any password
- **THEN** the new line does not contain `nobody-distinctive`

### Requirement: An event from `siderealctl` names no client

A `siderealctl` event's line SHALL hold source `siderealctl` and SHALL hold no client address and no user agent.

#### Scenario: A reset from the box
- **WHEN** Sam runs the reset from the box with a valid password
- **THEN** the line holds `source=siderealctl` and has no `client_address` and no `user_agent` field

### Requirement: A refused request is not a security event

A request or command that the system refuses, other than a failed sign-in answered 401 or the first 429 of a run, SHALL write no `security event:` line. This covers:

- a sign-in answered 400;
- a 429 to an address whose previous attempt was also refused;
- a claim answered 400 or 409;
- a sign-out answered 401;
- any answer of 503;
- a `siderealctl` create that finds an administrator already created, exit 1.

#### Scenario: A malformed sign-in body
- **WHEN** a caller sends `POST /api/session` with a body that is not credentials, answered 400
- **THEN** stderr gains no `security event:` line

#### Scenario: A second claim
- **WHEN** an administrator exists and a caller sends `POST /api/claim` with any credentials, answered 409
- **THEN** stderr gains no `security event:` line

#### Scenario: A sign-out without a session
- **WHEN** a caller sends `DELETE /api/session` with no cookie, answered 401
- **THEN** stderr gains no `security event:` line

#### Scenario: A create from the box that loses the race
- **WHEN** no administrator exists when the command starts, and a `POST /api/claim` creates one before the command writes
- **THEN** the command's stderr holds no `security event:` line, and the server's stderr holds only the browser claim's line

### Requirement: No secret reaches a line

No `security event:` line SHALL hold a password, a password hash, a session token or a session token hash.

#### Scenario: A distinctive password
- **WHEN** Sam claims, signs in, fails a sign-in and signs out, each with a distinctive password, and then resets the password from the box with another distinctive password
- **THEN** no `security event:` line, on the server's stderr or the command's, contains either password, a `$argon2` string, or the session cookie's value

### Requirement: The user agent is kept as sent, up to 256 characters

The `user_agent` value SHALL be the request's `User-Agent` header, cut to its first 256 characters. Bytes that are not valid UTF-8 SHALL each be kept as U+FFFD. The cut and the replacement SHALL happen before the line's escaping.

#### Scenario: A long user agent
- **WHEN** a caller fails sign-in with a `User-Agent` header of 1000 `a` characters
- **THEN** the new line holds `user_agent="` followed by 256 `a` characters and `"`

#### Scenario: A user agent that is not UTF-8
- **WHEN** a caller fails sign-in with a `User-Agent` header holding the bytes `ab\xffcd`
- **THEN** the new line holds `user_agent="ab` followed by U+FFFD and `cd"`
