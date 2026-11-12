## MODIFIED Requirements

### Requirement: The home screen shows the access state

The home screen SHALL show an access section beside the health status. The section SHALL show exactly one of three views. It shows the claim form while `GET /api/claim` answers open. It shows the sign-in form while the claim is closed and `GET /api/session` answers 401. It shows the signed-in user's username and a button labelled `Sign out` while `GET /api/session` answers 200. Before the first answers arrive it SHALL show none of the three. When `GET /api/claim` or `GET /api/session` fails with a network error or a status the section does not expect, it SHALL show the text `Something went wrong` and no form. Each form SHALL have a username field, a password field and one submit button, labelled `Claim` or `Sign in`. A submitted claim that succeeds SHALL show the sign-in form. A sign-in that succeeds SHALL show the username. A sign-out SHALL show the sign-in form. A claim or sign-in the server refuses SHALL show the text `Invalid username or password` for a 400 or 401 and `Something went wrong` otherwise, except as the next sentences say for a 429, and SHALL NOT show any text taken from the server's response. A sign-in answered 429 SHALL show `Too many failed sign-ins. Try again in N seconds.` when `Retry-After` holds a whole number of seconds under 60, and `Too many failed sign-ins. Try again in N minutes.` when it holds 60 or more, with N the seconds divided by 60 and rounded up. When N is 1 the unit SHALL be `second` or `minute`. When `Retry-After` is missing or is not a whole number of seconds, it SHALL show `Too many failed sign-ins. Try again later.` After a 429 the sign-in form SHALL stay on screen and usable.

#### Scenario: Unclaimed instance

- **WHEN** `GET /api/claim` answers `{"open":true}` and a user opens the home page
- **THEN** the page shows a form with a `Claim` button and no `Sign in` or `Sign out` button

#### Scenario: Signed out

- **WHEN** `GET /api/claim` answers `{"open":false}`, `GET /api/session` answers 401, and a user opens the home page
- **THEN** the page shows a form with a `Sign in` button and no `Claim` button

#### Scenario: Signed in

- **WHEN** `GET /api/session` answers `{"username":"sam","role":"admin"}` and a user opens the home page
- **THEN** the page shows the text `sam` and a `Sign out` button and no form

#### Scenario: The server is unavailable

- **WHEN** `GET /api/claim` answers 503 and a user opens the home page
- **THEN** the page shows the text `Something went wrong` and no `Claim`, `Sign in` or `Sign out` button

#### Scenario: Claim succeeds

- **WHEN** the page shows the claim form and the user submits it and `POST /api/claim` answers 201
- **THEN** the page shows the sign-in form

#### Scenario: Sign-in fails

- **WHEN** the page shows the sign-in form and the user submits it and `POST /api/session` answers 401 with `{"error":"<b>nope</b>"}`
- **THEN** the page shows the text `Invalid username or password`, still shows the sign-in form, and does not contain `nope`

#### Scenario: Sign-in is limited for seconds

- **WHEN** the page shows the sign-in form and the user submits it and `POST /api/session` answers 429 with `Retry-After: 30` and `{"error":"<b>nope</b>"}`
- **THEN** the page shows the text `Too many failed sign-ins. Try again in 30 seconds.`, still shows the sign-in form with its `Sign in` button enabled, and does not contain `nope`

#### Scenario: Sign-in is limited for minutes

- **WHEN** the page shows the sign-in form and the user submits it and `POST /api/session` answers 429 with `Retry-After: 3300`
- **THEN** the page shows the text `Too many failed sign-ins. Try again in 55 minutes.`

#### Scenario: A wait just over a minute rounds up

- **WHEN** the page shows the sign-in form and the user submits it and `POST /api/session` answers 429 with `Retry-After: 61`
- **THEN** the page shows the text `Too many failed sign-ins. Try again in 2 minutes.`

#### Scenario: A one-second wait

- **WHEN** the page shows the sign-in form and the user submits it and `POST /api/session` answers 429 with `Retry-After: 1`
- **THEN** the page shows the text `Too many failed sign-ins. Try again in 1 second.`

#### Scenario: A limit with no usable wait

- **WHEN** the page shows the sign-in form and the user submits it and `POST /api/session` answers 429 with no `Retry-After` header
- **THEN** the page shows the text `Too many failed sign-ins. Try again later.` and still shows the sign-in form

#### Scenario: Sign-out

- **WHEN** the page shows `sam` and the user presses `Sign out` and `DELETE /api/session` answers 204
- **THEN** the page shows the sign-in form

### Requirement: The access section has unit tests

The access section SHALL have unit tests that run with no server, with `fetch` and `WebSocket` replaced by stubs, covering the unclaimed, signed-out, signed-in and unavailable states and the claim, failed sign-in, limited sign-in and sign-out transitions. The limited sign-in tests SHALL cover a wait in seconds, a wait in minutes, and a 429 with no usable `Retry-After`. They SHALL also cover the socket: opening it on sign-in, closing it on sign-out, and a silent socket. They SHALL cover a 1008 close, and a close that is not 1008 followed by a 401 and by a 200. `pnpm test` in `web/` SHALL run them.

#### Scenario: The tests run with no server

- **WHEN** a contributor runs `pnpm test` in `web/` with nothing listening on the server's port
- **THEN** every access test passes

#### Scenario: A state breaks

- **WHEN** a contributor makes the signed-in state render the sign-in form and runs `pnpm test`
- **THEN** at least one access test fails

#### Scenario: The close handling breaks

- **WHEN** a contributor makes a 1008 close reopen the socket instead of showing the sign-in form, and runs `pnpm test`
- **THEN** at least one access test fails

#### Scenario: The wait message breaks

- **WHEN** a contributor makes a 429 from `POST /api/session` show `Invalid username or password` and runs `pnpm test`
- **THEN** at least one access test fails
