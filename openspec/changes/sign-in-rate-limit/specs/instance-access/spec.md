## MODIFIED Requirements

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

## ADDED Requirements

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

For each counted address the server SHALL apply three sliding windows: at most 1 failed sign-in in any 1 second, 5 in any 60 seconds, and 20 in any 3600 seconds. A tier is full while its window holds as many failures as its limit. While any tier is full, the address SHALL be limited. A tier SHALL have room again once its oldest counted failure is as old as its window.

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

#### Scenario: A known and an unknown username are limited alike
- **WHEN** a caller at `192.168.1.20` fails sign-in 5 times, 2 seconds apart, once as `sam` with a wrong password and otherwise as `nobody`, and 2 seconds later signs in as `sam` with correct credentials
- **THEN** the attempt answers 429 with `Retry-After: 50`, the same answer as when all 5 failures name `sam`

### Requirement: A limited address is answered 429 before any credential check

While a client address is limited, every `POST /api/session` from it SHALL answer 429 with the body `{"error":"too many attempts"}` and a `Retry-After` header. The server SHALL send this answer before it reads the credentials: with correct credentials, with wrong ones and with a malformed body alike. It SHALL verify no password hash, create no session and set no cookie. `Retry-After` SHALL hold the seconds until every full tier has room again, rounded up to a whole number and at least 1. A 429 SHALL NOT count as a failure and SHALL NOT make the wait longer. Other routes SHALL be unaffected by the limit.

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

When the server answers 429 to an address whose previous sign-in attempt it did not refuse, it SHALL write one warning line to its log. The line SHALL name the client address, or say that the address is unresolved, and SHALL name the wait in seconds as `Retry-After` gives it. It SHALL NOT name the username or hold any part of the request body. Further 429s to the same address, with no attempt between them that was not refused, SHALL write nothing.

#### Scenario: One line for a run of refusals
- **WHEN** a caller at `192.168.1.20` fails sign-in once and, within the same second, sends three more sign-ins as `sam`, each refused with 429
- **THEN** captured stderr holds exactly one warning line that contains `192.168.1.20` and does not contain `sam`

#### Scenario: A failure that fills a tier writes nothing
- **WHEN** a caller at `192.168.1.20` fails sign-in once and sends nothing more
- **THEN** captured stderr holds no warning line about a sign-in limit
