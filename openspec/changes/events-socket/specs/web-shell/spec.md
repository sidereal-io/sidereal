## ADDED Requirements

### Requirement: The signed-in view holds the events socket open

While the access section shows a signed-in user, the web shell SHALL hold one WebSocket open to `/api/events` on its own origin. It SHALL open the socket when it starts to show the signed-in view, and SHALL close it on sign-out or when it stops showing that view. It SHALL hold no socket in any other view.

#### Scenario: Signing in opens the socket

- **WHEN** the page shows the sign-in form and a sign-in succeeds
- **THEN** the page opens one WebSocket to `/api/events` on its own origin

#### Scenario: Signing out closes the socket

- **WHEN** the page shows `sam` with the socket open, and the user presses `Sign out` and `DELETE /api/session` answers 204
- **THEN** the page closes the socket and opens no other

#### Scenario: No socket while signed out

- **WHEN** `GET /api/claim` answers `{"open":false}`, `GET /api/session` answers 401, and a user opens the home page
- **THEN** the page opens no WebSocket

### Requirement: A policy close shows the sign-in form

When the server closes the socket with code 1008, the access section SHALL show the sign-in form at once, with no message, and SHALL NOT open the socket again.

#### Scenario: Signed out elsewhere

- **WHEN** the page shows `sam` and the server closes the socket with code 1008
- **THEN** the page shows the sign-in form, shows no `Sign out` button, and opens no new WebSocket

### Requirement: Any other close asks the server before it reconnects

When the socket closes with any code other than 1008, or fails to open, the access section SHALL keep showing the signed-in view. 5 seconds later it SHALL send `GET /api/session`. A 401 SHALL show the sign-in form. A 200 SHALL open the socket again. Any other answer, or a network error, SHALL wait 5 seconds and ask again.

#### Scenario: The server restarts

- **WHEN** the page shows `sam`, the socket closes with code 1006, and 5 seconds later `GET /api/session` answers 200
- **THEN** the page still shows `sam` and opens a new WebSocket to `/api/events`

#### Scenario: Signed out while disconnected

- **WHEN** the page shows `sam`, the socket closes with code 1006, and 5 seconds later `GET /api/session` answers 401
- **THEN** the page shows the sign-in form and opens no new WebSocket

#### Scenario: The database is down

- **WHEN** the page shows `sam`, the socket closes with code 1011, `GET /api/session` answers 503 after 5 seconds, and answers 200 after 10 seconds
- **THEN** the page shows `sam` throughout, and opens a new WebSocket after the 200

#### Scenario: Before the first ask

- **WHEN** the page shows `sam` and the socket closes with code 1006
- **THEN** for the next 5 seconds the page still shows `sam` and sends no `GET /api/session`

### Requirement: A silent socket counts as closed

When the open socket has delivered no message for 60 seconds, the access section SHALL close it and act as for a close with a code other than 1008.

#### Scenario: The heartbeat stops

- **WHEN** the page shows `sam`, the socket is open, and no message arrives on it for 60 seconds
- **THEN** the page closes the socket, and 5 seconds later sends `GET /api/session`

#### Scenario: Heartbeats keep the socket

- **WHEN** the page shows `sam` and a `{"type":"heartbeat"}` message arrives every 30 seconds for 3 minutes
- **THEN** the page keeps the same socket open and sends no `GET /api/session`

## MODIFIED Requirements

### Requirement: The web shell reaches the server through its own origin

In development, the web shell SHALL request the server's health at `/healthz` and the access routes and the events socket under `/api` on its own origin. The web shell's dev server SHALL forward `/healthz` and `/api` to the server at `http://localhost:5000`, WebSocket upgrades under `/api` included, and SHALL keep the browser's `Host` header. The server therefore needs no cross-origin settings for the web shell.

#### Scenario: The browser asks the web shell's origin

- **WHEN** a user opens the web shell's home page at `http://localhost:5173`
- **THEN** every health and access request the browser sends goes to `http://localhost:5173`

#### Scenario: The dev server forwards to the server

- **WHEN** the server and the web shell are both running
- **AND** a tester runs `curl -s http://localhost:5173/healthz`
- **THEN** the output is `{"status":"ok"}`

#### Scenario: The dev server forwards the access routes

- **WHEN** the server and the web shell are both running against an unclaimed database
- **AND** a tester runs `curl -s http://localhost:5173/api/claim`
- **THEN** the output is `{"open":true}`

#### Scenario: The dev server forwards the events socket

- **WHEN** the server and the web shell are both running and Sam is signed in at `http://localhost:5173`
- **AND** the page opens a WebSocket to `ws://localhost:5173/api/events`
- **THEN** the upgrade answers 101 and the socket receives `{"type":"heartbeat"}` within 35 seconds

### Requirement: The access section has unit tests

The access section SHALL have unit tests that run with no server, with `fetch` and `WebSocket` replaced by stubs, covering the unclaimed, signed-out, signed-in and unavailable states and the claim, failed sign-in and sign-out transitions. They SHALL also cover opening the socket on sign-in, closing it on sign-out, a 1008 close, a close that is not 1008 followed by a 401 and by a 200, and a silent socket. `pnpm test` in `web/` SHALL run them.

#### Scenario: The tests run with no server

- **WHEN** a contributor runs `pnpm test` in `web/` with nothing listening on the server's port
- **THEN** every access test passes

#### Scenario: A state breaks

- **WHEN** a contributor makes the signed-in state render the sign-in form and runs `pnpm test`
- **THEN** at least one access test fails

#### Scenario: The close handling breaks

- **WHEN** a contributor makes a 1008 close reopen the socket instead of showing the sign-in form, and runs `pnpm test`
- **THEN** at least one access test fails
