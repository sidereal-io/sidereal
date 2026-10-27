## MODIFIED Requirements

### Requirement: The web shell reaches the server through its own origin

In development, the web shell SHALL request the server's health at `/healthz` and the access routes under `/api` on its own origin. The web shell's dev server SHALL forward `/healthz` and `/api` to the server at `http://localhost:5000`. The server therefore needs no cross-origin settings for the web shell.

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

## ADDED Requirements

### Requirement: The home screen shows the access state

The home screen SHALL show, beside the health status, exactly one of: the claim form while `GET /api/claim` answers open; the sign-in form while the claim is closed and `GET /api/session` answers 401; the signed-in user's username and a button labelled `Sign out` while `GET /api/session` answers 200. Before the first answers arrive it SHALL show none of the three. Each form SHALL have a username field, a password field and one submit button, labelled `Claim` or `Sign in`. A submitted claim that succeeds SHALL show the sign-in form. A sign-in that succeeds SHALL show the username. A sign-out SHALL show the sign-in form. A claim or sign-in the server refuses SHALL show the text `Invalid username or password` for a 400 or 401 and `Something went wrong` otherwise, and SHALL NOT show any text taken from the server's response.

#### Scenario: Unclaimed instance

- **WHEN** `GET /api/claim` answers `{"open":true}` and a user opens the home page
- **THEN** the page shows a form with a `Claim` button and no `Sign in` or `Sign out` button

#### Scenario: Signed out

- **WHEN** `GET /api/claim` answers `{"open":false}`, `GET /api/session` answers 401, and a user opens the home page
- **THEN** the page shows a form with a `Sign in` button and no `Claim` button

#### Scenario: Signed in

- **WHEN** `GET /api/session` answers `{"username":"sam","role":"admin"}` and a user opens the home page
- **THEN** the page shows the text `sam` and a `Sign out` button and no form

#### Scenario: Claim succeeds

- **WHEN** the page shows the claim form and the user submits it and `POST /api/claim` answers 201
- **THEN** the page shows the sign-in form

#### Scenario: Sign-in fails

- **WHEN** the page shows the sign-in form and the user submits it and `POST /api/session` answers 401 with `{"error":"<b>nope</b>"}`
- **THEN** the page shows the text `Invalid username or password`, still shows the sign-in form, and does not contain `nope`

#### Scenario: Sign-out

- **WHEN** the page shows `sam` and the user presses `Sign out` and `DELETE /api/session` answers 204
- **THEN** the page shows the sign-in form

### Requirement: The access section uses the design tokens

The access section SHALL take every color, size, radius and font from the token file. The submit button SHALL use `violet-600` with a `white` label and `radius-4`. Each field SHALL use `radius-4`, a `border-width-1` border in `obsidian-800`, and `obsidian-900` as its background.

#### Scenario: The submit button shows the token values

- **WHEN** the page shows the claim or sign-in form
- **THEN** the submit button's computed background color is `rgb(124, 58, 237)` and its border radius is `4px`

### Requirement: The access section has unit tests

The access section SHALL have unit tests that run with no server, with `fetch` replaced by a stub, covering the unclaimed, signed-out and signed-in states and the claim, failed sign-in and sign-out transitions. `pnpm test` in `web/` SHALL run them.

#### Scenario: The tests run with no server

- **WHEN** a contributor runs `pnpm test` in `web/` with nothing listening on the server's port
- **THEN** every access test passes

#### Scenario: A state breaks

- **WHEN** a contributor makes the signed-in state render the sign-in form and runs `pnpm test`
- **THEN** at least one access test fails
