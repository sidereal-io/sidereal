## MODIFIED Requirements

### Requirement: Failed sign-ins are counted per client address

The server SHALL count failed sign-ins for each client address. A failed sign-in SHALL be a `POST /api/session` answered 401. An answer of 400, 429 or 503 SHALL NOT count. An IPv4 client address SHALL be counted by its full address. An IPv6 client address SHALL be counted by its /64 prefix, so every address in one /64 shares one count. Every request whose client address the server could not resolve SHALL share one count, kept apart from every resolved address. The counts SHALL live only in the server process: a restart SHALL clear them, and the server SHALL keep no count in PostgreSQL. The `security_events` rows that the `security-events` capability keeps for failed sign-ins are a record, not a count: the limit SHALL NOT read them.

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
- **THEN** the sign-in answers 200, though `security_events` still holds the failed sign-ins from before the restart

### Requirement: The server warns when an address becomes limited

When the server answers 429 to an address whose previous sign-in attempt it did not refuse, it SHALL record a failed sign-in with outcome `limited`, as the `security-events` capability says: one line that starts with `security event:`, names the client address unless it is unresolved, and names the wait in seconds as `Retry-After` gives it. The line SHALL NOT name the username or hold any part of the request body. Further 429s to the same address, with no attempt between them that was not refused, SHALL write nothing. The server SHALL write no line that starts with `sign-in limited:`.

#### Scenario: One line for a run of refusals
- **WHEN** a caller at `127.0.0.1` fails sign-in as `sam` 5 times within a minute and then sends three more sign-ins as `sam`, each refused with 429
- **THEN** captured stderr holds exactly one line that holds `outcome=limited`, and that line contains `client_address=127.0.0.1` and a `retry_after` field and does not contain `sam`, and no line starts with `sign-in limited:`

#### Scenario: A failure that fills a tier writes nothing
- **WHEN** a caller at `127.0.0.1` fails sign-in 5 times within a minute and sends nothing more
- **THEN** captured stderr holds no line that holds `outcome=limited`
