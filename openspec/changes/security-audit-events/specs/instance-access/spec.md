## MODIFIED Requirements

### Requirement: The server warns when an address becomes limited

When the server answers 429 to an address whose previous sign-in attempt it did not refuse, it SHALL write a failed sign-in's `security event:` line with outcome `limited`, as the `security-events` capability says: one line that starts with `security event:`, names the client address unless it is unresolved, and names the wait in seconds as `Retry-After` gives it. The line SHALL NOT name the username or hold any part of the request body. Further 429s to the same address, with no attempt between them that was not refused, SHALL write nothing. The server SHALL write no line that starts with `sign-in limited:`.

#### Scenario: One line for a run of refusals
- **WHEN** a caller at `127.0.0.1` fails sign-in as `sam` 5 times within a minute and then sends three more sign-ins as `sam`, each refused with 429
- **THEN** captured stderr holds exactly one line that holds `outcome=limited`, and that line contains `client_address=127.0.0.1` and a `retry_after` field and does not contain `sam`, and no line starts with `sign-in limited:`

#### Scenario: A failure that fills a tier writes nothing
- **WHEN** a caller at `127.0.0.1` fails sign-in 5 times within a minute and sends nothing more
- **THEN** captured stderr holds no line that holds `outcome=limited`
